//! Hosted Vault credential custody authenticated by the pod's projected Kubernetes identity.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use connector_secrets::vault::HttpTransport;
use connector_secrets::{CredentialRef, Secret, SecretStore, StoreError, VaultStore};
use futures_util::StreamExt as _;
use reqwest::StatusCode;
use serde::Deserialize;
use tokio::sync::Mutex;
use url::Url;
use zeroize::Zeroizing;

use crate::hosted_config::HostedVaultConfig;

const MAX_PROJECTED_TOKEN_BYTES: u64 = 32 * 1024;
const MAX_CA_BYTES: u64 = 1024 * 1024;
const MAX_AUTH_RESPONSE_BYTES: usize = 32 * 1024;
const AUTH_REFRESH_BUFFER: Duration = Duration::from_secs(60);

/// A rotating Vault store. Kubernetes login credentials and Vault tokens never cross its API.
pub struct HostedVaultStore {
    client: reqwest::Client,
    transport: HttpTransport,
    address: String,
    login_endpoint: Url,
    mount: String,
    role: String,
    token_file: PathBuf,
    session: Mutex<Option<VaultSession>>,
}

struct VaultSession {
    token: Secret,
    refresh_at: Instant,
}

#[derive(Debug, thiserror::Error)]
pub enum HostedVaultError {
    #[error("hosted Vault configuration is invalid")]
    InvalidConfig,
    #[error("hosted Vault trust material could not be read")]
    TrustMaterial,
    #[error("hosted Vault HTTP client could not be configured")]
    HttpClient,
    #[error("hosted Vault Kubernetes identity could not be read")]
    KubernetesIdentity,
    #[error("hosted Vault authentication is unavailable")]
    AuthenticationUnavailable,
    #[error("hosted Vault refused the Kubernetes identity")]
    AuthenticationRefused,
    #[error("hosted Vault authentication response was invalid")]
    AuthenticationProtocol,
}

#[derive(Deserialize)]
struct LoginResponse<'a> {
    #[serde(borrow)]
    auth: LoginAuth<'a>,
}

#[derive(Deserialize)]
struct LoginAuth<'a> {
    #[serde(borrow)]
    client_token: &'a str,
    lease_duration: u64,
}

impl HostedVaultStore {
    pub fn new(config: &HostedVaultConfig) -> Result<Self, HostedVaultError> {
        if !config.enabled {
            return Err(HostedVaultError::InvalidConfig);
        }
        let address = config
            .address
            .as_deref()
            .ok_or(HostedVaultError::InvalidConfig)?;
        let mut origin = Url::parse(address).map_err(|_| HostedVaultError::InvalidConfig)?;
        if origin.scheme() != "https"
            || origin.host_str().is_none()
            || !origin.username().is_empty()
            || origin.password().is_some()
            || !matches!(origin.path(), "" | "/")
            || origin.query().is_some()
            || origin.fragment().is_some()
        {
            return Err(HostedVaultError::InvalidConfig);
        }
        let role = config
            .role
            .as_deref()
            .filter(|role| valid_token(role, 128))
            .ok_or(HostedVaultError::InvalidConfig)?
            .to_owned();
        if !valid_token(&config.mount, 63) {
            return Err(HostedVaultError::InvalidConfig);
        }
        let token_file = config
            .token_file
            .as_ref()
            .filter(|path| path.is_absolute())
            .ok_or(HostedVaultError::InvalidConfig)?
            .clone();
        let ca_file = config
            .ca_file
            .as_ref()
            .filter(|path| path.is_absolute())
            .ok_or(HostedVaultError::InvalidConfig)?;
        let ca =
            read_bounded(ca_file, MAX_CA_BYTES).map_err(|_| HostedVaultError::TrustMaterial)?;
        let certificate =
            reqwest::Certificate::from_pem(&ca).map_err(|_| HostedVaultError::TrustMaterial)?;
        let client = reqwest::Client::builder()
            .https_only(true)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .add_root_certificate(certificate)
            .build()
            .map_err(|_| HostedVaultError::HttpClient)?;
        origin.set_path("/v1/auth/kubernetes/login");
        Ok(Self {
            transport: HttpTransport::with_client(client.clone()),
            client,
            address: address.trim_end_matches('/').to_owned(),
            login_endpoint: origin,
            mount: config.mount.clone(),
            role,
            token_file,
            session: Mutex::new(None),
        })
    }

    /// Fail startup while Vault is sealed, unreachable, or refuses this workload identity.
    pub async fn initialize(&self) -> Result<(), HostedVaultError> {
        let _ = self.current_token().await?;
        Ok(())
    }

    async fn current_token(&self) -> Result<Secret, HostedVaultError> {
        let mut session = self.session.lock().await;
        if let Some(current) = session.as_ref() {
            if Instant::now() < current.refresh_at {
                return Ok(current.token.clone());
            }
        }
        let authenticated = self.login().await?;
        let token = authenticated.token.clone();
        *session = Some(authenticated);
        Ok(token)
    }

    async fn invalidate(&self) {
        *self.session.lock().await = None;
    }

    async fn login(&self) -> Result<VaultSession, HostedVaultError> {
        let jwt = Zeroizing::new(
            String::from_utf8(
                read_bounded(&self.token_file, MAX_PROJECTED_TOKEN_BYTES)
                    .map_err(|_| HostedVaultError::KubernetesIdentity)?,
            )
            .map_err(|_| HostedVaultError::KubernetesIdentity)?,
        );
        let jwt = jwt.trim();
        if jwt.is_empty() || jwt.len() > MAX_PROJECTED_TOKEN_BYTES as usize {
            return Err(HostedVaultError::KubernetesIdentity);
        }
        let body = Zeroizing::new(
            serde_json::to_string(&serde_json::json!({"role": self.role, "jwt": jwt}))
                .map_err(|_| HostedVaultError::AuthenticationProtocol)?,
        );
        let response = self
            .client
            .post(self.login_endpoint.clone())
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.as_str().to_owned())
            .send()
            .await
            .map_err(|_| HostedVaultError::AuthenticationUnavailable)?;
        if matches!(
            response.status(),
            StatusCode::BAD_REQUEST | StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
        ) {
            return Err(HostedVaultError::AuthenticationRefused);
        }
        if !response.status().is_success()
            || response.content_length().is_some_and(|length| {
                length > u64::try_from(MAX_AUTH_RESPONSE_BYTES).expect("bound fits u64")
            })
        {
            return Err(HostedVaultError::AuthenticationUnavailable);
        }
        let bytes = bounded_response(response).await?;
        let admitted: LoginResponse<'_> =
            serde_json::from_slice(&bytes).map_err(|_| HostedVaultError::AuthenticationProtocol)?;
        if admitted.auth.client_token.is_empty()
            || admitted.auth.client_token.len() > 4096
            || admitted.auth.lease_duration <= AUTH_REFRESH_BUFFER.as_secs()
            || admitted.auth.lease_duration > 24 * 60 * 60
        {
            return Err(HostedVaultError::AuthenticationProtocol);
        }
        Ok(VaultSession {
            token: Secret::new(admitted.auth.client_token),
            refresh_at: Instant::now()
                + Duration::from_secs(admitted.auth.lease_duration)
                    .saturating_sub(AUTH_REFRESH_BUFFER),
        })
    }

    async fn store(&self) -> Result<VaultStore<HttpTransport>, StoreError> {
        let token = self.current_token().await.map_err(auth_store_error)?;
        Ok(VaultStore::new(self.transport.clone(), &self.address, token).with_mount(&self.mount))
    }
}

#[async_trait]
impl SecretStore for HostedVaultStore {
    async fn get(&self, reference: &CredentialRef) -> Result<Secret, StoreError> {
        let result = self.store().await?.get(reference).await;
        if !matches!(result, Err(StoreError::Denied { .. })) {
            return result;
        }
        self.invalidate().await;
        self.store().await?.get(reference).await
    }

    async fn put(&self, reference: &CredentialRef, secret: &Secret) -> Result<(), StoreError> {
        let result = self.store().await?.put(reference, secret).await;
        if !matches!(result, Err(StoreError::Denied { .. })) {
            return result;
        }
        self.invalidate().await;
        self.store().await?.put(reference, secret).await
    }

    async fn delete(&self, reference: &CredentialRef) -> Result<(), StoreError> {
        let result = self.store().await?.delete(reference).await;
        if !matches!(result, Err(StoreError::Denied { .. })) {
            return result;
        }
        self.invalidate().await;
        self.store().await?.delete(reference).await
    }
}

fn auth_store_error(error: HostedVaultError) -> StoreError {
    match error {
        HostedVaultError::AuthenticationRefused => StoreError::Denied {
            path: "vault:kubernetes-login".to_owned(),
            reason: "the workload identity was refused".to_owned(),
        },
        _ => StoreError::Unreachable {
            path: "vault:kubernetes-login".to_owned(),
            reason: "a usable Vault session could not be obtained".to_owned(),
        },
    }
}

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, std::io::Error> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err(std::io::Error::other("bounded regular file required"));
    }
    let bytes = fs::read(path)?;
    if bytes.len() as u64 > limit {
        return Err(std::io::Error::other("file exceeded bound"));
    }
    Ok(bytes)
}

async fn bounded_response(
    response: reqwest::Response,
) -> Result<Zeroizing<Vec<u8>>, HostedVaultError> {
    let mut bytes = Zeroizing::new(Vec::new());
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| HostedVaultError::AuthenticationUnavailable)?;
        if bytes.len().saturating_add(chunk.len()) > MAX_AUTH_RESPONSE_BYTES {
            return Err(HostedVaultError::AuthenticationProtocol);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

fn valid_token(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vault_origin_and_role_are_closed() {
        let mut config = HostedVaultConfig {
            enabled: true,
            address: Some("http://vault.example.test:8200".to_owned()),
            mount: "b10x-connectors".to_owned(),
            role: Some("b10x-connectors".to_owned()),
            token_file: Some(PathBuf::from("/var/run/secrets/token")),
            ca_file: Some(PathBuf::from("/var/run/secrets/ca.crt")),
        };
        assert!(matches!(
            HostedVaultStore::new(&config),
            Err(HostedVaultError::InvalidConfig)
        ));
        config.address = Some("https://vault.example.test:8200/path".to_owned());
        assert!(matches!(
            HostedVaultStore::new(&config),
            Err(HostedVaultError::InvalidConfig)
        ));
    }
}

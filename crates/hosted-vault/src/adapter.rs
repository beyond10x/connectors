//! Hosted Vault credential custody authenticated by the pod's projected Kubernetes identity.

use std::fs::OpenOptions;
use std::io::Read as _;
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
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

use connectors_config::HostedVaultConfig;

const MAX_PROJECTED_TOKEN_BYTES: u64 = 32 * 1024;
const MAX_CA_BYTES: u64 = 1024 * 1024;
const MAX_AUTH_RESPONSE_BYTES: usize = 32 * 1024;
const AUTH_REFRESH_BUFFER: Duration = Duration::from_secs(60);

/// A rotating Vault store. Kubernetes login credentials and Vault tokens never cross its API.
pub struct HostedVaultStore {
    client: reqwest::Client,
    transport: HttpTransport,
    address: String,
    health: Arc<dyn VaultHealthProbe>,
    session_probe: Arc<dyn VaultSessionProbe>,
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

#[async_trait]
trait VaultHealthProbe: Send + Sync {
    async fn ready(&self) -> Result<(), HostedVaultError>;
}

#[async_trait]
trait VaultSessionProbe: Send + Sync {
    async fn ready(&self, token: &Secret) -> Result<(), StoreError>;
}

struct HttpVaultHealthProbe {
    client: reqwest::Client,
    endpoint: Url,
}

#[async_trait]
impl VaultHealthProbe for HttpVaultHealthProbe {
    async fn ready(&self) -> Result<(), HostedVaultError> {
        let response = self
            .client
            .get(self.endpoint.clone())
            .send()
            .await
            .map_err(|_| HostedVaultError::AuthenticationUnavailable)?;
        health_status(response.status())
    }
}

struct StoredVaultSessionProbe {
    transport: HttpTransport,
    address: String,
    mount: String,
}

#[async_trait]
impl VaultSessionProbe for StoredVaultSessionProbe {
    async fn ready(&self, token: &Secret) -> Result<(), StoreError> {
        VaultStore::new(self.transport.clone(), &self.address, token.clone())
            .with_mount(&self.mount)
            .ready()
            .await
    }
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
        let mut health_endpoint = origin.clone();
        health_endpoint.set_path("/v1/sys/health");
        health_endpoint.set_query(Some("standbyok=true&perfstandbyok=true"));
        origin.set_path("/v1/auth/kubernetes/login");
        let transport = HttpTransport::with_client(client.clone());
        let address = address.trim_end_matches('/').to_owned();
        let mount = config.mount.clone();
        Ok(Self {
            transport: transport.clone(),
            health: Arc::new(HttpVaultHealthProbe {
                client: client.clone(),
                endpoint: health_endpoint,
            }),
            session_probe: Arc::new(StoredVaultSessionProbe {
                transport,
                address: address.clone(),
                mount: mount.clone(),
            }),
            client,
            address,
            login_endpoint: origin,
            mount,
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

    /// Prove Vault reachability and retain a usable workload-authenticated session without naming
    /// or reading any credential address or value. The bounded, zeroizing token-self response is
    /// neither decoded nor exposed.
    pub async fn ready(&self) -> Result<(), HostedVaultError> {
        self.health.ready().await?;
        self.authenticated_ready()
            .await
            .map_err(hosted_readiness_error)
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

    async fn authenticated_ready(&self) -> Result<(), StoreError> {
        let token = self.current_token().await.map_err(auth_store_error)?;
        let result = self.session_probe.ready(&token).await;
        if !matches!(result, Err(StoreError::Denied { .. })) {
            return result;
        }
        self.invalidate().await;
        let token = self.current_token().await.map_err(auth_store_error)?;
        self.session_probe.ready(&token).await
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
    async fn ready(&self) -> Result<(), StoreError> {
        HostedVaultStore::ready(self)
            .await
            .map_err(auth_store_error)
    }

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

fn hosted_readiness_error(error: StoreError) -> HostedVaultError {
    match error {
        StoreError::Denied { .. } => HostedVaultError::AuthenticationRefused,
        StoreError::NotFound { .. }
        | StoreError::Unreachable { .. }
        | StoreError::Backend { .. }
        | StoreError::Layout { .. }
        | StoreError::Conflict { .. }
        | StoreError::Unsupported { .. } => HostedVaultError::AuthenticationUnavailable,
    }
}

fn health_status(status: StatusCode) -> Result<(), HostedVaultError> {
    if status.is_success() {
        Ok(())
    } else {
        Err(HostedVaultError::AuthenticationUnavailable)
    }
}

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, std::io::Error> {
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(rustix::fs::OFlags::NONBLOCK.bits() as i32)
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err(std::io::Error::other("bounded regular file required"));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    (&mut file).take(limit + 1).read_to_end(&mut bytes)?;
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
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    struct FixedHealth(bool);

    #[async_trait]
    impl VaultHealthProbe for FixedHealth {
        async fn ready(&self) -> Result<(), HostedVaultError> {
            if self.0 {
                Ok(())
            } else {
                Err(HostedVaultError::AuthenticationUnavailable)
            }
        }
    }

    struct FixedSessionProbe {
        ready: bool,
        calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl VaultSessionProbe for FixedSessionProbe {
        async fn ready(&self, _token: &Secret) -> Result<(), StoreError> {
            self.calls.fetch_add(1, Ordering::AcqRel);
            if self.ready {
                Ok(())
            } else {
                Err(StoreError::Denied {
                    path: "vault:auth/token/lookup-self".to_owned(),
                    reason: "SENTINEL-NOT-A-REAL-SECRET".to_owned(),
                })
            }
        }
    }

    fn store_with_readiness(
        health_ready: bool,
        session_ready: bool,
    ) -> (HostedVaultStore, Arc<AtomicUsize>) {
        let client = reqwest::Client::builder()
            .https_only(true)
            .no_proxy()
            .build()
            .unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let store = HostedVaultStore {
            transport: HttpTransport::with_client(client.clone()),
            client,
            address: "https://vault.example.test:8200".to_owned(),
            health: Arc::new(FixedHealth(health_ready)),
            session_probe: Arc::new(FixedSessionProbe {
                ready: session_ready,
                calls: Arc::clone(&calls),
            }),
            login_endpoint: Url::parse("https://vault.example.test:8200/v1/auth/kubernetes/login")
                .unwrap(),
            mount: "b10x-connectors".to_owned(),
            role: "b10x-connectors".to_owned(),
            token_file: PathBuf::from("/unread/test-token"),
            session: Mutex::new(Some(VaultSession {
                token: Secret::new("SENTINEL-NOT-A-REAL-SECRET"),
                refresh_at: Instant::now() + Duration::from_secs(300),
            })),
        };
        (store, calls)
    }

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

    #[tokio::test]
    async fn readiness_requires_health_and_an_accepted_session_without_reading_a_credential() {
        let (ready, ready_calls) = store_with_readiness(true, true);
        ready.ready().await.unwrap();
        assert_eq!(ready_calls.load(Ordering::Acquire), 1);

        let (unhealthy, unhealthy_calls) = store_with_readiness(false, true);
        assert!(matches!(
            unhealthy.ready().await,
            Err(HostedVaultError::AuthenticationUnavailable)
        ));
        assert_eq!(unhealthy_calls.load(Ordering::Acquire), 0);

        let (revoked, revoked_calls) = store_with_readiness(true, false);
        let error = revoked.ready().await.unwrap_err();
        assert!(matches!(error, HostedVaultError::AuthenticationUnavailable));
        assert!(!error.to_string().contains("SENTINEL"));
        assert_eq!(revoked_calls.load(Ordering::Acquire), 1);
        assert!(revoked.session.lock().await.is_none());
    }

    #[test]
    fn only_healthy_vault_status_is_ready() {
        health_status(StatusCode::OK).unwrap();
        assert!(matches!(
            health_status(StatusCode::SERVICE_UNAVAILABLE),
            Err(HostedVaultError::AuthenticationUnavailable)
        ));
    }
}

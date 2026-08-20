//! Actual SIP/RTVBP session launcher and deployment-owned TLS connector.

use std::fs;
use std::io::Read as _;
use std::net::SocketAddr;
use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _};
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use connector_secrets::{CredentialRef, SecretStore, StoreError};
use ed25519_dalek::SigningKey;
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, RootCertStore};
use service::authority::AuthorityIssuer;
use service::{AdmittedVoicePlan, CredentialSet, SensitiveValue, VoiceApplicationRoute};
use tokio::net::TcpStream;
use tokio::sync::watch;
use tokio_rustls::TlsConnector;
use voice_runtime::{
    dial_establishment_channel, ApplicationConnector, ApplicationStream, CredentialSource,
    DependencyError, OsSessionMaterial, RuntimeConfig, SystemClock, VoiceRuntime,
    VoiceSessionControl,
};

use connectors_config::{AuthorityConfig, HostedSipCredentialConfig};

use crate::backend::{LaunchError, LaunchedSession, SessionLauncher};

/// Runtime launcher backed by the pinned sipx driver and RTVBP endpoint.
pub struct RuntimeLauncher {
    issuer: Arc<AuthorityIssuer>,
    application: Arc<TlsApplicationConnector>,
    credentials: Arc<dyn CredentialSource>,
}

impl RuntimeLauncher {
    #[must_use]
    pub fn new(
        issuer: Arc<AuthorityIssuer>,
        endpoint: String,
        connect_address: SocketAddr,
        tls_server_name: String,
    ) -> Self {
        Self::with_credential_source(
            issuer,
            endpoint,
            connect_address,
            tls_server_name,
            Arc::new(EmptyCredentials),
        )
    }

    /// Construct a launcher with an explicit post-admission credential capability.
    #[must_use]
    pub fn with_credential_source(
        issuer: Arc<AuthorityIssuer>,
        endpoint: String,
        connect_address: SocketAddr,
        tls_server_name: String,
        credentials: Arc<dyn CredentialSource>,
    ) -> Self {
        Self {
            issuer,
            application: Arc::new(TlsApplicationConnector::new(
                endpoint,
                connect_address,
                tls_server_name,
            )),
            credentials,
        }
    }
}

#[async_trait]
impl SessionLauncher for RuntimeLauncher {
    async fn ready(&self) -> Result<(), LaunchError> {
        self.credentials
            .ready()
            .await
            .map_err(|_| LaunchError::new("credential_source_unavailable"))
    }

    async fn launch(&self, admitted: AdmittedVoicePlan) -> Result<LaunchedSession, LaunchError> {
        let control = VoiceSessionControl::new();
        let task_control = control.clone();
        let issuer = Arc::clone(&self.issuer);
        let application = Arc::clone(&self.application);
        let credentials = Arc::clone(&self.credentials);
        let (observer, waiter) = dial_establishment_channel();
        let (completion_sender, completion) = watch::channel(None);
        tokio::spawn(async move {
            let runtime = VoiceRuntime::new(
                issuer.as_ref(),
                credentials.as_ref(),
                application.as_ref(),
                &SystemClock,
                &OsSessionMaterial,
                &observer,
                RuntimeConfig::default(),
            );
            let terminal = match runtime.run_outbound(&admitted, task_control).await {
                Ok(result) => termination(result.reason),
                Err(_) => protocol::operation::SessionTermination::Failed,
            };
            completion_sender.send_replace(Some(terminal));
        });
        let receipt = waiter.wait().await.map_err(|_| {
            control.terminate(domain::voice::TerminationReason::Cancelled);
            LaunchError::new("establishment_failed")
        })?;
        Ok(LaunchedSession {
            receipt,
            control,
            completion,
        })
    }
}

/// Tenant-pinned SIP digest credential source over an injected secret store.
pub struct StoredSipCredentials {
    store: Arc<dyn SecretStore>,
    tenant: String,
    username: CredentialRef,
    password: CredentialRef,
}

impl StoredSipCredentials {
    /// Validate and bind value-free credential addresses before any call is admitted.
    pub fn new(
        store: Arc<dyn SecretStore>,
        tenant: impl Into<String>,
        config: &HostedSipCredentialConfig,
    ) -> Result<Self, LaunchError> {
        let tenant = tenant.into();
        if config.authority != protocol::sip::SIP_DIAL_PROVIDER_AUTHORITY
            || config.service != "default"
        {
            return Err(LaunchError::new("sip_credential_provider_mismatch"));
        }
        let username = CredentialRef::new(
            &tenant,
            &config.authority,
            &config.service,
            &config.username_credential,
        )
        .map_err(|_| LaunchError::new("sip_credential_address_invalid"))?;
        let password = CredentialRef::new(
            &tenant,
            &config.authority,
            &config.service,
            &config.password_credential,
        )
        .map_err(|_| LaunchError::new("sip_credential_address_invalid"))?;
        if username == password {
            return Err(LaunchError::new("sip_credential_addresses_overlap"));
        }
        Ok(Self {
            store,
            tenant,
            username,
            password,
        })
    }

    async fn resolve_for_organization(
        &self,
        organization: &str,
    ) -> Result<CredentialSet, DependencyError> {
        if organization != self.tenant {
            return Err(DependencyError::new("sip_credential_tenant_mismatch"));
        }
        let username = self
            .store
            .get(&self.username)
            .await
            .map_err(credential_store_error)?;
        let password = self
            .store
            .get(&self.password)
            .await
            .map_err(credential_store_error)?;
        if username.is_empty()
            || password.is_empty()
            || username.expose_secret().len() > 1_024
            || password.expose_secret().len() > 1_024
        {
            return Err(DependencyError::new("sip_credentials_invalid"));
        }
        Ok(CredentialSet::new(vec![
            SensitiveValue::new(username.expose_secret().to_owned()),
            SensitiveValue::new(password.expose_secret().to_owned()),
        ]))
    }
}

#[async_trait]
impl CredentialSource for StoredSipCredentials {
    async fn ready(&self) -> Result<(), DependencyError> {
        self.store.ready().await.map_err(credential_store_error)
    }

    async fn resolve(
        &self,
        admitted: &AdmittedVoicePlan,
    ) -> Result<CredentialSet, DependencyError> {
        self.resolve_for_organization(admitted.sip().organization())
            .await
    }
}

fn credential_store_error(error: StoreError) -> DependencyError {
    match error {
        StoreError::NotFound { .. } => DependencyError::new("sip_credentials_not_configured"),
        StoreError::Denied { .. } => DependencyError::new("sip_credentials_access_denied"),
        StoreError::Unreachable { .. } => DependencyError::new("sip_credentials_unavailable"),
        StoreError::Backend { .. }
        | StoreError::Layout { .. }
        | StoreError::Conflict { .. }
        | StoreError::Unsupported { .. } => DependencyError::new("sip_credentials_invalid"),
    }
}

struct EmptyCredentials;

#[async_trait]
impl CredentialSource for EmptyCredentials {
    async fn ready(&self) -> Result<(), DependencyError> {
        Ok(())
    }

    async fn resolve(
        &self,
        _admitted: &AdmittedVoicePlan,
    ) -> Result<CredentialSet, DependencyError> {
        Ok(CredentialSet::default())
    }
}

struct TlsApplicationConnector {
    endpoint: String,
    connect_address: SocketAddr,
    tls_server_name: String,
    tls: Arc<ClientConfig>,
}

impl TlsApplicationConnector {
    fn new(endpoint: String, connect_address: SocketAddr, tls_server_name: String) -> Self {
        let roots = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let tls = ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        Self {
            endpoint,
            connect_address,
            tls_server_name,
            tls: Arc::new(tls),
        }
    }
}

#[async_trait]
impl ApplicationConnector for TlsApplicationConnector {
    async fn connect(
        &self,
        route: &VoiceApplicationRoute,
    ) -> Result<Box<dyn ApplicationStream>, DependencyError> {
        if route.endpoint != self.endpoint {
            return Err(DependencyError::new("application_route_mismatch"));
        }
        let server_name = ServerName::try_from(self.tls_server_name.clone())
            .map_err(|_| DependencyError::new("invalid_tls_server_name"))?;
        let tcp = TcpStream::connect(self.connect_address)
            .await
            .map_err(|_| DependencyError::new("application_tcp_connect_failed"))?;
        let stream = TlsConnector::from(Arc::clone(&self.tls))
            .connect(server_name, tcp)
            .await
            .map_err(|_| DependencyError::new("application_tls_connect_failed"))?;
        Ok(Box::new(stream))
    }
}

/// Load the deployment signing identity from an owner-only, non-symlink file.
pub fn load_authority_issuer(config: &AuthorityConfig) -> Result<AuthorityIssuer, LaunchError> {
    let path = Path::new(&config.signing_key_file);
    if !path.is_absolute() {
        return Err(LaunchError::new("authority_key_path_not_absolute"));
    }
    let canonical =
        fs::canonicalize(path).map_err(|_| LaunchError::new("authority_key_unreadable"))?;
    let working_tree = std::env::current_dir()
        .and_then(fs::canonicalize)
        .map_err(|_| LaunchError::new("working_tree_unavailable"))?;
    if canonical.starts_with(working_tree) {
        return Err(LaunchError::new("authority_key_inside_working_tree"));
    }
    let mut file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(path)
        .map_err(|_| LaunchError::new("authority_key_unreadable"))?;
    let metadata = file
        .metadata()
        .map_err(|_| LaunchError::new("authority_key_unreadable"))?;
    if !metadata.file_type().is_file()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err(LaunchError::new("authority_key_permissions_invalid"));
    }
    let mut bytes = Vec::with_capacity(65);
    (&mut file)
        .take(65)
        .read_to_end(&mut bytes)
        .map_err(|_| LaunchError::new("authority_key_unreadable"))?;
    let mut key: [u8; 32] = if bytes.len() == 32 {
        bytes
            .as_slice()
            .try_into()
            .map_err(|_| LaunchError::new("authority_key_invalid"))?
    } else if bytes.len() == 64
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
    {
        let mut decoded =
            hex::decode(&bytes).map_err(|_| LaunchError::new("authority_key_invalid"))?;
        let key = decoded
            .as_slice()
            .try_into()
            .map_err(|_| LaunchError::new("authority_key_invalid"))?;
        decoded.fill(0);
        key
    } else {
        return Err(LaunchError::new("authority_key_invalid"));
    };
    bytes.fill(0);
    let signing_key = SigningKey::from_bytes(&key);
    key.fill(0);
    Ok(AuthorityIssuer::new(
        &config.issuer,
        &config.key_id,
        signing_key,
    ))
}

/// Map a driver's terminal reason onto the protocol's session termination.
///
/// Shared with the raw SIP launcher: both bindings publish the same terminal vocabulary, which is
/// the point of the neutral session contract.
pub(crate) fn termination(
    reason: domain::voice::TerminationReason,
) -> protocol::operation::SessionTermination {
    use domain::voice::TerminationReason;
    use protocol::operation::SessionTermination;
    match reason {
        TerminationReason::Completed => SessionTermination::Completed,
        TerminationReason::Cancelled => SessionTermination::Cancelled,
        TerminationReason::RemoteHangup => SessionTermination::RemoteEnded,
        TerminationReason::AuthorityRevoked => SessionTermination::Revoked,
        TerminationReason::LeaseExpired => SessionTermination::LeaseExpired,
        TerminationReason::MediaOverload
        | TerminationReason::TransportLost
        | TerminationReason::ProtocolError => SessionTermination::Failed,
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::{symlink, PermissionsExt as _};
    use std::sync::atomic::{AtomicUsize, Ordering};

    use connector_secrets::{MemoryStore, Secret};

    use super::*;

    struct UnreadyStore {
        reads: AtomicUsize,
    }

    #[async_trait]
    impl SecretStore for UnreadyStore {
        async fn ready(&self) -> Result<(), StoreError> {
            Err(StoreError::Unreachable {
                path: "vault".to_owned(),
                reason: "test health refusal".to_owned(),
            })
        }

        async fn get(
            &self,
            _reference: &CredentialRef,
        ) -> Result<connector_secrets::Secret, StoreError> {
            self.reads.fetch_add(1, Ordering::AcqRel);
            Err(StoreError::Unreachable {
                path: "credential".to_owned(),
                reason: "test read refusal".to_owned(),
            })
        }

        async fn put(
            &self,
            _reference: &CredentialRef,
            _secret: &connector_secrets::Secret,
        ) -> Result<(), StoreError> {
            unreachable!("readiness never stores a credential")
        }

        async fn delete(&self, _reference: &CredentialRef) -> Result<(), StoreError> {
            unreachable!("readiness never deletes a credential")
        }
    }

    fn config(path: &Path) -> AuthorityConfig {
        AuthorityConfig {
            issuer: "https://connectors.example".to_owned(),
            key_id: "key-1".to_owned(),
            signing_key_file: path.display().to_string(),
        }
    }

    #[test]
    fn authority_key_must_be_an_owner_only_real_file() {
        let root = tempfile::tempdir().unwrap();
        let key = root.path().join("authority.key");
        fs::write(&key, [3_u8; 32]).unwrap();
        fs::set_permissions(&key, fs::Permissions::from_mode(0o600)).unwrap();
        let issuer = load_authority_issuer(&config(&key)).unwrap();
        assert_eq!(
            issuer.verifying_key(),
            SigningKey::from_bytes(&[3_u8; 32]).verifying_key()
        );

        fs::set_permissions(&key, fs::Permissions::from_mode(0o640)).unwrap();
        assert!(load_authority_issuer(&config(&key)).is_err());

        let link = root.path().join("authority-link.key");
        symlink(&key, &link).unwrap();
        assert!(load_authority_issuer(&config(&link)).is_err());
    }

    fn credential_config() -> HostedSipCredentialConfig {
        HostedSipCredentialConfig {
            authority: protocol::sip::SIP_DIAL_PROVIDER_AUTHORITY.to_owned(),
            service: "default".to_owned(),
            username_credential: "sip_username".to_owned(),
            password_credential: "sip_password".to_owned(),
        }
    }

    #[tokio::test]
    async fn stored_credentials_are_tenant_scoped_ordered_and_redacted() {
        let store = Arc::new(MemoryStore::new());
        let username = CredentialRef::new(
            "tenant-1",
            protocol::sip::SIP_DIAL_PROVIDER_AUTHORITY,
            "default",
            "sip_username",
        )
        .unwrap();
        let password = CredentialRef::new(
            "tenant-1",
            protocol::sip::SIP_DIAL_PROVIDER_AUTHORITY,
            "default",
            "sip_password",
        )
        .unwrap();
        store
            .put(&username, &Secret::new("caller-1"))
            .await
            .unwrap();
        store
            .put(&password, &Secret::new("SENTINEL-SIP-PASSWORD"))
            .await
            .unwrap();
        let source = StoredSipCredentials::new(
            store as Arc<dyn SecretStore>,
            "tenant-1",
            &credential_config(),
        )
        .unwrap();
        let credentials = source.resolve_for_organization("tenant-1").await.unwrap();
        assert_eq!(credentials.values()[0].expose_secret(), "caller-1");
        assert_eq!(
            credentials.values()[1].expose_secret(),
            "SENTINEL-SIP-PASSWORD"
        );
        assert!(!format!("{credentials:?}").contains("SENTINEL"));
        assert_eq!(
            source
                .resolve_for_organization("tenant-2")
                .await
                .unwrap_err()
                .code(),
            "sip_credential_tenant_mismatch"
        );
    }

    #[tokio::test]
    async fn missing_sip_credentials_fail_closed() {
        let mut wrong_provider = credential_config();
        wrong_provider.authority = "org.asterisk.ari".to_owned();
        assert!(StoredSipCredentials::new(
            Arc::new(MemoryStore::new()),
            "tenant-1",
            &wrong_provider,
        )
        .is_err());

        let source = StoredSipCredentials::new(
            Arc::new(MemoryStore::new()),
            "tenant-1",
            &credential_config(),
        )
        .unwrap();
        assert_eq!(
            source
                .resolve_for_organization("tenant-1")
                .await
                .unwrap_err()
                .code(),
            "sip_credentials_not_configured"
        );
    }

    #[tokio::test]
    async fn stored_credential_readiness_is_value_free_and_reports_store_unavailability() {
        let store = Arc::new(UnreadyStore {
            reads: AtomicUsize::new(0),
        });
        let source = Arc::new(
            StoredSipCredentials::new(
                store.clone() as Arc<dyn SecretStore>,
                "tenant-1",
                &credential_config(),
            )
            .unwrap(),
        );

        assert_eq!(
            source.ready().await.unwrap_err().code(),
            "sip_credentials_unavailable"
        );
        assert_eq!(store.reads.load(Ordering::Acquire), 0);
    }
}

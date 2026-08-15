//! Actual SIP/RTVBP session launcher and deployment-owned TLS connector.

use std::fs;
use std::io::Read as _;
use std::net::SocketAddr;
use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _};
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use ed25519_dalek::SigningKey;
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, RootCertStore};
use service::authority::AuthorityIssuer;
use service::{AdmittedVoicePlan, CredentialSet, VoiceApplicationRoute};
use tokio::net::TcpStream;
use tokio::sync::watch;
use tokio_rustls::TlsConnector;
use voice_runtime::{
    dial_establishment_channel, ApplicationConnector, ApplicationStream, CredentialSource,
    DependencyError, OsSessionMaterial, RuntimeConfig, SystemClock, VoiceRuntime,
    VoiceSessionControl,
};

use connectors_config::AuthorityConfig;

use crate::backend::{LaunchError, LaunchedSession, SessionLauncher};

/// Runtime launcher backed by the pinned sipx driver and RTVBP endpoint.
pub struct RuntimeLauncher {
    issuer: Arc<AuthorityIssuer>,
    application: Arc<TlsApplicationConnector>,
}

impl RuntimeLauncher {
    #[must_use]
    pub fn new(
        issuer: Arc<AuthorityIssuer>,
        endpoint: String,
        connect_address: SocketAddr,
        tls_server_name: String,
    ) -> Self {
        Self {
            issuer,
            application: Arc::new(TlsApplicationConnector::new(
                endpoint,
                connect_address,
                tls_server_name,
            )),
        }
    }
}

#[async_trait]
impl SessionLauncher for RuntimeLauncher {
    async fn launch(&self, admitted: AdmittedVoicePlan) -> Result<LaunchedSession, LaunchError> {
        let control = VoiceSessionControl::new();
        let task_control = control.clone();
        let issuer = Arc::clone(&self.issuer);
        let application = Arc::clone(&self.application);
        let (observer, waiter) = dial_establishment_channel();
        let (completion_sender, completion) = watch::channel(None);
        tokio::spawn(async move {
            let credentials = EmptyCredentials;
            let runtime = VoiceRuntime::new(
                issuer.as_ref(),
                &credentials,
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

struct EmptyCredentials;

#[async_trait]
impl CredentialSource for EmptyCredentials {
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

fn termination(
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

    use super::*;

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
}

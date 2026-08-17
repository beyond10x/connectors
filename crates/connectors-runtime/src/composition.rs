//! Product runtime assembly. Frontends select a mode and supply shutdown; they do not wire adapters.

use std::collections::BTreeSet;
use std::env;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use connector_secrets::{FileStore, MemoryStore, PreparedSecretStore, SecretStore};
use connectors_config::{HostedServerConfig, PersonalConfig};
use hosted_state::PostgresState;
use hosted_vault::{HostedVaultStore, PreparedVaultStore};
use identity_http::IdentityHttpVerifier;
use integration_b10x::{B10xBackend, B10xIntegrationError};
use integration_gitlab::GitlabBackend;
use integration_jira::JiraBackend;
use integration_kubernetes::{KubernetesLocalBackend, KubernetesStatusBackend};
use integration_monitoring::MonitoringBackend;
use integration_sip::{
    load_authority_issuer, RuntimeLauncher, SipOperationBackend, StoredSipCredentials,
};
use integration_slack::SlackBackend;
use serde_json::{json, Value};
use server::egress::{AddressScope, ConnectionEgress, DestinationRule};
use server::local::LocalOperationDaemon;
use service::{ConnectorBackend, EgressTransport};

use crate::BackendRegistry;

/// Failure to validate or assemble a complete Connector runtime.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("a personal-local state root could not be derived")]
    MissingStateRoot,
    #[error("a personal-local configuration path could not be derived")]
    MissingConfigPath,
    #[error("the state root must be absolute and outside the current working tree")]
    UnsafeStateRoot,
    #[error(transparent)]
    Config(#[from] connectors_config::ConfigError),
    #[error(transparent)]
    HostedConfig(#[from] connectors_config::HostedServerConfigError),
    #[error(transparent)]
    Voice(#[from] integration_sip::LaunchError),
    #[error(transparent)]
    Operation(#[from] protocol::operation::OperationError),
    #[error(transparent)]
    Slack(#[from] integration_slack::SlackError),
    #[error(transparent)]
    Gitlab(#[from] integration_gitlab::GitlabError),
    #[error(transparent)]
    Jira(#[from] integration_jira::JiraError),
    #[error(transparent)]
    Monitoring(#[from] integration_monitoring::MonitoringError),
    #[error(transparent)]
    Kubernetes(#[from] integration_kubernetes::KubernetesBackendError),
    #[error(transparent)]
    KubernetesLocal(#[from] integration_kubernetes::KubernetesLocalError),
    #[error(transparent)]
    B10x(#[from] B10xIntegrationError),
    #[error(transparent)]
    Identity(#[from] identity_http::IdentityVerifierConfigError),
    #[error(transparent)]
    Daemon(#[from] server::local::LocalDaemonError),
    #[error(transparent)]
    Egress(#[from] server::egress::EgressError),
    #[error("the personal credential store could not be opened")]
    CredentialStore,
    #[error("CONNECTORS_DATABASE_URL is required for hosted Connector state")]
    MissingHostedDatabase,
    #[error(transparent)]
    HostedState(#[from] hosted_state::StateError),
    #[error(transparent)]
    HostedVault(#[from] hosted_vault::HostedVaultError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Bound personal-local runtime with all configured adapters behind the exact registry.
pub struct PersonalRuntime {
    daemon: LocalOperationDaemon<BackendRegistry>,
    readiness: Value,
}

/// Credential capabilities supplied by an embedding personal runtime.
#[derive(Clone)]
pub struct PersonalCredentialStores {
    prepared: Arc<dyn PreparedSecretStore>,
    monitoring: Arc<dyn SecretStore>,
}

impl PersonalCredentialStores {
    #[must_use]
    pub fn new(prepared: Arc<dyn PreparedSecretStore>, monitoring: Arc<dyn SecretStore>) -> Self {
        Self {
            prepared,
            monitoring,
        }
    }
}

impl PersonalRuntime {
    /// Validate configuration and state, construct adapters, and bind the control socket.
    pub async fn bind(
        config_path: Option<&Path>,
        state_root: impl Into<PathBuf>,
    ) -> Result<Self, RuntimeError> {
        Self::bind_inner(config_path, state_root.into(), None).await
    }

    /// Bind with caller-supplied credential capabilities instead of the personal defaults.
    pub async fn bind_with_stores(
        config_path: Option<&Path>,
        state_root: impl Into<PathBuf>,
        stores: PersonalCredentialStores,
    ) -> Result<Self, RuntimeError> {
        Self::bind_inner(config_path, state_root.into(), Some(stores)).await
    }

    async fn bind_inner(
        config_path: Option<&Path>,
        state_root: PathBuf,
        supplied_stores: Option<PersonalCredentialStores>,
    ) -> Result<Self, RuntimeError> {
        validate_state_root(&state_root)?;
        let mut backends = Vec::<Arc<dyn ConnectorBackend>>::new();
        let mut verifying_key = None;
        let mut slack_connections = None;
        let mut monitoring_connections = None;
        let mut kubernetes_candidates = None;
        let mut kubernetes_connections = None;
        let mut b10x_configured = false;

        if let Some(config_path) = config_path {
            let config = PersonalConfig::read(config_path)?;
            let owner = config.principal_context()?;
            let stores = if let Some(stores) = supplied_stores {
                Some(stores)
            } else if config.slack.is_some() || config.grafana.is_some() {
                let prepared: Arc<dyn PreparedSecretStore> = if config.slack.is_some() {
                    Arc::new(
                        FileStore::open(state_root.join("credentials.store"))
                            .map_err(|_| RuntimeError::CredentialStore)?,
                    )
                } else {
                    Arc::new(MemoryStore::new())
                };
                let monitoring: Arc<dyn SecretStore> = Arc::new(MemoryStore::new());
                Some(PersonalCredentialStores::new(prepared, monitoring))
            } else {
                None
            };

            if let Some(voice) = config.voice()? {
                let issuer = Arc::new(load_authority_issuer(&voice.authority)?);
                verifying_key = Some(hex::encode(issuer.verifying_key().to_bytes()));
                let launcher = Arc::new(RuntimeLauncher::new(
                    Arc::clone(&issuer),
                    voice.application.endpoint.clone(),
                    voice.application.connect_address,
                    voice.application.tls_server_name.clone(),
                ));
                backends.push(Arc::new(SipOperationBackend::new(
                    voice,
                    launcher,
                    &state_root,
                )?));
            }
            if let Some(grafana) = config.grafana {
                let store = stores
                    .as_ref()
                    .expect("credential consumer selected the shared store")
                    .monitoring
                    .clone();
                let egress = monitoring_egress(&grafana.canonical_origin())?;
                let backend =
                    MonitoringBackend::open(owner.clone(), grafana, &state_root, store, egress)?;
                monitoring_connections = Some(backend.connection_count());
                backends.push(Arc::new(backend));
            }
            if let Some(kubernetes) = config.kubernetes {
                let backend = KubernetesLocalBackend::open(owner.clone(), kubernetes, &state_root)?;
                kubernetes_candidates = Some(backend.candidate_count());
                kubernetes_connections = Some(backend.connection_count());
                backends.push(Arc::new(backend));
            }
            if let Some(b10x) = config.b10x {
                backends.push(Arc::new(B10xBackend::personal(
                    b10x,
                    owner.clone(),
                    &state_root,
                )?));
                b10x_configured = true;
            }
            // Slack starts background supervision, so construct it only after every adapter whose
            // constructor can still fail. This keeps failed composition from leaking live tasks.
            if let Some(slack) = config.slack {
                let store = stores
                    .as_ref()
                    .expect("credential consumer selected the shared store")
                    .prepared
                    .clone();
                let backend =
                    SlackBackend::open(owner, slack, &state_root, store, slack_egress()?).await?;
                slack_connections = Some(backend.connection_count());
                backends.push(Arc::new(backend));
            }
        }

        let registry = Arc::new(BackendRegistry::new(backends));
        let daemon =
            match LocalOperationDaemon::bind(state_root.join("connectors.sock"), registry.clone())
                .await
            {
                Ok(daemon) => daemon,
                Err(error) => {
                    registry.shutdown().await;
                    return Err(error.into());
                }
            };
        let readiness = json!({
            "ready": true,
            "protocol": protocol::operation::CONTRACT,
            "protocols": [
                protocol::operation::CONTRACT,
                protocol::connection::CONTRACT,
                protocol::event::CONTRACT,
            ],
            "socket": daemon.socket_path(),
            "sip_dial_configured": verifying_key.is_some(),
            "voice_authority_verifying_key": verifying_key,
            "slack_configured": slack_connections.is_some(),
            "slack_connections": slack_connections,
            "grafana_configured": monitoring_connections.is_some(),
            "monitoring_connections": monitoring_connections,
            "kubernetes_configured": kubernetes_candidates.is_some(),
            "kubernetes_candidates": kubernetes_candidates,
            "kubernetes_connections": kubernetes_connections,
            "b10x_configured": b10x_configured,
        });
        Ok(Self { daemon, readiness })
    }

    #[must_use]
    pub fn readiness(&self) -> &Value {
        &self.readiness
    }

    pub async fn serve_until<F>(self, shutdown: F) -> Result<(), RuntimeError>
    where
        F: Future<Output = ()>,
    {
        self.daemon.serve_until(shutdown).await?;
        Ok(())
    }
}

/// Bound hosted runtime with a closed registry of configured stateless adapters.
pub struct HostedRuntime {
    listener: tokio::net::TcpListener,
    application: axum::Router,
    backend: Arc<BackendRegistry>,
    readiness: Value,
}

struct HostedCredentialStores {
    values: Option<Arc<dyn SecretStore>>,
    prepared: Option<Arc<dyn PreparedSecretStore>>,
}

impl HostedRuntime {
    /// Build the hosted backend registry and bind its TCP listener.
    pub async fn bind(config_path: &Path) -> Result<Self, RuntimeError> {
        let config = HostedServerConfig::read(config_path)?;
        validate_state_root(&config.storage.state_root)?;
        let database_url =
            env::var("CONNECTORS_DATABASE_URL").map_err(|_| RuntimeError::MissingHostedDatabase)?;
        let hosted_state = PostgresState::connect(&database_url)?;
        let credential_stores = if config.vault.enabled {
            let store = HostedVaultStore::new(&config.vault)?;
            store.initialize().await?;
            let store = Arc::new(store);
            let secret_store: Arc<dyn SecretStore> = store;
            let prepared = PreparedVaultStore::open_postgres(
                secret_store.clone(),
                hosted_state.clone(),
                "vault.prepared-transactions",
            )
            .map_err(|_| RuntimeError::CredentialStore)?;
            prepared
                .initialize()
                .await
                .map_err(|_| RuntimeError::CredentialStore)?;
            HostedCredentialStores {
                values: Some(secret_store),
                prepared: Some(Arc::new(prepared)),
            }
        } else {
            HostedCredentialStores {
                values: None,
                prepared: None,
            }
        };
        let identity_origin = url::Url::parse(&config.identity.origin)
            .map_err(|_| identity_http::IdentityVerifierConfigError::InvalidIdentityOrigin)?;
        let verifier = Arc::new(IdentityHttpVerifier::new(
            identity_origin,
            config.tenant_id.clone(),
        )?);
        let mut backends = Vec::<Arc<dyn ConnectorBackend>>::new();
        let kubernetes_namespace_access = config.kubernetes_namespace_access();
        let kubernetes_read_groups = kubernetes_namespace_access
            .iter()
            .flat_map(|access| access.read_groups.iter().cloned())
            .collect::<BTreeSet<_>>();
        let kubernetes_restart_groups = kubernetes_namespace_access
            .iter()
            .flat_map(|access| access.restart_groups.iter().cloned())
            .collect::<BTreeSet<_>>();
        let monitoring_enabled = config.grafana.enabled;
        let monitoring_read_groups = config
            .grafana
            .read_groups
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        if config.kubernetes.enabled {
            backends.push(Arc::new(KubernetesStatusBackend::in_cluster(
                config.tenant_id.clone(),
                kubernetes_namespace_access,
                config.authority.operator_groups.clone(),
                config.kubernetes.token_file.clone(),
                &config.kubernetes.ca_file,
            )?));
        }
        if config.sip.enabled {
            let deployment_config = config
                .sip
                .deployment_config
                .as_deref()
                .ok_or(connectors_config::HostedServerConfigError::Invalid)?;
            let personal = PersonalConfig::read_hosted(deployment_config)?;
            let voice = personal
                .voice()?
                .ok_or(connectors_config::HostedServerConfigError::Invalid)?;
            if voice.owner.tenant_id != config.tenant_id
                || voice
                    .sip
                    .targets
                    .iter()
                    .any(|target| Some(target.signaling_bind) != config.sip.listen)
            {
                return Err(connectors_config::HostedServerConfigError::Invalid.into());
            }
            let issuer = Arc::new(load_authority_issuer(&voice.authority)?);
            let launcher = if let Some(credentials) = config.sip.credentials.as_ref() {
                let store = credential_stores
                    .values
                    .as_ref()
                    .ok_or(connectors_config::HostedServerConfigError::Invalid)?
                    .clone();
                let source = Arc::new(StoredSipCredentials::new(
                    store,
                    config.tenant_id.clone(),
                    credentials,
                )?);
                Arc::new(RuntimeLauncher::with_credential_source(
                    issuer,
                    voice.application.endpoint.clone(),
                    voice.application.connect_address,
                    voice.application.tls_server_name.clone(),
                    source,
                ))
            } else {
                Arc::new(RuntimeLauncher::new(
                    issuer,
                    voice.application.endpoint.clone(),
                    voice.application.connect_address,
                    voice.application.tls_server_name.clone(),
                ))
            };
            backends.push(Arc::new(SipOperationBackend::new_postgres(
                voice,
                launcher,
                &config.storage.state_root,
                hosted_state.clone(),
            )?));
        }
        if config.grafana.enabled {
            let store = credential_stores
                .values
                .as_ref()
                .ok_or(connectors_config::HostedServerConfigError::Invalid)?
                .clone();
            let grafana_origin = config
                .grafana
                .origin
                .as_deref()
                .ok_or(connectors_config::HostedServerConfigError::Invalid)?;
            backends.push(Arc::new(
                MonitoringBackend::open_hosted(
                    config.tenant_id.clone(),
                    config.grafana.clone(),
                    config.authority.operator_groups.clone(),
                    &config.storage.state_root,
                    store,
                    hosted_state.clone(),
                    monitoring_egress(grafana_origin)?,
                )
                .await?,
            ));
        }
        let b10x_enabled = config.b10x.is_some();
        let admitted_module_tenants = config.admitted_module_tenants();
        if let Some(b10x) = config.b10x {
            backends.push(Arc::new(B10xBackend::hosted_postgres(
                b10x,
                admitted_module_tenants,
                &config.storage.state_root,
                hosted_state.clone(),
            )?));
        }
        let slack_enabled = config.slack.is_some();
        if let Some(slack) = config.slack {
            let public_origin = url::Url::parse(&slack.public_origin)
                .map_err(|_| connectors_config::HostedServerConfigError::Invalid)?;
            let store = credential_stores
                .prepared
                .as_ref()
                .ok_or(connectors_config::HostedServerConfigError::Invalid)?
                .clone();
            backends.push(Arc::new(
                SlackBackend::open_hosted(
                    config.tenant_id.clone(),
                    public_origin,
                    slack.policy(),
                    &config.storage.state_root,
                    store,
                    hosted_state.clone(),
                    slack_egress()?,
                )
                .await?,
            ));
        }
        let gitlab_enabled = config.gitlab.is_some();
        if let Some(gitlab) = config.gitlab {
            let store = credential_stores
                .prepared
                .as_ref()
                .ok_or(connectors_config::HostedServerConfigError::Invalid)?
                .clone();
            backends.push(Arc::new(
                GitlabBackend::open_hosted(
                    config.tenant_id.clone(),
                    gitlab,
                    store,
                    hosted_state.clone(),
                )
                .await?,
            ));
        }
        let jira_enabled = config.jira.is_some();
        if let Some(jira) = config.jira {
            let store = credential_stores
                .prepared
                .as_ref()
                .ok_or(connectors_config::HostedServerConfigError::Invalid)?
                .clone();
            backends.push(Arc::new(
                JiraBackend::open_hosted(
                    config.tenant_id.clone(),
                    jira,
                    store,
                    hosted_state.clone(),
                )
                .await?,
            ));
        }
        let backend = Arc::new(BackendRegistry::new(backends));
        let listener = tokio::net::TcpListener::bind(config.server.listen).await?;
        let admission =
            server::hosted::HostedAdmissionPolicy::new(config.authority.operator_groups.clone())
                .with_kubernetes_groups(kubernetes_read_groups, kubernetes_restart_groups)
                .with_monitoring_groups(monitoring_read_groups);
        let connector_router = server::hosted::router(verifier, backend.clone(), admission);
        let application = if config.server.base_path == "/" {
            connector_router
        } else {
            axum::Router::new().nest(&config.server.base_path, connector_router)
        };
        let readiness = json!({
            "ready": true,
            "protocol": protocol::operation::CONTRACT,
            "listen": config.server.listen,
            "base_path": config.server.base_path,
            "identity_audience": server::hosted::CONNECTORS_AUDIENCE,
            "egress_policy": config.egress.policy,
            "kubernetes_enabled": config.kubernetes.enabled,
            "monitoring_enabled": monitoring_enabled,
            "vault_enabled": config.vault.enabled,
            "sip_enabled": config.sip.enabled,
            "sip_listen": config.sip.listen,
            "b10x_enabled": b10x_enabled,
            "slack_enabled": slack_enabled,
            "gitlab_enabled": gitlab_enabled,
            "jira_enabled": jira_enabled,
        });
        Ok(Self {
            listener,
            application,
            backend,
            readiness,
        })
    }

    #[must_use]
    pub fn readiness(&self) -> &Value {
        &self.readiness
    }

    pub async fn serve_until<F>(self, shutdown: F) -> Result<(), RuntimeError>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let served = axum::serve(self.listener, self.application)
            .with_graceful_shutdown(shutdown)
            .await;
        let _ =
            tokio::time::timeout(std::time::Duration::from_secs(15), self.backend.shutdown()).await;
        served?;
        Ok(())
    }
}

fn monitoring_egress(origin: &str) -> Result<Arc<dyn EgressTransport>, RuntimeError> {
    let rule = DestinationRule::exact_origin(origin, AddressScope::OperatorNetwork)?;
    Ok(Arc::new(ConnectionEgress::new(vec![rule])?))
}

fn slack_egress() -> Result<Arc<dyn EgressTransport>, RuntimeError> {
    let rules = [
        DestinationRule::exact_origin("https://slack.com", AddressScope::Public),
        DestinationRule::exact_origin("wss://slack.com", AddressScope::Public),
        DestinationRule::dns_suffix("wss", ".slack.com", 443, AddressScope::Public),
    ]
    .into_iter()
    .collect::<Result<Vec<_>, _>>()?;
    Ok(Arc::new(ConnectionEgress::new(rules)?))
}

pub fn default_state_root() -> Result<PathBuf, RuntimeError> {
    if let Some(root) = env::var_os("XDG_STATE_HOME") {
        return Ok(PathBuf::from(root).join("b10x/connectors"));
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".local/state/b10x/connectors"))
        .ok_or(RuntimeError::MissingStateRoot)
}

pub fn default_config_path() -> Result<PathBuf, RuntimeError> {
    if let Some(root) = env::var_os("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(root).join("b10x/connectors.toml"));
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".config/b10x/connectors.toml"))
        .ok_or(RuntimeError::MissingConfigPath)
}

pub fn validate_state_root(root: &Path) -> Result<(), RuntimeError> {
    use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};

    if !root.is_absolute() {
        return Err(RuntimeError::UnsafeStateRoot);
    }
    let current = env::current_dir()
        .and_then(std::fs::canonicalize)
        .map_err(|_| RuntimeError::UnsafeStateRoot)?;
    let comparable = root
        .ancestors()
        .find(|candidate| candidate.exists())
        .and_then(|ancestor| std::fs::canonicalize(ancestor).ok())
        .ok_or(RuntimeError::UnsafeStateRoot)?;
    if comparable.starts_with(current) {
        return Err(RuntimeError::UnsafeStateRoot);
    }
    if !root.exists() {
        std::fs::create_dir_all(root).map_err(|_| RuntimeError::UnsafeStateRoot)?;
        std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| RuntimeError::UnsafeStateRoot)?;
    }
    let metadata = std::fs::symlink_metadata(root).map_err(|_| RuntimeError::UnsafeStateRoot)?;
    if !metadata.file_type().is_dir()
        || metadata.file_type().is_symlink()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err(RuntimeError::UnsafeStateRoot);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt as _;

    use super::*;

    #[tokio::test]
    async fn empty_personal_runtime_binds_and_cleans_without_a_credential_store() {
        let temporary = tempfile::tempdir().unwrap();
        fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let state_root = temporary.path().join("state");
        let runtime = PersonalRuntime::bind(None, &state_root).await.unwrap();
        assert_eq!(runtime.readiness()["ready"], true);
        assert!(state_root.join("connectors.sock").exists());
        assert!(!state_root.join("credentials.store").exists());
        runtime.serve_until(std::future::ready(())).await.unwrap();
        assert!(!state_root.join("connectors.sock").exists());
    }

    #[test]
    fn working_tree_state_roots_are_refused() {
        assert!(matches!(
            validate_state_root(Path::new(env!("CARGO_MANIFEST_DIR"))),
            Err(RuntimeError::UnsafeStateRoot)
        ));
    }
}

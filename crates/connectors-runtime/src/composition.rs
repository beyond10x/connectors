//! Product runtime assembly. Frontends select a mode and supply shutdown; they do not wire adapters.

use std::collections::BTreeSet;
use std::env;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use connector_secrets::{FileStore, KeyringStore, MemoryStore, PreparedSecretStore, SecretStore};
use connector_state::StateStore;
use connectors_config::{HostedServerConfig, PersonalConfig};
use domain::GrantSet;
use hosted_secrets::HostedSecretsStore;
use hosted_state::PostgresState;
use hosted_vault::{HostedVaultStore, PreparedVaultStore};
use identity_http::IdentityHttpVerifier;
use integration_catalog::{CatalogBackend, CatalogIntegrationError};
use integration_gitlab::GitlabBackend;
use integration_jira::JiraBackend;
use integration_kubernetes::{KubernetesLocalBackend, KubernetesStatusBackend};
use integration_monitoring::MonitoringBackend;
use integration_platform::{PlatformBackend, PlatformIntegrationError};
use integration_sip::{
    load_authority_issuer, RuntimeLauncher, SipLauncher, SipOperationBackend, StoredSipCredentials,
};
use integration_slack::SlackBackend;
use serde_json::{json, Value};
use server::egress::{AddressScope, ConnectionEgress, DestinationRule};
use server::local::LocalOperationDaemon;
use service::{ConnectorBackend, CredentialSet, EgressTransport};
use state_sqlite::SqliteState;
use subscription_custody::SubscriptionCustody;

use crate::claims::EventReplyClaims;
use crate::{BackendRegistry, ServiceBundle};

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
    ArgoCdAcquisition(integration_catalog::argocd::AcquireError),
    #[error(transparent)]
    KubernetesLocal(#[from] integration_kubernetes::KubernetesLocalError),
    #[error(transparent)]
    Platform(#[from] PlatformIntegrationError),
    #[error(transparent)]
    CatalogIntegration(#[from] CatalogIntegrationError),
    #[error(transparent)]
    Identity(#[from] identity_http::IdentityVerifierConfigError),
    #[error(transparent)]
    Daemon(#[from] server::local::LocalDaemonError),
    #[error(transparent)]
    Egress(#[from] server::egress::EgressError),
    #[error("the personal credential store could not be opened")]
    CredentialStore,
    /// A journal this build cannot read is unknown claim state; serving over it could post a
    /// reply its triggering event already authorized once (S-048).
    #[error("the local reply-claim journal could not be opened")]
    ReplyClaimJournal,
    #[error(
        "hosted Connector state needs exactly one store: set CONNECTORS_DATABASE_URL for \
         PostgreSQL, or CONNECTORS_SQLITE with a file path"
    )]
    MissingHostedState,
    #[error(
        "CONNECTORS_DATABASE_URL and CONNECTORS_SQLITE name two different stores for hosted \
         Connector state; set one and unset the other"
    )]
    AmbiguousHostedState,
    /// Carries the path, because the store the process could not open is the one fact the person
    /// who set the variable needs and the one the backend's own error does not carry.
    #[error("CONNECTORS_SQLITE names {path}, which could not be opened as a SQLite database")]
    UnusableSqliteState { path: String },
    /// Raised only by a build carrying `local-identity`. See `identity_http::local_identity`.
    #[error(
        "this binary was built with the loopback Identity exception, so it serves only a loopback \
         listener resolving a loopback plaintext Identity origin"
    )]
    LocalIdentityRefused,
    #[error(transparent)]
    ApprovalRecovery(#[from] server::hosted::RecoveryError),
    #[error(transparent)]
    HostedState(#[from] hosted_state::StateError),
    #[error("generated-service Grants could not be merged into hosted authority state")]
    ServiceGrantState,
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
        // The local dispatch seam's one-time claim (S-048): an approval-demanding invocation
        // presenting an `event:` reference spends it exactly once, durably, before any
        // Integration is reached. Wired for every personal placement — the journal exists from
        // first boot, not from the first Integration that needs it — and never for the hosted
        // placement, whose ApprovalGate redeems approval records upstream of the registry.
        // Opened before any adapter so a refusal here cannot leak a live supervision task.
        let claim_store: Arc<dyn StateStore> = Arc::new(
            SqliteState::open(&state_root.join("event-reply-claims.sqlite"))
                .map_err(|_| RuntimeError::ReplyClaimJournal)?,
        );
        let event_reply_claims =
            EventReplyClaims::open(claim_store).map_err(|_| RuntimeError::ReplyClaimJournal)?;
        let mut backends = Vec::<Arc<dyn ConnectorBackend>>::new();
        let mut verifying_key = None;
        let mut sip_dial_configured = false;
        let mut slack_connections = None;
        let mut monitoring_connections = None;
        let mut kubernetes_candidates = None;
        let mut kubernetes_connections = None;
        let mut platform_configured = false;
        let mut catalog_connections = None;
        let mut credential_backend = None;

        if let Some(config_path) = config_path {
            let config = PersonalConfig::read(config_path)?;
            let owner = config.principal_context()?;
            let stores = if let Some(stores) = supplied_stores {
                Some(stores)
            } else if config.slack.is_some()
                || config.grafana.is_some()
                || !config.catalog.is_empty()
            {
                // **One durable store, opened once.**
                //
                // The OS keyring first: a credential on a workstation should be sealed by the
                // desktop session, not by file permissions alone. `FileStore` protects a value with
                // owner + `0600`, which is a real guarantee against another user and none at all
                // against a copied backup or a synced home directory.
                //
                // The fallback is deliberate rather than silent. A server, a container or a CI
                // runner has no Secret Service, and refusing to start there would make the keyring
                // a deployment requirement instead of an improvement. Which one was bound is
                // published in readiness so `connectors doctor` can name an unencrypted store
                // rather than leaving an operator to assume the better one.
                let keyring: Option<Arc<KeyringStore>> = KeyringStore::open().ok().map(Arc::new);
                // **One `FileStore` per state root, opened once and shared.**
                //
                // The store takes an exclusive lease on its own file, so opening it twice in one
                // process is a self-inflicted `Conflict` — not a contended one, and no amount of
                // retrying clears it. It used to be opened once here for catalogued providers and
                // again below for Slack's prepared store, which worked on a workstation, where the
                // keyring is reachable and the first open is skipped, and refused to start
                // anywhere without a Secret Service: a server, a container, and any placement
                // spawned with a different `HOME` than the session that has the bus.
                let file: Option<Arc<FileStore>> = if config.slack.is_some()
                    || (keyring.is_none() && !config.catalog.is_empty())
                {
                    Some(Arc::new(
                        FileStore::open(state_root.join("credentials.store"))
                            .map_err(|_| RuntimeError::CredentialStore)?,
                    ))
                } else {
                    None
                };
                // The prepared (two-phase) store has no keyring implementation yet, so Slack keeps
                // the file-backed one. Named here rather than hidden: it is the remaining
                // unencrypted credential surface on a workstation, and it is why the reported
                // backend can be both at once.
                let prepared: Arc<dyn PreparedSecretStore> = match (&file, config.slack.is_some()) {
                    (Some(store), true) => Arc::clone(store) as Arc<dyn PreparedSecretStore>,
                    _ => Arc::new(MemoryStore::new()),
                };
                // Catalogued providers keep their credential across a restart, so the operator can
                // delete the file it was imported from. Grafana's own store stays in memory because
                // its credential is re-entered through a Connect Session each time.
                let monitoring: Arc<dyn SecretStore> = if config.catalog.is_empty() {
                    Arc::new(MemoryStore::new())
                } else if let Some(store) = &keyring {
                    Arc::clone(store) as Arc<dyn SecretStore>
                } else if let Some(store) = &file {
                    Arc::clone(store) as Arc<dyn SecretStore>
                } else {
                    Arc::new(MemoryStore::new())
                };
                // Reported as what was actually bound, not as the best one available. `doctor`
                // exists to tell an operator that a credential is sitting in an unencrypted file,
                // and a lone `keyring` on a Slack deployment — where the prepared store is still a
                // file — would say the opposite of the truth.
                credential_backend = Some(match (keyring.is_some(), file.is_some()) {
                    (true, true) => "keyring+file",
                    (true, false) => "keyring",
                    (false, true) => "file",
                    (false, false) => "memory",
                });
                Some(PersonalCredentialStores::new(prepared, monitoring))
            } else {
                None
            };

            if let Some(voice) = config.voice()? {
                // Recorded from what was composed, not from the authority key. A raw SIP trunk has
                // no authority -- that belongs to the application channel -- so deriving this from
                // the key told an operator with a working dialler that `sip.dial` was unconfigured.
                sip_dial_configured = true;
                // **Which launcher, decided by what was configured.** An application channel means
                // the call is carried onward and needs a session authority to issue its join
                // credential; without one the call terminates at the edge and neither exists. The
                // two are separate bindings of one neutral session contract, and composing the
                // composed launcher unconditionally is what made a raw SIP Connection impossible
                // to express.
                // Each arm yields the trait object: the two launchers are different types, and the
                // registry holds backends by capability rather than by which binding built them.
                let backend: Arc<dyn ConnectorBackend> =
                    match (&voice.authority, &voice.application) {
                        (Some(authority), Some(application)) => {
                            let issuer = Arc::new(load_authority_issuer(authority)?);
                            verifying_key = Some(hex::encode(issuer.verifying_key().to_bytes()));
                            let launcher = Arc::new(RuntimeLauncher::new(
                                Arc::clone(&issuer),
                                application.endpoint.clone(),
                                application.connect_address,
                                application.tls_server_name.clone(),
                            ));
                            Arc::new(SipOperationBackend::new(voice, launcher, &state_root)?)
                        }
                        _ => {
                            // Raw SIP: no signing key to load, no application endpoint to reach. The
                            // trunk's own credentials, when the peer authenticates, still arrive
                            // through the plan.
                            let launcher = Arc::new(SipLauncher::new(CredentialSet::default()));
                            Arc::new(SipOperationBackend::new(voice, launcher, &state_root)?)
                        }
                    };
                backends.push(backend);
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
            if let Some(platform) = config.platform {
                backends.push(Arc::new(PlatformBackend::personal(
                    platform,
                    owner.clone(),
                    &state_root,
                )?));
                platform_configured = true;
            }
            // Every provider the catalogue declares, served by one adapter. Composed before Slack
            // for the same reason everything else is: it can still fail, and a failed composition
            // must not leave Slack's supervision tasks running.
            if !config.catalog.is_empty() {
                let store = stores
                    .as_ref()
                    .expect("credential consumer selected the shared store")
                    .monitoring
                    .clone();
                let mut rules = Vec::new();
                for entry in &config.catalog {
                    // The scope is the provider's, not the deployment's: one placement may hold a
                    // public SaaS and a self-hosted instance at once, and widening the aperture for
                    // the second must not widen it for the first.
                    let scope = match entry.network {
                        connectors_config::NetworkScopeConfig::Public => AddressScope::Public,
                        connectors_config::NetworkScopeConfig::Operator => {
                            AddressScope::OperatorNetwork
                        }
                    };
                    for origin in integration_catalog::admitted_origins(entry)? {
                        rules.push(DestinationRule::exact_origin(&origin, scope)?);
                    }
                }
                let egress: Arc<dyn EgressTransport> = Arc::new(ConnectionEgress::new(rules)?);
                let backend = CatalogBackend::open(
                    owner.clone(),
                    &config.catalog,
                    &state_root,
                    store,
                    egress,
                )
                .await?;
                catalog_connections = Some(backend.connection_count());
                backends.push(Arc::new(backend));
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

        let registry = Arc::new(BackendRegistry::with_event_reply_claims(
            backends,
            event_reply_claims,
        ));
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
            "event_reply_claims": true,
            "sip_dial_configured": sip_dial_configured,
            "voice_authority_verifying_key": verifying_key,
            "slack_configured": slack_connections.is_some(),
            "slack_connections": slack_connections,
            "grafana_configured": monitoring_connections.is_some(),
            "monitoring_connections": monitoring_connections,
            "kubernetes_configured": kubernetes_candidates.is_some(),
            "kubernetes_candidates": kubernetes_candidates,
            "kubernetes_connections": kubernetes_connections,
            "platform_configured": platform_configured,
            "catalog_configured": catalog_connections.is_some(),
            "catalog_connections": catalog_connections,
            "credential_store": credential_backend,
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
        Self::bind_inner(config_path, None).await
    }

    /// Build the hosted runtime with one already-validated generated-service bundle.
    ///
    /// Merely constructing a bundle is still inert. This explicit composition call activates its
    /// backends, merges only its deployment-owned exact Grants, and admits its operations to reach
    /// the ordinary Grant/approval evaluator.
    pub async fn bind_with_service_bundle(
        config_path: &Path,
        bundle: ServiceBundle,
    ) -> Result<Self, RuntimeError> {
        Self::bind_inner(config_path, Some(bundle)).await
    }

    async fn bind_inner(
        config_path: &Path,
        bundle: Option<ServiceBundle>,
    ) -> Result<Self, RuntimeError> {
        let config = HostedServerConfig::read(config_path)?;
        let identity_origin = url::Url::parse(&config.identity.origin)
            .map_err(|_| identity_http::IdentityVerifierConfigError::InvalidIdentityOrigin)?;
        // This binary carries the loopback Identity exception, which lets it resolve access tokens
        // over plaintext HTTP. It refuses to serve anything a second machine could reach, or reach
        // out to, before it opens a database connection and before it binds a listener.
        #[cfg(feature = "local-identity")]
        if !identity_http::local_identity_admitted(&config.server.listen, &identity_origin) {
            return Err(RuntimeError::LocalIdentityRefused);
        }
        validate_state_root(&config.storage.state_root)?;
        let hosted_state = hosted_state_store(
            env::var("CONNECTORS_DATABASE_URL").ok(),
            env::var("CONNECTORS_SQLITE").ok(),
        )?;
        let generated_operation_refs = bundle
            .as_ref()
            .map_or_else(BTreeSet::new, ServiceBundle::operation_refs);
        let generated_service_refs = bundle.as_ref().map_or_else(Vec::new, |bundle| {
            bundle
                .services()
                .iter()
                .map(|service| service.manifest.service_ref.clone())
                .collect::<Vec<_>>()
        });
        if let Some(bundle) = bundle.as_ref() {
            GrantSet::merge_managed(&*hosted_state, &config.tenant_id, bundle.grants())
                .map_err(|_| RuntimeError::ServiceGrantState)?;
        }
        let credential_stores = if config.secrets.enabled {
            let store = Arc::new(
                HostedSecretsStore::new(&config.secrets)
                    .map_err(|_| RuntimeError::CredentialStore)?,
            );
            store
                .ready()
                .await
                .map_err(|_| RuntimeError::CredentialStore)?;
            let secret_store: Arc<dyn SecretStore> = store;
            let prepared = PreparedVaultStore::open_shared(
                secret_store.clone(),
                hosted_state.clone(),
                "secrets.prepared-transactions",
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
        } else if config.vault.enabled {
            let store = HostedVaultStore::new(&config.vault)?;
            store.initialize().await?;
            let store = Arc::new(store);
            let secret_store: Arc<dyn SecretStore> = store;
            let prepared = PreparedVaultStore::open_shared(
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
        let verifier = Arc::new(IdentityHttpVerifier::new(
            identity_origin.clone(),
            config.tenant_id.clone(),
        )?);
        let claude_code_enabled = config.claude_code.enabled;
        let subscription_custody = if claude_code_enabled {
            Some(Arc::new(
                SubscriptionCustody::with_claude_oauth(
                    credential_stores
                        .values
                        .as_ref()
                        .ok_or(connectors_config::HostedServerConfigError::Invalid)?
                        .clone(),
                    subscription_custody::ClaudeOAuthConfig::official()
                        .map_err(|_| RuntimeError::CredentialStore)?,
                )
                .map_err(|_| RuntimeError::CredentialStore)?,
            ))
        } else {
            None
        };
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
            // **A hosted placement may also terminate a call at the edge.**
            //
            // This used to demand an authority and an application channel, on the reasoning that a
            // hosted SIP deployment exists to carry calls onward. That is true of the cloud
            // deployment and false of a hosted placement running on someone's own machine, where
            // terminating the call locally is the whole point — and it made `sip.dial` unreachable
            // from the workbench, because the local stack serves hosted.
            //
            // So the arm is chosen by what is configured, exactly as the personal arm chooses it.
            // Neither posture decides whether a call is carried onward; the configuration does.
            let carried_onward = voice.authority.clone().zip(voice.application.clone());
            let backend: Arc<dyn ConnectorBackend> =
                if let Some((authority, application)) = carried_onward.as_ref() {
                    let issuer = Arc::new(load_authority_issuer(authority)?);
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
                            application.endpoint.clone(),
                            application.connect_address,
                            application.tls_server_name.clone(),
                            source,
                        ))
                    } else {
                        Arc::new(RuntimeLauncher::new(
                            issuer,
                            application.endpoint.clone(),
                            application.connect_address,
                            application.tls_server_name.clone(),
                        ))
                    };
                    Arc::new(SipOperationBackend::new_with_state(
                        voice,
                        launcher,
                        &config.storage.state_root,
                        hosted_state.clone(),
                    )?)
                } else {
                    // No application channel: the call is established and terminates here. The same
                    // launcher the personal posture uses, and the same media binding -- a hosted
                    // placement on a workstation has a speaker like any other.
                    Arc::new(SipOperationBackend::new_with_state(
                        voice,
                        Arc::new(integration_sip::SipLauncher::new(CredentialSet::default())),
                        &config.storage.state_root,
                        hosted_state.clone(),
                    )?)
                };
            backends.push(backend);
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
        let platform_enabled = config.platform.is_some();
        let admitted_module_tenants = config.admitted_module_tenants();
        if let Some(platform) = config.platform {
            backends.push(Arc::new(PlatformBackend::hosted_with_state(
                platform,
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
            let egress = gitlab_egress(&gitlab.origin)?;
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
                    egress,
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
        if let Some(bundle) = bundle {
            backends.extend(bundle.into_backends());
        }
        let backend = Arc::new(BackendRegistry::new(backends));
        let listener = tokio::net::TcpListener::bind(config.server.listen).await?;
        let admission =
            server::hosted::HostedAdmissionPolicy::new(config.authority.operator_groups.clone())
                .with_kubernetes_groups(kubernetes_read_groups, kubernetes_restart_groups)
                .with_monitoring_groups(monitoring_read_groups)
                .with_generated_service_operations(generated_operation_refs);
        // The enforcement authority binds the same store the hosted bookkeeping uses, and
        // accepts approval records from the deployment's one Identity issuer.
        let authority =
            server::hosted::HostedAuthority::bound(hosted_state, config.identity.origin.clone());
        // The S-045 crash-recovery scan settles every attempted approval presentation that has
        // no terminal outcome, before anything can present a new one. A journal that cannot be
        // settled is damaged approval authority, and this placement refuses to serve on it.
        authority.recover()?;
        let client_discovery = server::hosted::ClientDiscovery::new(&identity_origin);
        let connector_router = server::hosted::router_with_client_discovery(
            verifier,
            backend.clone(),
            admission,
            authority,
            subscription_custody,
            client_discovery,
        );
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
            "claude_code_enabled": claude_code_enabled,
            "sip_enabled": config.sip.enabled,
            "sip_listen": config.sip.listen,
            "platform_enabled": platform_enabled,
            "slack_enabled": slack_enabled,
            "gitlab_enabled": gitlab_enabled,
            "jira_enabled": jira_enabled,
            "generated_services": generated_service_refs,
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

fn gitlab_egress(origin: &str) -> Result<Arc<dyn EgressTransport>, RuntimeError> {
    let rule = DestinationRule::exact_origin(origin, AddressScope::OperatorNetwork)?;
    Ok(Arc::new(ConnectionEgress::new(vec![rule])?))
}

/// The Argo CD acquisition, packaged so a frontend can hand it to the console without unpacking it.
///
/// The composition root's whole job, in one function: it is the only place that may build a
/// transport, so it is the only place that can turn "acquire an Argo CD token" into a value with
/// the network already inside it. A frontend that assembled this itself would be composing an
/// adapter, which is the thing `architecture_fence.rs` measures and refuses.
#[must_use]
pub fn argocd_acquisition(operator_network: bool) -> integration_catalog::argocd::Acquire {
    Box::new(move |request| {
        Box::pin(async move {
            acquire_argocd_token(request, operator_network)
                .await
                .map_err(|error| error.to_string())
        })
    })
}

/// Mint an Argo CD API token, composing the one-origin aperture the acquisition may reach.
///
/// **This function exists so that nothing above it has to name a transport.** The product CLI's
/// reviewed dependency surface is parsing, presentation and these two packages, and
/// `connectors-console` is fenced against transports as well; both would have to grow an edge to
/// `service` merely to hold an `Arc<dyn EgressTransport>` on its way through. So the composition
/// root does what it is for — it builds the aperture, hands it to the acquisition, and returns a
/// value the caller can hold without linking anything.
///
/// The aperture is one origin and nothing else: the operator's own Argo CD, at the address they
/// approved. `operator_network` is theirs to assert, because an Argo CD reachable only inside their
/// network is the ordinary case and a public-only scope would refuse it — the same choice a
/// self-hosted `[[catalog]]` entry makes with `network = "operator"`.
///
/// # Errors
///
/// An origin no destination rule admits, or whatever the acquisition itself refuses.
pub async fn acquire_argocd_token(
    request: integration_catalog::argocd::AcquireRequest,
    operator_network: bool,
) -> Result<
    (
        integration_catalog::argocd::SecretString<String>,
        integration_catalog::argocd::AcquiredToken,
    ),
    RuntimeError,
> {
    let scope = if operator_network {
        AddressScope::OperatorNetwork
    } else {
        AddressScope::Public
    };
    let rule = DestinationRule::exact_origin(request.origin.trim_end_matches('/'), scope)?;
    let egress = ConnectionEgress::new(vec![rule])?;
    integration_catalog::argocd::acquire(&egress, request)
        .await
        .map_err(RuntimeError::ArgoCdAcquisition)
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

/// Bind the one durable store a hosted placement writes its Integrations' bookkeeping into.
///
/// # Why two environment variables rather than a derived path
///
/// A hosted Connector in the cluster keeps this in PostgreSQL because several replicas must agree
/// on which Connections exist. A person running the whole product on their own machine has one
/// replica and no reason to keep a database server alive for it — and keeping one alive is what
/// went wrong: a stale volume was recreated underneath a signed-in session, Identity was wiped, and
/// the workbench started refusing with `NotGranted` for reasons invisible from the page.
///
/// The SQLite path is named, not derived from `[storage] state_root`, because a derived path can
/// never be absent. "Neither variable is set" would silently become "write a database file
/// somewhere", so a deployment whose database URL failed to reach the process would come up serving
/// from a fresh empty local file and look healthy — the connection list gone, every Connection
/// apparently never granted. Naming it makes both stores refusable, and refusing is the only
/// honest answer to a placement that did not say where its state lives.
///
/// # Errors
///
/// [`RuntimeError::AmbiguousHostedState`] when both are set — two stores is not a merge, it is a
/// coin toss over which half of the bookkeeping a restart reads. [`RuntimeError::MissingHostedState`]
/// when neither is.
fn hosted_state_store(
    database_url: Option<String>,
    sqlite_path: Option<String>,
) -> Result<Arc<dyn StateStore>, RuntimeError> {
    match (
        database_url.filter(|value| !value.trim().is_empty()),
        sqlite_path.filter(|value| !value.trim().is_empty()),
    ) {
        (Some(_), Some(_)) => Err(RuntimeError::AmbiguousHostedState),
        (None, None) => Err(RuntimeError::MissingHostedState),
        (Some(database_url), None) => Ok(Arc::new(PostgresState::connect(&database_url)?)),
        (None, Some(path)) => {
            let store = SqliteState::open(Path::new(&path))
                .map_err(|_| RuntimeError::UnusableSqliteState { path })?;
            Ok(Arc::new(store))
        }
    }
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
        assert_eq!(runtime.readiness()["event_reply_claims"], true);
        assert!(state_root.join("connectors.sock").exists());
        assert!(
            state_root.join("event-reply-claims.sqlite").exists(),
            "the local reply-claim journal exists from first boot"
        );
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

    #[test]
    fn a_hosted_placement_keeps_its_state_in_a_file_when_no_database_is_offered() {
        let temporary = tempfile::tempdir().unwrap();
        let path = temporary.path().join("connectors.sqlite3");
        let store = hosted_state_store(
            None,
            Some(path.to_str().expect("a UTF-8 test path").to_owned()),
        )
        .expect("a named SQLite file is a complete answer to where hosted state lives");
        store.replace("connections", b"one", 64).unwrap();
        let read_back = store.read("connections", 64).unwrap();
        assert_eq!(read_back.as_deref(), Some(&b"one"[..]));
        assert!(path.exists(), "the bookkeeping went to the named file");
    }

    #[test]
    fn naming_both_stores_is_refused_rather_than_one_of_them_quietly_winning() {
        assert!(matches!(
            hosted_state_store(
                Some("postgres://localhost/connectors".to_owned()),
                Some("/var/lib/connectors/state.sqlite3".to_owned()),
            ),
            Err(RuntimeError::AmbiguousHostedState)
        ));
    }

    #[test]
    fn naming_no_store_is_refused_rather_than_a_database_file_appearing_somewhere() {
        assert!(matches!(
            hosted_state_store(None, None),
            Err(RuntimeError::MissingHostedState)
        ));
    }

    /// A supervisor that interpolates an unset variable hands the process an empty string, not an
    /// absent one. Treating that as a store would either dial an empty database URL or open a file
    /// called nothing; both come up looking healthy with no bookkeeping in them.
    #[test]
    fn an_empty_variable_is_no_store_at_all() {
        assert!(matches!(
            hosted_state_store(Some(String::new()), Some("   ".to_owned())),
            Err(RuntimeError::MissingHostedState)
        ));
    }

    /// A path that cannot hold a database is a typo in a variable, and the person who typed it
    /// needs to see which path the process tried rather than "state is unavailable".
    #[test]
    fn an_unopenable_sqlite_path_is_named_in_the_refusal() {
        let temporary = tempfile::tempdir().unwrap();
        let path = temporary.path().join("absent/connectors.sqlite3");
        let path = path.to_str().expect("a UTF-8 test path").to_owned();
        let Err(refusal) = hosted_state_store(None, Some(path.clone())) else {
            panic!("a SQLite path under a missing directory must be refused");
        };
        assert!(refusal.to_string().contains(&path), "{refusal}");
    }

    /// The sentence a person reads when the placement did not say where its state lives has to name
    /// what to set. The previous one named only the database URL, and stayed on the screen long
    /// after a second store existed.
    #[test]
    fn the_refusal_names_both_stores_a_deployment_may_choose() {
        let Err(refusal) = hosted_state_store(None, None) else {
            panic!("a placement that named no store must be refused");
        };
        let refusal = refusal.to_string();
        assert!(refusal.contains("CONNECTORS_DATABASE_URL"), "{refusal}");
        assert!(refusal.contains("CONNECTORS_SQLITE"), "{refusal}");
    }
}

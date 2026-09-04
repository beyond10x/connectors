//! Hosted, per-principal acquisition for catalog-declared Connect Session credentials.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use catalog::{Acquisition, Subject};
use connector_secrets::{
    CredentialScope, Layout as _, PreparedSecretStore, Secret, SecretBatch, SecretProposalDigest,
    SecretStore, SecretTransactionGeneration, SecretTransactionId, SecretTransactionState,
    TenantLayout,
};
use connector_state::StateStore;
use connectors_config::{CatalogIntegrationConfig, HostedCatalogConfig, InitiationConfig};
use protocol::catalog::{SetupProfileActor, SetupProfileSummary};
use protocol::connection::{
    ConnectSessionState, ConnectSessionStatus, ConnectionActor, ConnectionDescription,
    ConnectionError, ConnectionErrorCode, ConnectionInitiator, ConnectionRequest, ConnectionResult,
    ConnectionRoute, ConnectionScope, ConnectionState, ConnectionSummary,
};
use protocol::operation::{OperationError, OperationRequest, OperationResult};
use serde::{Deserialize, Serialize};
use service::{
    BackendCapabilities, BackendReadinessError, ConnectSessionAccess, ConnectorBackend,
    EgressTransport, HostedCompletionError, HostedCompletionPage, HostedCompletionSubmission,
    PrincipalContext,
};
use sha2::{Digest as _, Sha256};

use super::{credential_address, connection_ref, origin_of, CatalogBackend, CatalogIntegrationError};

const STATE_KEY: &str = "catalog.connections.v1";
const STATE_VERSION: u8 = 1;
const MAX_STATE_BYTES: usize = 2 * 1024 * 1024;
const MAX_PENDING_SESSIONS: usize = 256;
const MAX_SECRET_BYTES: usize = 8 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum HostedCatalogError {
    #[error("hosted catalog policy is invalid")]
    InvalidPolicy,
    #[error("hosted catalog state is unavailable")]
    State,
    #[error("hosted catalog credential custody is unavailable")]
    Credential,
    #[error(transparent)]
    Catalog(#[from] CatalogIntegrationError),
}

/// Generic catalog backend for principal-owned hosted Connections.
pub struct HostedCatalogBackend {
    inner: Arc<Inner>,
}

struct Inner {
    tenant_id: String,
    public_origin: url::Url,
    grant_ref: String,
    providers: BTreeSet<String>,
    excluded_providers: BTreeSet<String>,
    excluded_profiles: BTreeSet<(String, String)>,
    ttl_seconds: u64,
    values: Arc<dyn SecretStore>,
    prepared: Arc<dyn PreparedSecretStore>,
    state_store: Arc<dyn StateStore>,
    egress: Arc<dyn EgressTransport>,
    metadata: Mutex<StateFile>,
    sessions: Mutex<BTreeMap<String, Session>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StateFile {
    version: u8,
    next_transaction_generation: u64,
    connections: Vec<StoredConnection>,
    pending: Vec<PendingCommit>,
}

impl Default for StateFile {
    fn default() -> Self {
        Self {
            version: STATE_VERSION,
            next_transaction_generation: 1,
            connections: Vec::new(),
            pending: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredConnection {
    connection_ref: String,
    provider: String,
    instance: String,
    credential: String,
    label: String,
    owner_subject: String,
    actor: StoredActor,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    credential_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_verified_at_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StoredActor {
    User,
    App,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PendingCommit {
    transaction_id: String,
    connection: StoredConnection,
}

#[derive(Clone)]
struct Session {
    provider: String,
    credential: String,
    label: String,
    owner_subject: String,
    owner: PrincipalContext,
    capability_sha256: [u8; 32],
    expires_at_unix_ms: u64,
    state: ConnectSessionState,
    connection_ref: Option<String>,
}

impl HostedCatalogBackend {
    /// Open the hosted catalog adapter over shared state and prepared credential custody.
    #[allow(clippy::too_many_arguments)]
    pub async fn open(
        tenant_id: String,
        policy: HostedCatalogConfig,
        excluded_providers: BTreeSet<String>,
        excluded_profiles: BTreeSet<(String, String)>,
        values: Arc<dyn SecretStore>,
        prepared: Arc<dyn PreparedSecretStore>,
        state_store: Arc<dyn StateStore>,
        egress: Arc<dyn EgressTransport>,
    ) -> Result<Self, HostedCatalogError> {
        if !policy.enabled {
            return Err(HostedCatalogError::InvalidPolicy);
        }
        let public_origin = policy
            .public_origin
            .as_deref()
            .and_then(|value| url::Url::parse(value).ok())
            .ok_or(HostedCatalogError::InvalidPolicy)?;
        let grant_ref = policy.grant_ref.ok_or(HostedCatalogError::InvalidPolicy)?;
        let metadata = state_store
            .read(STATE_KEY, MAX_STATE_BYTES)
            .map_err(|_| HostedCatalogError::State)?
            .map_or_else(
                || Ok(StateFile::default()),
                |body| serde_json::from_slice(&body).map_err(|_| HostedCatalogError::State),
            )?;
        if metadata.version != STATE_VERSION || metadata.next_transaction_generation == 0 {
            return Err(HostedCatalogError::State);
        }
        let backend = Self {
            inner: Arc::new(Inner {
                tenant_id,
                public_origin,
                grant_ref,
                providers: policy.providers.into_iter().collect(),
                excluded_providers,
                excluded_profiles,
                ttl_seconds: policy.connect_session_ttl_seconds,
                values,
                prepared,
                state_store,
                egress,
                metadata: Mutex::new(metadata),
                sessions: Mutex::new(BTreeMap::new()),
            }),
        };
        backend.inner.recover_pending().await?;
        Ok(backend)
    }
}

impl Inner {
    fn provider_allowed(&self, provider: &str) -> bool {
        self.providers.is_empty() || self.providers.contains(provider)
    }

    fn profile(&self, provider_ref: &str, profile: &str) -> Option<&'static catalog::Credential> {
        if !self.provider_allowed(provider_ref)
            || self.excluded_providers.contains(provider_ref)
            || self
                .excluded_profiles
                .contains(&(provider_ref.to_owned(), profile.to_owned()))
        {
            return None;
        }
        let provider = catalog::provider(catalog::ProviderKey::id(provider_ref))?;
        if provider.authority.is_none()
            || provider.services.iter().any(|service| service.base_url.contains('{'))
        {
            return None;
        }
        provider.auth.iter().find(|credential| {
            credential.name == profile
                && matches!(credential.acquire, Acquisition::ConnectSession)
                && !matches!(credential.subject, Subject::Unstated)
        })
    }

    fn profiles(&self, provider_ref: &str) -> Vec<SetupProfileSummary> {
        let Some(provider) = catalog::provider(catalog::ProviderKey::id(provider_ref)) else {
            return Vec::new();
        };
        provider
            .auth
            .iter()
            .filter_map(|credential| {
                self.profile(provider_ref, credential.name)?;
                let actor = match credential.subject {
                    Subject::User => SetupProfileActor::Person,
                    Subject::App => SetupProfileActor::Application,
                    Subject::Unstated => return None,
                };
                Some(SetupProfileSummary {
                    auth_profile: credential.name.to_owned(),
                    actor,
                })
            })
            .collect()
    }

    fn check_context(&self, context: &PrincipalContext) -> Result<(), ConnectionError> {
        if context.tenant_id() == self.tenant_id {
            Ok(())
        } else {
            Err(connection_error(
                ConnectionErrorCode::StaleAuthority,
                "owner authority snapshot is not current",
            ))
        }
    }

    fn owned_connections(&self, context: &PrincipalContext) -> Vec<StoredConnection> {
        if context.tenant_id() != self.tenant_id {
            return Vec::new();
        }
        lock(&self.metadata)
            .connections
            .iter()
            .filter(|connection| connection.owner_subject == context.subject())
            .cloned()
            .collect()
    }

    fn config(connection: &StoredConnection, grant_ref: &str) -> CatalogIntegrationConfig {
        CatalogIntegrationConfig {
            provider: connection.provider.clone(),
            instance: Some(connection.instance.clone()),
            label: Some(connection.label.clone()),
            grant_ref: grant_ref.to_owned(),
            initiation: InitiationConfig::Platform,
            allow_writes: false,
            endpoints: BTreeMap::new(),
            // Hosted self-service stores no user half today, so a `basic` connector is not
            // connectable through it. Stated as an empty map rather than left implicit: the
            // personal placement fills this from `[catalog.usernames]`, and the hosted gap is a
            // missing acquisition surface, not a different resolution rule.
            usernames: BTreeMap::new(),
            operator_approved: false,
            credential: Some(connection.credential.clone()),
            network: connectors_config::NetworkScopeConfig::Public,
            credential_file: None,
        }
    }

    fn delegate(&self, context: &PrincipalContext) -> Result<CatalogBackend, OperationError> {
        let configured = self
            .owned_connections(context)
            .iter()
            .map(|connection| Self::config(connection, &self.grant_ref))
            .collect::<Vec<_>>();
        CatalogBackend::bind_stored(
            context.clone(),
            &configured,
            self.values.clone(),
            self.egress.clone(),
        )
        .map_err(|_| operation_unavailable())
    }

    fn create_session(
        &self,
        context: &PrincipalContext,
        provider: &str,
        credential: &str,
        label: String,
    ) -> Result<ConnectSessionStatus, ConnectionError> {
        self.expire_sessions();
        self.profile(provider, credential).ok_or_else(|| {
            connection_error(
                ConnectionErrorCode::InvalidInput,
                "the catalog does not admit that setup profile",
            )
        })?;
        let mut sessions = lock(&self.sessions);
        if sessions
            .values()
            .filter(|session| session.state == ConnectSessionState::Pending)
            .count()
            >= MAX_PENDING_SESSIONS
        {
            return Err(connection_unavailable());
        }
        let id = random_uuid().map_err(|_| connection_unavailable())?;
        let session_ref = format!("connect-session:{id}");
        let capability = random_token(32).map_err(|_| connection_unavailable())?;
        let expires_at_unix_ms = now_ms()
            .and_then(|now| now.checked_add(self.ttl_seconds.saturating_mul(1_000)))
            .ok_or_else(connection_unavailable)?;
        let mut url = self.public_origin.clone();
        url.path_segments_mut()
            .map_err(|_| connection_unavailable())?
            .push("connect-sessions")
            .push(&session_ref);
        url.set_fragment(Some(&format!("token={capability}")));
        sessions.insert(
            session_ref.clone(),
            Session {
                provider: provider.to_owned(),
                credential: credential.to_owned(),
                label,
                owner_subject: context.subject().to_owned(),
                owner: context.clone(),
                capability_sha256: Sha256::digest(capability.as_bytes()).into(),
                expires_at_unix_ms,
                state: ConnectSessionState::Pending,
                connection_ref: None,
            },
        );
        Ok(ConnectSessionStatus {
            connect_session_ref: session_ref,
            integration_ref: provider.to_owned(),
            state: ConnectSessionState::Pending,
            expires_at_unix_ms,
            completion_endpoint: None,
            browser_completion_url: Some(url.into()),
            connection_ref: None,
        })
    }

    fn session_status(
        &self,
        context: &PrincipalContext,
        session_ref: &str,
    ) -> Result<ConnectSessionStatus, ConnectionError> {
        self.expire_sessions();
        let sessions = lock(&self.sessions);
        let session = sessions.get(session_ref).filter(|session| {
            session.owner_subject == context.subject() && context.tenant_id() == self.tenant_id
        });
        let session = session.ok_or_else(connection_not_found)?;
        Ok(ConnectSessionStatus {
            connect_session_ref: session_ref.to_owned(),
            integration_ref: session.provider.clone(),
            state: session.state,
            expires_at_unix_ms: session.expires_at_unix_ms,
            completion_endpoint: None,
            browser_completion_url: None,
            connection_ref: session.connection_ref.clone(),
        })
    }

    fn expire_sessions(&self) {
        let Some(now) = now_ms() else { return };
        for session in lock(&self.sessions).values_mut() {
            if session.state == ConnectSessionState::Pending && now >= session.expires_at_unix_ms {
                session.state = ConnectSessionState::Expired;
                session.capability_sha256.fill(0);
            }
        }
    }

    fn persist(&self, state: &StateFile) -> Result<(), HostedCatalogError> {
        let body = serde_json::to_vec(state).map_err(|_| HostedCatalogError::State)?;
        self.state_store
            .replace(STATE_KEY, &body, MAX_STATE_BYTES)
            .map_err(|_| HostedCatalogError::State)
    }

    fn reserve_transaction(
        &self,
    ) -> Result<(SecretTransactionId, SecretTransactionGeneration), HostedCatalogError> {
        let mut metadata = lock(&self.metadata);
        let generation = SecretTransactionGeneration::from_protocol_bytes(
            metadata.next_transaction_generation.to_be_bytes(),
        )
        .ok_or(HostedCatalogError::State)?;
        metadata.next_transaction_generation = metadata
            .next_transaction_generation
            .checked_add(1)
            .ok_or(HostedCatalogError::State)?;
        self.persist(&metadata)?;
        let mut nonce = [0_u8; 24];
        getrandom::fill(&mut nonce).map_err(|_| HostedCatalogError::State)?;
        Ok((SecretTransactionId::new(generation, nonce), generation))
    }

    async fn commit_connection(
        &self,
        session_ref: &str,
        session: &Session,
        secret: Secret,
        verification: Option<(String, u64)>,
    ) -> Result<String, HostedCatalogError> {
        let provider = catalog::provider(catalog::ProviderKey::id(&session.provider))
            .ok_or(HostedCatalogError::InvalidPolicy)?;
        let authority = provider.authority.ok_or(HostedCatalogError::InvalidPolicy)?;
        let credential = self
            .profile(&session.provider, &session.credential)
            .ok_or(HostedCatalogError::InvalidPolicy)?;
        let instance = random_uuid().map_err(|_| HostedCatalogError::State)?;
        let entry = CatalogIntegrationConfig {
            provider: session.provider.clone(),
            instance: Some(instance.clone()),
            label: Some(session.label.clone()),
            grant_ref: self.grant_ref.clone(),
            initiation: InitiationConfig::Platform,
            allow_writes: false,
            endpoints: BTreeMap::new(),
            // Hosted self-service stores no user half today, so a `basic` connector is not
            // connectable through it. Stated as an empty map rather than left implicit: the
            // personal placement fills this from `[catalog.usernames]`, and the hosted gap is a
            // missing acquisition surface, not a different resolution rule.
            usernames: BTreeMap::new(),
            operator_approved: false,
            credential: Some(session.credential.clone()),
            network: connectors_config::NetworkScopeConfig::Public,
            credential_file: None,
        };
        let reference = credential_address(&self.tenant_id, authority, &entry, credential.leaf)?;
        let connection = StoredConnection {
            connection_ref: connection_ref(&session.provider, &instance),
            provider: session.provider.clone(),
            instance,
            credential: session.credential.clone(),
            label: session.label.clone(),
            owner_subject: session.owner_subject.clone(),
            actor: match credential.subject {
                Subject::User => StoredActor::User,
                Subject::App => StoredActor::App,
                Subject::Unstated => return Err(HostedCatalogError::InvalidPolicy),
            },
            credential_sha256: verification.as_ref().map(|value| value.0.clone()),
            last_verified_at_unix_ms: verification.map(|value| value.1),
        };
        let (transaction, generation) = self.reserve_transaction()?;
        let mut batch = SecretBatch::new(
            CredentialScope::new(&self.tenant_id, authority)
                .map_err(|_| HostedCatalogError::Credential)?,
        );
        batch
            .put(reference, secret)
            .map_err(|_| HostedCatalogError::Credential)?;
        self.prepared
            .prepare(transaction, proposal_digest(&batch), &batch)
            .await
            .map_err(|_| HostedCatalogError::Credential)?;
        let transaction_id = hex::encode(transaction.protocol_bytes());
        let persist_result = {
            let mut metadata = lock(&self.metadata);
            metadata.pending.push(PendingCommit {
                transaction_id: transaction_id.clone(),
                connection: connection.clone(),
            });
            let result = self.persist(&metadata);
            if result.is_err() {
                metadata
                    .pending
                    .retain(|pending| pending.transaction_id != transaction_id);
            }
            result
        };
        if let Err(error) = persist_result {
            let _ = self.prepared.abort(transaction).await;
            return Err(error);
        }
        self.prepared
            .commit(transaction)
            .await
            .map_err(|_| HostedCatalogError::Credential)?;
        {
            let mut metadata = lock(&self.metadata);
            metadata
                .pending
                .retain(|pending| pending.transaction_id != transaction_id);
            metadata
                .connections
                .retain(|candidate| candidate.connection_ref != connection.connection_ref);
            metadata.connections.push(connection.clone());
            metadata.connections.sort_by(|left, right| {
                left.connection_ref.cmp(&right.connection_ref)
            });
            self.persist(&metadata)?;
        }
        let _ = self.prepared.reclaim(generation).await;
        let _ = session_ref;
        Ok(connection.connection_ref)
    }

    async fn recover_pending(&self) -> Result<(), HostedCatalogError> {
        let pending = lock(&self.metadata).pending.clone();
        for record in pending {
            let transaction = decode_transaction(&record.transaction_id)?;
            match self
                .prepared
                .state(transaction)
                .await
                .map_err(|_| HostedCatalogError::Credential)?
            {
                SecretTransactionState::Prepared => {
                    self.prepared
                        .commit(transaction)
                        .await
                        .map_err(|_| HostedCatalogError::Credential)?;
                }
                SecretTransactionState::Committed => {}
                SecretTransactionState::Absent => {
                    let mut metadata = lock(&self.metadata);
                    metadata
                        .pending
                        .retain(|candidate| candidate.transaction_id != record.transaction_id);
                    self.persist(&metadata)?;
                    continue;
                }
            }
            let mut metadata = lock(&self.metadata);
            metadata
                .pending
                .retain(|candidate| candidate.transaction_id != record.transaction_id);
            metadata.connections.push(record.connection);
            metadata.connections.sort_by(|left, right| {
                left.connection_ref.cmp(&right.connection_ref)
            });
            metadata
                .connections
                .dedup_by(|left, right| left.connection_ref == right.connection_ref);
            self.persist(&metadata)?;
        }
        Ok(())
    }

    async fn verify_credential(
        &self,
        session: &Session,
        value: &str,
    ) -> Result<Option<(String, u64)>, HostedCompletionError> {
        let provider = catalog::provider(catalog::ProviderKey::id(&session.provider))
            .ok_or(HostedCompletionError::Unavailable)?;
        let Some(operation_ref) = provider.verify else {
            return Ok(None);
        };
        let authority = provider.authority.ok_or(HostedCompletionError::Unavailable)?;
        let credential = self
            .profile(&session.provider, &session.credential)
            .ok_or(HostedCompletionError::Unavailable)?;
        let instance = random_uuid().map_err(|_| HostedCompletionError::Unavailable)?;
        let connection = StoredConnection {
            connection_ref: connection_ref(&session.provider, &instance),
            provider: session.provider.clone(),
            instance,
            credential: session.credential.clone(),
            label: session.label.clone(),
            owner_subject: session.owner_subject.clone(),
            actor: match credential.subject {
                Subject::User => StoredActor::User,
                Subject::App => StoredActor::App,
                Subject::Unstated => return Err(HostedCompletionError::Unavailable),
            },
            credential_sha256: None,
            last_verified_at_unix_ms: None,
        };
        let entry = Self::config(&connection, &self.grant_ref);
        let address = credential_address(&self.tenant_id, authority, &entry, credential.leaf)
            .map_err(|_| HostedCompletionError::Unavailable)?;
        let ephemeral = Arc::new(connector_secrets::MemoryStore::new());
        ephemeral
            .put(&address, &Secret::new(value))
            .await
            .map_err(|_| HostedCompletionError::Unavailable)?;
        let values: Arc<dyn SecretStore> = ephemeral;
        let backend = CatalogBackend::bind_stored(
            session.owner.clone(),
            &[entry],
            values,
            self.egress.clone(),
        )
        .map_err(|_| HostedCompletionError::Unavailable)?;
        let description = backend
            .inner
            .describe(operation_ref)
            .map_err(|_| HostedCompletionError::Refused)?;
        backend
            .inner
            .invoke(
                operation_ref,
                &connection.connection_ref,
                &description.description_ref,
                serde_json::json!({}),
            )
            .await
            .map_err(|_| HostedCompletionError::Refused)?;
        let fingerprint = hex::encode(Sha256::digest(value.as_bytes()));
        let verified_at = now_ms().ok_or(HostedCompletionError::Unavailable)?;
        Ok(Some((fingerprint, verified_at)))
    }
}

#[async_trait]
impl ConnectorBackend for HostedCatalogBackend {
    async fn ready(&self) -> Result<(), BackendReadinessError> {
        self.inner
            .values
            .ready()
            .await
            .map_err(|_| BackendReadinessError)
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            operations: true,
            connections: true,
            events: false,
            datasources: false,
        }
    }

    fn owns_operation(&self, request: &OperationRequest) -> bool {
        match request {
            OperationRequest::Search(_) => true,
            OperationRequest::Describe(request) => catalog::operation(
                catalog::OperationKey::id(&request.operation_ref),
            )
            .is_some_and(|operation| {
                lock(&self.inner.metadata)
                    .connections
                    .iter()
                    .any(|connection| connection.provider == operation.provider)
            }),
            OperationRequest::Invoke(request) => lock(&self.inner.metadata)
                .connections
                .iter()
                .any(|connection| connection.connection_ref == request.connection_ref),
            _ => false,
        }
    }

    fn owns_connection(&self, request: &ConnectionRequest) -> bool {
        match request {
            ConnectionRequest::ConnectSessionCreate(request) => request
                .auth_profile
                .as_deref()
                .is_some_and(|profile| {
                    self.inner.profile(&request.integration_ref, profile).is_some()
                }),
            ConnectionRequest::ConnectSessionStatus(request) => {
                lock(&self.inner.sessions).contains_key(&request.connect_session_ref)
            }
            ConnectionRequest::Describe(request) => lock(&self.inner.metadata)
                .connections
                .iter()
                .any(|connection| connection.connection_ref == request.connection_ref),
            ConnectionRequest::Search(_) => false,
            _ => false,
        }
    }

    fn connect_session_access(
        &self,
        request: &protocol::connection::ConnectSessionCreateRequest,
    ) -> ConnectSessionAccess {
        if request.auth_profile.as_deref().is_some_and(|profile| {
            self.inner.profile(&request.integration_ref, profile).is_some()
        }) {
            ConnectSessionAccess::SelfService
        } else {
            ConnectSessionAccess::Operator
        }
    }

    fn setup_profiles(&self, provider_ref: &str) -> Vec<SetupProfileSummary> {
        self.inner.profiles(provider_ref)
    }

    fn owns_hosted_completion(&self, session_ref: &str) -> bool {
        lock(&self.inner.sessions).contains_key(session_ref)
    }

    fn hosted_completion_page(
        &self,
        session_ref: &str,
    ) -> Result<HostedCompletionPage, HostedCompletionError> {
        self.inner.expire_sessions();
        let sessions = lock(&self.inner.sessions);
        let session = sessions
            .get(session_ref)
            .filter(|session| session.state == ConnectSessionState::Pending)
            .ok_or(HostedCompletionError::NotFound)?;
        Ok(completion_page(&session.provider))
    }

    async fn complete_hosted_session(
        &self,
        session_ref: &str,
        capability: &str,
        submission: HostedCompletionSubmission,
    ) -> Result<(), HostedCompletionError> {
        self.inner.expire_sessions();
        if capability.len() < 32 || capability.len() > 256 {
            return Err(HostedCompletionError::Refused);
        }
        let session = {
            let sessions = lock(&self.inner.sessions);
            let session = sessions
                .get(session_ref)
                .filter(|session| session.state == ConnectSessionState::Pending)
                .ok_or(HostedCompletionError::NotFound)?;
            let actual: [u8; 32] = Sha256::digest(capability.as_bytes()).into();
            if !constant_time_equal(&session.capability_sha256, &actual) {
                return Err(HostedCompletionError::Refused);
            }
            session.clone()
        };
        let bytes = submission.expose_secret();
        let start = bytes.iter().position(|byte| !byte.is_ascii_whitespace());
        let end = bytes.iter().rposition(|byte| !byte.is_ascii_whitespace());
        let value = start
            .zip(end)
            .map(|(start, end)| &bytes[start..=end])
            .filter(|value| !value.is_empty() && value.len() <= MAX_SECRET_BYTES)
            .ok_or(HostedCompletionError::Invalid)?;
        let value = std::str::from_utf8(value).map_err(|_| HostedCompletionError::Invalid)?;
        let verification = self.inner.verify_credential(&session, value).await?;
        let connection_ref = self
            .inner
            .commit_connection(session_ref, &session, Secret::new(value), verification)
            .await
            .map_err(|_| HostedCompletionError::Unavailable)?;
        let mut sessions = lock(&self.inner.sessions);
        let current = sessions
            .get_mut(session_ref)
            .ok_or(HostedCompletionError::NotFound)?;
        current.state = ConnectSessionState::Completed;
        current.connection_ref = Some(connection_ref);
        current.capability_sha256.fill(0);
        Ok(())
    }

    async fn handle(
        &self,
        context: &PrincipalContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        self.inner.delegate(context)?.handle(context, request).await
    }

    async fn handle_connection(
        &self,
        context: &PrincipalContext,
        request: ConnectionRequest,
    ) -> Result<ConnectionResult, ConnectionError> {
        self.inner.check_context(context)?;
        match request {
            ConnectionRequest::Search(request) => {
                let query = request.query.to_ascii_lowercase();
                let mut connections = self
                    .inner
                    .owned_connections(context)
                    .into_iter()
                    .filter(|connection| {
                        query.is_empty()
                            || connection.provider.to_ascii_lowercase().contains(&query)
                            || connection.label.to_ascii_lowercase().contains(&query)
                    })
                    .map(connection_summary)
                    .collect::<Vec<_>>();
                connections.truncate(usize::from(request.limit));
                Ok(ConnectionResult::Search { connections })
            }
            ConnectionRequest::Describe(request) => self
                .inner
                .owned_connections(context)
                .into_iter()
                .find(|connection| connection.connection_ref == request.connection_ref)
                .map(|connection| ConnectionResult::Describe(ConnectionDescription {
                    summary: connection_summary(connection),
                    channels: Vec::new(),
                }))
                .ok_or_else(connection_not_found),
            ConnectionRequest::ConnectSessionCreate(request) => {
                let profile = request.auth_profile.as_deref().ok_or_else(|| {
                    connection_error(
                        ConnectionErrorCode::InvalidInput,
                        "a catalog setup profile is required",
                    )
                })?;
                self.inner
                    .create_session(
                        context,
                        &request.integration_ref,
                        profile,
                        request.label,
                    )
                    .map(ConnectionResult::ConnectSessionCreate)
            }
            ConnectionRequest::ConnectSessionStatus(request) => self
                .inner
                .session_status(context, &request.connect_session_ref)
                .map(ConnectionResult::ConnectSessionStatus),
            _ => Err(connection_not_found()),
        }
    }
}

/// Exact public origins needed by enabled generic providers.
pub fn hosted_admitted_origins(
    policy: &HostedCatalogConfig,
) -> Result<Vec<String>, HostedCatalogError> {
    let admitted = policy.providers.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let mut origins = BTreeSet::new();
    for provider in catalog::providers() {
        if !admitted.is_empty() && !admitted.contains(provider.id) {
            continue;
        }
        if provider.auth.iter().all(|credential| {
            !matches!(credential.acquire, Acquisition::ConnectSession)
                || matches!(credential.subject, Subject::Unstated)
        }) {
            continue;
        }
        if provider.services.iter().any(|service| service.base_url.contains('{')) {
            continue;
        }
        for service in provider.services {
            let origin = origin_of(service.base_url);
            if !origin.is_empty() {
                origins.insert(origin);
            }
        }
    }
    for provider in &policy.providers {
        if catalog::provider(catalog::ProviderKey::id(provider)).is_none() {
            return Err(HostedCatalogError::InvalidPolicy);
        }
    }
    Ok(origins.into_iter().collect())
}

fn connection_summary(connection: StoredConnection) -> ConnectionSummary {
    ConnectionSummary {
        connection_ref: connection.connection_ref,
        integration_ref: connection.provider,
        label: connection.label,
        state: ConnectionState::Callable,
        initiation: vec![ConnectionInitiator::Platform],
        route: ConnectionRoute::Direct,
        scope: Some(ConnectionScope::Principal),
        actor: Some(match connection.actor {
            StoredActor::User => ConnectionActor::User,
            StoredActor::App => ConnectionActor::App,
        }),
        auth_profile: Some(connection.credential),
    }
}

fn completion_page(provider: &str) -> HostedCompletionPage {
    let title = html_escape(provider);
    HostedCompletionPage {
        title: format!("Connect {title}"),
        html: format!(
            r#"<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>Connect {title}</title>
<style>body{{font:16px system-ui;max-width:38rem;margin:4rem auto;padding:1rem;background:#111;color:#eee}}label,input,button{{display:block;width:100%;box-sizing:border-box}}input,button{{padding:.8rem;margin-top:.5rem}}button{{margin-top:1rem}}</style>
<h1>Connect {title}</h1><p>Enter the provider credential once. It is sent only to Connectors and stored in the configured credential store.</p>
<form><label>Credential<input name="credential" type="password" autocomplete="off" maxlength="8192" required></label><button>Connect</button></form><p id="status"></p>
<script>const form=document.querySelector('form'),status=document.querySelector('#status'),button=document.querySelector('button');const capability=new URL(location.href).hash.match(/^#token=([A-Za-z0-9_-]{{32,256}})$/)?.[1];history.replaceState(null,'',location.pathname);form.addEventListener('submit',async event=>{{event.preventDefault();const field=form.elements.credential,value=field.value;if(!capability||!value||value.length>8192){{status.textContent='Check the credential value.';return;}}field.value='';button.disabled=true;status.textContent='Saving the credential…';try{{const response=await fetch(location.pathname,{{method:'POST',headers:{{'Content-Type':'application/octet-stream','X-Connect-Session':capability}},body:value}});if(response.ok){{status.textContent='{title} connected. You may close this tab.';return;}}status.textContent=response.status===503?'The credential store is unavailable. Start Connect again later.':'The connection was refused.';}}catch{{status.textContent='Hosted Connectors is unavailable. Start Connect again later.';}}button.disabled=false;}});</script>"#
        ),
    }
}

fn proposal_digest(batch: &SecretBatch) -> SecretProposalDigest {
    let mut digest = Sha256::new();
    digest.update(b"b10x/catalog-credential-transaction/v1\0");
    for (reference, secret) in batch
        .put_entries()
        .expect("catalog credential transactions contain puts only")
    {
        digest.update(TenantLayout.render(reference).as_bytes());
        digest.update(b"\0");
        digest.update(secret.expose_secret().as_bytes());
        digest.update(b"\0");
    }
    SecretProposalDigest::from_protocol_bytes(digest.finalize().into())
}

fn decode_transaction(value: &str) -> Result<SecretTransactionId, HostedCatalogError> {
    let bytes = hex::decode(value).map_err(|_| HostedCatalogError::State)?;
    let bytes: [u8; 32] = bytes.try_into().map_err(|_| HostedCatalogError::State)?;
    SecretTransactionId::from_protocol_bytes(bytes).ok_or(HostedCatalogError::State)
}

fn random_token(bytes: usize) -> Result<String, ()> {
    let mut value = vec![0_u8; bytes];
    getrandom::fill(&mut value).map_err(|_| ())?;
    Ok(hex::encode(value))
}

fn random_uuid() -> Result<String, ()> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| ())?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let encoded = hex::encode(bytes);
    Ok(format!(
        "{}-{}-{}-{}-{}",
        &encoded[0..8],
        &encoded[8..12],
        &encoded[12..16],
        &encoded[16..20],
        &encoded[20..32]
    ))
}

fn now_ms() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    let mut difference = left.len() ^ right.len();
    for index in 0..left.len().max(right.len()) {
        difference |= usize::from(
            left.get(index).copied().unwrap_or_default()
                ^ right.get(index).copied().unwrap_or_default(),
        );
    }
    difference == 0
}

fn connection_error(code: ConnectionErrorCode, message: &str) -> ConnectionError {
    ConnectionError::new(code, message, false)
}

fn connection_not_found() -> ConnectionError {
    connection_error(ConnectionErrorCode::NotFound, "no such catalog Connection")
}

fn connection_unavailable() -> ConnectionError {
    connection_error(
        ConnectionErrorCode::Unavailable,
        "catalog Connection setup is temporarily unavailable",
    )
}

fn operation_unavailable() -> OperationError {
    OperationError::new(
        protocol::operation::OperationErrorCode::Unavailable,
        "catalog Connection is temporarily unavailable",
        false,
    )
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use connector_secrets::MemoryStore;
    use connector_state::MemoryState;
    use service::{EgressHttpRequest, EgressHttpResponse, EgressTransportError, EgressWebSocket};

    const SENTINEL_ONE: &str = "SENTINEL-NOT-A-REAL-SECRET-ONE";
    const SENTINEL_TWO: &str = "SENTINEL-NOT-A-REAL-SECRET-TWO";

    struct SuccessfulEgress;

    #[async_trait]
    impl EgressTransport for SuccessfulEgress {
        async fn execute(
            &self,
            _authority_ref: &str,
            _request: EgressHttpRequest,
        ) -> Result<EgressHttpResponse, EgressTransportError> {
            Ok(EgressHttpResponse {
                status: 200,
                headers: BTreeMap::new(),
                body: br#"{"data":[]}"#.to_vec(),
            })
        }

        async fn connect_websocket(
            &self,
            _authority_ref: &str,
            _url: String,
            _maximum_message_bytes: usize,
        ) -> Result<Box<dyn EgressWebSocket>, EgressTransportError> {
            Err(EgressTransportError::Refused)
        }
    }

    fn principal(subject: &str) -> PrincipalContext {
        PrincipalContext::hosted(
            "tenant-test".to_owned(),
            subject.to_owned(),
            subject.to_owned(),
            Some(format!("{subject}@example.test")),
            "snapshot:test".to_owned(),
            "0".repeat(64),
        )
        .expect("valid hosted principal")
    }

    async fn connect(
        backend: &HostedCatalogBackend,
        principal: &PrincipalContext,
        label: &str,
        sentinel: &str,
    ) -> String {
        let created = backend
            .handle_connection(
                principal,
                ConnectionRequest::ConnectSessionCreate(
                    protocol::connection::ConnectSessionCreateRequest {
                        integration_ref: "anthropic".to_owned(),
                        label: label.to_owned(),
                        auth_profile: Some("anthropic.api_key".to_owned()),
                    },
                ),
            )
            .await
            .expect("session created");
        let ConnectionResult::ConnectSessionCreate(created) = created else {
            panic!("wrong result")
        };
        let url = url::Url::parse(
            created
                .browser_completion_url
                .as_deref()
                .expect("browser completion"),
        )
        .expect("completion URL");
        let capability = url
            .fragment()
            .and_then(|fragment| fragment.strip_prefix("token="))
            .expect("capability");
        backend
            .complete_hosted_session(
                &created.connect_session_ref,
                capability,
                HostedCompletionSubmission::new(sentinel.as_bytes().to_vec()),
            )
            .await
            .expect("session completed");
        let status = backend
            .handle_connection(
                principal,
                ConnectionRequest::ConnectSessionStatus(
                    protocol::connection::ConnectSessionStatusRequest {
                        connect_session_ref: created.connect_session_ref,
                    },
                ),
            )
            .await
            .expect("status");
        let ConnectionResult::ConnectSessionStatus(status) = status else {
            panic!("wrong status")
        };
        status.connection_ref.expect("connection")
    }

    async fn search(
        backend: &HostedCatalogBackend,
        principal: &PrincipalContext,
    ) -> ConnectionResult {
        backend
            .handle_connection(
                principal,
                ConnectionRequest::Search(protocol::connection::SearchRequest {
                    query: String::new(),
                    limit: 10,
                }),
            )
            .await
            .expect("search")
    }

    #[tokio::test]
    async fn two_people_receive_isolated_connections_and_credential_addresses() {
        let store = Arc::new(MemoryStore::new());
        let values: Arc<dyn SecretStore> = store.clone();
        let prepared: Arc<dyn PreparedSecretStore> = store.clone();
        let backend = HostedCatalogBackend::open(
            "tenant-test".to_owned(),
            HostedCatalogConfig {
                enabled: true,
                public_origin: Some("https://connectors.example.test/api/connectors/v1".to_owned()),
                grant_ref: Some("grant:catalog-read".to_owned()),
                providers: vec!["anthropic".to_owned()],
                connect_session_ttl_seconds: 300,
            },
            BTreeSet::new(),
            BTreeSet::new(),
            values,
            prepared,
            Arc::new(MemoryState::new()),
            Arc::new(SuccessfulEgress),
        )
        .await
        .expect("backend opens");
        assert_eq!(backend.setup_profiles("anthropic").len(), 1);

        let first = principal("person-one");
        let second = principal("person-two");
        let first_ref = connect(&backend, &first, "First key", SENTINEL_ONE).await;
        let second_ref = connect(&backend, &second, "Second key", SENTINEL_TWO).await;
        assert_ne!(first_ref, second_ref);

        let ConnectionResult::Search { connections: first_rows } = search(&backend, &first).await
        else {
            panic!("wrong search result")
        };
        let ConnectionResult::Search { connections: second_rows } = search(&backend, &second).await
        else {
            panic!("wrong search result")
        };
        assert_eq!(first_rows.len(), 1);
        assert_eq!(second_rows.len(), 1);
        assert_eq!(first_rows[0].connection_ref, first_ref);
        assert_eq!(second_rows[0].connection_ref, second_ref);

        let stored = lock(&backend.inner.metadata).connections.clone();
        assert_eq!(stored.len(), 2);
        let provider = catalog::provider(catalog::ProviderKey::id("anthropic")).unwrap();
        let authority = provider.authority.unwrap();
        for connection in stored {
            let entry = Inner::config(&connection, "grant:catalog-read");
            let address = credential_address("tenant-test", authority, &entry, "api_key").unwrap();
            let secret = store.get(&address).await.expect("credential stored");
            let expected = if connection.owner_subject == "person-one" {
                SENTINEL_ONE
            } else {
                SENTINEL_TWO
            };
            assert_eq!(secret.expose_secret(), expected);
        }

        let state = serde_json::to_string(&*lock(&backend.inner.metadata)).unwrap();
        assert!(!state.contains(SENTINEL_ONE));
        assert!(!state.contains(SENTINEL_TWO));
    }
}

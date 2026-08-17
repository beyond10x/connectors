//! Slack Socket Mode Integration, Connection custody, and durable event delivery.

mod hosted_setup;

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read as _, Write as _};
use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use connect_session_transport::{
    remove_endpoint, BoundCompletionEndpoint, CompletionTransportError,
};
use connector_secrets::{
    CredentialRef, CredentialScope, Layout, PreparedSecretStore, Secret, SecretBatch,
    SecretProposalDigest, SecretTransactionGeneration, SecretTransactionId, SecretTransactionState,
    TenantLayout,
};
use futures_util::{SinkExt as _, StreamExt as _};
use hosted_state::PostgresState;
use protocol::connection::{
    ChannelState, ChannelSummary as ConnectionChannelSummary, ConnectSessionStatus,
    ConnectionDescription, ConnectionError, ConnectionErrorCode, ConnectionInitiator,
    ConnectionRequest, ConnectionResult, ConnectionState, ConnectionSummary,
};
use protocol::event::{
    ChannelSummary as EventChannelSummary, DataEvent, EventError, EventErrorCode, EventProvenance,
    EventRequest, EventResult,
};
use protocol::operation::{
    ApprovalPosture, ConnectionSummary as OperationConnectionSummary, EffectClass,
    InvocationResult, InvokeRequest, OperationDescription, OperationError, OperationErrorCode,
    OperationRequest, OperationResult, OperationSummary,
};
use reqwest::redirect::Policy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use tokio::sync::{watch, Notify};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::protocol::{Message, WebSocketConfig};
use zeroize::Zeroizing;

use connectors_config::{InitiationConfig, SlackIntegrationConfig};
use service::{
    BackendCapabilities, BackendReadinessError, ConnectSessionLifecycle,
    ConnectSessionLifecycleError, ConnectSessionTerminal, ConnectorBackend, HostedCompletionError,
    HostedCompletionPage, HostedCompletionSubmission, PrincipalContext,
};

use hosted_setup::{
    classify_auth_test_response, hosted_completion_error, parse_hosted_submission,
    random_capability, valid_hosted_capability, MAX_AUTH_TEST_RESPONSE_BYTES,
};

const INTEGRATION_REF: &str = "slack";
const AUTHORITY: &str = "com.slack.api";
const SERVICE: &str = "default";
const APP_TOKEN_CREDENTIAL: &str = "app_token";
const BOT_TOKEN_CREDENTIAL: &str = "bot_token";
const USER_TOKEN_CREDENTIAL: &str = "user_token";
const SOCKET_BINDING_REF: &str = "com.slack.api:v1#socket";
const STATE_VERSION: u8 = 2;
const MAX_CONNECT_SESSIONS: usize = 16;
const MAX_APP_TOKEN_BYTES: usize = 1024;
const MAX_STATE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_EVENT_STORE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_AUDIT_BYTES: u64 = 16 * 1024 * 1024;
const MAX_STORED_EVENTS: usize = 10_000;
const CONNECTION_STATE_KEY: &str = "slack.connections";
const EVENT_STATE_KEY: &str = "slack.events";
const AUDIT_STATE_KEY: &str = "slack.audit";
const MAX_SOCKET_MESSAGE_BYTES: usize = 1024 * 1024;
const APPS_CONNECTIONS_OPEN: &str = "https://slack.com/api/apps.connections.open";
const AUTH_TEST: &str = "https://slack.com/api/auth.test";
const SLACK_ORIGIN: &str = "https://slack.com";
const SLACK_OPERATIONS: [&str; 4] = [
    "slack-chat-post-message",
    "slack-conversations-history",
    "slack-users-info",
    "slack-reactions-add",
];

/// Redaction-safe Slack runtime failure.
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("Slack runtime refused: {code}")]
pub struct SlackError {
    code: &'static str,
}

impl SlackError {
    const fn new(code: &'static str) -> Self {
        Self { code }
    }
}

/// Standalone Slack Connection and Event adapter.
pub struct SlackBackend {
    inner: Arc<SlackInner>,
}

struct SlackInner {
    admission: PrincipalAdmission,
    completion_mode: CompletionMode,
    policy: SlackIntegrationConfig,
    state_root: PathBuf,
    hosted_state: Option<PostgresState>,
    credential_store: Arc<dyn PreparedSecretStore>,
    metadata: Mutex<StateFile>,
    sessions: Mutex<ConnectSessionLifecycle>,
    session_owners: Mutex<BTreeMap<String, String>>,
    hosted_sessions: Mutex<BTreeMap<String, HostedSession>>,
    hosted_completion_lock: tokio::sync::Mutex<()>,
    event_store: Arc<EventStore>,
    audit: AuditJournal,
    channel_states: Mutex<BTreeMap<String, ChannelState>>,
    integration_channel_state: Mutex<ChannelState>,
    supervisor_started: Mutex<bool>,
    shutdown: watch::Sender<bool>,
    tasks: Mutex<Vec<JoinHandle<()>>>,
    http: reqwest::Client,
    supervision_enabled: bool,
}

#[derive(Clone)]
enum PrincipalAdmission {
    Exact(PrincipalContext),
    Tenant(String),
}

#[derive(Clone)]
enum CompletionMode {
    Local,
    Hosted { public_origin: url::Url },
}

struct HostedSession {
    capability_sha256: [u8; 32],
    expires_at_unix_ms: u64,
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
    instance_id: String,
    label: String,
    grant_ref: String,
    initiation: InitiationConfig,
    allowed_events: Vec<String>,
    #[serde(default)]
    owner_subject: String,
    #[serde(default)]
    team_id: String,
}

struct SlackCredentials {
    app_token: Secret,
    bot_token: Option<Secret>,
    user_token: Option<Secret>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PendingCommit {
    transaction_id: String,
    connection: StoredConnection,
}

struct EventStore {
    path: PathBuf,
    hosted_state: Option<PostgresState>,
    events: Mutex<Vec<StoredEvent>>,
    notify: Notify,
}

struct AuditJournal {
    path: PathBuf,
    hosted_state: Option<PostgresState>,
    state: Mutex<AuditJournalState>,
}

#[derive(Default)]
struct AuditJournalState {
    terminal_reservations: BTreeMap<String, u64>,
}

#[derive(Clone, Copy, Serialize)]
struct AuditEvent<'a> {
    audit_ref: &'a str,
    operation_ref: &'a str,
    connection_ref: &'a str,
    tenant_id: &'a str,
    actor_subject: &'a str,
    outcome: &'a str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredEvent {
    sequence: u64,
    delivery_id: String,
    event: DataEvent,
}

/// Secret-bearing response from Slack. Deliberately neither `Debug` nor `Serialize`: its URL is a
/// temporary bearer ticket and must not enter logs, state, events, or client responses.
#[derive(Deserialize)]
struct SocketTicket {
    ok: bool,
    #[serde(default)]
    url: Option<String>,
}

/// Slack's transport envelope. Only the inner `payload.event` is projected to a Connector event.
#[derive(Deserialize)]
struct SocketEnvelope {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    envelope_id: Option<String>,
    #[serde(default)]
    payload: Option<Value>,
}

impl SlackBackend {
    /// Open owner-only state, recover any decided credential transaction, and supervise every
    /// callable Slack Connection. The configuration contains policy only; no ambient secret source
    /// is consulted.
    pub async fn open(
        owner: PrincipalContext,
        policy: SlackIntegrationConfig,
        state_root: &Path,
        credential_store: Arc<dyn PreparedSecretStore>,
    ) -> Result<Self, SlackError> {
        Self::open_inner(
            PrincipalAdmission::Exact(owner),
            CompletionMode::Local,
            policy,
            state_root,
            credential_store,
            None,
            true,
        )
        .await
    }

    /// Open the hosted Slack Integration for one Identity tenant. Connect Sessions complete over
    /// the exact public Connector origin and every credential is committed through `credential_store`.
    pub async fn open_hosted(
        tenant_id: String,
        public_origin: url::Url,
        policy: SlackIntegrationConfig,
        state_root: &Path,
        credential_store: Arc<dyn PreparedSecretStore>,
        hosted_state: PostgresState,
    ) -> Result<Self, SlackError> {
        Self::open_inner(
            PrincipalAdmission::Tenant(tenant_id),
            CompletionMode::Hosted { public_origin },
            policy,
            state_root,
            credential_store,
            Some(hosted_state),
            true,
        )
        .await
    }

    #[cfg(test)]
    async fn open_with_supervision(
        owner: PrincipalContext,
        policy: SlackIntegrationConfig,
        state_root: &Path,
        credential_store: Arc<dyn PreparedSecretStore>,
        supervision_enabled: bool,
    ) -> Result<Self, SlackError> {
        Self::open_inner(
            PrincipalAdmission::Exact(owner),
            CompletionMode::Local,
            policy,
            state_root,
            credential_store,
            None,
            supervision_enabled,
        )
        .await
    }

    async fn open_inner(
        admission: PrincipalAdmission,
        completion_mode: CompletionMode,
        policy: SlackIntegrationConfig,
        state_root: &Path,
        credential_store: Arc<dyn PreparedSecretStore>,
        hosted_state: Option<PostgresState>,
        supervision_enabled: bool,
    ) -> Result<Self, SlackError> {
        let metadata = read_state(&state_root.join("connections.json"), hosted_state.as_ref())?;
        let event_store = Arc::new(EventStore::open(
            state_root.join("events.jsonl"),
            hosted_state.clone(),
        )?);
        let http = reqwest::Client::builder()
            .redirect(Policy::none())
            .no_proxy()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(15))
            .user_agent("b10x-connectors/0.1")
            .build()
            .map_err(|_| SlackError::new("http-client"))?;
        let (shutdown, _) = watch::channel(false);
        let inner = Arc::new(SlackInner {
            admission,
            completion_mode,
            policy,
            state_root: state_root.to_path_buf(),
            hosted_state: hosted_state.clone(),
            credential_store,
            metadata: Mutex::new(metadata),
            sessions: Mutex::new(
                ConnectSessionLifecycle::new(INTEGRATION_REF, MAX_CONNECT_SESSIONS)
                    .map_err(|_| SlackError::new("connect-session-lifecycle"))?,
            ),
            session_owners: Mutex::new(BTreeMap::new()),
            hosted_sessions: Mutex::new(BTreeMap::new()),
            hosted_completion_lock: tokio::sync::Mutex::new(()),
            event_store,
            audit: AuditJournal::new(state_root.join("slack-operation-audit.jsonl"), hosted_state),
            channel_states: Mutex::new(BTreeMap::new()),
            integration_channel_state: Mutex::new(ChannelState::Starting),
            supervisor_started: Mutex::new(false),
            shutdown,
            tasks: Mutex::new(Vec::new()),
            http,
            supervision_enabled,
        });
        inner.recover_pending().await?;
        for connection in lock(&inner.metadata).connections.clone() {
            if inner.connection_is_admitted(&connection) {
                inner.start_supervisor(connection);
            }
        }
        Ok(Self { inner })
    }

    #[must_use]
    pub fn connection_count(&self) -> usize {
        lock(&self.inner.metadata)
            .connections
            .iter()
            .filter(|connection| self.inner.connection_is_admitted(connection))
            .count()
    }
}

#[async_trait]
impl ConnectorBackend for SlackBackend {
    async fn ready(&self) -> Result<(), BackendReadinessError> {
        self.inner
            .credential_store
            .ready()
            .await
            .map_err(|_| BackendReadinessError)
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            operations: true,
            connections: true,
            events: true,
        }
    }

    fn owns_operation(&self, request: &OperationRequest) -> bool {
        match request {
            OperationRequest::Describe(request) => is_slack_operation(&request.operation_ref),
            OperationRequest::Invoke(request) => {
                is_slack_operation(&request.operation_ref)
                    && lock(&self.inner.metadata)
                        .connections
                        .iter()
                        .any(|connection| {
                            connection.connection_ref == request.connection_ref
                                && self.inner.connection_is_admitted(connection)
                        })
            }
            OperationRequest::Search(_)
            | OperationRequest::SessionStatus(_)
            | OperationRequest::SessionTerminate(_)
            | OperationRequest::SessionReconcile(_) => false,
        }
    }

    fn owns_connection(&self, request: &ConnectionRequest) -> bool {
        match request {
            ConnectionRequest::ConnectSessionCreate(request) => {
                request.integration_ref == INTEGRATION_REF
            }
            ConnectionRequest::ConnectSessionStatus(request) => {
                lock(&self.inner.sessions).owns(&request.connect_session_ref)
            }
            ConnectionRequest::Describe(request) => lock(&self.inner.metadata)
                .connections
                .iter()
                .any(|connection| {
                    connection.connection_ref == request.connection_ref
                        && self.inner.connection_is_admitted(connection)
                }),
            ConnectionRequest::Search(_) => false,
            ConnectionRequest::CandidateSearch(_)
            | ConnectionRequest::CandidateActivate(_)
            | ConnectionRequest::ObservationSearch(_)
            | ConnectionRequest::Materialize(_) => false,
        }
    }

    fn owns_event(&self, request: &EventRequest) -> bool {
        match request {
            EventRequest::Receive(request) => self.inner.has_channel(&request.channel_ref),
            EventRequest::Replay(request) => self
                .inner
                .event_store
                .replay(&request.event_ref)
                .is_some_and(|event| self.inner.has_channel(&event.channel_ref)),
            EventRequest::Search(_) => false,
        }
    }

    fn owns_hosted_completion(&self, session_ref: &str) -> bool {
        matches!(self.inner.completion_mode, CompletionMode::Hosted { .. })
            && lock(&self.inner.hosted_sessions).contains_key(session_ref)
    }

    fn hosted_completion_page(
        &self,
        session_ref: &str,
    ) -> Result<HostedCompletionPage, HostedCompletionError> {
        self.inner.expire_hosted_sessions();
        if !self.owns_hosted_completion(session_ref) {
            return Err(HostedCompletionError::NotFound);
        }
        Ok(hosted_setup::completion_page())
    }

    async fn complete_hosted_session(
        &self,
        session_ref: &str,
        capability: &str,
        submission: HostedCompletionSubmission,
    ) -> Result<(), HostedCompletionError> {
        self.inner.expire_hosted_sessions();
        if !valid_hosted_capability(capability) {
            return Err(HostedCompletionError::Refused);
        }
        let actual: [u8; 32] = Sha256::digest(capability.as_bytes()).into();
        {
            let mut sessions = lock(&self.inner.hosted_sessions);
            let expected = sessions
                .get(session_ref)
                .ok_or(HostedCompletionError::NotFound)?;
            if !constant_time_equal(&expected.capability_sha256, &actual) {
                return Err(HostedCompletionError::Refused);
            }
            sessions
                .remove(session_ref)
                .expect("the checked hosted session remains present");
        }
        let status = lock(&self.inner.sessions)
            .status(session_ref)
            .ok_or(HostedCompletionError::NotFound)?;
        if now_ms().is_none_or(|now| now >= status.expires_at_unix_ms) {
            let _ = lock(&self.inner.sessions).finish(session_ref, ConnectSessionTerminal::Expired);
            lock(&self.inner.session_owners).remove(session_ref);
            return Err(HostedCompletionError::Refused);
        }
        let _completion = self.inner.hosted_completion_lock.lock().await;
        let credentials = match parse_hosted_submission(submission.expose_secret()) {
            Ok(credentials) => credentials,
            Err(error) => {
                let _ =
                    lock(&self.inner.sessions).finish(session_ref, ConnectSessionTerminal::Failed);
                lock(&self.inner.session_owners).remove(session_ref);
                return Err(error);
            }
        };
        let team_id = match self.inner.verify_workspace_credentials(&credentials).await {
            Ok(team_id) => team_id,
            Err(error) => {
                let _ =
                    lock(&self.inner.sessions).finish(session_ref, ConnectSessionTerminal::Failed);
                lock(&self.inner.session_owners).remove(session_ref);
                return Err(hosted_completion_error(error));
            }
        };
        let owner_subject = lock(&self.inner.session_owners)
            .remove(session_ref)
            .ok_or(HostedCompletionError::NotFound)?;
        match self
            .inner
            .complete_connection(session_ref, owner_subject, team_id, credentials)
            .await
        {
            Ok(connection_ref) => {
                lock(&self.inner.sessions)
                    .finish(
                        session_ref,
                        ConnectSessionTerminal::Completed { connection_ref },
                    )
                    .map_err(|_| HostedCompletionError::Unavailable)?;
                Ok(())
            }
            Err(error) => {
                let _ =
                    lock(&self.inner.sessions).finish(session_ref, ConnectSessionTerminal::Failed);
                Err(hosted_completion_error(error))
            }
        }
    }

    async fn handle(
        &self,
        context: &PrincipalContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        self.inner.check_operation_context(context)?;
        match request {
            OperationRequest::Search(request) => Ok(OperationResult::Search {
                operations: self.inner.search_operations(context, &request.query),
            }),
            OperationRequest::Describe(request) => self
                .inner
                .describe_operation(context, &request.operation_ref),
            OperationRequest::Invoke(request) => self.inner.invoke(context, request).await,
            OperationRequest::SessionStatus(_)
            | OperationRequest::SessionTerminate(_)
            | OperationRequest::SessionReconcile(_) => Err(operation_not_found()),
        }
    }

    async fn handle_connection(
        &self,
        context: &PrincipalContext,
        request: ConnectionRequest,
    ) -> Result<ConnectionResult, ConnectionError> {
        self.inner.check_connection_context(context)?;
        match request {
            ConnectionRequest::CandidateSearch(_)
            | ConnectionRequest::CandidateActivate(_)
            | ConnectionRequest::Materialize(_) => Err(ConnectionError::new(
                ConnectionErrorCode::NotFound,
                "Slack Integration does not own this connection request",
                false,
            )),
            ConnectionRequest::Search(request) => {
                let query = request.query.to_ascii_lowercase();
                let stored = lock(&self.inner.metadata).connections.clone();
                let mut connections = stored
                    .into_iter()
                    .filter(|connection| self.inner.connection_is_admitted(connection))
                    .filter(|connection| self.inner.connection_owned_by(connection, context))
                    .filter(|connection| {
                        query.is_empty()
                            || connection.label.to_ascii_lowercase().contains(&query)
                            || INTEGRATION_REF.contains(&query)
                    })
                    .map(|connection| self.inner.connection_summary(&connection))
                    .collect::<Vec<_>>();
                connections.truncate(usize::from(request.limit));
                Ok(ConnectionResult::Search { connections })
            }
            ConnectionRequest::Describe(request) => {
                let connection = lock(&self.inner.metadata)
                    .connections
                    .iter()
                    .find(|connection| {
                        connection.connection_ref == request.connection_ref
                            && self.inner.connection_is_admitted(connection)
                            && self.inner.connection_owned_by(connection, context)
                    })
                    .cloned()
                    .ok_or_else(connection_not_found)?;
                Ok(ConnectionResult::Describe(self.inner.describe(&connection)))
            }
            ConnectionRequest::ObservationSearch(_) => Ok(ConnectionResult::ObservationSearch {
                observations: Vec::new(),
            }),
            ConnectionRequest::ConnectSessionCreate(request) => {
                if request.integration_ref != INTEGRATION_REF {
                    return Err(ConnectionError::new(
                        ConnectionErrorCode::NotFound,
                        "integration was not found",
                        false,
                    ));
                }
                let session = self.inner.create_session(context, request.label).await?;
                Ok(ConnectionResult::ConnectSessionCreate(session))
            }
            ConnectionRequest::ConnectSessionStatus(request) => {
                if lock(&self.inner.session_owners)
                    .get(&request.connect_session_ref)
                    .is_none_or(|owner| owner != context.subject())
                {
                    return Err(connection_not_found());
                }
                let session = self
                    .inner
                    .session_status(&request.connect_session_ref)
                    .ok_or_else(connection_not_found)?;
                Ok(ConnectionResult::ConnectSessionStatus(session))
            }
        }
    }

    async fn handle_event(
        &self,
        context: &PrincipalContext,
        request: EventRequest,
    ) -> Result<EventResult, EventError> {
        self.inner.check_event_context(context)?;
        match request {
            EventRequest::Search(request) => {
                let query = request.query.to_ascii_lowercase();
                let mut channels = lock(&self.inner.metadata)
                    .connections
                    .iter()
                    .filter(|connection| self.inner.connection_is_admitted(connection))
                    .filter(|connection| self.inner.connection_owned_by(connection, context))
                    .filter(|connection| {
                        query.is_empty()
                            || connection.label.to_ascii_lowercase().contains(&query)
                            || INTEGRATION_REF.contains(&query)
                            || connection
                                .allowed_events
                                .iter()
                                .any(|event| event.contains(&query))
                    })
                    .map(event_channel_summary)
                    .collect::<Vec<_>>();
                channels.truncate(usize::from(request.limit));
                Ok(EventResult::Search { channels })
            }
            EventRequest::Receive(request) => {
                self.inner.require_channel(&request.channel_ref, context)?;
                let after = request
                    .after
                    .as_deref()
                    .unwrap_or("0")
                    .parse::<u64>()
                    .map_err(|_| event_invalid())?;
                let (events, next) = self
                    .inner
                    .event_store
                    .receive(
                        &request.channel_ref,
                        after,
                        usize::from(request.limit),
                        Duration::from_millis(u64::from(request.wait_ms)),
                    )
                    .await;
                Ok(EventResult::Receive {
                    events,
                    next: next.to_string(),
                })
            }
            EventRequest::Replay(request) => {
                let event = self
                    .inner
                    .event_store
                    .replay(&request.event_ref)
                    .ok_or_else(event_not_found)?;
                self.inner.require_channel(&event.channel_ref, context)?;
                Ok(EventResult::Replay(event))
            }
        }
    }

    async fn shutdown(&self) {
        let _ = self.inner.shutdown.send(true);
        let tasks = std::mem::take(&mut *lock(&self.inner.tasks));
        for task in &tasks {
            task.abort();
        }
        for task in tasks {
            let _ = task.await;
        }
        for endpoint in lock(&self.inner.sessions).fail_pending() {
            let _ = remove_endpoint(Path::new(&endpoint));
        }
        lock(&self.inner.hosted_sessions).clear();
        lock(&self.inner.session_owners).clear();
    }
}

impl SlackInner {
    fn persist_metadata(&self, state: &StateFile) -> Result<(), SlackError> {
        write_state(
            &self.state_root.join("connections.json"),
            self.hosted_state.as_ref(),
            state,
        )
    }

    fn tenant_id(&self) -> &str {
        match &self.admission {
            PrincipalAdmission::Exact(owner) => owner.tenant_id(),
            PrincipalAdmission::Tenant(tenant) => tenant,
        }
    }

    fn connection_is_admitted(&self, connection: &StoredConnection) -> bool {
        connection.grant_ref == self.policy.grant_ref
            && connection.initiation == self.policy.initiation
            && connection.allowed_events.len() == self.policy.allowed_events.len()
            && connection
                .allowed_events
                .iter()
                .all(|event| self.policy.allowed_events.contains(event))
            && match self.admission {
                PrincipalAdmission::Exact(_) => true,
                PrincipalAdmission::Tenant(_) => {
                    !connection.owner_subject.is_empty() && !connection.team_id.is_empty()
                }
            }
    }

    fn connection_owned_by(
        &self,
        connection: &StoredConnection,
        context: &PrincipalContext,
    ) -> bool {
        self.context_admitted(context)
            && match &self.admission {
                PrincipalAdmission::Exact(owner) => {
                    connection.owner_subject.is_empty()
                        || connection.owner_subject == owner.subject()
                }
                PrincipalAdmission::Tenant(_) => connection.owner_subject == context.subject(),
            }
    }

    fn context_admitted(&self, actual: &PrincipalContext) -> bool {
        match &self.admission {
            PrincipalAdmission::Exact(owner) => owner == actual,
            PrincipalAdmission::Tenant(tenant) => tenant == actual.tenant_id(),
        }
    }

    fn check_connection_context(&self, actual: &PrincipalContext) -> Result<(), ConnectionError> {
        if self.context_admitted(actual) {
            Ok(())
        } else {
            Err(ConnectionError::new(
                ConnectionErrorCode::StaleAuthority,
                "owner authority snapshot is not current",
                false,
            ))
        }
    }

    fn check_event_context(&self, actual: &PrincipalContext) -> Result<(), EventError> {
        if self.context_admitted(actual) {
            Ok(())
        } else {
            Err(EventError::new(
                EventErrorCode::StaleAuthority,
                "owner authority snapshot is not current",
                false,
            ))
        }
    }

    fn check_operation_context(&self, actual: &PrincipalContext) -> Result<(), OperationError> {
        if self.context_admitted(actual) {
            Ok(())
        } else {
            Err(OperationError::new(
                OperationErrorCode::StaleAuthority,
                "owner authority snapshot is not current",
                false,
            ))
        }
    }

    fn operation_connections(&self, context: &PrincipalContext) -> Vec<OperationConnectionSummary> {
        lock(&self.metadata)
            .connections
            .iter()
            .filter(|connection| self.connection_is_admitted(connection))
            .filter(|connection| self.connection_owned_by(connection, context))
            .map(|connection| OperationConnectionSummary {
                connection_ref: connection.connection_ref.clone(),
                label: connection.label.clone(),
                provider: INTEGRATION_REF.to_owned(),
                audiences: vec!["workspace".to_owned()],
            })
            .collect()
    }

    fn search_operations(&self, context: &PrincipalContext, query: &str) -> Vec<OperationSummary> {
        let connections = self.operation_connections(context);
        if connections.is_empty() {
            return Vec::new();
        }
        let query = query.to_ascii_lowercase();
        SLACK_OPERATIONS
            .iter()
            .filter_map(|operation_ref| {
                let operation = connector_resolve::document::operation(operation_ref)?;
                let title = operation_title(operation_ref);
                (query.is_empty()
                    || operation_ref.contains(&query)
                    || title.to_ascii_lowercase().contains(&query)
                    || operation
                        .contract_description()
                        .to_ascii_lowercase()
                        .contains(&query))
                .then(|| OperationSummary {
                    operation_ref: (*operation_ref).to_owned(),
                    title: title.to_owned(),
                    effect: operation_effect(operation_ref),
                    approval: operation_approval(operation_ref),
                    connections: connections.clone(),
                })
            })
            .collect()
    }

    fn description_ref(&self, context: &PrincipalContext, operation_ref: &str) -> String {
        let mut digest = Sha256::new();
        digest.update(b"b10x/slack-description/v1\0");
        digest.update(serde_json::to_vec(context).expect("principal context serializes"));
        digest.update(b"\0");
        digest.update(operation_ref.as_bytes());
        digest.update(b"\0");
        digest.update(self.policy.grant_ref.as_bytes());
        for connection in self.operation_connections(context) {
            digest.update(b"\0");
            digest.update(connection.connection_ref.as_bytes());
        }
        format!("description-sha256-{:x}", digest.finalize())
    }

    fn describe_operation(
        &self,
        context: &PrincipalContext,
        operation_ref: &str,
    ) -> Result<OperationResult, OperationError> {
        if !is_slack_operation(operation_ref) {
            return Err(operation_not_found());
        }
        let operation = connector_resolve::document::operation(operation_ref)
            .ok_or_else(operation_not_found)?;
        let connections = self.operation_connections(context);
        if connections.is_empty() {
            return Err(operation_not_found());
        }
        Ok(OperationResult::Describe(OperationDescription {
            operation_ref: operation_ref.to_owned(),
            title: operation_title(operation_ref).to_owned(),
            description: operation.contract_description().to_owned(),
            input_schema: operation.input_schema().clone(),
            output_schema: serde_json::json!({"type":"object"}),
            effect: operation_effect(operation_ref),
            approval: operation_approval(operation_ref),
            connections,
            description_ref: self.description_ref(context, operation_ref),
        }))
    }

    async fn invoke(
        &self,
        context: &PrincipalContext,
        request: InvokeRequest,
    ) -> Result<OperationResult, OperationError> {
        if !is_slack_operation(&request.operation_ref) {
            return Err(operation_not_found());
        }
        let connection = lock(&self.metadata)
            .connections
            .iter()
            .find(|connection| {
                connection.connection_ref == request.connection_ref
                    && self.connection_is_admitted(connection)
                    && self.connection_owned_by(connection, context)
            })
            .cloned()
            .ok_or_else(operation_not_granted)?;
        if request.description_ref != self.description_ref(context, &request.operation_ref) {
            return Err(OperationError::new(
                OperationErrorCode::StaleAuthority,
                "operation description lease is stale",
                false,
            ));
        }
        if operation_approval(&request.operation_ref) == ApprovalPosture::Required
            && request.approval_evidence_ref.is_none()
        {
            return Err(OperationError::new(
                OperationErrorCode::ApprovalRequired,
                "this Slack write requires correlated approval evidence",
                false,
            ));
        }
        let operation = connector_resolve::document::operation(&request.operation_ref)
            .ok_or_else(operation_not_found)?;
        let validator = jsonschema::validator_for(operation.input_schema())
            .map_err(|_| operation_unavailable())?;
        if !validator.is_valid(&request.input) {
            return Err(operation_invalid());
        }
        let credential_name = if operation_effect(&request.operation_ref) == EffectClass::ReadOnly {
            USER_TOKEN_CREDENTIAL
        } else {
            BOT_TOKEN_CREDENTIAL
        };
        let credential_ref = self
            .connection_credential_ref(&connection, credential_name)
            .map_err(|_| operation_not_granted())?;
        let credential = self
            .credential_store
            .get(&credential_ref)
            .await
            .map_err(|_| operation_not_granted())?;
        let declared_name = if credential_name == USER_TOKEN_CREDENTIAL {
            "slack.user_token"
        } else {
            "slack.bot_token"
        };
        let assembled = connector_resolve::auth::Assembled::new(
            declared_name,
            credential.expose_secret().to_owned(),
            catalog::Placement::Header {
                name: "Authorization",
                prefix: "Bearer ",
            },
        );
        drop(credential);
        let plan = connector_resolve::resolve(
            operation,
            SLACK_ORIGIN,
            &request.input,
            &BTreeMap::new(),
            &[assembled],
        )
        .map_err(|_| operation_invalid())?;
        let target = url::Url::parse(&plan.request.url).map_err(|_| operation_unavailable())?;
        if target.scheme() != "https"
            || target.host_str() != Some("slack.com")
            || target.port_or_known_default() != Some(443)
            || !target.username().is_empty()
            || target.password().is_some()
            || target.fragment().is_some()
        {
            return Err(operation_not_granted());
        }
        let method = reqwest::Method::from_bytes(plan.request.method.as_bytes())
            .map_err(|_| operation_unavailable())?;
        let mut outbound = self.http.request(method, target);
        for (name, value) in plan.request.headers {
            outbound = outbound.header(name, value);
        }
        if let Some(body) = plan.request.body {
            outbound = outbound.body(body);
        }
        let audit_ref = format!(
            "audit:slack:{}",
            random_uuid().map_err(|_| operation_unavailable())?
        );
        let audit = AuditEvent {
            audit_ref: &audit_ref,
            operation_ref: &request.operation_ref,
            connection_ref: &request.connection_ref,
            tenant_id: context.tenant_id(),
            actor_subject: context.actor_subject(),
            outcome: "attempted",
        };
        // No request reaches Slack unless the attempted record and capacity for its terminal
        // outcome are durable first.
        self.audit
            .begin(audit)
            .map_err(|_| operation_unavailable())?;
        let dispatched = async {
            let mut response = outbound.send().await.map_err(|_| operation_unavailable())?;
            if !response.status().is_success()
                || response
                    .content_length()
                    .is_some_and(|length| length > protocol::operation::MAX_RESULT_BYTES as u64)
            {
                return Err(operation_unavailable());
            }
            let mut bytes = Vec::new();
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|_| operation_unavailable())?
            {
                if bytes.len().saturating_add(chunk.len()) > protocol::operation::MAX_RESULT_BYTES {
                    return Err(OperationError::new(
                        OperationErrorCode::ResultTooLarge,
                        "Slack operation result exceeds the admitted bound",
                        false,
                    ));
                }
                bytes.extend_from_slice(&chunk);
            }
            serde_json::from_slice(&bytes).map_err(|_| operation_unavailable())
        }
        .await;
        let output = match dispatched {
            Ok(output) => output,
            Err(error) => {
                self.audit
                    .finish(AuditEvent {
                        outcome: "indeterminate",
                        ..audit
                    })
                    .map_err(|_| post_dispatch_error(&request.operation_ref))?;
                return Err(
                    if operation_effect(&request.operation_ref) == EffectClass::ReadOnly {
                        error
                    } else {
                        post_dispatch_error(&request.operation_ref)
                    },
                );
            }
        };
        self.audit
            .finish(AuditEvent {
                outcome: "completed",
                ..audit
            })
            .map_err(|_| post_dispatch_error(&request.operation_ref))?;
        Ok(OperationResult::Invoke(InvocationResult {
            operation_ref: request.operation_ref,
            output,
            connector_audit_ref: audit_ref,
            execution_ref: None,
        }))
    }

    fn describe(&self, connection: &StoredConnection) -> ConnectionDescription {
        let state = lock(&self.channel_states)
            .get(&connection.connection_ref)
            .copied()
            .unwrap_or(ChannelState::Starting);
        ConnectionDescription {
            summary: self.connection_summary(connection),
            channels: vec![ConnectionChannelSummary {
                channel_ref: channel_ref(connection),
                binding_ref: SOCKET_BINDING_REF.to_owned(),
                state,
                events: connection.allowed_events.clone(),
            }],
        }
    }

    fn connection_summary(&self, connection: &StoredConnection) -> ConnectionSummary {
        let state = lock(&self.channel_states)
            .get(&connection.connection_ref)
            .copied()
            .unwrap_or(ChannelState::Starting);
        let state = match state {
            ChannelState::Starting => ConnectionState::Authorized,
            ChannelState::Connected => ConnectionState::Callable,
            ChannelState::Reconnecting | ChannelState::Stopped => ConnectionState::Degraded,
        };
        ConnectionSummary {
            connection_ref: connection.connection_ref.clone(),
            integration_ref: INTEGRATION_REF.to_owned(),
            label: connection.label.clone(),
            state,
            initiation: initiation(connection.initiation),
            route: protocol::connection::ConnectionRoute::Direct,
        }
    }

    fn require_channel(
        &self,
        requested: &str,
        context: &PrincipalContext,
    ) -> Result<StoredConnection, EventError> {
        lock(&self.metadata)
            .connections
            .iter()
            .find(|connection| {
                channel_ref(connection) == requested
                    && self.connection_is_admitted(connection)
                    && self.connection_owned_by(connection, context)
            })
            .cloned()
            .ok_or_else(event_not_found)
    }

    fn has_channel(&self, requested: &str) -> bool {
        lock(&self.metadata).connections.iter().any(|connection| {
            channel_ref(connection) == requested && self.connection_is_admitted(connection)
        })
    }

    async fn create_session(
        self: &Arc<Self>,
        owner: &PrincipalContext,
        label: String,
    ) -> Result<ConnectSessionStatus, ConnectionError> {
        self.expire_hosted_sessions();
        let id = random_uuid().map_err(|_| connection_unavailable())?;
        let session_ref = format!("connect-session:{id}");
        let expires_at_unix_ms = now_ms()
            .and_then(|now| {
                now.checked_add(self.policy.connect_session_ttl_seconds.saturating_mul(1000))
            })
            .ok_or_else(connection_unavailable)?;
        let status = match &self.completion_mode {
            CompletionMode::Local => {
                let directory = self.state_root.join("connect-sessions");
                let endpoint = BoundCompletionEndpoint::bind(&directory, &id)
                    .map_err(|_| connection_unavailable())?;
                let endpoint_path = endpoint.path().to_path_buf();
                let browser_completion_url = endpoint.browser_url();
                let status = match lock(&self.sessions).reserve_with_browser(
                    session_ref.clone(),
                    label,
                    expires_at_unix_ms,
                    endpoint_path.display().to_string(),
                    Some(browser_completion_url),
                ) {
                    Ok(status) => status,
                    Err(error) => {
                        drop(endpoint);
                        return Err(connect_session_error(error));
                    }
                };
                let inner = Arc::clone(self);
                let task_session_ref = session_ref.clone();
                lock(&self.tasks).push(tokio::spawn(async move {
                    inner.serve_completion(endpoint, task_session_ref).await;
                }));
                status
            }
            CompletionMode::Hosted { public_origin } => {
                let capability = random_capability().map_err(|_| connection_unavailable())?;
                let mut url = public_origin.clone();
                url.path_segments_mut()
                    .map_err(|_| connection_unavailable())?
                    .push("connect-sessions")
                    .push(&session_ref);
                url.set_fragment(Some(&format!("token={capability}")));
                let status = lock(&self.sessions)
                    .reserve_browser(session_ref.clone(), label, expires_at_unix_ms, url.into())
                    .map_err(connect_session_error)?;
                lock(&self.hosted_sessions).insert(
                    session_ref.clone(),
                    HostedSession {
                        capability_sha256: Sha256::digest(capability.as_bytes()).into(),
                        expires_at_unix_ms,
                    },
                );
                status
            }
        };
        lock(&self.session_owners).insert(session_ref, owner.subject().to_owned());
        Ok(status)
    }

    fn session_status(&self, session_ref: &str) -> Option<ConnectSessionStatus> {
        self.expire_hosted_sessions();
        lock(&self.sessions).status(session_ref)
    }

    fn expire_hosted_sessions(&self) {
        let Some(now) = now_ms() else {
            return;
        };
        let expired = {
            let mut hosted_sessions = lock(&self.hosted_sessions);
            let expired = hosted_sessions
                .iter()
                .filter(|(_, session)| now >= session.expires_at_unix_ms)
                .map(|(session_ref, _)| session_ref.clone())
                .collect::<Vec<_>>();
            for session_ref in &expired {
                hosted_sessions.remove(session_ref);
            }
            expired
        };
        let mut sessions = lock(&self.sessions);
        let mut session_owners = lock(&self.session_owners);
        for session_ref in expired {
            let _ = sessions.finish(&session_ref, ConnectSessionTerminal::Expired);
            session_owners.remove(&session_ref);
        }
    }

    async fn serve_completion(
        self: Arc<Self>,
        endpoint: BoundCompletionEndpoint,
        session_ref: String,
    ) {
        let submission = match endpoint
            .receive(
                Duration::from_secs(self.policy.connect_session_ttl_seconds),
                Duration::from_secs(30),
                MAX_APP_TOKEN_BYTES,
            )
            .await
        {
            Ok(submission) => submission,
            Err(CompletionTransportError::Expired) => {
                let _ = lock(&self.sessions).finish(&session_ref, ConnectSessionTerminal::Expired);
                return;
            }
            Err(_) => {
                let _ = lock(&self.sessions).finish(&session_ref, ConnectSessionTerminal::Failed);
                return;
            }
        };
        let secret = submission.secret();
        let valid = secret.expose_secret().starts_with("xapp-");
        let result = if valid {
            let owner = lock(&self.session_owners)
                .get(&session_ref)
                .cloned()
                .ok_or_else(|| SlackError::new("connect-session"));
            match owner {
                Ok(owner_subject) => {
                    self.complete_connection(
                        &session_ref,
                        owner_subject,
                        String::new(),
                        SlackCredentials {
                            app_token: Secret::new(secret.expose_secret()),
                            bot_token: None,
                            user_token: None,
                        },
                    )
                    .await
                }
                Err(error) => Err(error),
            }
        } else {
            Err(SlackError::new("credential-shape"))
        };
        let accepted = match result {
            Ok(connection_ref) => {
                let _ = lock(&self.sessions).finish(
                    &session_ref,
                    ConnectSessionTerminal::Completed { connection_ref },
                );
                true
            }
            Err(_) => {
                let _ = lock(&self.sessions).finish(&session_ref, ConnectSessionTerminal::Failed);
                false
            }
        };
        let _ = submission.respond(accepted).await;
    }

    async fn complete_connection(
        self: &Arc<Self>,
        session_ref: &str,
        owner_subject: String,
        team_id: String,
        credentials: SlackCredentials,
    ) -> Result<String, SlackError> {
        let label = lock(&self.sessions)
            .pending_label(session_ref)
            .map_err(|_| SlackError::new("connect-session"))?;
        let instance_id = random_uuid()?;
        let connection_ref = format!("connection:slack:{instance_id}");
        let connection = StoredConnection {
            connection_ref: connection_ref.clone(),
            instance_id: instance_id.clone(),
            label,
            grant_ref: self.policy.grant_ref.clone(),
            initiation: self.policy.initiation,
            allowed_events: self.policy.allowed_events.clone(),
            owner_subject,
            team_id,
        };
        let app_credential_ref = self.app_credential_ref()?;
        if !lock(&self.metadata).connections.is_empty() {
            let current = self
                .credential_store
                .get(&app_credential_ref)
                .await
                .map_err(|_| SlackError::new("credential-resolve"))?;
            if !constant_time_equal(
                current.expose_secret().as_bytes(),
                credentials.app_token.expose_secret().as_bytes(),
            ) {
                return Err(SlackError::new("app-token-conflict"));
            }
        }
        let bot_credential_ref =
            self.connection_credential_ref(&connection, BOT_TOKEN_CREDENTIAL)?;
        let user_credential_ref =
            self.connection_credential_ref(&connection, USER_TOKEN_CREDENTIAL)?;
        let (transaction, generation) = self.reserve_transaction()?;
        let mut batch = SecretBatch::new(
            CredentialScope::new(self.tenant_id(), AUTHORITY)
                .map_err(|_| SlackError::new("credential-address"))?,
        );
        batch
            .put(app_credential_ref, credentials.app_token)
            .map_err(|_| SlackError::new("credential-batch"))?;
        if let Some(bot_token) = credentials.bot_token {
            batch
                .put(bot_credential_ref, bot_token)
                .map_err(|_| SlackError::new("credential-batch"))?;
        }
        if let Some(user_token) = credentials.user_token {
            batch
                .put(user_credential_ref, user_token)
                .map_err(|_| SlackError::new("credential-batch"))?;
        }
        let digest = proposal_digest(&batch);
        self.credential_store
            .prepare(transaction, digest, &batch)
            .await
            .map_err(|_| SlackError::new("credential-prepare"))?;

        let transaction_hex = hex::encode(transaction.protocol_bytes());
        let pending_persisted = {
            let mut state = lock(&self.metadata);
            state.pending.push(PendingCommit {
                transaction_id: transaction_hex.clone(),
                connection: connection.clone(),
            });
            let persisted = self.persist_metadata(&state).is_ok();
            if !persisted {
                state
                    .pending
                    .retain(|pending| pending.transaction_id != transaction_hex);
            }
            persisted
        };
        if !pending_persisted {
            let _ = self.credential_store.abort(transaction).await;
            return Err(SlackError::new("connection-state"));
        }

        self.credential_store
            .commit(transaction)
            .await
            .map_err(|_| SlackError::new("credential-commit"))?;
        {
            let mut state = lock(&self.metadata);
            let prior = state.clone();
            state
                .pending
                .retain(|pending| pending.transaction_id != transaction_hex);
            state.connections.push(connection.clone());
            state
                .connections
                .sort_by(|a, b| a.connection_ref.cmp(&b.connection_ref));
            if let Err(error) = self.persist_metadata(&state) {
                *state = prior;
                return Err(error);
            }
        }
        let _ = self.credential_store.reclaim(generation).await;
        self.start_supervisor(connection);
        Ok(connection_ref)
    }

    fn reserve_transaction(
        &self,
    ) -> Result<(SecretTransactionId, SecretTransactionGeneration), SlackError> {
        let mut state = lock(&self.metadata);
        let generation_value = state.next_transaction_generation;
        let generation =
            SecretTransactionGeneration::from_protocol_bytes(generation_value.to_be_bytes())
                .ok_or_else(|| SlackError::new("transaction-generation"))?;
        state.next_transaction_generation = generation_value
            .checked_add(1)
            .ok_or_else(|| SlackError::new("transaction-generation"))?;
        self.persist_metadata(&state)?;
        let mut nonce = [0_u8; 24];
        getrandom::fill(&mut nonce).map_err(|_| SlackError::new("randomness"))?;
        Ok((SecretTransactionId::new(generation, nonce), generation))
    }

    async fn recover_pending(&self) -> Result<(), SlackError> {
        let pending = lock(&self.metadata).pending.clone();
        for record in pending {
            let transaction = decode_transaction(&record.transaction_id)?;
            match self
                .credential_store
                .state(transaction)
                .await
                .map_err(|_| SlackError::new("credential-recovery"))?
            {
                SecretTransactionState::Prepared => {
                    self.credential_store
                        .commit(transaction)
                        .await
                        .map_err(|_| SlackError::new("credential-recovery"))?;
                }
                SecretTransactionState::Committed => {}
                SecretTransactionState::Absent => {
                    let mut state = lock(&self.metadata);
                    state
                        .pending
                        .retain(|candidate| candidate.transaction_id != record.transaction_id);
                    self.persist_metadata(&state)?;
                    continue;
                }
            }
            let mut state = lock(&self.metadata);
            state
                .pending
                .retain(|candidate| candidate.transaction_id != record.transaction_id);
            if !state
                .connections
                .iter()
                .any(|connection| connection.connection_ref == record.connection.connection_ref)
            {
                state.connections.push(record.connection);
                state
                    .connections
                    .sort_by(|a, b| a.connection_ref.cmp(&b.connection_ref));
            }
            self.persist_metadata(&state)?;
        }
        Ok(())
    }

    async fn verify_workspace_credentials(
        &self,
        credentials: &SlackCredentials,
    ) -> Result<String, SlackError> {
        let bot = credentials
            .bot_token
            .as_ref()
            .ok_or_else(|| SlackError::new("credential-shape"))?;
        let user = credentials
            .user_token
            .as_ref()
            .ok_or_else(|| SlackError::new("credential-shape"))?;
        let bot_team = self.auth_test(bot).await?;
        let user_team = self.auth_test(user).await?;
        if bot_team != user_team {
            return Err(SlackError::new("credential-workspace"));
        }
        if lock(&self.metadata)
            .connections
            .iter()
            .any(|connection| connection.team_id == bot_team)
        {
            return Err(SlackError::new("connection-conflict"));
        }
        Ok(bot_team)
    }

    async fn auth_test(&self, token: &Secret) -> Result<String, SlackError> {
        let mut response = self
            .http
            .post(AUTH_TEST)
            .bearer_auth(token.expose_secret())
            .send()
            .await
            .map_err(|_| SlackError::new("credential-verify-unavailable"))?;
        if !response.status().is_success()
            || response
                .content_length()
                .is_some_and(|length| length > MAX_AUTH_TEST_RESPONSE_BYTES as u64)
        {
            return Err(SlackError::new("credential-verify-unavailable"));
        }
        let status = response.status();
        let content_length = response.content_length();
        let mut bytes = Zeroizing::new(Vec::with_capacity(MAX_AUTH_TEST_RESPONSE_BYTES));
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| SlackError::new("credential-verify-unavailable"))?
        {
            if bytes.len().saturating_add(chunk.len()) > MAX_AUTH_TEST_RESPONSE_BYTES {
                return Err(SlackError::new("credential-verify-unavailable"));
            }
            bytes.extend_from_slice(&chunk);
        }
        classify_auth_test_response(status, content_length, &bytes)
    }

    fn app_credential_ref(&self) -> Result<CredentialRef, SlackError> {
        CredentialRef::new(self.tenant_id(), AUTHORITY, SERVICE, APP_TOKEN_CREDENTIAL)
            .map_err(|_| SlackError::new("credential-address"))
    }

    fn connection_credential_ref(
        &self,
        connection: &StoredConnection,
        credential: &str,
    ) -> Result<CredentialRef, SlackError> {
        CredentialRef::for_instance(
            self.tenant_id(),
            AUTHORITY,
            &connection.instance_id,
            SERVICE,
            credential,
        )
        .map_err(|_| SlackError::new("credential-address"))
    }

    fn start_supervisor(self: &Arc<Self>, connection: StoredConnection) {
        let state = *lock(&self.integration_channel_state);
        lock(&self.channel_states).insert(connection.connection_ref, state);
        if !self.supervision_enabled {
            return;
        }
        let mut started = lock(&self.supervisor_started);
        if *started {
            return;
        }
        *started = true;
        drop(started);
        let inner = Arc::clone(self);
        let shutdown = self.shutdown.subscribe();
        lock(&self.tasks).push(tokio::spawn(async move {
            inner.supervise(shutdown).await;
        }));
    }

    async fn supervise(self: Arc<Self>, mut shutdown: watch::Receiver<bool>) {
        let mut backoff = Duration::from_secs(1);
        loop {
            if *shutdown.borrow() {
                break;
            }
            self.set_channel_state(ChannelState::Reconnecting);
            let outcome = self.run_socket(&mut shutdown).await;
            if *shutdown.borrow() {
                break;
            }
            if outcome.is_ok() {
                backoff = Duration::from_secs(1);
            }
            tokio::select! {
                _ = tokio::time::sleep(backoff) => {}
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() { break; }
                }
            }
            backoff = (backoff * 2).min(Duration::from_secs(30));
        }
        self.set_channel_state(ChannelState::Stopped);
    }

    fn set_channel_state(&self, state: ChannelState) {
        *lock(&self.integration_channel_state) = state;
        let connections = lock(&self.metadata).connections.clone();
        let mut states = lock(&self.channel_states);
        for connection in connections {
            if self.connection_is_admitted(&connection) {
                states.insert(connection.connection_ref, state);
            }
        }
    }

    async fn run_socket(&self, shutdown: &mut watch::Receiver<bool>) -> Result<(), SlackError> {
        let credential_ref = self.app_credential_ref()?;
        let token = self
            .credential_store
            .get(&credential_ref)
            .await
            .map_err(|_| SlackError::new("credential-resolve"))?;
        let mut response = self
            .http
            .post(APPS_CONNECTIONS_OPEN)
            .bearer_auth(token.expose_secret())
            .send()
            .await
            .map_err(|_| SlackError::new("socket-ticket-request"))?;
        drop(token);
        if !response.status().is_success()
            || response
                .content_length()
                .is_some_and(|size| size > 64 * 1024)
        {
            return Err(SlackError::new("socket-ticket-response"));
        }
        let mut bytes = Zeroizing::new(Vec::with_capacity(
            response.content_length().unwrap_or(0).min(64 * 1024) as usize,
        ));
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| SlackError::new("socket-ticket-response"))?
        {
            if bytes
                .len()
                .checked_add(chunk.len())
                .is_none_or(|size| size > 64 * 1024)
            {
                return Err(SlackError::new("socket-ticket-response"));
            }
            bytes.extend_from_slice(&chunk);
        }
        let ticket: SocketTicket = serde_json::from_slice(&bytes)
            .map_err(|_| SlackError::new("socket-ticket-response"))?;
        if !ticket.ok {
            return Err(SlackError::new("socket-ticket-refused"));
        }
        let url = Zeroizing::new(
            ticket
                .url
                .ok_or_else(|| SlackError::new("socket-ticket-response"))?,
        );
        validate_socket_url(&url)?;
        let websocket = WebSocketConfig::default()
            .max_message_size(Some(MAX_SOCKET_MESSAGE_BYTES))
            .max_frame_size(Some(MAX_SOCKET_MESSAGE_BYTES));
        let (mut socket, _) =
            tokio_tungstenite::connect_async_with_config(&*url, Some(websocket), false)
                .await
                .map_err(|_| SlackError::new("socket-connect"))?;
        self.set_channel_state(ChannelState::Connected);
        loop {
            tokio::select! {
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        let _ = socket.close(None).await;
                        return Ok(());
                    }
                }
                message = socket.next() => {
                    match message {
                        Some(Ok(Message::Text(text))) => {
                            self.handle_socket_text(text.as_ref(), &mut socket).await?;
                        }
                        Some(Ok(Message::Ping(payload))) => {
                            socket.send(Message::Pong(payload)).await.map_err(|_| SlackError::new("socket-write"))?;
                        }
                        Some(Ok(Message::Close(_))) | None => return Err(SlackError::new("socket-closed")),
                        Some(Ok(_)) => {}
                        Some(Err(_)) => return Err(SlackError::new("socket-read")),
                    }
                }
            }
        }
    }

    async fn handle_socket_text<S>(&self, text: &str, socket: &mut S) -> Result<(), SlackError>
    where
        S: futures_util::Sink<Message> + Unpin,
    {
        if text.len() > MAX_SOCKET_MESSAGE_BYTES {
            return Err(SlackError::new("socket-message-bound"));
        }
        let envelope: SocketEnvelope =
            serde_json::from_str(text).map_err(|_| SlackError::new("socket-envelope"))?;
        if envelope.kind == "disconnect" {
            return Err(SlackError::new("socket-refresh"));
        }
        let Some(envelope_id) = envelope.envelope_id else {
            return Ok(());
        };
        if envelope_id.is_empty() || envelope_id.len() > 512 {
            return Err(SlackError::new("socket-envelope"));
        }
        if envelope.kind == "events_api" {
            let connection = envelope
                .payload
                .as_ref()
                .and_then(|payload| payload.get("team_id"))
                .and_then(Value::as_str)
                .and_then(|team_id| {
                    lock(&self.metadata)
                        .connections
                        .iter()
                        .find(|connection| {
                            connection.team_id == team_id && self.connection_is_admitted(connection)
                        })
                        .cloned()
                });
            if let Some(connection) = connection {
                if let Some((delivery_id, event_type, payload)) =
                    project_data_event(envelope.payload.as_ref(), &connection.allowed_events)
                {
                    self.event_store
                        .append(&connection, &delivery_id, &event_type, payload)?;
                }
            }
        }
        let acknowledgement = serde_json::to_string(&serde_json::json!({
            "envelope_id": envelope_id,
        }))
        .map_err(|_| SlackError::new("socket-ack"))?;
        socket
            .send(Message::Text(acknowledgement.into()))
            .await
            .map_err(|_| SlackError::new("socket-ack"))
    }
}

impl AuditJournal {
    fn new(path: PathBuf, hosted_state: Option<PostgresState>) -> Self {
        Self {
            path,
            hosted_state,
            state: Mutex::new(AuditJournalState::default()),
        }
    }

    fn current_len(&self) -> Result<u64, SlackError> {
        if let Some(hosted_state) = &self.hosted_state {
            return hosted_state
                .read(AUDIT_STATE_KEY, MAX_AUDIT_BYTES as usize)
                .map_err(|_| SlackError::new("audit-store"))?
                .map_or(Ok(0), |body| {
                    u64::try_from(body.len()).map_err(|_| SlackError::new("audit-bound"))
                });
        }
        open_owner_append(&self.path)?
            .metadata()
            .map(|metadata| metadata.len())
            .map_err(|_| SlackError::new("audit-store"))
    }

    fn append_line(&self, line: &[u8]) -> Result<(), SlackError> {
        if let Some(hosted_state) = &self.hosted_state {
            return hosted_state
                .append(AUDIT_STATE_KEY, line, MAX_AUDIT_BYTES as usize)
                .map(|_| ())
                .map_err(|error| match error {
                    hosted_state::StateError::Capacity => SlackError::new("audit-bound"),
                    _ => SlackError::new("audit-store"),
                });
        }
        let mut file = open_owner_append(&self.path)?;
        file.write_all(line)
            .and_then(|()| file.sync_data())
            .map_err(|_| SlackError::new("audit-store"))
    }

    fn begin(&self, event: AuditEvent<'_>) -> Result<(), SlackError> {
        if event.outcome != "attempted" {
            return Err(SlackError::new("audit-outcome"));
        }
        let mut state = lock(&self.state);
        if state.terminal_reservations.contains_key(event.audit_ref) {
            return Err(SlackError::new("audit-duplicate"));
        }
        let attempted = audit_line(event, now_ms().ok_or_else(|| SlackError::new("clock"))?)?;
        let terminal_reservation = u64::try_from(
            audit_line(
                AuditEvent {
                    outcome: "indeterminate",
                    ..event
                },
                u64::MAX,
            )?
            .len(),
        )
        .map_err(|_| SlackError::new("audit-bound"))?;
        let outstanding = state
            .terminal_reservations
            .values()
            .try_fold(0_u64, |sum, value| sum.checked_add(*value))
            .ok_or_else(|| SlackError::new("audit-bound"))?;
        let current = self.current_len()?;
        let attempted_bytes =
            u64::try_from(attempted.len()).map_err(|_| SlackError::new("audit-bound"))?;
        if current
            .checked_add(outstanding)
            .and_then(|length| length.checked_add(attempted_bytes))
            .and_then(|length| length.checked_add(terminal_reservation))
            .is_none_or(|length| length > MAX_AUDIT_BYTES)
        {
            return Err(SlackError::new("audit-bound"));
        }
        self.append_line(&attempted)?;
        state
            .terminal_reservations
            .insert(event.audit_ref.to_owned(), terminal_reservation);
        Ok(())
    }

    fn finish(&self, event: AuditEvent<'_>) -> Result<(), SlackError> {
        if !matches!(event.outcome, "completed" | "indeterminate") {
            return Err(SlackError::new("audit-outcome"));
        }
        let mut state = lock(&self.state);
        let reservation = state
            .terminal_reservations
            .get(event.audit_ref)
            .copied()
            .ok_or_else(|| SlackError::new("audit-missing"))?;
        let terminal = audit_line(event, now_ms().ok_or_else(|| SlackError::new("clock"))?)?;
        let terminal_bytes =
            u64::try_from(terminal.len()).map_err(|_| SlackError::new("audit-bound"))?;
        if terminal_bytes > reservation {
            return Err(SlackError::new("audit-bound"));
        }
        let outstanding_other = state
            .terminal_reservations
            .values()
            .try_fold(0_u64, |sum, value| sum.checked_add(*value))
            .and_then(|sum| sum.checked_sub(reservation))
            .ok_or_else(|| SlackError::new("audit-bound"))?;
        if self
            .current_len()?
            .checked_add(outstanding_other)
            .and_then(|length| length.checked_add(terminal_bytes))
            .is_none_or(|length| length > MAX_AUDIT_BYTES)
        {
            return Err(SlackError::new("audit-bound"));
        }
        self.append_line(&terminal)?;
        state.terminal_reservations.remove(event.audit_ref);
        Ok(())
    }
}

fn audit_line(event: AuditEvent<'_>, at_unix_ms: u64) -> Result<Vec<u8>, SlackError> {
    let mut line = serde_json::to_vec(&serde_json::json!({
        "at_unix_ms": at_unix_ms,
        "event": event,
    }))
    .map_err(|_| SlackError::new("audit-store"))?;
    line.push(b'\n');
    Ok(line)
}

impl EventStore {
    fn open(path: PathBuf, hosted_state: Option<PostgresState>) -> Result<Self, SlackError> {
        let events = if let Some(hosted_state) = &hosted_state {
            let bytes = hosted_state
                .read(EVENT_STATE_KEY, MAX_EVENT_STORE_BYTES as usize)
                .map_err(|_| SlackError::new("event-store"))?
                .unwrap_or_default();
            decode_events(&bytes)?
        } else {
            read_events(&path)?
        };
        Ok(Self {
            path,
            hosted_state,
            events: Mutex::new(events),
            notify: Notify::new(),
        })
    }

    fn append(
        &self,
        connection: &StoredConnection,
        delivery_id: &str,
        event_type: &str,
        payload: Value,
    ) -> Result<(), SlackError> {
        let mut events = lock(&self.events);
        if events.iter().any(|stored| {
            stored.delivery_id == delivery_id
                && stored.event.connection_ref == connection.connection_ref
        }) {
            return Ok(());
        }
        if events.len() >= MAX_STORED_EVENTS {
            return Err(SlackError::new("event-store-capacity"));
        }
        let sequence = events
            .last()
            .map_or(1, |event| event.sequence.saturating_add(1));
        let stored = StoredEvent {
            sequence,
            delivery_id: delivery_id.to_owned(),
            event: DataEvent {
                event_ref: format!("event:{}", random_uuid()?),
                channel_ref: channel_ref(connection),
                connection_ref: connection.connection_ref.clone(),
                integration_ref: INTEGRATION_REF.to_owned(),
                event_type: event_type.to_owned(),
                provenance: EventProvenance::Native,
                received_at_unix_ms: now_ms().ok_or_else(|| SlackError::new("clock"))?,
                payload,
            },
        };
        let mut line = serde_json::to_vec(&stored).map_err(|_| SlackError::new("event-store"))?;
        line.push(b'\n');
        if let Some(hosted_state) = &self.hosted_state {
            hosted_state
                .append(EVENT_STATE_KEY, &line, MAX_EVENT_STORE_BYTES as usize)
                .map_err(|error| match error {
                    hosted_state::StateError::Capacity => SlackError::new("event-store-capacity"),
                    _ => SlackError::new("event-store"),
                })?;
        } else {
            let mut file = open_owner_append(&self.path)?;
            let current = file
                .metadata()
                .map_err(|_| SlackError::new("event-store"))?
                .len();
            if current
                .checked_add(line.len() as u64)
                .is_none_or(|size| size > MAX_EVENT_STORE_BYTES)
            {
                return Err(SlackError::new("event-store-capacity"));
            }
            file.write_all(&line)
                .and_then(|()| file.sync_data())
                .map_err(|_| SlackError::new("event-store"))?;
        }
        events.push(stored);
        drop(events);
        self.notify.notify_waiters();
        Ok(())
    }

    async fn receive(
        &self,
        channel_ref: &str,
        after: u64,
        limit: usize,
        wait: Duration,
    ) -> (Vec<DataEvent>, u64) {
        let select = || {
            lock(&self.events)
                .iter()
                .filter(|stored| stored.sequence > after && stored.event.channel_ref == channel_ref)
                .take(limit)
                .cloned()
                .collect::<Vec<_>>()
        };
        let mut selected = select();
        if selected.is_empty() && !wait.is_zero() {
            let notified = self.notify.notified();
            tokio::pin!(notified);
            if select().is_empty() {
                let _ = tokio::time::timeout(wait, &mut notified).await;
            }
            selected = select();
        }
        let next = selected.last().map_or(after, |event| event.sequence);
        (
            selected.into_iter().map(|stored| stored.event).collect(),
            next,
        )
    }

    fn replay(&self, event_ref: &str) -> Option<DataEvent> {
        lock(&self.events)
            .iter()
            .find(|stored| stored.event.event_ref == event_ref)
            .map(|stored| stored.event.clone())
    }
}

fn project_data_event(
    payload: Option<&Value>,
    allowed_events: &[String],
) -> Option<(String, String, Value)> {
    let payload = payload?.as_object()?;
    let delivery_id = payload.get("event_id")?.as_str()?;
    if delivery_id.is_empty() || delivery_id.len() > 512 {
        return None;
    }
    let event = payload.get("event")?.as_object()?;
    let event_type = event.get("type")?.as_str()?;
    let kind = match event_type {
        "app_mention" => "app_mention",
        "message" if event.get("channel_type").and_then(Value::as_str) == Some("channel") => {
            "message.channels"
        }
        _ => return None,
    };
    if !allowed_events.iter().any(|allowed| allowed == kind) {
        return None;
    }
    if event_type == "message" && (event.contains_key("bot_id") || event.contains_key("subtype")) {
        return None;
    }
    let projected = Value::Object(event.clone());
    if serde_json::to_vec(&projected)
        .map_or(true, |bytes| bytes.len() > protocol::event::MAX_EVENT_BYTES)
    {
        return None;
    }
    Some((delivery_id.to_owned(), kind.to_owned(), projected))
}

fn event_channel_summary(connection: &StoredConnection) -> EventChannelSummary {
    EventChannelSummary {
        channel_ref: channel_ref(connection),
        connection_ref: connection.connection_ref.clone(),
        integration_ref: INTEGRATION_REF.to_owned(),
        binding_ref: SOCKET_BINDING_REF.to_owned(),
        events: connection.allowed_events.clone(),
    }
}

fn is_slack_operation(operation_ref: &str) -> bool {
    SLACK_OPERATIONS.contains(&operation_ref)
}

fn operation_title(operation_ref: &str) -> &'static str {
    match operation_ref {
        "slack-chat-post-message" => "Post Slack message",
        "slack-conversations-history" => "Read Slack conversation history",
        "slack-users-info" => "Look up Slack user",
        "slack-reactions-add" => "Add Slack reaction",
        _ => "Slack operation",
    }
}

fn operation_effect(operation_ref: &str) -> EffectClass {
    match operation_ref {
        "slack-conversations-history" | "slack-users-info" => EffectClass::ReadOnly,
        "slack-chat-post-message" | "slack-reactions-add" => EffectClass::Mutating,
        _ => EffectClass::Destructive,
    }
}

fn operation_approval(operation_ref: &str) -> ApprovalPosture {
    if operation_effect(operation_ref) == EffectClass::ReadOnly {
        ApprovalPosture::NotRequired
    } else {
        ApprovalPosture::Required
    }
}

fn operation_not_found() -> OperationError {
    OperationError::new(
        OperationErrorCode::NotFound,
        "Slack operation was not found",
        false,
    )
}

fn operation_not_granted() -> OperationError {
    OperationError::new(
        OperationErrorCode::NotGranted,
        "Slack operation is not granted for this Connection",
        false,
    )
}

fn operation_invalid() -> OperationError {
    OperationError::new(
        OperationErrorCode::InvalidInput,
        "Slack operation input is invalid",
        false,
    )
}

fn operation_unavailable() -> OperationError {
    OperationError::new(
        OperationErrorCode::Unavailable,
        "Slack operation is unavailable",
        true,
    )
}

fn post_dispatch_error(operation_ref: &str) -> OperationError {
    OperationError::new(
        OperationErrorCode::OutcomeUnknown,
        format!(
            "Slack operation {operation_ref} may have reached Slack; do not retry automatically"
        ),
        false,
    )
}

fn initiation(config: InitiationConfig) -> Vec<ConnectionInitiator> {
    match config {
        InitiationConfig::B10x => vec![ConnectionInitiator::B10x],
        InitiationConfig::Provider => vec![ConnectionInitiator::Provider],
        InitiationConfig::Both => vec![
            ConnectionInitiator::B10x,
            ConnectionInitiator::Provider,
        ],
    }
}

fn channel_ref(connection: &StoredConnection) -> String {
    format!("channel:slack:{}:socket", connection.instance_id)
}

fn proposal_digest(batch: &SecretBatch) -> SecretProposalDigest {
    let mut digest = Sha256::new();
    digest.update(b"b10x/slack-connect/v2\0");
    for (reference, secret) in batch
        .put_entries()
        .expect("Slack connect batches contain puts only")
    {
        digest.update(TenantLayout.render(reference).as_bytes());
        digest.update(b"\0");
        digest.update(secret.expose_secret().as_bytes());
        digest.update(b"\0");
    }
    SecretProposalDigest::from_protocol_bytes(digest.finalize().into())
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

fn decode_transaction(encoded: &str) -> Result<SecretTransactionId, SlackError> {
    let bytes = hex::decode(encoded).map_err(|_| SlackError::new("transaction-state"))?;
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| SlackError::new("transaction-state"))?;
    SecretTransactionId::from_protocol_bytes(bytes)
        .ok_or_else(|| SlackError::new("transaction-state"))
}

fn validate_socket_url(value: &str) -> Result<(), SlackError> {
    let url = url::Url::parse(value).map_err(|_| SlackError::new("socket-ticket-url"))?;
    let host = url
        .host_str()
        .ok_or_else(|| SlackError::new("socket-ticket-url"))?;
    if url.scheme() != "wss"
        || !(host == "slack.com" || host.ends_with(".slack.com"))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.port().is_some_and(|port| port != 443)
        || url.query().is_none()
    {
        return Err(SlackError::new("socket-ticket-url"));
    }
    Ok(())
}

fn random_uuid() -> Result<String, SlackError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| SlackError::new("randomness"))?;
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

fn read_state(path: &Path, hosted_state: Option<&PostgresState>) -> Result<StateFile, SlackError> {
    if let Some(hosted_state) = hosted_state {
        let Some(bytes) = hosted_state
            .read(CONNECTION_STATE_KEY, MAX_STATE_BYTES as usize)
            .map_err(|_| SlackError::new("connection-state"))?
        else {
            let state = StateFile::default();
            write_state(path, Some(hosted_state), &state)?;
            return Ok(state);
        };
        return decode_state(&bytes);
    }
    let Some(mut file) = open_owner_read(path, MAX_STATE_BYTES)? else {
        let state = StateFile::default();
        write_state(path, None, &state)?;
        return Ok(state);
    };
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|_| SlackError::new("connection-state"))?;
    decode_state(&bytes)
}

fn decode_state(bytes: &[u8]) -> Result<StateFile, SlackError> {
    let mut state: StateFile =
        serde_json::from_slice(bytes).map_err(|_| SlackError::new("connection-state"))?;
    if !matches!(state.version, 1 | STATE_VERSION) || state.next_transaction_generation == 0 {
        return Err(SlackError::new("connection-state-version"));
    }
    state.version = STATE_VERSION;
    Ok(state)
}

fn write_state(
    path: &Path,
    hosted_state: Option<&PostgresState>,
    state: &StateFile,
) -> Result<(), SlackError> {
    let bytes = serde_json::to_vec(state).map_err(|_| SlackError::new("connection-state"))?;
    if bytes.len() as u64 > MAX_STATE_BYTES {
        return Err(SlackError::new("connection-state-bound"));
    }
    if let Some(hosted_state) = hosted_state {
        return hosted_state
            .replace(CONNECTION_STATE_KEY, &bytes, MAX_STATE_BYTES as usize)
            .map_err(|_| SlackError::new("connection-state"));
    }
    let parent = path
        .parent()
        .ok_or_else(|| SlackError::new("connection-state"))?;
    ensure_owner_directory(parent)?;
    let temporary = parent.join(".connections.json.tmp");
    refuse_existing_non_owner_file(&temporary)?;
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(&temporary)
        .map_err(|_| SlackError::new("connection-state"))?;
    inspect_owner_file(&file, "connection-state")?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| SlackError::new("connection-state"))?;
    fs::rename(&temporary, path).map_err(|_| SlackError::new("connection-state"))?;
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| SlackError::new("connection-state"))
}

fn read_events(path: &Path) -> Result<Vec<StoredEvent>, SlackError> {
    let Some(mut file) = open_owner_read(path, MAX_EVENT_STORE_BYTES)? else {
        let _ = open_owner_append(path)?;
        return Ok(Vec::new());
    };
    let mut text = String::new();
    file.read_to_string(&mut text)
        .map_err(|_| SlackError::new("event-store"))?;
    decode_events(text.as_bytes())
}

fn decode_events(bytes: &[u8]) -> Result<Vec<StoredEvent>, SlackError> {
    let text = std::str::from_utf8(bytes).map_err(|_| SlackError::new("event-store"))?;
    let mut events = Vec::new();
    for line in text.lines() {
        if line.is_empty() || events.len() >= MAX_STORED_EVENTS {
            return Err(SlackError::new("event-store"));
        }
        let event: StoredEvent =
            serde_json::from_str(line).map_err(|_| SlackError::new("event-store"))?;
        if event.sequence
            != events
                .last()
                .map_or(1, |prior: &StoredEvent| prior.sequence + 1)
        {
            return Err(SlackError::new("event-store-sequence"));
        }
        events.push(event);
    }
    Ok(events)
}

fn open_owner_read(path: &Path, maximum: u64) -> Result<Option<File>, SlackError> {
    let file = match OpenOptions::new()
        .read(true)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(SlackError::new("owner-state")),
    };
    inspect_owner_file(&file, "owner-state")?;
    if file
        .metadata()
        .map_err(|_| SlackError::new("owner-state"))?
        .len()
        > maximum
    {
        return Err(SlackError::new("owner-state-bound"));
    }
    Ok(Some(file))
}

fn open_owner_append(path: &Path) -> Result<File, SlackError> {
    let parent = path
        .parent()
        .ok_or_else(|| SlackError::new("owner-state"))?;
    ensure_owner_directory(parent)?;
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(path)
        .map_err(|_| SlackError::new("owner-state"))?;
    inspect_owner_file(&file, "owner-state")?;
    Ok(file)
}

fn inspect_owner_file(file: &File, code: &'static str) -> Result<(), SlackError> {
    let metadata = file.metadata().map_err(|_| SlackError::new(code))?;
    if !metadata.file_type().is_file()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err(SlackError::new(code));
    }
    Ok(())
}

fn ensure_owner_directory(path: &Path) -> Result<(), SlackError> {
    if !path.exists() {
        fs::create_dir_all(path).map_err(|_| SlackError::new("owner-state-directory"))?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|_| SlackError::new("owner-state-directory"))?;
    }
    let metadata =
        fs::symlink_metadata(path).map_err(|_| SlackError::new("owner-state-directory"))?;
    if !metadata.file_type().is_dir()
        || metadata.file_type().is_symlink()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err(SlackError::new("owner-state-directory"));
    }
    Ok(())
}

fn refuse_existing_non_owner_file(path: &Path) -> Result<(), SlackError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(SlackError::new("owner-state")),
    };
    if !metadata.file_type().is_file()
        || metadata.file_type().is_symlink()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err(SlackError::new("owner-state"));
    }
    fs::remove_file(path).map_err(|_| SlackError::new("owner-state"))
}

fn connection_unavailable() -> ConnectionError {
    ConnectionError::new(
        ConnectionErrorCode::Unavailable,
        "connection management is temporarily unavailable",
        true,
    )
}

fn connect_session_error(error: ConnectSessionLifecycleError) -> ConnectionError {
    match error {
        ConnectSessionLifecycleError::Capacity => ConnectionError::new(
            ConnectionErrorCode::Conflict,
            "too many Connect Sessions are pending",
            true,
        ),
        _ => connection_unavailable(),
    }
}

fn connection_not_found() -> ConnectionError {
    ConnectionError::new(
        ConnectionErrorCode::NotFound,
        "connection or Connect Session was not found",
        false,
    )
}

fn event_not_found() -> EventError {
    EventError::new(
        EventErrorCode::NotFound,
        "event or channel was not found",
        false,
    )
}

fn event_invalid() -> EventError {
    EventError::new(
        EventErrorCode::InvalidInput,
        "event request is invalid",
        false,
    )
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| {
        mutex.clear_poison();
        poisoned.into_inner()
    })
}

include!("backend_tests.rs");

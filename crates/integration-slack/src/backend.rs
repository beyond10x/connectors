//! Slack Socket Mode Integration, Connection custody, and durable event delivery.

mod hosted_setup;
mod state_file;

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read as _, Write as _};
use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use base64::Engine as _;
use connect_session_transport::{
    remove_endpoint, BoundCompletionEndpoint, CompletionTransportError,
};
use connector_secrets::{
    CredentialRef, CredentialScope, Layout, PreparedSecretStore, Secret, SecretBatch,
    SecretProposalDigest, SecretTransactionGeneration, SecretTransactionId, SecretTransactionState,
    TenantLayout,
};
use connector_state::{StateError, StateStore};
use protocol::connection::{
    ChannelState, ChannelSummary as ConnectionChannelSummary, ConnectSessionStatus,
    ConnectionActor, ConnectionDescription, ConnectionError, ConnectionErrorCode,
    ConnectionInitiator, ConnectionRequest, ConnectionResult, ConnectionScope, ConnectionState,
    ConnectionSummary,
};
use protocol::datasource::{
    AccessMode as DatasourceAccessMode, Completeness as DatasourceCompleteness, DatasourceBinding,
    DatasourceDescription, DatasourceError, DatasourceErrorCode, DatasourcePage,
    DatasourceProvenance, DatasourceRead, DatasourceRecord, DatasourceRequest, DatasourceResult,
    DatasourceSummary, ReadRequest as DatasourceReadRequest, ReadVerb as DatasourceReadVerb,
    RecordView as DatasourceRecordView,
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
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use tokio::sync::{watch, Notify};
use tokio::task::JoinHandle;
use zeroize::Zeroizing;

use connectors_config::{
    InitiationConfig, SlackInstanceConfig, SlackInstanceProfile, SlackIntegrationConfig,
};
use service::{
    BackendCapabilities, BackendReadinessError, ConnectSessionAccess, ConnectSessionLifecycle,
    ConnectSessionLifecycleError, ConnectSessionTerminal, ConnectorBackend, EgressHttpRequest,
    EgressTransport, EgressWebSocket, EgressWebSocketFrame, HostedCompletionError,
    HostedCompletionPage, HostedCompletionSubmission, PrincipalContext,
};

use hosted_setup::{
    classify_auth_test_response, hosted_completion_error, parse_hosted_submission,
    random_capability, valid_hosted_capability, valid_slack_token, MAX_AUTH_TEST_RESPONSE_BYTES,
};
use state_file::{read_state, write_state};

const INTEGRATION_REF: &str = "slack";
const AUTHORITY: &str = "com.slack.api";
const SERVICE: &str = "default";
const APP_TOKEN_CREDENTIAL: &str = "app_token";
const BOT_TOKEN_CREDENTIAL: &str = "bot_token";
const USER_TOKEN_CREDENTIAL: &str = "user_token";
const SOCKET_BINDING_REF: &str = "com.slack.api:v1#socket";
// 4: `StoredConnection.purpose`. The field defaults, so an older state file still reads; the
// version moves so datasource bindings issued before it are refused rather than served without the
// hint a caller now expects to be there.
const STATE_VERSION: u8 = 4;
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
const USERS_INFO: &str = "https://slack.com/api/users.info";
const USERS_LIST: &str = "https://slack.com/api/users.list";
const CONVERSATIONS_LIST: &str = "https://slack.com/api/conversations.list";
const CONVERSATIONS_HISTORY: &str = "https://slack.com/api/conversations.history";
const USER_OAUTH_ACCESS: &str = "https://slack.com/api/oauth.v2.user.access";
const SLACK_ORIGIN: &str = "https://slack.com";
const PROFILE_ORG_BOT: &str = "slack.org_bot";
/// The one tenant-wide organisation bot install, whose credential has no instance in its address.
const ORG_BOT_CONNECTION: &str = "connection:slack:org-bot";
const PROFILE_ORG_USER: &str = "slack.org_user";
const PROFILE_COMPANION_BOT: &str = "slack.companion_bot";
const OAUTH_CLIENT_SECRET_CREDENTIAL: &str = "oauth_client_secret";
const OAUTH_REFRESH_TOKEN_CREDENTIAL: &str = "oauth_refresh_token";
const USER_OAUTH_SCOPES: &str = "channels:read,channels:history,groups:read,groups:history,im:read,im:history,mpim:read,mpim:history,users:read,users:read.email,chat:write,reactions:write";
const SLACK_OPERATIONS: [&str; 4] = [
    "slack-chat-post-message",
    "slack-conversations-history",
    "slack-users-info",
    "slack-reactions-add",
];
const SLACK_DATASOURCES: [&str; 2] = ["slack.conversations", "slack.users"];
const VALUE_PROJECTION_PROTOCOL: &str = "b10x.value-projection.v1";

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
    hosted_state: Option<Arc<dyn StateStore>>,
    credential_store: Arc<dyn PreparedSecretStore>,
    metadata: Mutex<StateFile>,
    sessions: Mutex<ConnectSessionLifecycle>,
    session_owners: Mutex<BTreeMap<String, SessionOwner>>,
    hosted_sessions: Mutex<BTreeMap<String, HostedSession>>,
    oauth_states: Mutex<BTreeMap<String, OAuthPending>>,
    hosted_completion_lock: tokio::sync::Mutex<()>,
    event_store: Arc<EventStore>,
    audit: AuditJournal,
    channel_states: Mutex<BTreeMap<String, ChannelState>>,
    supervisors_started: Mutex<std::collections::BTreeSet<String>>,
    shutdown: watch::Sender<bool>,
    tasks: Mutex<Vec<JoinHandle<()>>>,
    egress: Arc<dyn EgressTransport>,
    supervision_enabled: bool,
}

#[derive(Clone)]
enum PrincipalAdmission {
    Exact(Box<PrincipalContext>),
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
    profile: SlackConnectionProfile,
    oauth_authorize_url: Option<String>,
}

#[derive(Clone)]
struct SessionOwner {
    subject: String,
    email: Option<String>,
    profile: SlackConnectionProfile,
}

struct OAuthPending {
    session_ref: String,
    owner: SessionOwner,
    expires_at_unix_ms: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SlackConnectionProfile {
    #[default]
    Legacy,
    OrgBot,
    OrgUser,
    CompanionBot,
}

impl SlackConnectionProfile {
    fn parse(value: Option<&str>, hosted: bool) -> Option<Self> {
        match value {
            Some(PROFILE_ORG_USER) => Some(Self::OrgUser),
            Some(PROFILE_COMPANION_BOT) => Some(Self::CompanionBot),
            Some(PROFILE_ORG_BOT) if !hosted => Some(Self::OrgBot),
            None if !hosted => Some(Self::Legacy),
            _ => None,
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Legacy => "slack.legacy_combined",
            Self::OrgBot => PROFILE_ORG_BOT,
            Self::OrgUser => PROFILE_ORG_USER,
            Self::CompanionBot => PROFILE_COMPANION_BOT,
        }
    }

    const fn scope(self) -> ConnectionScope {
        match self {
            Self::OrgBot => ConnectionScope::Tenant,
            Self::Legacy | Self::OrgUser | Self::CompanionBot => ConnectionScope::Principal,
        }
    }

    const fn actor(self) -> ConnectionActor {
        match self {
            Self::OrgUser => ConnectionActor::User,
            Self::Legacy | Self::OrgBot | Self::CompanionBot => ConnectionActor::App,
        }
    }

    const fn receives_events(self) -> bool {
        matches!(self, Self::Legacy | Self::CompanionBot)
    }
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
    #[serde(default)]
    profile: SlackConnectionProfile,
    #[serde(default)]
    external_subject_id: String,
    #[serde(default)]
    scopes: Vec<String>,
    /// When an agent should reach for this Connection rather than another one of the same kind.
    ///
    /// Free text a person wrote, carried to the workbench beside the label. It exists because a
    /// workstation reaches Slack as several actors at once — the workspace bot, the operator, an
    /// assistant bot — and "which of these three should I post as" is not answerable from a profile
    /// name. Empty for every Connection acquired through a Connect Session, which nobody named.
    #[serde(default)]
    purpose: String,
    /// Whether this Connection publishes operations, as opposed to datasources alone.
    ///
    /// Defaults true — every Connection that predates named instances, and every one a Connect
    /// Session creates. A placement holding several identities marks exactly one, because the
    /// agent's capability projection admits one Connection per operation reference and two
    /// identities publishing `slack-conversations-history` refuse the whole session. Datasources
    /// are unaffected: a binding names its Connection, so every identity stays readable.
    #[serde(default = "default_carries_operations")]
    carries_operations: bool,
}

const fn default_carries_operations() -> bool {
    true
}

struct SlackCredentials {
    app_token: Option<Secret>,
    bot_token: Option<Secret>,
    user_token: Option<Secret>,
    refresh_token: Option<Secret>,
}

struct WorkspaceEvidence {
    team_id: String,
    subject_id: String,
    scopes: Vec<String>,
    is_bot: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PendingCommit {
    transaction_id: String,
    connection: StoredConnection,
}

struct EventStore {
    path: PathBuf,
    hosted_state: Option<Arc<dyn StateStore>>,
    events: Mutex<Vec<StoredEvent>>,
    notify: Notify,
}

struct AuditJournal {
    path: PathBuf,
    hosted_state: Option<Arc<dyn StateStore>>,
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

#[derive(Deserialize)]
struct UserOAuthResponse {
    ok: bool,
    #[serde(default)]
    access_token: Option<String>,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    authed_user: Option<UserOAuthActor>,
    #[serde(default)]
    team: Option<UserOAuthTeam>,
}

#[derive(Deserialize)]
struct UserOAuthActor {
    id: String,
    scope: String,
}

#[derive(Deserialize)]
struct UserOAuthTeam {
    id: String,
}

#[derive(Deserialize)]
struct SlackUserInfoResponse {
    ok: bool,
    #[serde(default)]
    user: Option<SlackUserInfo>,
}

#[derive(Deserialize)]
struct SlackUserInfo {
    profile: SlackUserProfile,
}

#[derive(Deserialize)]
struct SlackUserProfile {
    #[serde(default)]
    email: Option<String>,
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
        egress: Arc<dyn EgressTransport>,
    ) -> Result<Self, SlackError> {
        Self::open_inner(SlackOpenContext {
            admission: PrincipalAdmission::Exact(Box::new(owner)),
            completion_mode: CompletionMode::Local,
            policy,
            state_root,
            credential_store,
            egress,
            hosted_state: None,
            supervision_enabled: true,
        })
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
        hosted_state: Arc<dyn StateStore>,
        egress: Arc<dyn EgressTransport>,
    ) -> Result<Self, SlackError> {
        Self::open_inner(SlackOpenContext {
            admission: PrincipalAdmission::Tenant(tenant_id),
            completion_mode: CompletionMode::Hosted { public_origin },
            policy,
            state_root,
            credential_store,
            egress,
            hosted_state: Some(hosted_state),
            supervision_enabled: true,
        })
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
        Self::open_inner(SlackOpenContext {
            admission: PrincipalAdmission::Exact(Box::new(owner)),
            completion_mode: CompletionMode::Local,
            policy,
            state_root,
            credential_store,
            egress: test_egress(),
            hosted_state: None,
            supervision_enabled,
        })
        .await
    }

    async fn open_inner(context: SlackOpenContext<'_>) -> Result<Self, SlackError> {
        let SlackOpenContext {
            admission,
            completion_mode,
            policy,
            state_root,
            credential_store,
            egress,
            hosted_state,
            supervision_enabled,
        } = context;
        let metadata = read_state(state_root, hosted_state.as_deref())?;
        let event_store = Arc::new(EventStore::open(
            state_root.join("events.jsonl"),
            hosted_state.clone(),
        )?);
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
            oauth_states: Mutex::new(BTreeMap::new()),
            hosted_completion_lock: tokio::sync::Mutex::new(()),
            event_store,
            audit: AuditJournal::new(state_root.join("slack-operation-audit.jsonl"), hosted_state),
            channel_states: Mutex::new(BTreeMap::new()),
            supervisors_started: Mutex::new(std::collections::BTreeSet::new()),
            shutdown,
            tasks: Mutex::new(Vec::new()),
            egress,
            supervision_enabled,
        });
        inner.recover_pending().await?;
        inner.ensure_org_connection().await?;
        inner.ensure_declared_instances().await?;
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

struct SlackOpenContext<'a> {
    admission: PrincipalAdmission,
    completion_mode: CompletionMode,
    policy: SlackIntegrationConfig,
    state_root: &'a Path,
    credential_store: Arc<dyn PreparedSecretStore>,
    egress: Arc<dyn EgressTransport>,
    hosted_state: Option<Arc<dyn StateStore>>,
    supervision_enabled: bool,
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
            datasources: true,
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
            | OperationRequest::SessionReconcile(_)
            | OperationRequest::SessionSignal(_) => false,
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

    fn connect_session_access(
        &self,
        request: &protocol::connection::ConnectSessionCreateRequest,
    ) -> ConnectSessionAccess {
        if request.integration_ref == INTEGRATION_REF
            && request
                .auth_profile
                .as_deref()
                .is_some_and(|profile| matches!(profile, PROFILE_ORG_USER | PROFILE_COMPANION_BOT))
        {
            ConnectSessionAccess::SelfService
        } else {
            ConnectSessionAccess::Operator
        }
    }

    /// Publish only the flows this deployment can finish.
    ///
    /// `slack.org_user` needs an OAuth client, a redirect URI and an expected workspace before it
    /// can send anyone to Slack — the same three values `oauth_authorize_url` requires — so
    /// without them it is not offered rather than offered and refused. Both flows also need the
    /// hosted completion mode: a local placement's operator configures Slack directly.
    fn setup_profiles(&self, provider_ref: &str) -> Vec<protocol::catalog::SetupProfileSummary> {
        if provider_ref != INTEGRATION_REF
            || !matches!(self.inner.completion_mode, CompletionMode::Hosted { .. })
        {
            return Vec::new();
        }
        let mut profiles = Vec::new();
        if self.inner.policy.oauth_client_id.is_some()
            && self.inner.policy.oauth_redirect_uri.is_some()
            && self.inner.policy.expected_team_id.is_some()
        {
            profiles.push(protocol::catalog::SetupProfileSummary {
                auth_profile: PROFILE_ORG_USER.to_owned(),
                actor: protocol::catalog::SetupProfileActor::Person,
            });
        }
        profiles.push(protocol::catalog::SetupProfileSummary {
            auth_profile: PROFILE_COMPANION_BOT.to_owned(),
            actor: protocol::catalog::SetupProfileActor::Application,
        });
        profiles
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

    fn owns_datasource(&self, request: &DatasourceRequest) -> bool {
        match request {
            DatasourceRequest::Search(_) => false,
            DatasourceRequest::Describe(request) => {
                SLACK_DATASOURCES.contains(&request.datasource_ref.as_str())
            }
            DatasourceRequest::Bindings(request) => {
                SLACK_DATASOURCES.contains(&request.datasource_ref.as_str())
            }
            // Ownership is a claim about the datasource, never about the binding. Requiring a
            // Slack-shaped binding ref here dropped a read out of every backend's claim, and the
            // registry then answered `NotFound: no Integration owns this datasource` for a
            // datasource this Integration had just described. An unrecognised binding is this
            // Integration's refusal to make, in words that name the binding.
            DatasourceRequest::Read(request) => {
                SLACK_DATASOURCES.contains(&request.datasource_ref.as_str())
            }
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
        let sessions = lock(&self.inner.hosted_sessions);
        let session = sessions
            .get(session_ref)
            .ok_or(HostedCompletionError::NotFound)?;
        Ok(hosted_setup::completion_page(
            session.profile,
            session.oauth_authorize_url.as_deref(),
        ))
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
            if expected.profile != SlackConnectionProfile::CompanionBot {
                return Err(HostedCompletionError::Refused);
            }
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
        let evidence = match self
            .inner
            .verify_companion_credentials(session_ref, &credentials)
            .await
        {
            Ok(evidence) => evidence,
            Err(error) => {
                let _ =
                    lock(&self.inner.sessions).finish(session_ref, ConnectSessionTerminal::Failed);
                lock(&self.inner.session_owners).remove(session_ref);
                return Err(hosted_completion_error(error));
            }
        };
        let owner = lock(&self.inner.session_owners)
            .get(session_ref)
            .cloned()
            .ok_or(HostedCompletionError::NotFound)?;
        match self
            .inner
            .complete_connection(
                session_ref,
                owner,
                evidence.team_id,
                evidence.subject_id,
                evidence.scopes,
                credentials,
            )
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

    fn owns_hosted_oauth_state(&self, integration_ref: &str, state: &str) -> bool {
        integration_ref == INTEGRATION_REF && lock(&self.inner.oauth_states).contains_key(state)
    }

    async fn complete_hosted_oauth(
        &self,
        integration_ref: &str,
        state: &str,
        code: Option<&str>,
        error: Option<&str>,
    ) -> Result<(), HostedCompletionError> {
        if integration_ref != INTEGRATION_REF {
            return Err(HostedCompletionError::NotFound);
        }
        self.inner
            .complete_user_oauth(state, code, error)
            .await
            .map_err(hosted_completion_error)
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
            | OperationRequest::SessionSignal(_)
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
                let hosted = matches!(self.inner.completion_mode, CompletionMode::Hosted { .. });
                let profile =
                    SlackConnectionProfile::parse(request.auth_profile.as_deref(), hosted)
                        .ok_or_else(|| {
                            ConnectionError::new(
                                ConnectionErrorCode::InvalidInput,
                                "Slack setup requires an admitted auth profile",
                                false,
                            )
                        })?;
                let session = self
                    .inner
                    .create_session(context, request.label, profile)
                    .await?;
                Ok(ConnectionResult::ConnectSessionCreate(session))
            }
            ConnectionRequest::ConnectSessionStatus(request) => {
                if lock(&self.inner.session_owners)
                    .get(&request.connect_session_ref)
                    .is_none_or(|owner| owner.subject != context.subject())
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
                    .filter(|connection| connection.profile != SlackConnectionProfile::OrgBot)
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

    async fn handle_datasource(
        &self,
        context: &PrincipalContext,
        request: DatasourceRequest,
    ) -> Result<DatasourceResult, DatasourceError> {
        self.inner.check_datasource_context(context)?;
        match request {
            DatasourceRequest::Search(request) => Ok(DatasourceResult::Search {
                definitions: self.inner.search_datasources(context, &request.query),
            }),
            DatasourceRequest::Describe(request) => self
                .inner
                .describe_datasource(context, &request.datasource_ref)
                .map(DatasourceResult::Describe),
            DatasourceRequest::Bindings(request) => Ok(DatasourceResult::Bindings {
                bindings: self.inner.datasource_bindings(
                    context,
                    &request.datasource_ref,
                    &request.query,
                    usize::from(request.limit),
                )?,
            }),
            DatasourceRequest::Read(request) => self
                .inner
                .read_datasource(context, request)
                .await
                .map(DatasourceResult::Read),
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
        lock(&self.inner.oauth_states).clear();
        lock(&self.inner.session_owners).clear();
        lock(&self.inner.supervisors_started).clear();
    }
}

mod api_runtime;
mod connection_runtime;
impl AuditJournal {
    fn new(path: PathBuf, hosted_state: Option<Arc<dyn StateStore>>) -> Self {
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
                    StateError::Capacity => SlackError::new("audit-bound"),
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
    fn open(path: PathBuf, hosted_state: Option<Arc<dyn StateStore>>) -> Result<Self, SlackError> {
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
                    StateError::Capacity => SlackError::new("event-store-capacity"),
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

fn datasource_summary(datasource_ref: &str) -> Option<DatasourceSummary> {
    let title = match datasource_ref {
        "slack.conversations" => "Slack conversations and recent messages",
        "slack.users" => "Slack workspace users",
        _ => return None,
    };
    Some(DatasourceSummary {
        datasource_ref: datasource_ref.to_owned(),
        title: title.to_owned(),
        access_mode: DatasourceAccessMode::Live,
        verbs: vec![DatasourceReadVerb::List, DatasourceReadVerb::Get],
    })
}

fn datasource_declaration(datasource_ref: &str) -> Option<(&'static str, Value, Value, Value)> {
    match datasource_ref {
        "slack.conversations" => Some((
            "List conversations visible to this exact Slack token, or get a bounded recent-message projection for one conversation id.",
            serde_json::json!({"type":"string","pattern":"^[A-Za-z0-9]{2,64}$"}),
            serde_json::json!({
                "type":"object",
                "required":["id","label","kind","is_private","is_member"],
                "properties":{
                    "id":{"type":"string"},
                    "label":{"type":"string"},
                    "kind":{"enum":["public_channel","private_channel","im","mpim"]},
                    "is_private":{"type":"boolean"},
                    "is_member":{"type":"boolean"}
                },
                "additionalProperties":false
            }),
            serde_json::json!({
                "type":"object",
                "required":["id","messages"],
                "properties":{
                    "id":{"type":"string"},
                    "messages":{
                        "type":"array",
                        "maxItems":25,
                        "items":{
                            "type":"object",
                            "required":["ts","text"],
                            "properties":{
                                "ts":{"type":"string"},
                                "text":{"type":"string"},
                                "user":{"type":"string"},
                                "thread_ts":{"type":"string"},
                                "reply_count":{"type":"integer","minimum":0}
                            },
                            "additionalProperties":false
                        }
                    }
                },
                "additionalProperties":false
            }),
        )),
        "slack.users" => Some((
            "List or get bounded, non-secret workspace user profiles visible to this exact Slack token.",
            serde_json::json!({"type":"string","pattern":"^[A-Za-z0-9]{2,64}$"}),
            serde_json::json!({
                "type":"object",
                "required":["id","name","display_name","is_bot","deleted"],
                "properties":{
                    "id":{"type":"string"},
                    "name":{"type":"string"},
                    "display_name":{"type":"string"},
                    "is_bot":{"type":"boolean"},
                    "deleted":{"type":"boolean"}
                },
                "additionalProperties":false
            }),
            serde_json::json!({
                "type":"object",
                "required":["id","name","real_name","display_name","is_bot","deleted"],
                "properties":{
                    "id":{"type":"string"},
                    "name":{"type":"string"},
                    "real_name":{"type":"string"},
                    "display_name":{"type":"string"},
                    "is_bot":{"type":"boolean"},
                    "deleted":{"type":"boolean"}
                },
                "additionalProperties":false
            }),
        )),
        _ => None,
    }
}

fn datasource_projection_sha256(datasource_ref: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(b"b10x/slack-value-projection/v1\0");
    digest.update(datasource_ref.as_bytes());
    if let Some((description, key, compact, detail)) = datasource_declaration(datasource_ref) {
        digest.update(b"\0");
        digest.update(description.as_bytes());
        for value in [key, compact, detail] {
            digest.update(b"\0");
            digest.update(serde_json::to_vec(&value).expect("datasource schema serializes"));
        }
    }
    format!("{:x}", digest.finalize())
}

fn datasource_binding_ref(datasource_ref: &str, connection: &StoredConnection) -> String {
    let name = datasource_ref.strip_prefix("slack.").unwrap_or("unknown");
    format!("datasource-binding:slack:{name}:{}", connection.instance_id)
}

const fn datasource_scope_label(profile: SlackConnectionProfile) -> &'static str {
    match profile {
        SlackConnectionProfile::OrgBot => "organization read-only",
        SlackConnectionProfile::OrgUser => "your Slack user",
        SlackConnectionProfile::CompanionBot => "your companion bot",
        SlackConnectionProfile::Legacy => "legacy local connection",
    }
}

type DatasourceRequestPlan = (
    &'static str,
    Vec<(String, String)>,
    DatasourceRecordView,
    Option<String>,
);

fn datasource_request_plan(
    datasource_ref: &str,
    profile: SlackConnectionProfile,
    read: &DatasourceRead,
) -> Result<DatasourceRequestPlan, DatasourceError> {
    match (datasource_ref, read) {
        ("slack.conversations", DatasourceRead::List { limit, cursor }) => {
            let types = match profile {
                SlackConnectionProfile::OrgBot => "public_channel",
                SlackConnectionProfile::OrgUser | SlackConnectionProfile::Legacy => {
                    "public_channel,private_channel,im,mpim"
                }
                SlackConnectionProfile::CompanionBot => "public_channel,private_channel",
            };
            let mut params = vec![
                ("limit".to_owned(), limit.to_string()),
                ("exclude_archived".to_owned(), "true".to_owned()),
                ("types".to_owned(), types.to_owned()),
            ];
            if let Some(cursor) = cursor {
                params.push(("cursor".to_owned(), cursor.clone()));
            }
            Ok((
                CONVERSATIONS_LIST,
                params,
                DatasourceRecordView::Compact,
                None,
            ))
        }
        ("slack.conversations", DatasourceRead::Get { key }) => {
            let channel = key
                .as_str()
                .filter(|value| valid_slack_id(value))
                .ok_or_else(datasource_invalid)?;
            Ok((
                CONVERSATIONS_HISTORY,
                vec![
                    ("channel".to_owned(), channel.to_owned()),
                    (
                        "limit".to_owned(),
                        protocol::datasource::MAX_RESULTS.to_string(),
                    ),
                ],
                DatasourceRecordView::Detail,
                Some(channel.to_owned()),
            ))
        }
        ("slack.users", DatasourceRead::List { limit, cursor }) => {
            let mut params = vec![("limit".to_owned(), limit.to_string())];
            if let Some(cursor) = cursor {
                params.push(("cursor".to_owned(), cursor.clone()));
            }
            Ok((USERS_LIST, params, DatasourceRecordView::Compact, None))
        }
        ("slack.users", DatasourceRead::Get { key }) => {
            let user = key
                .as_str()
                .filter(|value| valid_slack_id(value))
                .ok_or_else(datasource_invalid)?;
            Ok((
                USERS_INFO,
                vec![("user".to_owned(), user.to_owned())],
                DatasourceRecordView::Detail,
                Some(user.to_owned()),
            ))
        }
        _ => Err(datasource_not_found()),
    }
}

fn normalize_datasource_response(
    datasource_ref: &str,
    view: DatasourceRecordView,
    requested_key: Option<&str>,
    payload: &Value,
) -> Result<
    (
        Vec<DatasourceRecord>,
        Option<String>,
        DatasourceCompleteness,
    ),
    DatasourceError,
> {
    let next_cursor = slack_next_cursor(payload);
    let (records, partial) = match (datasource_ref, view) {
        ("slack.conversations", DatasourceRecordView::Compact) => {
            let channels = payload
                .get("channels")
                .and_then(Value::as_array)
                .ok_or_else(datasource_protocol)?;
            let records = channels
                .iter()
                .map(normalize_conversation)
                .collect::<Result<Vec<_>, _>>()?;
            (records, next_cursor.is_some())
        }
        ("slack.conversations", DatasourceRecordView::Detail) => {
            let id = requested_key.ok_or_else(datasource_protocol)?;
            let messages = payload
                .get("messages")
                .and_then(Value::as_array)
                .ok_or_else(datasource_protocol)?;
            let messages = messages
                .iter()
                .take(usize::from(protocol::datasource::MAX_RESULTS))
                .map(normalize_message)
                .collect::<Result<Vec<_>, _>>()?;
            let partial = payload.get("has_more").and_then(Value::as_bool) == Some(true);
            (
                vec![DatasourceRecord {
                    key: Value::String(id.to_owned()),
                    view,
                    value: serde_json::json!({"id": id, "messages": messages}),
                }],
                partial,
            )
        }
        ("slack.users", DatasourceRecordView::Compact) => {
            let members = payload
                .get("members")
                .and_then(Value::as_array)
                .ok_or_else(datasource_protocol)?;
            let records = members
                .iter()
                .map(|user| normalize_user(user, DatasourceRecordView::Compact))
                .collect::<Result<Vec<_>, _>>()?;
            (records, next_cursor.is_some())
        }
        ("slack.users", DatasourceRecordView::Detail) => {
            let user = payload.get("user").ok_or_else(datasource_protocol)?;
            let record = normalize_user(user, DatasourceRecordView::Detail)?;
            if requested_key.is_none_or(|key| record.key != Value::String(key.to_owned())) {
                return Err(datasource_protocol());
            }
            (vec![record], false)
        }
        _ => return Err(datasource_not_found()),
    };
    Ok((
        records,
        if view == DatasourceRecordView::Compact {
            next_cursor
        } else {
            None
        },
        if partial {
            DatasourceCompleteness::Partial
        } else {
            DatasourceCompleteness::Complete
        },
    ))
}

fn normalize_conversation(value: &Value) -> Result<DatasourceRecord, DatasourceError> {
    let id = bounded_string(value, "id", 64)
        .filter(|id| valid_slack_id(id))
        .ok_or_else(datasource_protocol)?;
    let kind = if value.get("is_im").and_then(Value::as_bool) == Some(true) {
        "im"
    } else if value.get("is_mpim").and_then(Value::as_bool) == Some(true) {
        "mpim"
    } else if value.get("is_private").and_then(Value::as_bool) == Some(true) {
        "private_channel"
    } else {
        "public_channel"
    };
    let label = value
        .get("name")
        .and_then(Value::as_str)
        .or_else(|| value.get("user").and_then(Value::as_str))
        .filter(|label| !label.is_empty() && label.len() <= 512)
        .unwrap_or(id);
    let normalized = serde_json::json!({
        "id": id,
        "label": label,
        "kind": kind,
        "is_private": value.get("is_private").and_then(Value::as_bool).unwrap_or(kind != "public_channel"),
        "is_member": value.get("is_member").and_then(Value::as_bool).unwrap_or(kind == "im" || kind == "mpim")
    });
    Ok(DatasourceRecord {
        key: Value::String(id.to_owned()),
        view: DatasourceRecordView::Compact,
        value: normalized,
    })
}

fn normalize_message(value: &Value) -> Result<Value, DatasourceError> {
    let ts = bounded_string(value, "ts", 64)
        .filter(|value| valid_slack_timestamp(value))
        .ok_or_else(datasource_protocol)?;
    let text = bounded_string(value, "text", 40_000).ok_or_else(datasource_protocol)?;
    let mut normalized = serde_json::Map::from_iter([
        ("ts".to_owned(), Value::String(ts.to_owned())),
        ("text".to_owned(), Value::String(text.to_owned())),
    ]);
    if let Some(user) = bounded_string(value, "user", 64).filter(|value| valid_slack_id(value)) {
        normalized.insert("user".to_owned(), Value::String(user.to_owned()));
    }
    if let Some(thread_ts) =
        bounded_string(value, "thread_ts", 64).filter(|value| valid_slack_timestamp(value))
    {
        normalized.insert("thread_ts".to_owned(), Value::String(thread_ts.to_owned()));
    }
    if let Some(reply_count) = value.get("reply_count").and_then(Value::as_u64) {
        normalized.insert("reply_count".to_owned(), Value::from(reply_count));
    }
    Ok(Value::Object(normalized))
}

fn normalize_user(
    value: &Value,
    view: DatasourceRecordView,
) -> Result<DatasourceRecord, DatasourceError> {
    let id = bounded_string(value, "id", 64)
        .filter(|value| valid_slack_id(value))
        .ok_or_else(datasource_protocol)?;
    let name = bounded_string(value, "name", 512).unwrap_or("");
    let profile = value.get("profile").and_then(Value::as_object);
    let display_name = profile
        .and_then(|profile| profile.get("display_name"))
        .and_then(Value::as_str)
        .filter(|value| value.len() <= 512)
        .unwrap_or("");
    let mut normalized = serde_json::Map::from_iter([
        ("id".to_owned(), Value::String(id.to_owned())),
        ("name".to_owned(), Value::String(name.to_owned())),
        (
            "display_name".to_owned(),
            Value::String(display_name.to_owned()),
        ),
        (
            "is_bot".to_owned(),
            Value::Bool(
                value
                    .get("is_bot")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            ),
        ),
        (
            "deleted".to_owned(),
            Value::Bool(
                value
                    .get("deleted")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            ),
        ),
    ]);
    if view == DatasourceRecordView::Detail {
        let real_name = bounded_string(value, "real_name", 512)
            .or_else(|| {
                profile
                    .and_then(|profile| profile.get("real_name"))
                    .and_then(Value::as_str)
                    .filter(|value| value.len() <= 512)
            })
            .unwrap_or("");
        normalized.insert("real_name".to_owned(), Value::String(real_name.to_owned()));
    }
    Ok(DatasourceRecord {
        key: Value::String(id.to_owned()),
        view,
        value: Value::Object(normalized),
    })
}

fn bounded_string<'a>(value: &'a Value, key: &str, maximum: usize) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| value.len() <= maximum && !value.contains('\0'))
}

fn slack_next_cursor(payload: &Value) -> Option<String> {
    payload
        .pointer("/response_metadata/next_cursor")
        .and_then(Value::as_str)
        .filter(|cursor| {
            !cursor.is_empty()
                && cursor.len() <= 512
                && cursor.bytes().all(|byte| byte.is_ascii_graphic())
        })
        .map(str::to_owned)
}

fn is_slack_operation(operation_ref: &str) -> bool {
    SLACK_OPERATIONS.contains(&operation_ref)
}

fn connection_supports_operation(connection: &StoredConnection, operation_ref: &str) -> bool {
    if !is_slack_operation(operation_ref) {
        return false;
    }
    match connection.profile {
        SlackConnectionProfile::OrgBot => operation_effect(operation_ref) == EffectClass::ReadOnly,
        SlackConnectionProfile::OrgUser | SlackConnectionProfile::CompanionBot => true,
        SlackConnectionProfile::Legacy => true,
    }
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

fn parse_scopes(value: &str) -> Vec<String> {
    let mut scopes = value
        .split([',', ' '])
        .filter(|scope| {
            !scope.is_empty()
                && scope.len() <= 128
                && scope.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b':' | b'.' | b'-' | b'_')
                })
        })
        .map(str::to_owned)
        .collect::<Vec<_>>();
    scopes.sort();
    scopes.dedup();
    scopes
}

fn valid_slack_id(value: &str) -> bool {
    (2..=64).contains(&value.len()) && value.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

fn valid_slack_timestamp(value: &str) -> bool {
    (3..=64).contains(&value.len())
        && value.contains('.')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'.')
}

fn normalize_email(value: &str) -> Option<String> {
    let value = value.trim().to_ascii_lowercase();
    (value.len() <= 254
        && value.is_ascii()
        && !value.chars().any(char::is_whitespace)
        && value.split('@').count() == 2)
        .then_some(value)
}

fn bearer_request(method: &str, url: String, token: &Secret) -> connector_resolve::Request {
    connector_resolve::Request {
        method: method.to_owned(),
        url,
        headers: BTreeMap::from([(
            "Authorization".to_owned(),
            format!("Bearer {}", token.expose_secret()),
        )]),
        body: None,
    }
}

fn operation_not_found() -> OperationError {
    OperationError::new(
        OperationErrorCode::NotFound,
        "Slack operation was not found",
        false,
    )
}

fn datasource_error(
    code: DatasourceErrorCode,
    message: &'static str,
    retriable: bool,
) -> DatasourceError {
    DatasourceError::new(code, message, retriable)
}

fn datasource_not_found() -> DatasourceError {
    datasource_error(
        DatasourceErrorCode::NotFound,
        "Slack datasource or record was not found",
        false,
    )
}

fn datasource_not_granted() -> DatasourceError {
    datasource_error(
        DatasourceErrorCode::NotGranted,
        "Slack datasource is not granted for this Connection",
        false,
    )
}

/// One Slack datasource exists but no Connection of this principal is bound to it.
///
/// Retriable: connecting Slack, or a grant landing, clears it without any change to this
/// deployment. The same principal saw one Slack binding and then none minutes later against the
/// deployed build, so a flat refusal here would escalate a condition that resolves itself.
fn datasource_binding_not_granted(datasource_ref: &str) -> DatasourceError {
    DatasourceError::new(
        DatasourceErrorCode::NotGranted,
        format!(
            "no Slack Connection of this principal is currently bound to `{datasource_ref}`; connect Slack or wait for the grant to be admitted, then retry"
        ),
        true,
    )
}

fn datasource_invalid() -> DatasourceError {
    datasource_error(
        DatasourceErrorCode::InvalidInput,
        "Slack datasource input is invalid",
        false,
    )
}

fn datasource_protocol() -> DatasourceError {
    datasource_error(
        DatasourceErrorCode::Protocol,
        "Slack returned an incompatible datasource response",
        false,
    )
}

fn datasource_unavailable() -> DatasourceError {
    datasource_error(
        DatasourceErrorCode::Unavailable,
        "Slack datasource is unavailable",
        true,
    )
}

fn datasource_slack_refusal(value: &Value) -> DatasourceError {
    match value.get("error").and_then(Value::as_str) {
        Some("channel_not_found" | "user_not_found") => datasource_not_found(),
        Some(
            "missing_scope"
            | "not_allowed_token_type"
            | "not_authed"
            | "invalid_auth"
            | "token_expired"
            | "token_revoked"
            | "account_inactive",
        ) => datasource_not_granted(),
        _ => datasource_unavailable(),
    }
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
        InitiationConfig::Platform => vec![ConnectionInitiator::Platform],
        InitiationConfig::Provider => vec![ConnectionInitiator::Provider],
        InitiationConfig::Both => {
            vec![ConnectionInitiator::Platform, ConnectionInitiator::Provider]
        }
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

#[cfg(test)]
struct RefusingEgress;

#[cfg(test)]
#[async_trait]
impl EgressTransport for RefusingEgress {
    async fn execute(
        &self,
        _authority_ref: &str,
        _request: EgressHttpRequest,
    ) -> Result<service::EgressHttpResponse, service::EgressTransportError> {
        Err(service::EgressTransportError::Refused)
    }

    async fn connect_websocket(
        &self,
        _authority_ref: &str,
        _url: String,
        _maximum_message_bytes: usize,
    ) -> Result<Box<dyn EgressWebSocket>, service::EgressTransportError> {
        Err(service::EgressTransportError::Refused)
    }
}

#[cfg(test)]
fn test_egress() -> Arc<dyn EgressTransport> {
    Arc::new(RefusingEgress)
}

/// The stable machine identity of one declared instance, derived from the name a person chose.
///
/// `CredentialRef::for_instance` admits only a canonical uuid, so `"babelforce-bot"` cannot be an
/// instance id directly. Deriving one from the name means the Connection ref, every datasource
/// binding ref and the credential address are the same on every start — where a random id makes a
/// saved binding reference dead the moment the placement restarts. The name stays the thing anybody
/// reads; this is only its machine spelling.
fn instance_id_for_name(name: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(b"b10x/slack-instance/v1\0");
    digest.update(name.as_bytes());
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest.finalize()[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    format_uuid(bytes)
}

fn format_uuid(bytes: [u8; 16]) -> String {
    let encoded = hex::encode(bytes);
    format!(
        "{}-{}-{}-{}-{}",
        &encoded[0..8],
        &encoded[8..12],
        &encoded[12..16],
        &encoded[16..20],
        &encoded[20..32]
    )
}

fn random_uuid() -> Result<String, SlackError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| SlackError::new("randomness"))?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Ok(format_uuid(bytes))
}

fn now_ms() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
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

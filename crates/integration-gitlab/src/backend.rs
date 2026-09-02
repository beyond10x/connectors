//! Hosted GitLab OAuth/PAT custody, operation dispatch, and safe datasource projections.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::state::GitlabState;
use async_trait::async_trait;
use connector_oauth::{
    AuthorizeParams, Pending, PendingStates, Pkce, ScopePolicy, ScopeSeparator, TokenPolicy,
    TokenResponse, DEFAULT_PENDING_CAPACITY,
};
use connector_secrets::{
    CredentialRef, CredentialScope, Layout, PreparedSecretStore, Secret, SecretBatch,
    SecretProposalDigest, SecretTransactionGeneration, SecretTransactionId, SecretTransactionState,
    TenantLayout,
};
use connectors_config::{HostedGitlabConfig, InitiationConfig};
use protocol::connection::{
    ConnectSessionStatus, ConnectionActor, ConnectionDescription, ConnectionError,
    ConnectionErrorCode, ConnectionInitiator, ConnectionRequest, ConnectionResult, ConnectionScope,
    ConnectionState, ConnectionSummary,
};
use protocol::datasource::{
    AccessMode as DatasourceAccessMode, Completeness as DatasourceCompleteness, DatasourceBinding,
    DatasourceDescription, DatasourceError, DatasourceErrorCode, DatasourcePage,
    DatasourceProvenance, DatasourceRead, DatasourceRecord, DatasourceRequest, DatasourceResult,
    DatasourceSummary, ReadRequest as DatasourceReadRequest, ReadVerb as DatasourceReadVerb,
    RecordView as DatasourceRecordView,
};
use protocol::operation::{
    ApprovalPosture, ConnectionSummary as OperationConnectionSummary, EffectClass,
    InvocationResult, InvokeRequest, OperationDescription, OperationError, OperationErrorCode,
    OperationRequest, OperationResult, OperationSummary,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use service::{
    BackendCapabilities, BackendReadinessError, ConnectSessionAccess, ConnectSessionLifecycle,
    ConnectSessionTerminal, ConnectorBackend, EgressHttpRequest, EgressHttpResponse,
    EgressTransport, HostedCompletionError, HostedCompletionPage, HostedCompletionSubmission,
    PrincipalContext,
};
use sha2::{Digest as _, Sha256};
use zeroize::Zeroizing;

use crate::transport::{
    bearer_headers, decode_page_response, decode_response, decode_value_response, form_body,
    http_request,
};

pub(crate) const INTEGRATION_REF: &str = "gitlab";
pub(crate) const AUTHORITY: &str = "com.gitlab.api";
const SERVICE: &str = "default";
pub(crate) const LOGIN_SERVICE: &str = "login";
const PROFILE_OAUTH: &str = "gitlab.oauth_user";
const PROFILE_PAT: &str = "gitlab.personal_token";
const ACCESS_TOKEN_CREDENTIAL: &str = "access_token";
const REFRESH_TOKEN_CREDENTIAL: &str = "refresh_token";
pub(crate) const OAUTH_CLIENT_SECRET_CREDENTIAL: &str = "oauth_client_secret";
pub(crate) const STATE_KEY: &str = "gitlab.connections";
const AUDIT_KEY: &str = "gitlab.audit";
pub(crate) const STATE_VERSION: u8 = 1;
pub(crate) const MAX_STATE_BYTES: usize = 4 * 1024 * 1024;
const MAX_AUDIT_BYTES: usize = 16 * 1024 * 1024;
pub(crate) const MAX_CONNECT_SESSIONS: usize = 32;
const MAX_PROVIDER_RESPONSE_BYTES: usize = 256 * 1024;
const VALUE_PROJECTION_PROTOCOL: &str = "b10x.value-projection.v1";
const GITLAB_OPERATIONS: [&str; 11] = [
    "gitlab-user-get",
    "gitlab-group-list",
    "gitlab-project-list",
    "gitlab-issue-list",
    "gitlab-issue-get",
    "gitlab-issue-create",
    "gitlab-merge-request-list",
    "gitlab-pipeline-get",
    "gitlab-branch-list",
    "gitlab-repository-tree-list",
    "gitlab-repository-file-get",
];
const GITLAB_DATASOURCES: [&str; 6] = [
    "gitlab.users",
    "gitlab.groups",
    "gitlab.projects",
    "gitlab.issues",
    "gitlab.merge_requests",
    "gitlab.branches",
];

/// Redaction-safe GitLab runtime failure.
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("GitLab runtime refused: {code}")]
pub struct GitlabError {
    code: &'static str,
}

impl GitlabError {
    pub(crate) const fn new(code: &'static str) -> Self {
        Self { code }
    }
}

/// Hosted GitLab Integration for one Identity tenant.
pub struct GitlabBackend {
    inner: Arc<GitlabInner>,
}

pub(crate) struct GitlabInner {
    tenant_id: String,
    policy: HostedGitlabConfig,
    origin: url::Url,
    public_origin: url::Url,
    state_store: GitlabState,
    credential_store: Arc<dyn PreparedSecretStore>,
    metadata: Mutex<StateFile>,
    sessions: Mutex<ConnectSessionLifecycle>,
    session_owners: Mutex<BTreeMap<String, SessionOwner>>,
    hosted_sessions: Mutex<BTreeMap<String, HostedSession>>,
    oauth_states: Mutex<PendingStates<OAuthPending>>,
    completion_lock: tokio::sync::Mutex<()>,
    refresh_lock: tokio::sync::Mutex<()>,
    egress: Arc<dyn EgressTransport>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum GitlabProfile {
    OAuthUser,
    PersonalToken,
}

impl GitlabProfile {
    fn parse(value: Option<&str>) -> Option<Self> {
        match value {
            Some(PROFILE_OAUTH) => Some(Self::OAuthUser),
            Some(PROFILE_PAT) => Some(Self::PersonalToken),
            _ => None,
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::OAuthUser => PROFILE_OAUTH,
            Self::PersonalToken => PROFILE_PAT,
        }
    }
}

#[derive(Clone)]
struct SessionOwner {
    subject: String,
    email: String,
    profile: GitlabProfile,
}

struct HostedSession {
    capability_sha256: [u8; 32],
    expires_at_unix_ms: u64,
    profile: GitlabProfile,
    oauth_authorize_url: Option<String>,
}

struct OAuthPending {
    session_ref: String,
    owner: SessionOwner,
    verifier: Zeroizing<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StateFile {
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
    owner_subject: String,
    external_user_id: u64,
    username: String,
    email_sha256: String,
    profile: GitlabProfile,
    scopes: Vec<String>,
    credential_generation: u64,
    observed_at_unix_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    expires_at_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PendingCommit {
    transaction_id: String,
    connection: StoredConnection,
}

struct VerifiedCredential {
    user: GitlabUser,
    scopes: Vec<String>,
    expires_at_unix_ms: Option<u64>,
}

struct CredentialValues {
    access_token: Secret,
    refresh_token: Option<Secret>,
}

#[derive(Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
    created_at: u64,
    scope: String,
    token_type: String,
}

#[derive(Deserialize)]
struct OAuthTokenInfo {
    resource_owner_id: u64,
    scopes: Vec<String>,
    expires_in_seconds: u64,
    created_at: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct GitlabUser {
    id: u64,
    username: String,
    state: String,
    #[serde(default)]
    bot: bool,
    #[serde(default)]
    email: Option<String>,
}

#[derive(Deserialize)]
struct PersonalTokenInfo {
    active: bool,
    revoked: bool,
    scopes: Vec<String>,
    #[serde(default)]
    expires_at: Option<String>,
}

impl GitlabBackend {
    pub(crate) async fn open_inner(
        tenant_id: String,
        policy: HostedGitlabConfig,
        credential_store: Arc<dyn PreparedSecretStore>,
        state_store: GitlabState,
        egress: Arc<dyn EgressTransport>,
    ) -> Result<Self, GitlabError> {
        let origin = parse_origin(&policy.origin)?;
        let public_origin = url::Url::parse(&policy.public_origin)
            .map_err(|_| GitlabError::new("public-origin"))?;
        let metadata = state_store.read(STATE_KEY, MAX_STATE_BYTES)?.map_or_else(
            || Ok(StateFile::default()),
            |bytes| {
                serde_json::from_slice::<StateFile>(&bytes)
                    .map_err(|_| GitlabError::new("connection-state"))
            },
        )?;
        if metadata.version != STATE_VERSION
            || metadata.next_transaction_generation == 0
            || metadata.connections.len() > 1_024
            || metadata.pending.len() > 32
        {
            return Err(GitlabError::new("connection-state"));
        }
        let inner = Arc::new(GitlabInner {
            tenant_id,
            policy,
            origin,
            public_origin,
            state_store,
            credential_store,
            metadata: Mutex::new(metadata),
            sessions: Mutex::new(
                ConnectSessionLifecycle::new(INTEGRATION_REF, MAX_CONNECT_SESSIONS)
                    .map_err(|_| GitlabError::new("connect-session"))?,
            ),
            session_owners: Mutex::new(BTreeMap::new()),
            hosted_sessions: Mutex::new(BTreeMap::new()),
            oauth_states: Mutex::new(PendingStates::new(DEFAULT_PENDING_CAPACITY)),
            completion_lock: tokio::sync::Mutex::new(()),
            refresh_lock: tokio::sync::Mutex::new(()),
            egress,
        });
        inner.recover_pending().await?;
        Ok(Self { inner })
    }

    #[must_use]
    pub fn connection_count(&self) -> usize {
        lock(&self.inner.metadata).connections.len()
    }
}

#[async_trait]
impl ConnectorBackend for GitlabBackend {
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
            events: false,
            datasources: true,
        }
    }

    fn owns_operation(&self, request: &OperationRequest) -> bool {
        match request {
            OperationRequest::Describe(request) => is_gitlab_operation(&request.operation_ref),
            OperationRequest::Invoke(request) => {
                is_gitlab_operation(&request.operation_ref)
                    && lock(&self.inner.metadata)
                        .connections
                        .iter()
                        .any(|connection| connection.connection_ref == request.connection_ref)
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
                .any(|connection| connection.connection_ref == request.connection_ref),
            ConnectionRequest::Search(_)
            | ConnectionRequest::CandidateSearch(_)
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
            && GitlabProfile::parse(request.auth_profile.as_deref()).is_some()
        {
            ConnectSessionAccess::SelfService
        } else {
            ConnectSessionAccess::Operator
        }
    }

    fn owns_datasource(&self, request: &DatasourceRequest) -> bool {
        match request {
            DatasourceRequest::Search(_) => false,
            DatasourceRequest::Describe(request) => {
                GITLAB_DATASOURCES.contains(&request.datasource_ref.as_str())
            }
            DatasourceRequest::Bindings(request) => {
                GITLAB_DATASOURCES.contains(&request.datasource_ref.as_str())
            }
            DatasourceRequest::Read(request) => {
                GITLAB_DATASOURCES.contains(&request.datasource_ref.as_str())
                    && request
                        .binding_ref
                        .starts_with("datasource-binding:gitlab:")
            }
        }
    }

    fn owns_hosted_completion(&self, session_ref: &str) -> bool {
        lock(&self.inner.hosted_sessions).contains_key(session_ref)
    }

    fn hosted_completion_page(
        &self,
        session_ref: &str,
    ) -> Result<HostedCompletionPage, HostedCompletionError> {
        self.inner.expire_sessions();
        let sessions = lock(&self.inner.hosted_sessions);
        let session = sessions
            .get(session_ref)
            .ok_or(HostedCompletionError::NotFound)?;
        Ok(completion_page(
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
        self.inner.expire_sessions();
        if !valid_capability(capability) {
            return Err(HostedCompletionError::Refused);
        }
        let actual: [u8; 32] = Sha256::digest(capability.as_bytes()).into();
        {
            let mut sessions = lock(&self.inner.hosted_sessions);
            let expected = sessions
                .get(session_ref)
                .ok_or(HostedCompletionError::NotFound)?;
            if expected.profile != GitlabProfile::PersonalToken
                || !constant_time_equal(&expected.capability_sha256, &actual)
            {
                return Err(HostedCompletionError::Refused);
            }
            sessions.remove(session_ref);
        }
        let _completion = self.inner.completion_lock.lock().await;
        let owner = lock(&self.inner.session_owners)
            .get(session_ref)
            .cloned()
            .ok_or(HostedCompletionError::NotFound)?;
        let token = parse_pat(submission.expose_secret())?;
        let outcome = async {
            let evidence = self
                .inner
                .verify_pat(session_ref, &token, &owner.email)
                .await?;
            self.inner
                .commit_connection(
                    session_ref,
                    owner,
                    evidence,
                    CredentialValues {
                        access_token: token,
                        refresh_token: None,
                    },
                )
                .await
        }
        .await;
        self.inner.finish_session(session_ref, outcome)
    }

    fn owns_hosted_oauth_state(&self, integration_ref: &str, state: &str) -> bool {
        integration_ref == INTEGRATION_REF && lock(&self.inner.oauth_states).contains_any(state)
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
        self.inner.complete_oauth(state, code, error).await
    }

    async fn handle(
        &self,
        context: &PrincipalContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        self.inner
            .check_context(context)
            .map_err(operation_from_context)?;
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
                            || connection.label.to_ascii_lowercase().contains(&query)
                            || INTEGRATION_REF.contains(&query)
                    })
                    .map(|connection| connection_summary(connection, self.inner.policy.initiation))
                    .collect::<Vec<_>>();
                connections.truncate(usize::from(request.limit));
                Ok(ConnectionResult::Search { connections })
            }
            ConnectionRequest::Describe(request) => self
                .inner
                .owned_connections(context)
                .into_iter()
                .find(|connection| connection.connection_ref == request.connection_ref)
                .map(|connection| {
                    ConnectionResult::Describe(ConnectionDescription {
                        summary: connection_summary(connection, self.inner.policy.initiation),
                        channels: Vec::new(),
                    })
                })
                .ok_or_else(connection_not_found),
            ConnectionRequest::ConnectSessionCreate(request) => {
                let profile =
                    GitlabProfile::parse(request.auth_profile.as_deref()).ok_or_else(|| {
                        ConnectionError::new(
                            ConnectionErrorCode::InvalidInput,
                            "GitLab setup requires gitlab.oauth_user or gitlab.personal_token",
                            false,
                        )
                    })?;
                self.inner
                    .create_session(context, request.label, profile)
                    .map(ConnectionResult::ConnectSessionCreate)
            }
            ConnectionRequest::ConnectSessionStatus(request) => {
                if lock(&self.inner.session_owners)
                    .get(&request.connect_session_ref)
                    .is_none_or(|owner| owner.subject != context.subject())
                    && lock(&self.inner.sessions)
                        .status(&request.connect_session_ref)
                        .is_none_or(|status| status.connection_ref.is_none())
                {
                    return Err(connection_not_found());
                }
                lock(&self.inner.sessions)
                    .status(&request.connect_session_ref)
                    .map(ConnectionResult::ConnectSessionStatus)
                    .ok_or_else(connection_not_found)
            }
            ConnectionRequest::ObservationSearch(_) => Ok(ConnectionResult::ObservationSearch {
                observations: Vec::new(),
            }),
            ConnectionRequest::CandidateSearch(_)
            | ConnectionRequest::CandidateActivate(_)
            | ConnectionRequest::Materialize(_) => Err(connection_not_found()),
        }
    }

    async fn handle_datasource(
        &self,
        context: &PrincipalContext,
        request: DatasourceRequest,
    ) -> Result<DatasourceResult, DatasourceError> {
        self.inner
            .check_context(context)
            .map_err(datasource_from_context)?;
        match request {
            DatasourceRequest::Search(request) => Ok(DatasourceResult::Search {
                definitions: self.inner.search_datasources(context, &request.query),
            }),
            DatasourceRequest::Describe(request) => self
                .inner
                .describe_datasource(context, &request.datasource_ref)
                .map(DatasourceResult::Describe),
            DatasourceRequest::Bindings(request) => self
                .inner
                .datasource_bindings(
                    context,
                    &request.datasource_ref,
                    &request.query,
                    usize::from(request.limit),
                )
                .await
                .map(|bindings| DatasourceResult::Bindings { bindings }),
            DatasourceRequest::Read(request) => self
                .inner
                .read_datasource(context, request)
                .await
                .map(DatasourceResult::Read),
        }
    }
}

impl GitlabInner {
    fn persist(&self, state: &StateFile) -> Result<(), GitlabError> {
        let body = serde_json::to_vec(state).map_err(|_| GitlabError::new("connection-state"))?;
        self.state_store.replace(STATE_KEY, &body, MAX_STATE_BYTES)
    }

    fn check_context(&self, context: &PrincipalContext) -> Result<(), ConnectionError> {
        if context.tenant_id() == self.tenant_id {
            Ok(())
        } else {
            Err(ConnectionError::new(
                ConnectionErrorCode::StaleAuthority,
                "owner authority snapshot is not current",
                false,
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

    fn create_session(
        &self,
        owner: &PrincipalContext,
        label: String,
        profile: GitlabProfile,
    ) -> Result<ConnectSessionStatus, ConnectionError> {
        self.expire_sessions();
        let email = owner.email().ok_or_else(|| {
            ConnectionError::new(
                ConnectionErrorCode::NotGranted,
                "GitLab user connection requires a verified Identity email",
                false,
            )
        })?;
        let id = random_uuid().map_err(|_| connection_unavailable())?;
        let session_ref = format!("connect-session:{id}");
        let expires_at_unix_ms = now_ms()
            .and_then(|now| {
                now.checked_add(
                    self.policy
                        .connect_session_ttl_seconds
                        .saturating_mul(1_000),
                )
            })
            .ok_or_else(connection_unavailable)?;
        let capability = random_token(32).map_err(|_| connection_unavailable())?;
        let session_owner = SessionOwner {
            subject: owner.subject().to_owned(),
            email: email.to_owned(),
            profile,
        };
        let (oauth_state, oauth_authorize_url) = if profile == GitlabProfile::OAuthUser {
            let state = random_token(32).map_err(|_| connection_unavailable())?;
            let pkce = Pkce::generate().map_err(|_| connection_unavailable())?;
            let authorize = self
                .oauth_authorize_url(&state, pkce.challenge())
                .map_err(|_| connection_unavailable())?;
            (Some((state, pkce.into_verifier())), Some(authorize))
        } else {
            (None, None)
        };
        let mut url = self.public_origin.clone();
        url.path_segments_mut()
            .map_err(|_| connection_unavailable())?
            .push("connect-sessions")
            .push(&session_ref);
        url.set_fragment(Some(&format!("token={capability}")));
        let status = lock(&self.sessions)
            .reserve_browser(session_ref.clone(), label, expires_at_unix_ms, url.into())
            .map_err(connect_session_error)?;
        // The fallible inserts run before the infallible ones. A clock failure or a full pending
        // map returns from here, and everything written so far stays written: ordering it this way
        // means a refusal leaves no hosted session that `session_owners` has no row for. The
        // browser reservation above still self-heals at the connect-session TTL.
        if let Some((state, verifier)) = oauth_state {
            let now = now_ms().ok_or_else(connection_unavailable)?;
            lock(&self.oauth_states)
                .insert(
                    state,
                    OAuthPending {
                        session_ref: session_ref.clone(),
                        owner: session_owner.clone(),
                        verifier,
                    },
                    expires_at_unix_ms,
                    now,
                )
                .map_err(|_| connection_unavailable())?;
        }
        lock(&self.hosted_sessions).insert(
            session_ref.clone(),
            HostedSession {
                capability_sha256: Sha256::digest(capability.as_bytes()).into(),
                expires_at_unix_ms,
                profile,
                oauth_authorize_url,
            },
        );
        lock(&self.session_owners).insert(session_ref, session_owner);
        Ok(status)
    }

    fn expire_sessions(&self) {
        let Some(now) = now_ms() else {
            return;
        };
        let expired = lock(&self.hosted_sessions)
            .iter()
            .filter(|(_, session)| now >= session.expires_at_unix_ms)
            .map(|(session_ref, _)| session_ref.clone())
            .collect::<Vec<_>>();
        for session_ref in expired {
            lock(&self.hosted_sessions).remove(&session_ref);
            lock(&self.session_owners).remove(&session_ref);
            let _ = lock(&self.sessions).finish(&session_ref, ConnectSessionTerminal::Expired);
        }
        lock(&self.oauth_states).expire(now);
    }

    fn oauth_authorize_url(&self, state: &str, challenge: &str) -> Result<String, GitlabError> {
        connector_oauth::authorize_url(
            &self.origin,
            "/oauth/authorize",
            &AuthorizeParams {
                client_id: &self.policy.oauth_client_id,
                redirect_uri: &self.policy.oauth_redirect_uri,
                scope: "api",
                state,
                code_challenge: Some(challenge),
                extra: &[],
            },
        )
        .map_err(|error| GitlabError::new(error.code()))
    }

    async fn complete_oauth(
        &self,
        state: &str,
        code: Option<&str>,
        error: Option<&str>,
    ) -> Result<(), HostedCompletionError> {
        self.expire_sessions();
        let Pending {
            payload: pending,
            expires_at_unix_ms,
        } = lock(&self.oauth_states)
            .remove(state)
            .ok_or(HostedCompletionError::NotFound)?;
        lock(&self.hosted_sessions).remove(&pending.session_ref);
        if error.is_some() || code.is_none() || now_ms().is_none_or(|now| now >= expires_at_unix_ms)
        {
            let _ =
                lock(&self.sessions).finish(&pending.session_ref, ConnectSessionTerminal::Failed);
            return Err(HostedCompletionError::Refused);
        }
        let _completion = self.completion_lock.lock().await;
        let outcome = async {
            let token = self
                .exchange_oauth_code(
                    &pending.session_ref,
                    code.expect("checked code"),
                    &pending.verifier,
                )
                .await?;
            let evidence = self
                .verify_oauth(
                    &pending.session_ref,
                    &token.access_token,
                    &pending.owner.email,
                )
                .await?;
            self.commit_connection(&pending.session_ref, pending.owner, evidence, token)
                .await
        }
        .await;
        self.finish_session(&pending.session_ref, outcome)
    }

    fn finish_session(
        &self,
        session_ref: &str,
        outcome: Result<String, GitlabError>,
    ) -> Result<(), HostedCompletionError> {
        match outcome {
            Ok(connection_ref) => lock(&self.sessions)
                .finish(
                    session_ref,
                    ConnectSessionTerminal::Completed { connection_ref },
                )
                .map(|_| ())
                .map_err(|_| HostedCompletionError::Unavailable),
            Err(error) => {
                let _ = lock(&self.sessions).finish(session_ref, ConnectSessionTerminal::Failed);
                Err(hosted_error(error))
            }
        }
    }

    async fn exchange_oauth_code(
        &self,
        session_ref: &str,
        code: &str,
        verifier: &str,
    ) -> Result<CredentialValues, GitlabError> {
        let secret_ref = CredentialRef::new(
            &self.tenant_id,
            AUTHORITY,
            LOGIN_SERVICE,
            OAUTH_CLIENT_SECRET_CREDENTIAL,
        )
        .map_err(|_| GitlabError::new("credential-address"))?;
        let client_secret = self
            .credential_store
            .get(&secret_ref)
            .await
            .map_err(|_| GitlabError::new("oauth-config"))?;
        let mut token_url = self.origin.clone();
        token_url.set_path("/oauth/token");
        let body = form_body(&[
            ("client_id", self.policy.oauth_client_id.as_str()),
            ("client_secret", client_secret.expose_secret()),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", self.policy.oauth_redirect_uri.as_str()),
            ("code_verifier", verifier),
        ]);
        let response = self
            .execute(
                session_ref,
                http_request(
                    "POST",
                    token_url,
                    BTreeMap::from([(
                        "content-type".to_owned(),
                        "application/x-www-form-urlencoded".to_owned(),
                    )]),
                    Some(body),
                ),
                64 * 1024,
                Vec::new(),
            )
            .await
            .map_err(|_| GitlabError::new("oauth-exchange"))?;
        drop(client_secret);
        let value: OAuthTokenResponse = decode_response(response)?;
        let token = connector_oauth::validate(token_response(value), &EXCHANGE_POLICY)
            .map_err(|_| GitlabError::new("oauth-exchange"))?;
        Ok(CredentialValues {
            access_token: Secret::new(token.access_token.to_string()),
            refresh_token: token
                .refresh_token
                .map(|refresh| Secret::new(refresh.to_string())),
        })
    }

    async fn verify_oauth(
        &self,
        authority_ref: &str,
        token: &Secret,
        expected_email: &str,
    ) -> Result<VerifiedCredential, GitlabError> {
        let info: OAuthTokenInfo = self
            .provider_json(authority_ref, "/oauth/token/info", token, &[])
            .await?;
        let user = self.current_user(authority_ref, token).await?;
        if info.resource_owner_id != user.id
            || info.expires_in_seconds == 0
            || info.created_at == 0
            || !info.scopes.iter().any(|scope| scope == "api")
        {
            return Err(GitlabError::new("oauth-evidence"));
        }
        verify_user(&user, expected_email)?;
        let expires_at_unix_ms = info
            .created_at
            .checked_add(info.expires_in_seconds)
            .and_then(|seconds| seconds.checked_mul(1_000))
            .ok_or_else(|| GitlabError::new("oauth-evidence"))?;
        Ok(VerifiedCredential {
            user,
            scopes: canonical_scopes(info.scopes),
            expires_at_unix_ms: Some(expires_at_unix_ms),
        })
    }

    async fn verify_pat(
        &self,
        authority_ref: &str,
        token: &Secret,
        expected_email: &str,
    ) -> Result<VerifiedCredential, GitlabError> {
        let info: PersonalTokenInfo = self
            .provider_json(
                authority_ref,
                "/api/v4/personal_access_tokens/self",
                token,
                &[],
            )
            .await?;
        if !info.active
            || info.revoked
            || info.expires_at.as_deref().is_some_and(str::is_empty)
            || !info
                .scopes
                .iter()
                .any(|scope| matches!(scope.as_str(), "read_api" | "api"))
        {
            return Err(GitlabError::new("pat-evidence"));
        }
        let user = self.current_user(authority_ref, token).await?;
        verify_user(&user, expected_email)?;
        Ok(VerifiedCredential {
            user,
            scopes: canonical_scopes(info.scopes),
            expires_at_unix_ms: None,
        })
    }

    async fn current_user(
        &self,
        authority_ref: &str,
        token: &Secret,
    ) -> Result<GitlabUser, GitlabError> {
        self.provider_json(authority_ref, "/api/v4/user", token, &[])
            .await
    }

    async fn commit_connection(
        &self,
        session_ref: &str,
        owner: SessionOwner,
        evidence: VerifiedCredential,
        credentials: CredentialValues,
    ) -> Result<String, GitlabError> {
        let label = lock(&self.sessions)
            .pending_label(session_ref)
            .map_err(|_| GitlabError::new("connect-session"))?;
        let existing = lock(&self.metadata)
            .connections
            .iter()
            .find(|connection| {
                connection.owner_subject == owner.subject && connection.profile == owner.profile
            })
            .cloned();
        let (instance_id, connection_ref) = existing.map_or_else(
            || {
                random_uuid().map(|id| {
                    let reference = format!("connection:gitlab:{id}");
                    (id, reference)
                })
            },
            |connection| Ok((connection.instance_id, connection.connection_ref)),
        )?;
        let (transaction, generation) = self.reserve_transaction()?;
        let generation_value = u64::from_be_bytes(generation.protocol_bytes());
        let connection = StoredConnection {
            connection_ref: connection_ref.clone(),
            instance_id,
            label,
            owner_subject: owner.subject,
            external_user_id: evidence.user.id,
            username: bounded_string(&evidence.user.username, 128),
            email_sha256: email_sha256(&owner.email),
            profile: owner.profile,
            scopes: evidence.scopes,
            credential_generation: generation_value,
            observed_at_unix_ms: now_ms().ok_or_else(|| GitlabError::new("clock"))?,
            expires_at_unix_ms: evidence.expires_at_unix_ms,
        };
        self.commit_credentials(transaction, generation, connection, credentials)
            .await?;
        Ok(connection_ref)
    }

    async fn commit_credentials(
        &self,
        transaction: SecretTransactionId,
        generation: SecretTransactionGeneration,
        connection: StoredConnection,
        credentials: CredentialValues,
    ) -> Result<(), GitlabError> {
        let mut batch = SecretBatch::new(
            CredentialScope::new(&self.tenant_id, AUTHORITY)
                .map_err(|_| GitlabError::new("credential-address"))?,
        );
        batch
            .put(
                self.connection_credential_ref(&connection, ACCESS_TOKEN_CREDENTIAL)?,
                credentials.access_token,
            )
            .map_err(|_| GitlabError::new("credential-batch"))?;
        if let Some(refresh) = credentials.refresh_token {
            batch
                .put(
                    self.connection_credential_ref(&connection, REFRESH_TOKEN_CREDENTIAL)?,
                    refresh,
                )
                .map_err(|_| GitlabError::new("credential-batch"))?;
        }
        self.credential_store
            .prepare(transaction, proposal_digest(&batch), &batch)
            .await
            .map_err(|_| GitlabError::new("credential-prepare"))?;
        let transaction_id = hex::encode(transaction.protocol_bytes());
        let persisted = {
            let mut state = lock(&self.metadata);
            state.pending.push(PendingCommit {
                transaction_id: transaction_id.clone(),
                connection: connection.clone(),
            });
            let result = self.persist(&state);
            if result.is_err() {
                state
                    .pending
                    .retain(|pending| pending.transaction_id != transaction_id);
            }
            result
        };
        if let Err(error) = persisted {
            let _ = self.credential_store.abort(transaction).await;
            return Err(error);
        }
        self.credential_store
            .commit(transaction)
            .await
            .map_err(|_| GitlabError::new("credential-commit"))?;
        {
            let mut state = lock(&self.metadata);
            state
                .pending
                .retain(|pending| pending.transaction_id != transaction_id);
            upsert_connection(&mut state.connections, connection);
            self.persist(&state)?;
        }
        let _ = self.credential_store.reclaim(generation).await;
        Ok(())
    }

    fn reserve_transaction(
        &self,
    ) -> Result<(SecretTransactionId, SecretTransactionGeneration), GitlabError> {
        let mut state = lock(&self.metadata);
        let generation = SecretTransactionGeneration::from_protocol_bytes(
            state.next_transaction_generation.to_be_bytes(),
        )
        .ok_or_else(|| GitlabError::new("transaction-generation"))?;
        state.next_transaction_generation = state
            .next_transaction_generation
            .checked_add(1)
            .ok_or_else(|| GitlabError::new("transaction-generation"))?;
        self.persist(&state)?;
        let mut nonce = [0_u8; 24];
        getrandom::fill(&mut nonce).map_err(|_| GitlabError::new("randomness"))?;
        Ok((SecretTransactionId::new(generation, nonce), generation))
    }

    async fn recover_pending(&self) -> Result<(), GitlabError> {
        let pending_transactions = lock(&self.metadata).pending.clone();
        for pending in pending_transactions {
            let transaction = decode_transaction(&pending.transaction_id)?;
            match self
                .credential_store
                .state(transaction)
                .await
                .map_err(|_| GitlabError::new("credential-recovery"))?
            {
                SecretTransactionState::Prepared => {
                    self.credential_store
                        .commit(transaction)
                        .await
                        .map_err(|_| GitlabError::new("credential-recovery"))?;
                }
                SecretTransactionState::Committed => {}
                SecretTransactionState::Absent => {
                    let mut state = lock(&self.metadata);
                    state
                        .pending
                        .retain(|candidate| candidate.transaction_id != pending.transaction_id);
                    self.persist(&state)?;
                    continue;
                }
            }
            let mut state = lock(&self.metadata);
            state
                .pending
                .retain(|candidate| candidate.transaction_id != pending.transaction_id);
            upsert_connection(&mut state.connections, pending.connection);
            self.persist(&state)?;
        }
        Ok(())
    }

    fn connection_credential_ref(
        &self,
        connection: &StoredConnection,
        credential: &str,
    ) -> Result<CredentialRef, GitlabError> {
        CredentialRef::for_instance(
            &self.tenant_id,
            AUTHORITY,
            &connection.instance_id,
            SERVICE,
            credential,
        )
        .map_err(|_| GitlabError::new("credential-address"))
    }

    fn operation_connections(
        &self,
        context: &PrincipalContext,
        operation_ref: &str,
    ) -> Vec<OperationConnectionSummary> {
        self.owned_connections(context)
            .into_iter()
            .filter(|connection| supports_operation(connection, operation_ref))
            .map(|connection| OperationConnectionSummary {
                connection_ref: connection.connection_ref,
                label: connection.label,
                provider: INTEGRATION_REF.to_owned(),
                audiences: vec!["delegated-user".to_owned()],
                purpose: None,
            })
            .collect()
    }

    fn search_operations(&self, context: &PrincipalContext, query: &str) -> Vec<OperationSummary> {
        let query = query.to_ascii_lowercase();
        GITLAB_OPERATIONS
            .iter()
            .filter_map(|operation_ref| {
                let connections = self.operation_connections(context, operation_ref);
                if connections.is_empty() {
                    return None;
                }
                let operation = connector_resolve::document::operation(operation_ref)?;
                let title = operation_ref.replace('-', " ");
                (query.is_empty()
                    || operation_ref.contains(&query)
                    || operation
                        .contract_description()
                        .to_ascii_lowercase()
                        .contains(&query))
                .then(|| OperationSummary {
                    operation_ref: (*operation_ref).to_owned(),
                    title,
                    effect: operation_effect(operation_ref),
                    approval: operation_approval(operation_ref),
                    connections,
                })
            })
            .collect()
    }

    fn operation_description_ref(&self, context: &PrincipalContext, operation_ref: &str) -> String {
        let mut digest = Sha256::new();
        digest.update(b"b10x/gitlab-operation-description/v1\0");
        digest.update(context.stable_authority_seed());
        digest.update(b"\0");
        digest.update(operation_ref.as_bytes());
        for connection in self.operation_connections(context, operation_ref) {
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
        if !is_gitlab_operation(operation_ref) {
            return Err(operation_not_found());
        }
        let operation = connector_resolve::document::operation(operation_ref)
            .ok_or_else(operation_not_found)?;
        let connections = self.operation_connections(context, operation_ref);
        if connections.is_empty() {
            return Err(operation_not_found());
        }
        Ok(OperationResult::Describe(OperationDescription {
            operation_ref: operation_ref.to_owned(),
            title: operation_ref.replace('-', " "),
            description: operation.contract_description().to_owned(),
            input_schema: operation.input_schema().clone(),
            output_schema: serde_json::json!({"type":"object"}),
            effect: operation_effect(operation_ref),
            approval: operation_approval(operation_ref),
            connections,
            description_ref: self.operation_description_ref(context, operation_ref),
        }))
    }

    async fn invoke(
        &self,
        context: &PrincipalContext,
        request: InvokeRequest,
    ) -> Result<OperationResult, OperationError> {
        if !is_gitlab_operation(&request.operation_ref) {
            return Err(operation_not_found());
        }
        let connection = self
            .owned_connections(context)
            .into_iter()
            .find(|connection| {
                connection.connection_ref == request.connection_ref
                    && supports_operation(connection, &request.operation_ref)
            })
            .ok_or_else(operation_not_granted)?;
        if request.description_ref
            != self.operation_description_ref(context, &request.operation_ref)
        {
            return Err(OperationError::new(
                OperationErrorCode::StaleAuthority,
                "GitLab operation description lease is stale",
                false,
            ));
        }
        // Writes describe `ApprovalPosture::Required`; the demanded approval is verified and
        // spent upstream by the sealed proof chain (S-045, S-046) before this Integration is
        // reachable, so no local reading of the evidence reference decides admission (S-047).
        let operation = connector_resolve::document::operation(&request.operation_ref)
            .ok_or_else(operation_not_found)?;
        let validator = jsonschema::validator_for(operation.input_schema())
            .map_err(|_| operation_unavailable())?;
        if !validator.is_valid(&request.input) {
            return Err(operation_invalid());
        }
        let token = self
            .connection_token(&connection)
            .await
            .map_err(|_| operation_not_granted())?;
        let declared_name = match connection.profile {
            GitlabProfile::OAuthUser => "gitlab.oauth_token",
            GitlabProfile::PersonalToken => "gitlab.token",
        };
        let assembled = connector_resolve::auth::Assembled::new(
            declared_name,
            token.expose_secret().to_owned(),
            catalog::Placement::Header {
                name: "Authorization",
                prefix: "Bearer ",
            },
        );
        drop(token);
        let base = format!("{}/api/v4", self.origin.as_str().trim_end_matches('/'));
        let plan = connector_resolve::resolve(
            operation,
            &base,
            &request.input,
            &BTreeMap::new(),
            &[assembled],
        )
        .map_err(|_| operation_invalid())?;
        let target = url::Url::parse(&plan.request.url).map_err(|_| operation_unavailable())?;
        if !same_origin(&self.origin, &target) || !target.path().starts_with("/api/v4/") {
            return Err(operation_not_granted());
        }
        let audit_ref = format!(
            "audit:gitlab:{}",
            random_uuid().map_err(|_| operation_unavailable())?
        );
        self.audit(
            &audit_ref,
            &request.operation_ref,
            &request.connection_ref,
            context,
            "attempted",
        )
        .map_err(|_| operation_unavailable())?;
        let response = self
            .egress
            .execute(
                &request.connection_ref,
                EgressHttpRequest {
                    request: plan.request,
                    maximum_response_bytes: protocol::operation::MAX_RESULT_BYTES,
                    response_headers: Vec::new(),
                },
            )
            .await;
        let output = match response {
            Ok(response) => decode_value_response(response).map_err(|_| {
                if operation_effect(&request.operation_ref) == EffectClass::ReadOnly {
                    operation_unavailable()
                } else {
                    operation_outcome_unknown(&request.operation_ref)
                }
            }),
            Err(_) => Err(
                if operation_effect(&request.operation_ref) == EffectClass::ReadOnly {
                    operation_unavailable()
                } else {
                    operation_outcome_unknown(&request.operation_ref)
                },
            ),
        };
        match output {
            Ok(output) => {
                self.audit(
                    &audit_ref,
                    &request.operation_ref,
                    &request.connection_ref,
                    context,
                    "completed",
                )
                .map_err(|_| operation_outcome_unknown(&request.operation_ref))?;
                Ok(OperationResult::Invoke(InvocationResult {
                    operation_ref: request.operation_ref,
                    output,
                    connector_audit_ref: audit_ref,
                    execution_ref: None,
                }))
            }
            Err(error) => {
                let _ = self.audit(
                    &audit_ref,
                    &request.operation_ref,
                    &request.connection_ref,
                    context,
                    "indeterminate",
                );
                Err(error)
            }
        }
    }

    async fn connection_token(&self, connection: &StoredConnection) -> Result<Secret, GitlabError> {
        if connection.profile == GitlabProfile::OAuthUser
            && connection.expires_at_unix_ms.is_some_and(|expires| {
                now_ms().is_none_or(|now| {
                    expires
                        <= now
                            .saturating_add(self.policy.refresh_skew_seconds.saturating_mul(1_000))
                })
            })
        {
            self.refresh_oauth(&connection.connection_ref).await?;
        }
        let current = lock(&self.metadata)
            .connections
            .iter()
            .find(|candidate| candidate.connection_ref == connection.connection_ref)
            .cloned()
            .ok_or_else(|| GitlabError::new("connection-state"))?;
        self.credential_store
            .get(&self.connection_credential_ref(&current, ACCESS_TOKEN_CREDENTIAL)?)
            .await
            .map_err(|_| GitlabError::new("credential-resolve"))
    }

    async fn refresh_oauth(&self, connection_ref: &str) -> Result<(), GitlabError> {
        let _refresh = self.refresh_lock.lock().await;
        let connection = lock(&self.metadata)
            .connections
            .iter()
            .find(|candidate| candidate.connection_ref == connection_ref)
            .cloned()
            .ok_or_else(|| GitlabError::new("connection-state"))?;
        if connection.profile != GitlabProfile::OAuthUser
            || connection.expires_at_unix_ms.is_none_or(|expires| {
                now_ms().is_some_and(|now| {
                    expires
                        > now.saturating_add(self.policy.refresh_skew_seconds.saturating_mul(1_000))
                })
            })
        {
            return Ok(());
        }
        let refresh_ref = self.connection_credential_ref(&connection, REFRESH_TOKEN_CREDENTIAL)?;
        let refresh_token = self
            .credential_store
            .get(&refresh_ref)
            .await
            .map_err(|_| GitlabError::new("oauth-refresh"))?;
        let secret_ref = CredentialRef::new(
            &self.tenant_id,
            AUTHORITY,
            LOGIN_SERVICE,
            OAUTH_CLIENT_SECRET_CREDENTIAL,
        )
        .map_err(|_| GitlabError::new("credential-address"))?;
        let client_secret = self
            .credential_store
            .get(&secret_ref)
            .await
            .map_err(|_| GitlabError::new("oauth-config"))?;
        let mut token_url = self.origin.clone();
        token_url.set_path("/oauth/token");
        let body = form_body(&[
            ("client_id", self.policy.oauth_client_id.as_str()),
            ("client_secret", client_secret.expose_secret()),
            ("refresh_token", refresh_token.expose_secret()),
            ("grant_type", "refresh_token"),
            ("redirect_uri", self.policy.oauth_redirect_uri.as_str()),
        ]);
        let response = self
            .execute(
                connection_ref,
                http_request(
                    "POST",
                    token_url,
                    BTreeMap::from([(
                        "content-type".to_owned(),
                        "application/x-www-form-urlencoded".to_owned(),
                    )]),
                    Some(body),
                ),
                64 * 1024,
                Vec::new(),
            )
            .await
            .map_err(|_| GitlabError::new("oauth-refresh"))?;
        drop(client_secret);
        drop(refresh_token);
        let exchanged: OAuthTokenResponse = decode_response(response)?;
        let refreshed = connector_oauth::validate(token_response(exchanged), &REFRESH_POLICY)
            .map_err(|_| GitlabError::new("oauth-refresh"))?;
        let access = Secret::new(refreshed.access_token.to_string());
        let info: OAuthTokenInfo = self
            .provider_json(connection_ref, "/oauth/token/info", &access, &[])
            .await?;
        let user = self.current_user(connection_ref, &access).await?;
        if user.state != "active"
            || user.bot
            || info.resource_owner_id != connection.external_user_id
            || user
                .email
                .as_deref()
                .is_none_or(|email| email_sha256(email) != connection.email_sha256)
        {
            return Err(GitlabError::new("oauth-refresh-evidence"));
        }
        let (transaction, generation) = self.reserve_transaction()?;
        let mut updated = connection;
        updated.scopes = canonical_scopes(info.scopes);
        updated.credential_generation = u64::from_be_bytes(generation.protocol_bytes());
        updated.observed_at_unix_ms = now_ms().ok_or_else(|| GitlabError::new("clock"))?;
        updated.expires_at_unix_ms = info
            .created_at
            .checked_add(info.expires_in_seconds)
            .and_then(|seconds| seconds.checked_mul(1_000));
        if updated.expires_at_unix_ms.is_none() {
            return Err(GitlabError::new("oauth-refresh-evidence"));
        }
        self.commit_credentials(
            transaction,
            generation,
            updated,
            CredentialValues {
                access_token: access,
                refresh_token: refreshed
                    .refresh_token
                    .map(|refresh| Secret::new(refresh.to_string())),
            },
        )
        .await
    }

    fn audit(
        &self,
        audit_ref: &str,
        operation_ref: &str,
        connection_ref: &str,
        context: &PrincipalContext,
        outcome: &str,
    ) -> Result<(), GitlabError> {
        let mut line = serde_json::to_vec(&serde_json::json!({
            "at_unix_ms": now_ms().ok_or_else(|| GitlabError::new("clock"))?,
            "audit_ref": audit_ref,
            "operation_ref": operation_ref,
            "connection_ref": connection_ref,
            "tenant_id": context.tenant_id(),
            "actor_subject": context.actor_subject(),
            "outcome": outcome,
        }))
        .map_err(|_| GitlabError::new("audit-store"))?;
        line.push(b'\n');
        self.state_store.append(AUDIT_KEY, &line, MAX_AUDIT_BYTES)
    }

    async fn provider_json<T: for<'de> Deserialize<'de>>(
        &self,
        authority_ref: &str,
        path: &str,
        token: &Secret,
        query: &[(String, String)],
    ) -> Result<T, GitlabError> {
        let mut target = self.provider_url(path)?;
        target.query_pairs_mut().extend_pairs(query);
        let response = self
            .execute(
                authority_ref,
                http_request("GET", target, bearer_headers(token), None),
                MAX_PROVIDER_RESPONSE_BYTES,
                Vec::new(),
            )
            .await
            .map_err(|_| GitlabError::new("provider-unavailable"))?;
        decode_response(response)
    }

    async fn execute(
        &self,
        authority_ref: &str,
        request: connector_resolve::Request,
        maximum_response_bytes: usize,
        response_headers: Vec<String>,
    ) -> Result<EgressHttpResponse, GitlabError> {
        self.egress
            .execute(
                authority_ref,
                EgressHttpRequest {
                    request,
                    maximum_response_bytes,
                    response_headers,
                },
            )
            .await
            .map_err(|_| GitlabError::new("provider-unavailable"))
    }

    fn provider_url(&self, path: &str) -> Result<url::Url, GitlabError> {
        if !path.starts_with('/') || path.contains("//") {
            return Err(GitlabError::new("provider-path"));
        }
        let mut target = self.origin.clone();
        target.set_path(path);
        if !same_origin(&self.origin, &target) {
            return Err(GitlabError::new("provider-origin"));
        }
        Ok(target)
    }

    fn search_datasources(
        &self,
        context: &PrincipalContext,
        query: &str,
    ) -> Vec<DatasourceSummary> {
        if self.owned_connections(context).is_empty() {
            return Vec::new();
        }
        let query = query.to_ascii_lowercase();
        GITLAB_DATASOURCES
            .iter()
            .filter_map(|datasource_ref| {
                let summary = datasource_summary(datasource_ref)?;
                (query.is_empty()
                    || datasource_ref.contains(&query)
                    || summary.title.to_ascii_lowercase().contains(&query))
                .then_some(summary)
            })
            .collect()
    }

    fn datasource_description_ref(
        &self,
        context: &PrincipalContext,
        datasource_ref: &str,
    ) -> String {
        let mut digest = Sha256::new();
        digest.update(b"b10x/gitlab-datasource-description/v1\0");
        digest.update(context.stable_authority_seed());
        digest.update(b"\0");
        digest.update(datasource_ref.as_bytes());
        digest.update(b"\0");
        digest.update(datasource_projection_sha256(datasource_ref).as_bytes());
        for connection in self.owned_connections(context) {
            digest.update(b"\0");
            digest.update(connection.connection_ref.as_bytes());
            digest.update(b"\0");
            digest.update(connection.credential_generation.to_be_bytes());
        }
        format!("datasource-description:gitlab:{:x}", digest.finalize())
    }

    fn describe_datasource(
        &self,
        context: &PrincipalContext,
        datasource_ref: &str,
    ) -> Result<DatasourceDescription, DatasourceError> {
        if self.owned_connections(context).is_empty() {
            return Err(datasource_not_granted());
        }
        let summary = datasource_summary(datasource_ref).ok_or_else(datasource_not_found)?;
        let (description, key_schema, compact_schema, detail_schema) =
            datasource_declaration(datasource_ref).ok_or_else(datasource_not_found)?;
        Ok(DatasourceDescription {
            summary,
            description: description.to_owned(),
            key_schema,
            compact_schema,
            detail_schema,
            projection_protocol: VALUE_PROJECTION_PROTOCOL.to_owned(),
            projection_sha256: datasource_projection_sha256(datasource_ref),
            description_ref: self.datasource_description_ref(context, datasource_ref),
        })
    }

    async fn datasource_bindings(
        &self,
        context: &PrincipalContext,
        datasource_ref: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<DatasourceBinding>, DatasourceError> {
        if datasource_summary(datasource_ref).is_none() {
            return Err(datasource_not_found());
        }
        let query = query.to_ascii_lowercase();
        let mut bindings = Vec::new();
        for connection in self.owned_connections(context) {
            if project_bound(datasource_ref) {
                let token = self
                    .connection_token(&connection)
                    .await
                    .map_err(|_| datasource_unavailable())?;
                let projects: Vec<Value> = self
                    .provider_json(
                        &connection.connection_ref,
                        "/api/v4/projects",
                        &token,
                        &[
                            ("membership".to_owned(), "true".to_owned()),
                            ("simple".to_owned(), "true".to_owned()),
                            ("order_by".to_owned(), "last_activity_at".to_owned()),
                            ("per_page".to_owned(), "100".to_owned()),
                        ],
                    )
                    .await
                    .map_err(|_| datasource_unavailable())?;
                drop(token);
                for project in projects {
                    let Some(project_id) = project.get("id").and_then(Value::as_u64) else {
                        continue;
                    };
                    let label = project
                        .get("path_with_namespace")
                        .and_then(Value::as_str)
                        .map(|value| bounded_string(value, 240))
                        .unwrap_or_else(|| format!("GitLab project {project_id}"));
                    if !query.is_empty() && !label.to_ascii_lowercase().contains(&query) {
                        continue;
                    }
                    bindings.push(DatasourceBinding {
                        datasource_ref: datasource_ref.to_owned(),
                        binding_ref: datasource_binding_ref(
                            datasource_ref,
                            &connection.connection_ref,
                            connection.credential_generation,
                            Some(project_id),
                        ),
                        connection_ref: connection.connection_ref.clone(),
                        label,
                        generation: connection.credential_generation,
                        purpose: None,
                    });
                    if bindings.len() >= limit {
                        return Ok(bindings);
                    }
                }
            } else if query.is_empty() || connection.label.to_ascii_lowercase().contains(&query) {
                bindings.push(DatasourceBinding {
                    datasource_ref: datasource_ref.to_owned(),
                    binding_ref: datasource_binding_ref(
                        datasource_ref,
                        &connection.connection_ref,
                        connection.credential_generation,
                        None,
                    ),
                    connection_ref: connection.connection_ref.clone(),
                    label: connection.label.clone(),
                    generation: connection.credential_generation,
                    purpose: None,
                });
                if bindings.len() >= limit {
                    return Ok(bindings);
                }
            }
        }
        Ok(bindings)
    }

    async fn read_datasource(
        &self,
        context: &PrincipalContext,
        request: DatasourceReadRequest,
    ) -> Result<DatasourcePage, DatasourceError> {
        if request.description_ref
            != self.datasource_description_ref(context, &request.datasource_ref)
        {
            return Err(datasource_error(
                DatasourceErrorCode::StaleAuthority,
                "GitLab datasource description lease is stale",
                false,
            ));
        }
        let connection = self
            .owned_connections(context)
            .into_iter()
            .find(|connection| {
                connection.connection_ref == binding_connection(&request.binding_ref)
            })
            .ok_or_else(datasource_not_granted)?;
        if connection.credential_generation
            != binding_generation_hint(&request.binding_ref)
                .unwrap_or(connection.credential_generation)
        {
            return Err(datasource_error(
                DatasourceErrorCode::StaleAuthority,
                "GitLab datasource binding is stale",
                false,
            ));
        }
        let token = self
            .connection_token(&connection)
            .await
            .map_err(|_| datasource_not_granted())?;
        let project_id = if project_bound(&request.datasource_ref) {
            Some(
                self.resolve_project_binding(
                    &request.datasource_ref,
                    &request.binding_ref,
                    &connection,
                    &token,
                )
                .await?,
            )
        } else {
            let expected = datasource_binding_ref(
                &request.datasource_ref,
                &connection.connection_ref,
                connection.credential_generation,
                None,
            );
            if expected != request.binding_ref {
                return Err(datasource_not_granted());
            }
            None
        };
        let plan = datasource_request_plan(
            &request.datasource_ref,
            project_id,
            &request.read,
            &connection,
        )?;
        let audit_ref = format!(
            "audit:gitlab:{}",
            random_uuid().map_err(|_| datasource_unavailable())?
        );
        self.audit(
            &audit_ref,
            &request.datasource_ref,
            &connection.connection_ref,
            context,
            "attempted",
        )
        .map_err(|_| datasource_unavailable())?;
        let mut target = self
            .provider_url(&plan.path)
            .map_err(|_| datasource_unavailable())?;
        target.query_pairs_mut().extend_pairs(&plan.query);
        let response = self
            .execute(
                &connection.connection_ref,
                http_request("GET", target, bearer_headers(&token), None),
                protocol::datasource::MAX_RESULT_BYTES,
                vec!["x-next-page".to_owned()],
            )
            .await;
        drop(token);
        let (payload, next_page) = match response {
            Ok(response) => match decode_page_response(response) {
                Ok(value) => value,
                Err(_) => {
                    let _ = self.audit(
                        &audit_ref,
                        &request.datasource_ref,
                        &connection.connection_ref,
                        context,
                        "indeterminate",
                    );
                    return Err(datasource_unavailable());
                }
            },
            Err(_) => {
                let _ = self.audit(
                    &audit_ref,
                    &request.datasource_ref,
                    &connection.connection_ref,
                    context,
                    "indeterminate",
                );
                return Err(datasource_unavailable());
            }
        };
        let records = normalize_records(&request.datasource_ref, plan.view, payload)?;
        let description = self.describe_datasource(context, &request.datasource_ref)?;
        let schema = if plan.view == DatasourceRecordView::Compact {
            &description.compact_schema
        } else {
            &description.detail_schema
        };
        let validator = jsonschema::validator_for(schema).map_err(|_| datasource_unavailable())?;
        if records
            .iter()
            .any(|record| !validator.is_valid(&record.value))
        {
            return Err(datasource_error(
                DatasourceErrorCode::Protocol,
                "GitLab datasource projection did not match its declaration",
                false,
            ));
        }
        self.audit(
            &audit_ref,
            &request.datasource_ref,
            &connection.connection_ref,
            context,
            "completed",
        )
        .map_err(|_| datasource_unavailable())?;
        let next_cursor = next_page.map(|page| {
            datasource_cursor(
                &request.datasource_ref,
                &connection.connection_ref,
                project_id,
                page,
            )
        });
        Ok(DatasourcePage {
            datasource_ref: request.datasource_ref,
            records,
            next_cursor,
            completeness: if next_page.is_some() {
                DatasourceCompleteness::Partial
            } else {
                DatasourceCompleteness::Complete
            },
            observed_at_unix_ms: now_ms().ok_or_else(datasource_unavailable)?,
            provenance: DatasourceProvenance {
                binding_ref: request.binding_ref,
                projection_sha256: description.projection_sha256,
                connector_audit_ref: audit_ref,
            },
        })
    }

    async fn resolve_project_binding(
        &self,
        datasource_ref: &str,
        binding_ref: &str,
        connection: &StoredConnection,
        token: &Secret,
    ) -> Result<u64, DatasourceError> {
        let projects: Vec<Value> = self
            .provider_json(
                &connection.connection_ref,
                "/api/v4/projects",
                token,
                &[
                    ("membership".to_owned(), "true".to_owned()),
                    ("simple".to_owned(), "true".to_owned()),
                    ("per_page".to_owned(), "100".to_owned()),
                ],
            )
            .await
            .map_err(|_| datasource_unavailable())?;
        projects
            .into_iter()
            .filter_map(|project| project.get("id").and_then(Value::as_u64))
            .find(|project_id| {
                datasource_binding_ref(
                    datasource_ref,
                    &connection.connection_ref,
                    connection.credential_generation,
                    Some(*project_id),
                ) == binding_ref
            })
            .ok_or_else(datasource_not_granted)
    }
}

struct DatasourcePlan {
    path: String,
    query: Vec<(String, String)>,
    view: DatasourceRecordView,
}

fn datasource_request_plan(
    datasource_ref: &str,
    project_id: Option<u64>,
    read: &DatasourceRead,
    connection: &StoredConnection,
) -> Result<DatasourcePlan, DatasourceError> {
    let (view, key, limit, page) = match read {
        DatasourceRead::List { limit, cursor } => {
            let page = cursor
                .as_deref()
                .map(|cursor| {
                    parse_datasource_cursor(
                        cursor,
                        datasource_ref,
                        &connection.connection_ref,
                        project_id,
                    )
                })
                .transpose()?
                .unwrap_or(1);
            (DatasourceRecordView::Compact, None, Some(*limit), page)
        }
        DatasourceRead::Get { key } => (
            DatasourceRecordView::Detail,
            Some(
                key.as_u64()
                    .filter(|value| *value > 0)
                    .ok_or_else(datasource_invalid)?,
            ),
            None,
            1,
        ),
    };
    let mut query = Vec::new();
    if let Some(limit) = limit {
        query.push(("per_page".to_owned(), limit.to_string()));
        query.push(("page".to_owned(), page.to_string()));
    }
    let path = match (datasource_ref, key, project_id) {
        ("gitlab.users", None, None) => "/api/v4/users".to_owned(),
        ("gitlab.users", Some(id), None) => format!("/api/v4/users/{id}"),
        ("gitlab.groups", None, None) => "/api/v4/groups".to_owned(),
        ("gitlab.groups", Some(id), None) => format!("/api/v4/groups/{id}"),
        ("gitlab.projects", None, None) => {
            query.push(("membership".to_owned(), "true".to_owned()));
            query.push(("order_by".to_owned(), "last_activity_at".to_owned()));
            "/api/v4/projects".to_owned()
        }
        ("gitlab.projects", Some(id), None) => format!("/api/v4/projects/{id}"),
        ("gitlab.issues", None, Some(project)) => {
            query.push(("order_by".to_owned(), "updated_at".to_owned()));
            format!("/api/v4/projects/{project}/issues")
        }
        ("gitlab.issues", Some(iid), Some(project)) => {
            format!("/api/v4/projects/{project}/issues/{iid}")
        }
        ("gitlab.merge_requests", None, Some(project)) => {
            query.push(("order_by".to_owned(), "updated_at".to_owned()));
            format!("/api/v4/projects/{project}/merge_requests")
        }
        ("gitlab.merge_requests", Some(iid), Some(project)) => {
            format!("/api/v4/projects/{project}/merge_requests/{iid}")
        }
        ("gitlab.branches", None, Some(project)) => {
            format!("/api/v4/projects/{project}/repository/branches")
        }
        ("gitlab.branches", Some(_), Some(_)) => return Err(datasource_invalid()),
        _ => return Err(datasource_invalid()),
    };
    Ok(DatasourcePlan { path, query, view })
}

fn normalize_records(
    datasource_ref: &str,
    view: DatasourceRecordView,
    payload: Value,
) -> Result<Vec<DatasourceRecord>, DatasourceError> {
    let values = match view {
        DatasourceRecordView::Compact => payload
            .as_array()
            .cloned()
            .ok_or_else(datasource_protocol)?,
        DatasourceRecordView::Detail => vec![payload],
    };
    if values.len() > usize::from(protocol::datasource::MAX_RESULTS) {
        return Err(datasource_error(
            DatasourceErrorCode::ResultTooLarge,
            "GitLab datasource returned too many records",
            false,
        ));
    }
    values
        .into_iter()
        .map(|value| {
            let projected = project_record(datasource_ref, view, &value)?;
            let key = record_key(datasource_ref, &projected)?;
            Ok(DatasourceRecord {
                key,
                view,
                value: projected,
            })
        })
        .collect()
}

fn project_record(
    datasource_ref: &str,
    view: DatasourceRecordView,
    value: &Value,
) -> Result<Value, DatasourceError> {
    let object = value.as_object().ok_or_else(datasource_protocol)?;
    let fields: &[&str] = match datasource_ref {
        "gitlab.users" => &[
            "id",
            "username",
            "name",
            "state",
            "bot",
            "web_url",
            "avatar_url",
        ],
        "gitlab.groups" => &[
            "id",
            "name",
            "path",
            "full_path",
            "description",
            "visibility",
            "web_url",
        ],
        "gitlab.projects" => &[
            "id",
            "name",
            "path",
            "path_with_namespace",
            "description",
            "default_branch",
            "visibility",
            "web_url",
            "last_activity_at",
        ],
        "gitlab.issues" => {
            if view == DatasourceRecordView::Detail {
                &[
                    "id",
                    "iid",
                    "project_id",
                    "title",
                    "description",
                    "state",
                    "confidential",
                    "labels",
                    "author",
                    "web_url",
                    "created_at",
                    "updated_at",
                    "closed_at",
                ]
            } else {
                &[
                    "id",
                    "iid",
                    "project_id",
                    "title",
                    "state",
                    "confidential",
                    "labels",
                    "author",
                    "web_url",
                    "created_at",
                    "updated_at",
                ]
            }
        }
        "gitlab.merge_requests" => {
            if view == DatasourceRecordView::Detail {
                &[
                    "id",
                    "iid",
                    "project_id",
                    "title",
                    "description",
                    "state",
                    "draft",
                    "source_branch",
                    "target_branch",
                    "sha",
                    "author",
                    "web_url",
                    "created_at",
                    "updated_at",
                    "merged_at",
                    "closed_at",
                ]
            } else {
                &[
                    "id",
                    "iid",
                    "project_id",
                    "title",
                    "state",
                    "draft",
                    "source_branch",
                    "target_branch",
                    "sha",
                    "author",
                    "web_url",
                    "created_at",
                    "updated_at",
                ]
            }
        }
        "gitlab.branches" => &[
            "name",
            "merged",
            "protected",
            "default",
            "can_push",
            "web_url",
            "commit",
        ],
        _ => return Err(datasource_not_found()),
    };
    let mut projected = Map::new();
    for field in fields {
        let Some(candidate) = object.get(*field) else {
            continue;
        };
        let value = match *field {
            "author" => project_nested(candidate, &["id", "username", "name"]),
            "commit" => project_nested(
                candidate,
                &["id", "short_id", "title", "author_name", "authored_date"],
            ),
            "labels" => Value::Array(
                candidate
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .take(25)
                    .map(|item| Value::String(bounded_string(item, 128)))
                    .collect(),
            ),
            _ => bound_json(candidate, 16 * 1024)?,
        };
        projected.insert((*field).to_owned(), value);
    }
    let projected = Value::Object(projected);
    if serde_json::to_vec(&projected).map_or(true, |bytes| {
        bytes.len() > protocol::datasource::MAX_RESULT_BYTES
    }) {
        return Err(datasource_error(
            DatasourceErrorCode::ResultTooLarge,
            "GitLab datasource projection exceeded its result bound",
            false,
        ));
    }
    Ok(projected)
}

fn project_nested(value: &Value, fields: &[&str]) -> Value {
    let mut projected = Map::new();
    if let Some(object) = value.as_object() {
        for field in fields {
            if let Some(value) = object.get(*field) {
                if let Ok(value) = bound_json(value, 2 * 1024) {
                    projected.insert((*field).to_owned(), value);
                }
            }
        }
    }
    Value::Object(projected)
}

fn bound_json(value: &Value, max_string: usize) -> Result<Value, DatasourceError> {
    match value {
        Value::String(value) => Ok(Value::String(bounded_string(value, max_string))),
        Value::Null | Value::Bool(_) | Value::Number(_) => Ok(value.clone()),
        _ => Err(datasource_protocol()),
    }
}

fn record_key(datasource_ref: &str, value: &Value) -> Result<Value, DatasourceError> {
    let field = if datasource_ref == "gitlab.branches" {
        "name"
    } else if matches!(datasource_ref, "gitlab.issues" | "gitlab.merge_requests") {
        "iid"
    } else {
        "id"
    };
    value.get(field).cloned().ok_or_else(datasource_protocol)
}

fn datasource_summary(datasource_ref: &str) -> Option<DatasourceSummary> {
    let (title, verbs) = match datasource_ref {
        "gitlab.users" => (
            "GitLab users",
            vec![DatasourceReadVerb::List, DatasourceReadVerb::Get],
        ),
        "gitlab.groups" => (
            "GitLab groups",
            vec![DatasourceReadVerb::List, DatasourceReadVerb::Get],
        ),
        "gitlab.projects" => (
            "GitLab projects",
            vec![DatasourceReadVerb::List, DatasourceReadVerb::Get],
        ),
        "gitlab.issues" => (
            "GitLab project issues",
            vec![DatasourceReadVerb::List, DatasourceReadVerb::Get],
        ),
        "gitlab.merge_requests" => (
            "GitLab project merge requests",
            vec![DatasourceReadVerb::List, DatasourceReadVerb::Get],
        ),
        "gitlab.branches" => ("GitLab project branches", vec![DatasourceReadVerb::List]),
        _ => return None,
    };
    Some(DatasourceSummary {
        datasource_ref: datasource_ref.to_owned(),
        title: title.to_owned(),
        access_mode: DatasourceAccessMode::Live,
        verbs,
    })
}

fn datasource_declaration(datasource_ref: &str) -> Option<(&'static str, Value, Value, Value)> {
    let key_schema = if datasource_ref == "gitlab.branches" {
        serde_json::json!({"not":{}})
    } else {
        serde_json::json!({"type":"integer","minimum":1})
    };
    let required: &[&str] = match datasource_ref {
        "gitlab.users" => &["id", "username", "state"],
        "gitlab.groups" => &["id", "name", "path", "full_path"],
        "gitlab.projects" => &["id", "name", "path_with_namespace"],
        "gitlab.issues" | "gitlab.merge_requests" => &["id", "iid", "project_id", "title", "state"],
        "gitlab.branches" => &["name"],
        _ => return None,
    };
    let properties = match datasource_ref {
        "gitlab.users" => serde_json::json!({
            "id":{"type":"integer"},"username":{"type":"string"},"name":{"type":"string"},
            "state":{"type":"string"},"bot":{"type":"boolean"},"web_url":{"type":"string"},
            "avatar_url":{"type":["string","null"]}
        }),
        "gitlab.groups" => serde_json::json!({
            "id":{"type":"integer"},"name":{"type":"string"},"path":{"type":"string"},
            "full_path":{"type":"string"},"description":{"type":["string","null"]},
            "visibility":{"type":"string"},"web_url":{"type":"string"}
        }),
        "gitlab.projects" => serde_json::json!({
            "id":{"type":"integer"},"name":{"type":"string"},"path":{"type":"string"},
            "path_with_namespace":{"type":"string"},"description":{"type":["string","null"]},
            "default_branch":{"type":["string","null"]},"visibility":{"type":"string"},
            "web_url":{"type":"string"},"last_activity_at":{"type":"string"}
        }),
        "gitlab.issues" => serde_json::json!({
            "id":{"type":"integer"},"iid":{"type":"integer"},"project_id":{"type":"integer"},
            "title":{"type":"string"},"description":{"type":["string","null"]},"state":{"type":"string"},
            "confidential":{"type":"boolean"},"labels":{"type":"array","maxItems":25,"items":{"type":"string"}},
            "author":{"type":"object"},"web_url":{"type":"string"},"created_at":{"type":"string"},
            "updated_at":{"type":"string"},"closed_at":{"type":["string","null"]}
        }),
        "gitlab.merge_requests" => serde_json::json!({
            "id":{"type":"integer"},"iid":{"type":"integer"},"project_id":{"type":"integer"},
            "title":{"type":"string"},"description":{"type":["string","null"]},"state":{"type":"string"},
            "draft":{"type":"boolean"},"source_branch":{"type":"string"},"target_branch":{"type":"string"},
            "sha":{"type":"string"},"author":{"type":"object"},"web_url":{"type":"string"},
            "created_at":{"type":"string"},"updated_at":{"type":"string"},
            "merged_at":{"type":["string","null"]},"closed_at":{"type":["string","null"]}
        }),
        "gitlab.branches" => serde_json::json!({
            "name":{"type":"string"},"merged":{"type":"boolean"},"protected":{"type":"boolean"},
            "default":{"type":"boolean"},"can_push":{"type":"boolean"},"web_url":{"type":"string"},
            "commit":{"type":"object"}
        }),
        _ => return None,
    };
    let schema = serde_json::json!({
        "type":"object",
        "required":required,
        "properties":properties,
        "additionalProperties":false
    });
    Some((
        "Bounded, allowlisted records visible to the exact delegated GitLab user Connection.",
        key_schema,
        schema.clone(),
        schema,
    ))
}

fn datasource_projection_sha256(datasource_ref: &str) -> String {
    let declaration = datasource_declaration(datasource_ref);
    let mut digest = Sha256::new();
    digest.update(b"b10x/gitlab-datasource-projection/v1\0");
    digest.update(datasource_ref.as_bytes());
    if let Some((description, key, compact, detail)) = declaration {
        digest.update(description.as_bytes());
        digest.update(serde_json::to_vec(&(key, compact, detail)).expect("schemas serialize"));
    }
    hex::encode(digest.finalize())
}

fn project_bound(datasource_ref: &str) -> bool {
    matches!(
        datasource_ref,
        "gitlab.issues" | "gitlab.merge_requests" | "gitlab.branches"
    )
}

fn datasource_binding_ref(
    datasource_ref: &str,
    connection_ref: &str,
    generation: u64,
    project_id: Option<u64>,
) -> String {
    let instance = connection_ref
        .strip_prefix("connection:gitlab:")
        .unwrap_or("invalid");
    let mut digest = Sha256::new();
    digest.update(b"b10x/gitlab-datasource-binding/v1\0");
    digest.update(datasource_ref.as_bytes());
    digest.update(b"\0");
    digest.update(connection_ref.as_bytes());
    digest.update(b"\0");
    digest.update(generation.to_be_bytes());
    digest.update(project_id.unwrap_or_default().to_be_bytes());
    format!(
        "datasource-binding:gitlab:{instance}:{generation}:{:x}",
        digest.finalize()
    )
}

fn binding_connection(binding_ref: &str) -> String {
    binding_ref
        .strip_prefix("datasource-binding:gitlab:")
        .and_then(|rest| rest.split(':').next())
        .map(|instance| format!("connection:gitlab:{instance}"))
        .unwrap_or_default()
}

fn binding_generation_hint(_binding_ref: &str) -> Option<u64> {
    _binding_ref
        .strip_prefix("datasource-binding:gitlab:")
        .and_then(|rest| rest.split(':').nth(1))
        .and_then(|value| value.parse().ok())
}

fn datasource_cursor(
    datasource_ref: &str,
    connection_ref: &str,
    project_id: Option<u64>,
    page: u64,
) -> String {
    let digest = cursor_digest(datasource_ref, connection_ref, project_id, page);
    format!("cursor:gitlab:{page}:{digest}")
}

fn parse_datasource_cursor(
    cursor: &str,
    datasource_ref: &str,
    connection_ref: &str,
    project_id: Option<u64>,
) -> Result<u64, DatasourceError> {
    let mut parts = cursor.split(':');
    if parts.next() != Some("cursor") || parts.next() != Some("gitlab") {
        return Err(datasource_cursor_expired());
    }
    let page = parts
        .next()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|page| (2..=10_000).contains(page))
        .ok_or_else(datasource_cursor_expired)?;
    let digest = parts.next().ok_or_else(datasource_cursor_expired)?;
    if parts.next().is_some()
        || !constant_time_equal(
            digest.as_bytes(),
            cursor_digest(datasource_ref, connection_ref, project_id, page).as_bytes(),
        )
    {
        return Err(datasource_cursor_expired());
    }
    Ok(page)
}

fn cursor_digest(
    datasource_ref: &str,
    connection_ref: &str,
    project_id: Option<u64>,
    page: u64,
) -> String {
    let mut digest = Sha256::new();
    digest.update(b"b10x/gitlab-datasource-cursor/v1\0");
    digest.update(datasource_ref.as_bytes());
    digest.update(b"\0");
    digest.update(connection_ref.as_bytes());
    digest.update(b"\0");
    digest.update(project_id.unwrap_or_default().to_be_bytes());
    digest.update(page.to_be_bytes());
    hex::encode(digest.finalize())
}

fn completion_page(profile: GitlabProfile, oauth_url: Option<&str>) -> HostedCompletionPage {
    let html = match profile {
        GitlabProfile::OAuthUser => {
            let link = oauth_url.map(html_escape).unwrap_or_default();
            format!(
                "<!doctype html><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width\"><title>Connect GitLab</title><style>body{{font:16px system-ui;max-width:38rem;margin:4rem auto;padding:1rem;background:#111;color:#eee}}a{{display:inline-block;padding:.8rem 1rem;background:#7759c2;color:white;border-radius:.4rem;text-decoration:none}}</style><h1>Connect GitLab</h1><p>Authorize the platform to use GitLab with your own account permissions.</p><p><a href=\"{link}\" rel=\"noreferrer\">Continue to GitLab</a></p>"
            )
        }
        GitlabProfile::PersonalToken => PAT_SETUP_PAGE.to_owned(),
    };
    HostedCompletionPage {
        title: "Connect GitLab".to_owned(),
        html,
    }
}

const PAT_SETUP_PAGE: &str = r#"<!doctype html>
<meta charset="utf-8"><meta name="viewport" content="width=device-width">
<title>Connect GitLab token</title>
<style>body{font:16px system-ui;max-width:38rem;margin:4rem auto;padding:1rem;background:#111;color:#eee}label,input,button{display:block;width:100%;box-sizing:border-box}input,button{padding:.8rem;margin-top:.5rem}button{margin-top:1rem}</style>
<h1>Connect a personal GitLab token</h1>
<p>The token must belong to your signed-in email and have <code>read_api</code> or <code>api</code>. It is sent only to Connectors and stored in Vault.</p>
<form><label>Personal access token<input name="token" type="password" autocomplete="off" maxlength="4096" required></label><button>Connect</button></form><p id="status"></p>
<script>
const form=document.querySelector('form'),status=document.querySelector('#status'),button=document.querySelector('button');
const capability=new URL(location.href).hash.match(/^#token=([A-Za-z0-9_-]{32,256})$/)?.[1];history.replaceState(null,'',location.pathname);
form.addEventListener('submit',async event=>{event.preventDefault();const field=form.elements.token,token=field.value.trim();
if(!capability||token.length<8||token.length>4096||/\s/.test(token)){status.textContent='Check the token value.';return;}
field.value='';button.disabled=true;status.textContent='Checking GitLab and saving the token in Vault…';
try{const response=await fetch(location.pathname,{method:'POST',headers:{'Content-Type':'application/octet-stream','X-Connect-Session':capability},body:token});
if(response.ok){status.textContent='GitLab connected. You may close this tab.';return;}
if(response.status===403){status.textContent='GitLab refused the token, its scopes, or its account email.';}else if(response.status===503){status.textContent='GitLab or Vault is unavailable. Start Connect again later.';}else{status.textContent='The GitLab connection was refused.';}}
catch{status.textContent='Hosted Connectors is unavailable. Start Connect again later.';}button.disabled=false;});
</script>"#;

fn parse_pat(bytes: &[u8]) -> Result<Secret, HostedCompletionError> {
    if bytes.len() < 8
        || bytes.len() > 4_096
        || !bytes.is_ascii()
        || bytes.iter().any(u8::is_ascii_whitespace)
    {
        return Err(HostedCompletionError::Invalid);
    }
    let token = std::str::from_utf8(bytes).map_err(|_| HostedCompletionError::Invalid)?;
    Ok(Secret::new(token))
}

fn verify_user(user: &GitlabUser, expected_email: &str) -> Result<(), GitlabError> {
    if user.state != "active"
        || user.bot
        || user
            .email
            .as_deref()
            .is_none_or(|email| normalize_email(email) != normalize_email(expected_email))
    {
        return Err(GitlabError::new("credential-subject"));
    }
    Ok(())
}

fn normalize_email(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn email_sha256(value: &str) -> String {
    hex::encode(Sha256::digest(normalize_email(value).as_bytes()))
}

/// The scopes this connector records as held. A token granted more is stored as this subset,
/// because the stored list is what later authorization reads.
const RETAINED_SCOPES: &[&str] = &["read_api", "api"];

const SCOPE_POLICY: ScopePolicy<'static> = ScopePolicy {
    separator: ScopeSeparator::Whitespace,
    retain: Some(RETAINED_SCOPES),
};

/// The exchange response carries the full set of facts and every one of them is required.
const EXCHANGE_POLICY: TokenPolicy<'static> = TokenPolicy {
    expect_token_type: "Bearer",
    require_refresh_token: true,
    required_scopes: &["api"],
    scopes: SCOPE_POLICY,
    require_created_at: true,
    require_expires_in: true,
    max_secret_len: 4_096,
};

/// The refresh response is judged more loosely, and deliberately: this path recomputes expiry
/// from the separate `/oauth/token/info` call rather than from the response, so `created_at` and
/// `expires_in` here are fields the connector never reads.
const REFRESH_POLICY: TokenPolicy<'static> = TokenPolicy {
    require_created_at: false,
    require_expires_in: false,
    ..EXCHANGE_POLICY
};

fn token_response(value: OAuthTokenResponse) -> TokenResponse {
    TokenResponse {
        access_token: value.access_token,
        refresh_token: Some(value.refresh_token),
        expires_in: value.expires_in,
        created_at: Some(value.created_at),
        scope: value.scope,
        token_type: value.token_type,
    }
}

fn canonical_scopes(scopes: Vec<String>) -> Vec<String> {
    connector_oauth::parse_scopes(&scopes.join(" "), &SCOPE_POLICY)
}

fn supports_operation(connection: &StoredConnection, operation_ref: &str) -> bool {
    is_gitlab_operation(operation_ref)
        && (operation_ref != "gitlab-issue-create"
            || connection.scopes.iter().any(|scope| scope == "api"))
}

fn is_gitlab_operation(value: &str) -> bool {
    GITLAB_OPERATIONS.contains(&value)
}

fn operation_effect(operation_ref: &str) -> EffectClass {
    if operation_ref == "gitlab-issue-create" {
        EffectClass::Mutating
    } else {
        EffectClass::ReadOnly
    }
}

fn operation_approval(operation_ref: &str) -> ApprovalPosture {
    if operation_ref == "gitlab-issue-create" {
        ApprovalPosture::Required
    } else {
        ApprovalPosture::NotRequired
    }
}

fn connection_summary(
    connection: StoredConnection,
    configured_initiation: InitiationConfig,
) -> ConnectionSummary {
    ConnectionSummary {
        connection_ref: connection.connection_ref,
        integration_ref: INTEGRATION_REF.to_owned(),
        label: connection.label,
        state: ConnectionState::Callable,
        initiation: initiation(configured_initiation),
        route: protocol::connection::ConnectionRoute::Direct,
        scope: Some(ConnectionScope::Principal),
        actor: Some(ConnectionActor::User),
        auth_profile: Some(connection.profile.as_str().to_owned()),
    }
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

fn upsert_connection(connections: &mut Vec<StoredConnection>, connection: StoredConnection) {
    connections.retain(|candidate| candidate.connection_ref != connection.connection_ref);
    connections.push(connection);
    connections.sort_by(|left, right| left.connection_ref.cmp(&right.connection_ref));
}

fn proposal_digest(batch: &SecretBatch) -> SecretProposalDigest {
    let mut digest = Sha256::new();
    digest.update(b"b10x/gitlab-credential-transaction/v1\0");
    for (reference, secret) in batch
        .put_entries()
        .expect("GitLab credential transactions contain puts only")
    {
        digest.update(TenantLayout.render(reference).as_bytes());
        digest.update(b"\0");
        digest.update(secret.expose_secret().as_bytes());
        digest.update(b"\0");
    }
    SecretProposalDigest::from_protocol_bytes(digest.finalize().into())
}

fn decode_transaction(encoded: &str) -> Result<SecretTransactionId, GitlabError> {
    let bytes = hex::decode(encoded).map_err(|_| GitlabError::new("transaction-state"))?;
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| GitlabError::new("transaction-state"))?;
    SecretTransactionId::from_protocol_bytes(bytes)
        .ok_or_else(|| GitlabError::new("transaction-state"))
}

pub(crate) fn parse_origin(value: &str) -> Result<url::Url, GitlabError> {
    let origin = url::Url::parse(value).map_err(|_| GitlabError::new("origin"))?;
    if origin.scheme() != "https"
        || origin.host_str().is_none()
        || origin.port_or_known_default() != Some(443)
        || !origin.username().is_empty()
        || origin.password().is_some()
        || origin.query().is_some()
        || origin.fragment().is_some()
        || !matches!(origin.path(), "" | "/")
    {
        return Err(GitlabError::new("origin"));
    }
    Ok(origin)
}

fn same_origin(expected: &url::Url, actual: &url::Url) -> bool {
    expected.scheme() == actual.scheme()
        && expected.host_str() == actual.host_str()
        && expected.port_or_known_default() == actual.port_or_known_default()
        && actual.username().is_empty()
        && actual.password().is_none()
        && actual.fragment().is_none()
}

fn random_token(bytes: usize) -> Result<String, GitlabError> {
    // Delegated rather than duplicated. This crate is where `connector-oauth` was extracted from,
    // so a second local copy of the one thing that generates unguessable values is the shape the
    // extraction existed to remove. The error text is unchanged: `OauthError::Randomness.code()`
    // is `"randomness"`.
    connector_oauth::random_token(bytes).map_err(|error| GitlabError::new(error.code()))
}

fn random_uuid() -> Result<String, GitlabError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| GitlabError::new("randomness"))?;
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

fn bounded_string(value: &str, maximum: usize) -> String {
    value.chars().take(maximum).collect()
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn valid_capability(value: &str) -> bool {
    (32..=256).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
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

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn hosted_error(error: GitlabError) -> HostedCompletionError {
    match error.code {
        "credential-subject" | "oauth-evidence" | "pat-evidence" | "provider-refused" => {
            HostedCompletionError::Refused
        }
        "provider-response" | "provider-response-bound" => HostedCompletionError::Invalid,
        _ => HostedCompletionError::Unavailable,
    }
}

fn connect_session_error(_error: service::ConnectSessionLifecycleError) -> ConnectionError {
    connection_unavailable()
}

fn connection_unavailable() -> ConnectionError {
    ConnectionError::new(
        ConnectionErrorCode::Unavailable,
        "GitLab connection setup is unavailable",
        true,
    )
}

fn connection_not_found() -> ConnectionError {
    ConnectionError::new(
        ConnectionErrorCode::NotFound,
        "GitLab Connection was not found",
        false,
    )
}

fn operation_from_context(error: ConnectionError) -> OperationError {
    OperationError::new(OperationErrorCode::StaleAuthority, error.message, false)
}

fn operation_not_found() -> OperationError {
    OperationError::new(
        OperationErrorCode::NotFound,
        "GitLab operation was not found",
        false,
    )
}

fn operation_not_granted() -> OperationError {
    OperationError::new(
        OperationErrorCode::NotGranted,
        "GitLab operation is not granted to this Connection",
        false,
    )
}

fn operation_invalid() -> OperationError {
    OperationError::new(
        OperationErrorCode::InvalidInput,
        "GitLab operation input is invalid",
        false,
    )
}

fn operation_unavailable() -> OperationError {
    OperationError::new(
        OperationErrorCode::Unavailable,
        "GitLab operation is unavailable",
        true,
    )
}

fn operation_outcome_unknown(operation_ref: &str) -> OperationError {
    OperationError::new(
        OperationErrorCode::OutcomeUnknown,
        format!(
            "GitLab operation {operation_ref} may have reached GitLab; do not retry automatically"
        ),
        false,
    )
}

fn datasource_from_context(error: ConnectionError) -> DatasourceError {
    datasource_error(DatasourceErrorCode::StaleAuthority, error.message, false)
}

fn datasource_error(
    code: DatasourceErrorCode,
    message: impl Into<String>,
    retriable: bool,
) -> DatasourceError {
    DatasourceError::new(code, message, retriable)
}

fn datasource_not_found() -> DatasourceError {
    datasource_error(
        DatasourceErrorCode::NotFound,
        "GitLab datasource was not found",
        false,
    )
}

fn datasource_not_granted() -> DatasourceError {
    datasource_error(
        DatasourceErrorCode::NotGranted,
        "GitLab datasource binding is not granted",
        false,
    )
}

fn datasource_invalid() -> DatasourceError {
    datasource_error(
        DatasourceErrorCode::InvalidInput,
        "GitLab datasource input is invalid",
        false,
    )
}

fn datasource_protocol() -> DatasourceError {
    datasource_error(
        DatasourceErrorCode::Protocol,
        "GitLab datasource returned an invalid projection",
        false,
    )
}

fn datasource_unavailable() -> DatasourceError {
    datasource_error(
        DatasourceErrorCode::Unavailable,
        "GitLab datasource is unavailable",
        true,
    )
}

fn datasource_cursor_expired() -> DatasourceError {
    datasource_error(
        DatasourceErrorCode::CursorExpired,
        "GitLab datasource cursor is invalid or belongs to another binding",
        false,
    )
}

include!("backend_tests.rs");

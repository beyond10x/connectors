//! Hosted Jira Cloud connection ownership and exact backend dispatch.

mod auth;
mod datasource;
mod operations;

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use connector_oauth::{PendingStates, DEFAULT_PENDING_CAPACITY};
use connector_secrets::{PreparedSecretStore, Secret};
use connector_state::StateStore;
use connectors_config::{HostedJiraConfig, InitiationConfig, JiraSharedAuth};
use protocol::connection::{
    ConnectionActor, ConnectionDescription, ConnectionError, ConnectionErrorCode,
    ConnectionInitiator, ConnectionRequest, ConnectionResult, ConnectionRoute, ConnectionScope,
    ConnectionState, ConnectionSummary,
};
use protocol::datasource::{DatasourceError, DatasourceRequest, DatasourceResult};
use protocol::operation::{OperationError, OperationRequest, OperationResult};
use reqwest::redirect::Policy;
use serde::{Deserialize, Serialize};
use service::{
    BackendCapabilities, BackendReadinessError, ConnectSessionAccess, ConnectSessionLifecycle,
    ConnectorBackend, HostedCompletionError, HostedCompletionPage, HostedCompletionSubmission,
    PrincipalContext,
};
use sha2::{Digest as _, Sha256};

pub(super) const INTEGRATION_REF: &str = "jira";
pub(super) const AUTHORITY: &str = "com.atlassian.jira";
pub(super) const SERVICE: &str = "default";
pub(super) const LOGIN_SERVICE: &str = "login";
pub(super) const PROFILE_ORGANIZATION: &str = "jira.organization_read";
pub(super) const PROFILE_USER: &str = "jira.oauth_user";
pub(super) const ORG_CONNECTION_REF: &str = "connection:jira:organization-read";
pub(super) const ACCESS_TOKEN_CREDENTIAL: &str = "access_token";
pub(super) const REFRESH_TOKEN_CREDENTIAL: &str = "refresh_token";
pub(super) const USER_CLIENT_SECRET_CREDENTIAL: &str = "user_oauth_client_secret";
pub(super) const SERVICE_CLIENT_SECRET_CREDENTIAL: &str = "service_oauth_client_secret";
pub(super) const SERVICE_API_TOKEN_CREDENTIAL: &str = "service_api_token";
pub(super) const STATE_KEY: &str = "jira.connections";
pub(super) const AUDIT_KEY: &str = "jira.audit";
pub(super) const STATE_VERSION: u8 = 1;
pub(super) const MAX_STATE_BYTES: usize = 4 * 1024 * 1024;
pub(super) const MAX_AUDIT_BYTES: usize = 16 * 1024 * 1024;
pub(super) const MAX_CONNECT_SESSIONS: usize = 32;
pub(super) const MAX_PROVIDER_RESPONSE_BYTES: usize = 256 * 1024;
pub(super) const VALUE_PROJECTION_PROTOCOL: &str = "b10x.value-projection.v1";
pub(super) const USER_SCOPES: [&str; 4] = [
    "offline_access",
    "read:jira-work",
    "read:me",
    "write:jira-work",
];
pub(super) const JIRA_OPERATIONS: [&str; 9] = [
    "jira-issue-get",
    "jira-issue-create",
    "jira-issue-comment-list",
    "jira-issue-comment-add",
    "jira-issue-transitions-list",
    "jira-issue-transition",
    "jira-issue-edit",
    "jira-issue-comment-edit",
    "jira-issue-link-add",
];
pub(super) const JIRA_DATASOURCE: &str = "jira.issues";

/// Redaction-safe Jira runtime failure.
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("Jira runtime refused: {code}")]
pub struct JiraError {
    code: &'static str,
}

impl JiraError {
    pub(super) const fn new(code: &'static str) -> Self {
        Self { code }
    }
}

/// Hosted Jira Integration for one Identity tenant and one exact Jira Cloud site.
pub struct JiraBackend {
    inner: Arc<JiraInner>,
}

pub(super) struct JiraInner {
    pub(super) tenant_id: String,
    pub(super) policy: HostedJiraConfig,
    pub(super) site_origin: url::Url,
    pub(super) public_origin: url::Url,
    pub(super) gateway_origin: url::Url,
    pub(super) state_store: Arc<dyn StateStore>,
    pub(super) credential_store: Arc<dyn PreparedSecretStore>,
    pub(super) metadata: Mutex<StateFile>,
    pub(super) sessions: Mutex<ConnectSessionLifecycle>,
    pub(super) session_owners: Mutex<BTreeMap<String, SessionOwner>>,
    pub(super) hosted_sessions: Mutex<BTreeMap<String, HostedSession>>,
    pub(super) oauth_states: Mutex<PendingStates<OAuthPending>>,
    pub(super) completion_lock: tokio::sync::Mutex<()>,
    pub(super) refresh_lock: tokio::sync::Mutex<()>,
    pub(super) service_token: tokio::sync::Mutex<Option<CachedServiceToken>>,
    /// Runtime evidence that the fixed organization credential completed a Jira API read. The
    /// synthetic tenant Connection starts degraded and becomes callable only after that evidence
    /// exists; configuration or Secret Store presence alone never claims live authority.
    pub(super) service_callable: AtomicBool,
    pub(super) cursor_key: [u8; 32],
    pub(super) http: reqwest::Client,
}

#[derive(Clone)]
pub(super) struct SessionOwner {
    pub(super) subject: String,
    pub(super) email: String,
}

pub(super) struct HostedSession {
    pub(super) expires_at_unix_ms: u64,
    pub(super) oauth_authorize_url: String,
}

pub(super) struct OAuthPending {
    pub(super) session_ref: String,
    pub(super) owner: SessionOwner,
}

pub(super) struct CachedServiceToken {
    pub(super) token: Secret,
    pub(super) expires_at_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StateFile {
    pub(super) version: u8,
    pub(super) next_transaction_generation: u64,
    pub(super) connections: Vec<StoredConnection>,
    pub(super) pending: Vec<PendingCommit>,
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
pub(super) struct StoredConnection {
    pub(super) connection_ref: String,
    pub(super) instance_id: String,
    pub(super) label: String,
    pub(super) grant_ref: String,
    pub(super) owner_subject: String,
    pub(super) account_id: String,
    pub(super) display_name: String,
    pub(super) email_sha256: String,
    pub(super) scopes: Vec<String>,
    pub(super) credential_generation: u64,
    pub(super) observed_at_unix_ms: u64,
    pub(super) expires_at_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PendingCommit {
    pub(super) transaction_id: String,
    pub(super) connection: StoredConnection,
}

pub(super) struct CredentialValues {
    pub(super) access_token: Secret,
    pub(super) refresh_token: Secret,
    pub(super) expires_at_unix_ms: u64,
}

impl JiraBackend {
    /// Open a hosted adapter. Configuration is value-free; every fixed or delegated secret stays
    /// behind the injected prepared Secret Store.
    pub async fn open_hosted(
        tenant_id: String,
        policy: HostedJiraConfig,
        credential_store: Arc<dyn PreparedSecretStore>,
        state_store: Arc<dyn StateStore>,
    ) -> Result<Self, JiraError> {
        let site_origin = parse_origin(&policy.site_origin)?;
        let public_origin =
            url::Url::parse(&policy.public_origin).map_err(|_| JiraError::new("public-origin"))?;
        let gateway_origin = url::Url::parse(&format!(
            "https://api.atlassian.com/ex/jira/{}/",
            policy.cloud_id
        ))
        .map_err(|_| JiraError::new("cloud-id"))?;
        let metadata = state_store
            .read(STATE_KEY, MAX_STATE_BYTES)
            .map_err(|_| JiraError::new("connection-state"))?
            .map_or_else(
                || Ok(StateFile::default()),
                |bytes| {
                    serde_json::from_slice::<StateFile>(&bytes)
                        .map_err(|_| JiraError::new("connection-state"))
                },
            )?;
        if metadata.version != STATE_VERSION
            || metadata.next_transaction_generation == 0
            || metadata.connections.len() > 1_024
            || metadata.pending.len() > 32
        {
            return Err(JiraError::new("connection-state"));
        }
        let http = reqwest::Client::builder()
            .redirect(Policy::none())
            .no_proxy()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(20))
            .user_agent("b10x-connectors/0.1")
            .build()
            .map_err(|_| JiraError::new("http-client"))?;
        let mut cursor_key = [0_u8; 32];
        getrandom::fill(&mut cursor_key).map_err(|_| JiraError::new("randomness"))?;
        let inner = Arc::new(JiraInner {
            tenant_id,
            policy,
            site_origin,
            public_origin,
            gateway_origin,
            state_store,
            credential_store,
            metadata: Mutex::new(metadata),
            sessions: Mutex::new(
                ConnectSessionLifecycle::new(INTEGRATION_REF, MAX_CONNECT_SESSIONS)
                    .map_err(|_| JiraError::new("connect-session"))?,
            ),
            session_owners: Mutex::new(BTreeMap::new()),
            hosted_sessions: Mutex::new(BTreeMap::new()),
            oauth_states: Mutex::new(PendingStates::new(DEFAULT_PENDING_CAPACITY)),
            completion_lock: tokio::sync::Mutex::new(()),
            refresh_lock: tokio::sync::Mutex::new(()),
            service_token: tokio::sync::Mutex::new(None),
            service_callable: AtomicBool::new(false),
            cursor_key,
            http,
        });
        inner.recover_pending().await?;
        Ok(Self { inner })
    }

    #[must_use]
    pub fn connection_count(&self) -> usize {
        lock(&self.inner.metadata).connections.len() + 1
    }
}

#[async_trait]
impl ConnectorBackend for JiraBackend {
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
            OperationRequest::Describe(request) => is_jira_operation(&request.operation_ref),
            OperationRequest::Invoke(request) => {
                is_jira_operation(&request.operation_ref)
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
            ConnectionRequest::Describe(request) => {
                request.connection_ref == ORG_CONNECTION_REF
                    || lock(&self.inner.metadata)
                        .connections
                        .iter()
                        .any(|connection| connection.connection_ref == request.connection_ref)
            }
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
            && request.auth_profile.as_deref() == Some(PROFILE_USER)
        {
            ConnectSessionAccess::SelfService
        } else {
            ConnectSessionAccess::Operator
        }
    }

    fn owns_datasource(&self, request: &DatasourceRequest) -> bool {
        match request {
            DatasourceRequest::Search(_) => false,
            DatasourceRequest::Describe(request) => request.datasource_ref == JIRA_DATASOURCE,
            DatasourceRequest::Bindings(request) => request.datasource_ref == JIRA_DATASOURCE,
            DatasourceRequest::Read(request) => {
                request.datasource_ref == JIRA_DATASOURCE
                    && request
                        .binding_ref
                        .starts_with("datasource-binding:jira:issues:")
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
        Ok(oauth_completion_page(&session.oauth_authorize_url))
    }

    async fn complete_hosted_session(
        &self,
        _session_ref: &str,
        _capability: &str,
        _submission: HostedCompletionSubmission,
    ) -> Result<(), HostedCompletionError> {
        Err(HostedCompletionError::Invalid)
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
            | OperationRequest::SessionTerminate(_)
            | OperationRequest::SessionReconcile(_)
            | OperationRequest::SessionSignal(_) => Err(operation_not_found()),
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
                let mut connections = vec![organization_connection_summary(
                    self.inner.policy.initiation,
                    self.inner.service_callable.load(Ordering::Acquire),
                )];
                connections.extend(self.inner.owned_user_connections(context).into_iter().map(
                    |connection| user_connection_summary(connection, self.inner.policy.initiation),
                ));
                connections.retain(|connection| {
                    query.is_empty()
                        || connection.label.to_ascii_lowercase().contains(&query)
                        || INTEGRATION_REF.contains(&query)
                });
                connections.truncate(usize::from(request.limit));
                Ok(ConnectionResult::Search { connections })
            }
            ConnectionRequest::Describe(request)
                if request.connection_ref == ORG_CONNECTION_REF =>
            {
                Ok(ConnectionResult::Describe(ConnectionDescription {
                    summary: organization_connection_summary(
                        self.inner.policy.initiation,
                        self.inner.service_callable.load(Ordering::Acquire),
                    ),
                    channels: Vec::new(),
                }))
            }
            ConnectionRequest::Describe(request) => self
                .inner
                .owned_user_connections(context)
                .into_iter()
                .find(|connection| connection.connection_ref == request.connection_ref)
                .map(|connection| {
                    ConnectionResult::Describe(ConnectionDescription {
                        summary: user_connection_summary(connection, self.inner.policy.initiation),
                        channels: Vec::new(),
                    })
                })
                .ok_or_else(connection_not_found),
            ConnectionRequest::ConnectSessionCreate(request) => {
                if request.auth_profile.as_deref() != Some(PROFILE_USER) {
                    return Err(ConnectionError::new(
                        ConnectionErrorCode::InvalidInput,
                        "Jira self-service setup requires jira.oauth_user",
                        false,
                    ));
                }
                self.inner
                    .create_session(context, request.label)
                    .map(ConnectionResult::ConnectSessionCreate)
            }
            ConnectionRequest::ConnectSessionStatus(request) => {
                if lock(&self.inner.session_owners)
                    .get(&request.connect_session_ref)
                    .is_none_or(|owner| owner.subject != context.subject())
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
        self.inner.handle_datasource_request(context, request).await
    }
}

impl JiraInner {
    pub(super) fn persist(&self, state: &StateFile) -> Result<(), JiraError> {
        let body = serde_json::to_vec(state).map_err(|_| JiraError::new("connection-state"))?;
        self.state_store
            .replace(STATE_KEY, &body, MAX_STATE_BYTES)
            .map_err(|_| JiraError::new("connection-state"))
    }

    pub(super) fn check_context(&self, context: &PrincipalContext) -> Result<(), ConnectionError> {
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

    pub(super) fn owned_user_connections(
        &self,
        context: &PrincipalContext,
    ) -> Vec<StoredConnection> {
        if context.tenant_id() != self.tenant_id {
            return Vec::new();
        }
        lock(&self.metadata)
            .connections
            .iter()
            .filter(|connection| {
                user_connection_is_admitted(
                    connection,
                    context.subject(),
                    &self.policy.user_grant_ref,
                )
            })
            .cloned()
            .collect()
    }

    pub(super) fn audit(
        &self,
        audit_ref: &str,
        operation_ref: &str,
        connection_ref: &str,
        context: &PrincipalContext,
        outcome: &str,
    ) -> Result<(), JiraError> {
        let mut line = serde_json::to_vec(&serde_json::json!({
            "at_unix_ms": now_ms().ok_or_else(|| JiraError::new("clock"))?,
            "audit_ref": audit_ref,
            "operation_ref": operation_ref,
            "connection_ref": connection_ref,
            "tenant_id": context.tenant_id(),
            "actor_subject": context.actor_subject(),
            "outcome": outcome,
        }))
        .map_err(|_| JiraError::new("audit-store"))?;
        line.push(b'\n');
        self.state_store
            .append(AUDIT_KEY, &line, MAX_AUDIT_BYTES)
            .map(|_| ())
            .map_err(|_| JiraError::new("audit-store"))
    }
}

pub(super) fn organization_connection_summary(
    init: InitiationConfig,
    callable: bool,
) -> ConnectionSummary {
    ConnectionSummary {
        connection_ref: ORG_CONNECTION_REF.to_owned(),
        integration_ref: INTEGRATION_REF.to_owned(),
        label: "Organization Jira read-only".to_owned(),
        state: if callable {
            ConnectionState::Callable
        } else {
            ConnectionState::Degraded
        },
        initiation: initiation(init),
        route: ConnectionRoute::Direct,
        scope: Some(ConnectionScope::Tenant),
        actor: Some(ConnectionActor::App),
        auth_profile: Some(PROFILE_ORGANIZATION.to_owned()),
    }
}

pub(super) fn user_connection_summary(
    connection: StoredConnection,
    init: InitiationConfig,
) -> ConnectionSummary {
    ConnectionSummary {
        connection_ref: connection.connection_ref,
        integration_ref: INTEGRATION_REF.to_owned(),
        label: connection.label,
        state: ConnectionState::Callable,
        initiation: initiation(init),
        route: ConnectionRoute::Direct,
        scope: Some(ConnectionScope::Principal),
        actor: Some(ConnectionActor::User),
        auth_profile: Some(PROFILE_USER.to_owned()),
    }
}

fn user_connection_is_admitted(
    connection: &StoredConnection,
    owner_subject: &str,
    expected_grant_ref: &str,
) -> bool {
    connection.owner_subject == owner_subject && connection.grant_ref == expected_grant_ref
}

pub(super) fn initiation(config: InitiationConfig) -> Vec<ConnectionInitiator> {
    match config {
        InitiationConfig::Platform => vec![ConnectionInitiator::Platform],
        InitiationConfig::Provider => vec![ConnectionInitiator::Provider],
        InitiationConfig::Both => {
            vec![ConnectionInitiator::Platform, ConnectionInitiator::Provider]
        }
    }
}

pub(super) fn is_jira_operation(value: &str) -> bool {
    JIRA_OPERATIONS.contains(&value)
}

pub(super) fn parse_origin(value: &str) -> Result<url::Url, JiraError> {
    let origin = url::Url::parse(value).map_err(|_| JiraError::new("site-origin"))?;
    if origin.scheme() != "https"
        || origin.host_str().is_none()
        || origin.port_or_known_default() != Some(443)
        || !origin.username().is_empty()
        || origin.password().is_some()
        || origin.query().is_some()
        || origin.fragment().is_some()
        || !matches!(origin.path(), "" | "/")
    {
        return Err(JiraError::new("site-origin"));
    }
    Ok(origin)
}

pub(super) fn random_token(bytes: usize) -> Result<String, JiraError> {
    use base64::Engine as _;
    let mut value = vec![0_u8; bytes];
    getrandom::fill(&mut value).map_err(|_| JiraError::new("randomness"))?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(value))
}

pub(super) fn random_uuid() -> Result<String, JiraError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| JiraError::new("randomness"))?;
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

pub(super) fn now_ms() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
}

pub(super) fn email_sha256(value: &str) -> String {
    hex::encode(Sha256::digest(value.trim().to_ascii_lowercase().as_bytes()))
}

pub(super) fn bounded_string(value: &str, maximum: usize) -> String {
    value.chars().take(maximum).collect()
}

pub(super) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn oauth_completion_page(authorize_url: &str) -> HostedCompletionPage {
    let link = authorize_url
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    HostedCompletionPage {
        title: "Connect Jira".to_owned(),
        html: format!(
            "<!doctype html><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width\"><title>Connect Jira</title><style>body{{font:16px system-ui;max-width:38rem;margin:4rem auto;padding:1rem;background:#111;color:#eee}}a{{display:inline-block;padding:.8rem 1rem;background:#1868db;color:white;border-radius:.4rem;text-decoration:none}}</style><h1>Connect Jira</h1><p>Authorize the platform to use Jira with your own account permissions. Organization Jira remains a separate read-only connection.</p><p><a href=\"{link}\" rel=\"noreferrer\">Continue to Atlassian</a></p>"
        ),
    }
}

pub(super) fn connection_not_found() -> ConnectionError {
    ConnectionError::new(
        ConnectionErrorCode::NotFound,
        "Jira connection was not found",
        false,
    )
}

pub(super) fn connection_unavailable() -> ConnectionError {
    ConnectionError::new(
        ConnectionErrorCode::Unavailable,
        "Jira connection setup is unavailable",
        true,
    )
}

pub(super) fn operation_from_context(error: ConnectionError) -> OperationError {
    OperationError::new(
        protocol::operation::OperationErrorCode::StaleAuthority,
        error.message,
        false,
    )
}

pub(super) fn operation_not_found() -> OperationError {
    OperationError::new(
        protocol::operation::OperationErrorCode::NotFound,
        "Jira operation was not found",
        false,
    )
}

pub(super) fn datasource_from_context(error: ConnectionError) -> DatasourceError {
    DatasourceError::new(
        protocol::datasource::DatasourceErrorCode::StaleAuthority,
        error.message,
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn organization_and_user_profiles_are_distinct() {
        let organization = organization_connection_summary(InitiationConfig::Platform, false);
        assert_eq!(organization.scope, Some(ConnectionScope::Tenant));
        assert_eq!(organization.actor, Some(ConnectionActor::App));
        assert_eq!(organization.state, ConnectionState::Degraded);
        assert_eq!(
            organization.auth_profile.as_deref(),
            Some(PROFILE_ORGANIZATION)
        );
        assert!(!is_jira_operation(PROFILE_ORGANIZATION));
        assert_eq!(JIRA_DATASOURCE, "jira.issues");
    }

    #[test]
    fn delegated_connection_is_withdrawn_when_its_grant_changes() {
        let connection = StoredConnection {
            connection_ref: "connection:jira:test".to_owned(),
            instance_id: "test".to_owned(),
            label: "My Jira".to_owned(),
            grant_ref: "grant:jira:delegated-user:v1".to_owned(),
            owner_subject: "user:one".to_owned(),
            account_id: "atlassian-account".to_owned(),
            display_name: "User One".to_owned(),
            email_sha256: "a".repeat(64),
            scopes: USER_SCOPES.iter().map(ToString::to_string).collect(),
            credential_generation: 1,
            observed_at_unix_ms: 1,
            expires_at_unix_ms: 2,
        };
        assert!(user_connection_is_admitted(
            &connection,
            "user:one",
            "grant:jira:delegated-user:v1"
        ));
        assert!(!user_connection_is_admitted(
            &connection,
            "user:one",
            "grant:jira:delegated-user:v2"
        ));
        assert!(!user_connection_is_admitted(
            &connection,
            "user:two",
            "grant:jira:delegated-user:v1"
        ));
    }
}

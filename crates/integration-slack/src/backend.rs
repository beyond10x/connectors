//! Slack Socket Mode Integration, Connection custody, and durable event delivery.

use std::collections::{BTreeMap, BTreeSet};
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
use protocol::connection::{
    ChannelState, ChannelSummary as ConnectionChannelSummary, ConnectSessionStatus,
    ConnectionDescription, ConnectionError, ConnectionErrorCode, ConnectionInitiator,
    ConnectionRequest, ConnectionResult, ConnectionState, ConnectionSummary,
};
use protocol::event::{
    ChannelSummary as EventChannelSummary, DataEvent, EventError, EventErrorCode, EventProvenance,
    EventRequest, EventResult,
};
use protocol::operation::{OperationError, OperationErrorCode, OperationRequest, OperationResult};
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
    BackendCapabilities, ConnectSessionLifecycle, ConnectSessionLifecycleError,
    ConnectSessionTerminal, ConnectorBackend, PrincipalContext,
};

const INTEGRATION_REF: &str = "slack";
const AUTHORITY: &str = "com.slack.api";
const SERVICE: &str = "default";
const APP_TOKEN_CREDENTIAL: &str = "app_token";
const SOCKET_BINDING_REF: &str = "com.slack.api:v1#socket";
const STATE_VERSION: u8 = 1;
const MAX_CONNECT_SESSIONS: usize = 16;
const MAX_APP_TOKEN_BYTES: usize = 1024;
const MAX_STATE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_EVENT_STORE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_STORED_EVENTS: usize = 10_000;
const MAX_SOCKET_MESSAGE_BYTES: usize = 1024 * 1024;
const APPS_CONNECTIONS_OPEN: &str = "https://slack.com/api/apps.connections.open";

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
    owner: PrincipalContext,
    policy: SlackIntegrationConfig,
    state_root: PathBuf,
    credential_store: Arc<dyn PreparedSecretStore>,
    metadata: Mutex<StateFile>,
    sessions: Mutex<ConnectSessionLifecycle>,
    event_store: Arc<EventStore>,
    channel_states: Mutex<BTreeMap<String, ChannelState>>,
    started_supervisors: Mutex<BTreeSet<String>>,
    shutdown: watch::Sender<bool>,
    tasks: Mutex<Vec<JoinHandle<()>>>,
    http: reqwest::Client,
    supervision_enabled: bool,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PendingCommit {
    transaction_id: String,
    connection: StoredConnection,
}

struct EventStore {
    path: PathBuf,
    events: Mutex<Vec<StoredEvent>>,
    notify: Notify,
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
        Self::open_with_supervision(owner, policy, state_root, credential_store, true).await
    }

    async fn open_with_supervision(
        owner: PrincipalContext,
        policy: SlackIntegrationConfig,
        state_root: &Path,
        credential_store: Arc<dyn PreparedSecretStore>,
        supervision_enabled: bool,
    ) -> Result<Self, SlackError> {
        let metadata = read_state(&state_root.join("connections.json"))?;
        let event_store = Arc::new(EventStore::open(state_root.join("events.jsonl"))?);
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
            owner,
            policy,
            state_root: state_root.to_path_buf(),
            credential_store,
            metadata: Mutex::new(metadata),
            sessions: Mutex::new(
                ConnectSessionLifecycle::new(INTEGRATION_REF, MAX_CONNECT_SESSIONS)
                    .map_err(|_| SlackError::new("connect-session-lifecycle"))?,
            ),
            event_store,
            channel_states: Mutex::new(BTreeMap::new()),
            started_supervisors: Mutex::new(BTreeSet::new()),
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
    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            operations: false,
            connections: true,
            events: true,
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

    async fn handle(
        &self,
        _context: &PrincipalContext,
        _request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        Err(OperationError::new(
            OperationErrorCode::NotFound,
            "Slack Integration does not provide operations",
            false,
        ))
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
                let session = self.inner.create_session(request.label).await?;
                Ok(ConnectionResult::ConnectSessionCreate(session))
            }
            ConnectionRequest::ConnectSessionStatus(request) => {
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
                self.inner.require_channel(&request.channel_ref)?;
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
                self.inner.require_channel(&event.channel_ref)?;
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
    }
}

impl SlackInner {
    fn connection_is_admitted(&self, connection: &StoredConnection) -> bool {
        connection.grant_ref == self.policy.grant_ref
            && connection.initiation == self.policy.initiation
            && connection.allowed_events.len() == self.policy.allowed_events.len()
            && connection
                .allowed_events
                .iter()
                .all(|event| self.policy.allowed_events.contains(event))
    }

    fn check_connection_context(&self, actual: &PrincipalContext) -> Result<(), ConnectionError> {
        if actual == &self.owner {
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
        if actual == &self.owner {
            Ok(())
        } else {
            Err(EventError::new(
                EventErrorCode::StaleAuthority,
                "owner authority snapshot is not current",
                false,
            ))
        }
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

    fn require_channel(&self, requested: &str) -> Result<StoredConnection, EventError> {
        lock(&self.metadata)
            .connections
            .iter()
            .find(|connection| {
                channel_ref(connection) == requested && self.connection_is_admitted(connection)
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
        label: String,
    ) -> Result<ConnectSessionStatus, ConnectionError> {
        let id = random_uuid().map_err(|_| connection_unavailable())?;
        let session_ref = format!("connect-session:{id}");
        let directory = self.state_root.join("connect-sessions");
        let endpoint =
            BoundCompletionEndpoint::bind(&directory, &id).map_err(|_| connection_unavailable())?;
        let endpoint_path = endpoint.path().to_path_buf();
        let expires_at_unix_ms = now_ms()
            .and_then(|now| {
                now.checked_add(self.policy.connect_session_ttl_seconds.saturating_mul(1000))
            })
            .ok_or_else(connection_unavailable)?;
        let status = match lock(&self.sessions).reserve(
            session_ref.clone(),
            label,
            expires_at_unix_ms,
            endpoint_path.display().to_string(),
        ) {
            Ok(status) => status,
            Err(error) => {
                drop(endpoint);
                return Err(connect_session_error(error));
            }
        };
        let inner = Arc::clone(self);
        let task_session_ref = session_ref;
        lock(&self.tasks).push(tokio::spawn(async move {
            inner.serve_completion(endpoint, task_session_ref).await;
        }));
        Ok(status)
    }

    fn session_status(&self, session_ref: &str) -> Option<ConnectSessionStatus> {
        lock(&self.sessions).status(session_ref)
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
            self.complete_connection(&session_ref, Secret::new(secret.expose_secret()))
                .await
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
        secret: Secret,
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
        };
        let credential_ref = self.credential_ref(&connection)?;
        let (transaction, generation) = self.reserve_transaction()?;
        let mut batch = SecretBatch::new(
            CredentialScope::new(self.owner.tenant_id(), AUTHORITY)
                .map_err(|_| SlackError::new("credential-address"))?,
        );
        let digest = proposal_digest(&TenantLayout.render(&credential_ref), &secret);
        batch
            .put(credential_ref, secret)
            .map_err(|_| SlackError::new("credential-batch"))?;
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
            let persisted = write_state(&self.state_root.join("connections.json"), &state).is_ok();
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
            if let Err(error) = write_state(&self.state_root.join("connections.json"), &state) {
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
        write_state(&self.state_root.join("connections.json"), &state)?;
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
                    write_state(&self.state_root.join("connections.json"), &state)?;
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
            write_state(&self.state_root.join("connections.json"), &state)?;
        }
        Ok(())
    }

    fn credential_ref(&self, connection: &StoredConnection) -> Result<CredentialRef, SlackError> {
        CredentialRef::for_instance(
            self.owner.tenant_id(),
            AUTHORITY,
            &connection.instance_id,
            SERVICE,
            APP_TOKEN_CREDENTIAL,
        )
        .map_err(|_| SlackError::new("credential-address"))
    }

    fn start_supervisor(self: &Arc<Self>, connection: StoredConnection) {
        if !lock(&self.started_supervisors).insert(connection.connection_ref.clone()) {
            return;
        }
        lock(&self.channel_states)
            .insert(connection.connection_ref.clone(), ChannelState::Starting);
        if !self.supervision_enabled {
            return;
        }
        let inner = Arc::clone(self);
        let shutdown = self.shutdown.subscribe();
        lock(&self.tasks).push(tokio::spawn(async move {
            inner.supervise(connection, shutdown).await;
        }));
    }

    async fn supervise(
        self: Arc<Self>,
        connection: StoredConnection,
        mut shutdown: watch::Receiver<bool>,
    ) {
        let mut backoff = Duration::from_secs(1);
        loop {
            if *shutdown.borrow() {
                break;
            }
            lock(&self.channel_states).insert(
                connection.connection_ref.clone(),
                ChannelState::Reconnecting,
            );
            let outcome = self.run_socket(&connection, &mut shutdown).await;
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
        lock(&self.channel_states).insert(connection.connection_ref.clone(), ChannelState::Stopped);
    }

    async fn run_socket(
        &self,
        connection: &StoredConnection,
        shutdown: &mut watch::Receiver<bool>,
    ) -> Result<(), SlackError> {
        let credential_ref = self.credential_ref(connection)?;
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
        lock(&self.channel_states)
            .insert(connection.connection_ref.clone(), ChannelState::Connected);
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
                            self.handle_socket_text(connection, text.as_ref(), &mut socket).await?;
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

    async fn handle_socket_text<S>(
        &self,
        connection: &StoredConnection,
        text: &str,
        socket: &mut S,
    ) -> Result<(), SlackError>
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
            if let Some((delivery_id, event_type, payload)) =
                project_data_event(envelope.payload.as_ref(), &connection.allowed_events)
            {
                self.event_store
                    .append(connection, &delivery_id, &event_type, payload)?;
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

impl EventStore {
    fn open(path: PathBuf) -> Result<Self, SlackError> {
        let events = read_events(&path)?;
        Ok(Self {
            path,
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

fn proposal_digest(address: &str, secret: &Secret) -> SecretProposalDigest {
    let mut digest = Sha256::new();
    digest.update(b"b10x/slack-connect/v1\0");
    digest.update(address.as_bytes());
    digest.update(b"\0");
    digest.update(secret.expose_secret().as_bytes());
    SecretProposalDigest::from_protocol_bytes(digest.finalize().into())
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

fn read_state(path: &Path) -> Result<StateFile, SlackError> {
    let Some(mut file) = open_owner_read(path, MAX_STATE_BYTES)? else {
        let state = StateFile::default();
        write_state(path, &state)?;
        return Ok(state);
    };
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|_| SlackError::new("connection-state"))?;
    let state: StateFile =
        serde_json::from_slice(&bytes).map_err(|_| SlackError::new("connection-state"))?;
    if state.version != STATE_VERSION || state.next_transaction_generation == 0 {
        return Err(SlackError::new("connection-state-version"));
    }
    Ok(state)
}

fn write_state(path: &Path, state: &StateFile) -> Result<(), SlackError> {
    let parent = path
        .parent()
        .ok_or_else(|| SlackError::new("connection-state"))?;
    ensure_owner_directory(parent)?;
    let bytes = serde_json::to_vec(state).map_err(|_| SlackError::new("connection-state"))?;
    if bytes.len() as u64 > MAX_STATE_BYTES {
        return Err(SlackError::new("connection-state-bound"));
    }
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

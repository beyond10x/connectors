//! Slack Socket Mode Integration, Connection custody, and durable event delivery.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read as _, Write as _};
use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use connector_secrets::{
    CredentialRef, CredentialScope, FileStore, PreparedSecretStore, Secret, SecretBatch,
    SecretProposalDigest, SecretStore, SecretTransactionGeneration, SecretTransactionId,
    SecretTransactionState,
};
use futures_util::{SinkExt as _, StreamExt as _};
use protocol::connection::{
    ChannelState, ChannelSummary as ConnectionChannelSummary, ConnectSessionState,
    ConnectSessionStatus, ConnectionDescription, ConnectionError, ConnectionErrorCode,
    ConnectionInitiator, ConnectionRequest, ConnectionResult, ConnectionState, ConnectionSummary,
};
use protocol::event::{
    ChannelSummary as EventChannelSummary, DataEvent, EventError, EventErrorCode, EventProvenance,
    EventRequest, EventResult,
};
use protocol::operation::{OperationError, OperationRequest, OperationResult, OwnerContext};
use reqwest::redirect::Policy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{watch, Notify};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::protocol::{Message, WebSocketConfig};
use zeroize::Zeroizing;

use crate::{InitiationConfig, SlackIntegrationConfig};
use server::local::OperationBackend;

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

/// Composition backend: operation calls stay on the existing backend while Connection and Event
/// calls terminate here.
pub struct SlackBackend {
    operation: Arc<dyn OperationBackend>,
    inner: Arc<SlackInner>,
}

struct SlackInner {
    owner: OwnerContext,
    policy: SlackIntegrationConfig,
    state_root: PathBuf,
    credential_store: Arc<FileStore>,
    metadata: Mutex<StateFile>,
    sessions: Mutex<BTreeMap<String, SessionRecord>>,
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

#[derive(Clone)]
struct SessionRecord {
    label: String,
    state: ConnectSessionState,
    expires_at_unix_ms: u64,
    endpoint: Option<PathBuf>,
    connection_ref: Option<String>,
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
        owner: OwnerContext,
        policy: SlackIntegrationConfig,
        state_root: &Path,
        operation: Arc<dyn OperationBackend>,
    ) -> Result<Self, SlackError> {
        Self::open_with_supervision(owner, policy, state_root, operation, true).await
    }

    async fn open_with_supervision(
        owner: OwnerContext,
        policy: SlackIntegrationConfig,
        state_root: &Path,
        operation: Arc<dyn OperationBackend>,
        supervision_enabled: bool,
    ) -> Result<Self, SlackError> {
        let credential_store = Arc::new(
            FileStore::open(state_root.join("credentials.store"))
                .map_err(|_| SlackError::new("credential-store"))?,
        );
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
            sessions: Mutex::new(BTreeMap::new()),
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
            inner.start_supervisor(connection);
        }
        Ok(Self { operation, inner })
    }

    #[must_use]
    pub fn connection_count(&self) -> usize {
        lock(&self.inner.metadata).connections.len()
    }
}

#[async_trait]
impl OperationBackend for SlackBackend {
    async fn handle(
        &self,
        context: &OwnerContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        self.operation.handle(context, request).await
    }

    async fn handle_connection(
        &self,
        context: &OwnerContext,
        request: ConnectionRequest,
    ) -> Result<ConnectionResult, ConnectionError> {
        self.inner.check_connection_context(context)?;
        match request {
            ConnectionRequest::CandidateSearch(request) => {
                self.operation
                    .handle_connection(context, ConnectionRequest::CandidateSearch(request))
                    .await
            }
            ConnectionRequest::CandidateActivate(request) => {
                self.operation
                    .handle_connection(context, ConnectionRequest::CandidateActivate(request))
                    .await
            }
            ConnectionRequest::Search(request) => {
                let query = request.query.to_ascii_lowercase();
                let stored = lock(&self.inner.metadata).connections.clone();
                let mut connections = stored
                    .into_iter()
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
                    .find(|connection| connection.connection_ref == request.connection_ref)
                    .cloned()
                    .ok_or_else(connection_not_found)?;
                Ok(ConnectionResult::Describe(self.inner.describe(&connection)))
            }
            ConnectionRequest::ObservationSearch(_) => Ok(ConnectionResult::ObservationSearch {
                observations: Vec::new(),
            }),
            ConnectionRequest::Materialize(request) => {
                self.operation
                    .handle_connection(context, ConnectionRequest::Materialize(request))
                    .await
            }
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
        context: &OwnerContext,
        request: EventRequest,
    ) -> Result<EventResult, EventError> {
        self.inner.check_event_context(context)?;
        match request {
            EventRequest::Search(request) => {
                let query = request.query.to_ascii_lowercase();
                let mut channels = lock(&self.inner.metadata)
                    .connections
                    .iter()
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
        for session in lock(&self.inner.sessions).values_mut() {
            if let Some(endpoint) = session.endpoint.take() {
                let _ = remove_owned_socket(&endpoint);
            }
            if session.state == ConnectSessionState::Pending {
                session.state = ConnectSessionState::Failed;
            }
        }
        let tasks = std::mem::take(&mut *lock(&self.inner.tasks));
        for task in &tasks {
            task.abort();
        }
        for task in tasks {
            let _ = task.await;
        }
        self.operation.shutdown().await;
    }
}

impl SlackInner {
    fn check_connection_context(&self, actual: &OwnerContext) -> Result<(), ConnectionError> {
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

    fn check_event_context(&self, actual: &OwnerContext) -> Result<(), EventError> {
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
            .find(|connection| channel_ref(connection) == requested)
            .cloned()
            .ok_or_else(event_not_found)
    }

    async fn create_session(
        self: &Arc<Self>,
        label: String,
    ) -> Result<ConnectSessionStatus, ConnectionError> {
        if lock(&self.sessions)
            .values()
            .filter(|session| session.state == ConnectSessionState::Pending)
            .count()
            >= MAX_CONNECT_SESSIONS
        {
            return Err(ConnectionError::new(
                ConnectionErrorCode::Conflict,
                "too many Connect Sessions are pending",
                true,
            ));
        }
        let id = random_uuid().map_err(|_| connection_unavailable())?;
        let session_ref = format!("connect-session:{id}");
        let directory = self.state_root.join("connect-sessions");
        ensure_owner_directory(&directory).map_err(|_| connection_unavailable())?;
        let endpoint = directory.join(format!("{id}.sock"));
        refuse_existing_path(&endpoint).map_err(|_| connection_unavailable())?;
        let listener = UnixListener::bind(&endpoint).map_err(|_| connection_unavailable())?;
        fs::set_permissions(&endpoint, fs::Permissions::from_mode(0o600))
            .map_err(|_| connection_unavailable())?;
        let expires_at_unix_ms = now_ms()
            .and_then(|now| {
                now.checked_add(self.policy.connect_session_ttl_seconds.saturating_mul(1000))
            })
            .ok_or_else(connection_unavailable)?;
        lock(&self.sessions).insert(
            session_ref.clone(),
            SessionRecord {
                label,
                state: ConnectSessionState::Pending,
                expires_at_unix_ms,
                endpoint: Some(endpoint.clone()),
                connection_ref: None,
            },
        );
        let status = self
            .session_status(&session_ref)
            .expect("session was inserted before projection");
        let inner = Arc::clone(self);
        let task_session_ref = session_ref;
        lock(&self.tasks).push(tokio::spawn(async move {
            inner
                .serve_completion(listener, task_session_ref, endpoint)
                .await;
        }));
        Ok(status)
    }

    fn session_status(&self, session_ref: &str) -> Option<ConnectSessionStatus> {
        let session = lock(&self.sessions).get(session_ref)?.clone();
        Some(ConnectSessionStatus {
            connect_session_ref: session_ref.to_owned(),
            integration_ref: INTEGRATION_REF.to_owned(),
            state: session.state,
            expires_at_unix_ms: session.expires_at_unix_ms,
            completion_endpoint: session
                .endpoint
                .as_ref()
                .map(|path| path.display().to_string()),
            connection_ref: session.connection_ref,
        })
    }

    async fn serve_completion(
        self: Arc<Self>,
        listener: UnixListener,
        session_ref: String,
        endpoint: PathBuf,
    ) {
        let accepted = tokio::time::timeout(
            Duration::from_secs(self.policy.connect_session_ttl_seconds),
            accept_owner(listener),
        )
        .await;
        let _ = remove_owned_socket(&endpoint);
        let (result, mut stream) = match accepted {
            Ok(Ok(stream)) => match read_submitted_secret(stream).await {
                Ok((secret, stream)) => {
                    let result = self.complete_connection(&session_ref, secret).await;
                    (result, Some(stream))
                }
                Err(error) => (Err(error), None),
            },
            Ok(Err(error)) => (Err(error), None),
            Err(_) => {
                self.finish_session(&session_ref, ConnectSessionState::Expired, None);
                return;
            }
        };
        let accepted = match result {
            Ok(connection_ref) => {
                self.finish_session(
                    &session_ref,
                    ConnectSessionState::Completed,
                    Some(connection_ref),
                );
                true
            }
            Err(_) => {
                self.finish_session(&session_ref, ConnectSessionState::Failed, None);
                false
            }
        };
        if let Some(stream) = &mut stream {
            let response = if accepted {
                b"{\"accepted\":true}\n".as_slice()
            } else {
                b"{\"accepted\":false}\n".as_slice()
            };
            let _ = stream.write_all(response).await;
            let _ = stream.shutdown().await;
        }
    }

    fn finish_session(
        &self,
        session_ref: &str,
        state: ConnectSessionState,
        connection_ref: Option<String>,
    ) {
        if let Some(session) = lock(&self.sessions).get_mut(session_ref) {
            session.state = state;
            session.endpoint = None;
            session.connection_ref = connection_ref;
        }
    }

    async fn complete_connection(
        self: &Arc<Self>,
        session_ref: &str,
        secret: Secret,
    ) -> Result<String, SlackError> {
        let label = lock(&self.sessions)
            .get(session_ref)
            .filter(|session| session.state == ConnectSessionState::Pending)
            .map(|session| session.label.clone())
            .ok_or_else(|| SlackError::new("connect-session"))?;
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
            CredentialScope::new(&self.owner.tenant_id, AUTHORITY)
                .map_err(|_| SlackError::new("credential-address"))?,
        );
        let digest = proposal_digest(&self.credential_store.address(&credential_ref), &secret);
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
            state
                .pending
                .retain(|pending| pending.transaction_id != transaction_hex);
            state.connections.push(connection.clone());
            state
                .connections
                .sort_by(|a, b| a.connection_ref.cmp(&b.connection_ref));
            write_state(&self.state_root.join("connections.json"), &state)?;
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
            &self.owner.tenant_id,
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

async fn accept_owner(listener: UnixListener) -> Result<UnixStream, SlackError> {
    loop {
        let (stream, _) = listener
            .accept()
            .await
            .map_err(|_| SlackError::new("connect-session-accept"))?;
        let credential = stream
            .peer_cred()
            .map_err(|_| SlackError::new("connect-session-peer"))?;
        if credential.uid() == rustix::process::geteuid().as_raw() {
            return Ok(stream);
        }
    }
}

async fn read_submitted_secret(mut stream: UnixStream) -> Result<(Secret, UnixStream), SlackError> {
    let mut bytes = Zeroizing::new(Vec::with_capacity(256));
    {
        let reader = BufReader::new(&mut stream);
        let mut bounded = reader.take((MAX_APP_TOKEN_BYTES + 3) as u64);
        tokio::time::timeout(
            Duration::from_secs(30),
            bounded.read_until(b'\n', &mut bytes),
        )
        .await
        .map_err(|_| SlackError::new("connect-session-timeout"))?
        .map_err(|_| SlackError::new("connect-session-read"))?;
    }
    if bytes.last() != Some(&b'\n') || bytes.len() > MAX_APP_TOKEN_BYTES + 2 {
        return Err(SlackError::new("credential-shape"));
    }
    bytes.pop();
    if bytes.last() == Some(&b'\r') {
        bytes.pop();
    }
    let value = std::str::from_utf8(&bytes).map_err(|_| SlackError::new("credential-shape"))?;
    if !value.starts_with("xapp-")
        || value.len() > MAX_APP_TOKEN_BYTES
        || value
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
    {
        return Err(SlackError::new("credential-shape"));
    }
    let bytes = std::mem::take(&mut *bytes);
    let value = String::from_utf8(bytes).expect("credential bytes were validated as UTF-8");
    Ok((Secret::new(value), stream))
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

fn refuse_existing_path(path: &Path) -> Result<(), SlackError> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        _ => Err(SlackError::new("connect-session-path")),
    }
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

fn remove_owned_socket(path: &Path) -> Result<(), SlackError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(SlackError::new("connect-session-path")),
    };
    if !std::os::unix::fs::FileTypeExt::is_socket(&metadata.file_type())
        || metadata.uid() != rustix::process::geteuid().as_raw()
    {
        return Err(SlackError::new("connect-session-path"));
    }
    fs::remove_file(path).map_err(|_| SlackError::new("connect-session-path"))
}

fn connection_unavailable() -> ConnectionError {
    ConnectionError::new(
        ConnectionErrorCode::Unavailable,
        "connection management is temporarily unavailable",
        true,
    )
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RefusingBackend;

    const SENTINEL: &str = "SENTINEL-NOT-A-REAL-SECRET";

    #[test]
    fn only_the_inner_admitted_event_is_projected() {
        let payload = serde_json::json!({
            "token": SENTINEL,
            "event_id": "Ev01",
            "event": {
                "type": "message",
                "channel_type": "channel",
                "channel": "C01",
                "user": "U01",
                "text": "hello",
                "ts": "1.0"
            }
        });
        let (_, kind, projected) =
            project_data_event(Some(&payload), &["message.channels".to_owned()]).unwrap();
        assert_eq!(kind, "message.channels");
        let encoded = serde_json::to_string(&projected).unwrap();
        assert!(!encoded.contains(SENTINEL));
        assert!(projected.get("event").is_none());
        assert_eq!(projected["text"], "hello");
    }

    #[test]
    fn message_loop_guards_and_closed_event_grants_are_applied_before_storage() {
        let bot = serde_json::json!({
            "event_id": "Ev02",
            "event": {"type": "message", "channel_type": "channel", "bot_id": "B01", "text": "own"}
        });
        assert!(project_data_event(Some(&bot), &["message.channels".to_owned()]).is_none());
        let unknown = serde_json::json!({
            "event_id": "Ev03",
            "event": {"type": "reaction_added"}
        });
        assert!(project_data_event(Some(&unknown), &["message.channels".to_owned()]).is_none());
    }

    #[test]
    fn socket_ticket_destination_is_closed_to_slack_tls_hosts() {
        assert!(validate_socket_url("wss://wss-primary.slack.com/link/?ticket=sentinel").is_ok());
        assert!(
            validate_socket_url("wss://slack.com.example.invalid/link/?ticket=sentinel").is_err()
        );
        assert!(validate_socket_url("ws://wss-primary.slack.com/link/?ticket=sentinel").is_err());
    }

    fn owner() -> OwnerContext {
        OwnerContext {
            tenant_id: "tenant-local".to_owned(),
            agent_id: "agent-dev".to_owned(),
            agent_revision: 1,
            authority_snapshot_id: "authority-1".to_owned(),
            authority_snapshot_sha256: "a".repeat(64),
        }
    }

    fn policy() -> SlackIntegrationConfig {
        SlackIntegrationConfig {
            grant_ref: "grant:slack-inbound".to_owned(),
            initiation: InitiationConfig::Provider,
            allowed_events: vec!["app_mention".to_owned(), "message.channels".to_owned()],
            connect_session_ttl_seconds: 30,
        }
    }

    #[tokio::test]
    async fn one_use_completion_publishes_only_value_free_connection_state() {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let backend = SlackBackend::open_with_supervision(
            owner(),
            policy(),
            root.path(),
            Arc::new(RefusingBackend),
            false,
        )
        .await
        .unwrap();
        let created = backend
            .handle_connection(
                &owner(),
                ConnectionRequest::ConnectSessionCreate(
                    protocol::connection::ConnectSessionCreateRequest {
                        integration_ref: INTEGRATION_REF.to_owned(),
                        label: "Development Slack".to_owned(),
                    },
                ),
            )
            .await
            .unwrap();
        let ConnectionResult::ConnectSessionCreate(created) = created else {
            panic!("wrong result");
        };
        let endpoint = PathBuf::from(created.completion_endpoint.clone().unwrap());
        let submitted = format!("xapp-{SENTINEL}");
        let mut stream = UnixStream::connect(&endpoint).await.unwrap();
        stream.write_all(submitted.as_bytes()).await.unwrap();
        stream.write_all(b"\n").await.unwrap();
        let mut response = String::new();
        BufReader::new(&mut stream)
            .read_line(&mut response)
            .await
            .unwrap();
        assert_eq!(response, "{\"accepted\":true}\n");
        assert!(!endpoint.exists());
        assert!(UnixStream::connect(&endpoint).await.is_err());

        let status = backend
            .handle_connection(
                &owner(),
                ConnectionRequest::ConnectSessionStatus(
                    protocol::connection::ConnectSessionStatusRequest {
                        connect_session_ref: created.connect_session_ref,
                    },
                ),
            )
            .await
            .unwrap();
        let ConnectionResult::ConnectSessionStatus(status) = status else {
            panic!("wrong result");
        };
        assert_eq!(status.state, ConnectSessionState::Completed);
        assert!(status.completion_endpoint.is_none());
        let connection_ref = status.connection_ref.unwrap();
        let description = backend
            .handle_connection(
                &owner(),
                ConnectionRequest::Describe(protocol::connection::DescribeRequest {
                    connection_ref,
                }),
            )
            .await
            .unwrap();
        let ConnectionResult::Describe(description) = description else {
            panic!("wrong result");
        };
        assert_eq!(description.summary.state, ConnectionState::Authorized);
        assert_eq!(description.channels[0].state, ChannelState::Starting);

        let metadata = fs::read_to_string(root.path().join("connections.json")).unwrap();
        assert!(!metadata.contains(SENTINEL));
        assert!(!metadata.contains("completion_endpoint"));
        let connection = lock(&backend.inner.metadata).connections[0].clone();
        let credential = backend
            .inner
            .credential_store
            .get(&backend.inner.credential_ref(&connection).unwrap())
            .await
            .unwrap();
        assert_eq!(credential.expose_secret(), submitted);
        backend.shutdown().await;
    }

    #[tokio::test]
    async fn event_is_durable_and_deduplicated_before_pull_and_replay() {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let store = EventStore::open(root.path().join("events.jsonl")).unwrap();
        let connection = StoredConnection {
            connection_ref: "connection:slack:00000000-0000-4000-8000-000000000001".to_owned(),
            instance_id: "00000000-0000-4000-8000-000000000001".to_owned(),
            label: "Development Slack".to_owned(),
            grant_ref: "grant:slack-inbound".to_owned(),
            initiation: InitiationConfig::Provider,
            allowed_events: vec!["message.channels".to_owned()],
        };
        let payload = serde_json::json!({"type":"message","channel":"C01","text":"hello"});
        store
            .append(&connection, "Ev01", "message.channels", payload.clone())
            .unwrap();
        store
            .append(&connection, "Ev01", "message.channels", payload)
            .unwrap();
        let (events, cursor) = store
            .receive(&channel_ref(&connection), 0, 10, Duration::ZERO)
            .await;
        assert_eq!(events.len(), 1);
        assert_eq!(cursor, 1);
        assert_eq!(events[0].provenance, EventProvenance::Native);
        assert_eq!(store.replay(&events[0].event_ref), Some(events[0].clone()));

        let reopened = EventStore::open(root.path().join("events.jsonl")).unwrap();
        let (events, cursor) = reopened
            .receive(&channel_ref(&connection), 0, 10, Duration::ZERO)
            .await;
        assert_eq!(events.len(), 1);
        assert_eq!(cursor, 1);
    }
}

//! Grafana discovery plus mediated Prometheus, Loki, and Alertmanager execution.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use connect_session_transport::{
    remove_endpoint, BoundCompletionEndpoint, CompletionTransportError,
};
use connector_resolve::auth::{acquire, Assembled};
use connector_resolve::document::ProtocolDriver;
use connector_resolve::{resolve, Request};
use connector_secrets::{CredentialRef, Secret, SecretStore};
use domain::{
    AdmittedOperation, Capability, ConnectionAuthority, DriverId, InitiationPolicy, ProtocolPlan,
    RouteAdapter as DomainRouteAdapter,
};
use protocol::connection::{
    ConnectSessionStatus, ConnectionDescription, ConnectionError, ConnectionErrorCode,
    ConnectionInitiator, ConnectionRequest, ConnectionResult, ConnectionRoute, ConnectionState,
    ConnectionSummary as ControlConnectionSummary, DiscoveryObservationState,
    DiscoveryObservationSummary, RouteAdapter,
};
use protocol::operation::{
    ApprovalPosture, ConnectionSummary, DescribeRequest, InvocationResult, InvokeRequest,
    OperationDescription, OperationError, OperationErrorCode, OperationRequest, OperationResult,
    OperationSummary,
};
use reqwest::redirect::Policy;
use serde_json::Value;
use service::{
    plan_operation, BackendCapabilities, BackendReadinessError, ConnectSessionLifecycle,
    ConnectSessionTerminal, ConnectorBackend, PlanningEnvironment, PrincipalContext,
};
use sha2::{Digest as _, Sha256};
use tokio::task::JoinHandle;

use connectors_config::{
    GrafanaIntegrationConfig, HostedGrafanaConfig, HostedGrafanaTargetConfig, InitiationConfig,
};
use monitoring_model::{
    audiences_for_operation, document_text, effect, operation_document, operation_ids,
    provider_for_operation, response_schema, supported_operation, target_provider, title,
    validate_input, DISCOVERY_REF, GRAFANA, GRAFANA_DATASOURCES_LIST,
};

use crate::errors::*;

const GRAFANA_CREDENTIAL: &str = "grafana.service_account_token";
const GRAFANA_AUTHORITY: &str = "com.grafana.api";
const GRAFANA_CREDENTIAL_LEAF: &str = "service_account_token";
const AUDIT_STATE_KEY: &str = "monitoring.audit";
const DEFAULT_SERVICE: &str = "default";
const TARGET_BASE: &str = "https://mediated-target.invalid";
const MAX_CONNECT_SESSIONS: usize = 16;
const MAX_TOKEN_BYTES: usize = 8 * 1024;
const MAX_AUDIT_BYTES: u64 = 16 * 1024 * 1024;

/// Redaction-safe monitoring runtime failure.
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("monitoring runtime refused: {code}")]
pub struct MonitoringError {
    code: &'static str,
}

impl MonitoringError {
    const fn new(code: &'static str) -> Self {
        Self { code }
    }
}

/// HTTP seam used by the production reqwest transport and deterministic conformance tests.
#[async_trait]
trait HttpExecutor: Send + Sync + 'static {
    async fn execute(&self, request: Request) -> Result<Value, MonitoringError>;
}

struct ReqwestExecutor {
    client: reqwest::Client,
}

#[async_trait]
impl HttpExecutor for ReqwestExecutor {
    async fn execute(&self, request: Request) -> Result<Value, MonitoringError> {
        let method = reqwest::Method::from_bytes(request.method.as_bytes())
            .map_err(|_| MonitoringError::new("http-method"))?;
        let parsed =
            url::Url::parse(&request.url).map_err(|_| MonitoringError::new("http-destination"))?;
        if parsed.scheme() != "https"
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.fragment().is_some()
        {
            return Err(MonitoringError::new("http-destination"));
        }
        let mut outbound = self.client.request(method, parsed);
        for (name, value) in request.headers {
            let name = reqwest::header::HeaderName::from_bytes(name.as_bytes())
                .map_err(|_| MonitoringError::new("http-header"))?;
            let value = reqwest::header::HeaderValue::from_str(&value)
                .map_err(|_| MonitoringError::new("http-header"))?;
            outbound = outbound.header(name, value);
        }
        if let Some(body) = request.body {
            outbound = outbound.body(body);
        }
        let mut response = outbound
            .send()
            .await
            .map_err(|_| MonitoringError::new("http-request"))?;
        if !response.status().is_success()
            || response
                .content_length()
                .is_some_and(|size| size > protocol::operation::MAX_RESULT_BYTES as u64)
        {
            return Err(MonitoringError::new("http-response"));
        }
        let mut bytes = Vec::with_capacity(
            response
                .content_length()
                .unwrap_or(0)
                .min(protocol::operation::MAX_RESULT_BYTES as u64) as usize,
        );
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| MonitoringError::new("http-response"))?
        {
            if bytes
                .len()
                .checked_add(chunk.len())
                .is_none_or(|size| size > protocol::operation::MAX_RESULT_BYTES)
            {
                return Err(MonitoringError::new("http-response-bound"));
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes).map_err(|_| MonitoringError::new("http-json"))
    }
}

/// Standalone monitoring Integration adapter.
pub struct MonitoringBackend {
    inner: Arc<MonitoringInner>,
}

struct MonitoringInner {
    owner: PrincipalContext,
    access: MonitoringAccess,
    policy: GrafanaIntegrationConfig,
    hosted_targets: Option<Vec<HostedGrafanaTargetConfig>>,
    state_root: PathBuf,
    state: Mutex<MonitoringState>,
    sessions: Mutex<ConnectSessionLifecycle>,
    completion: tokio::sync::Mutex<()>,
    credential_store: Arc<dyn SecretStore>,
    credential_ref: CredentialRef,
    tasks: Mutex<Vec<JoinHandle<()>>>,
    executor: Arc<dyn HttpExecutor>,
    audit: Mutex<()>,
    hosted_state: Option<hosted_state::PostgresState>,
}

#[derive(Clone)]
enum MonitoringAccess {
    ExactOwner,
    HostedGroups {
        tenant_id: String,
        read_groups: BTreeSet<String>,
        operator_groups: BTreeSet<String>,
    },
}

#[derive(Clone, Default)]
struct MonitoringState {
    parent: Option<ParentConnection>,
    observations: BTreeMap<String, StoredObservation>,
    children: BTreeMap<String, ChildConnection>,
    evidence_generation: u64,
}

#[derive(Clone)]
struct ParentConnection {
    connection_ref: String,
    label: String,
}

#[derive(Clone)]
struct StoredObservation {
    observation_ref: String,
    source_connection_ref: String,
    observed_type: String,
    title: String,
    resource_binding: String,
    target_provider: Option<String>,
    active: bool,
    evidence_generation: u64,
    evidence_sha256: String,
    connection_ref: Option<String>,
}

#[derive(Clone)]
struct ChildConnection {
    connection_ref: String,
    label: String,
    provider: String,
    grant_ref: String,
    parent_connection_ref: String,
    observation_ref: String,
    resource_binding: String,
}

#[derive(serde::Serialize)]
struct AuditEvent<'a> {
    audit_ref: &'a str,
    operation_ref: &'a str,
    connection_ref: &'a str,
    parent_connection_ref: Option<&'a str>,
    route_adapter: Option<&'a str>,
    tenant_id: &'a str,
    agent_id: &'a str,
    outcome: &'a str,
}

impl MonitoringBackend {
    /// Construct a Grafana monitoring adapter over caller-owned credential custody.
    pub fn open(
        owner: PrincipalContext,
        policy: GrafanaIntegrationConfig,
        state_root: &Path,
        credential_store: Arc<dyn SecretStore>,
    ) -> Result<Self, MonitoringError> {
        ensure_owner_directory(state_root)?;
        let credential_ref = grafana_credential_ref(&owner)?;
        let client = reqwest::Client::builder()
            .redirect(Policy::none())
            .no_proxy()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .user_agent("b10x-connectors/0.1")
            .build()
            .map_err(|_| MonitoringError::new("http-client"))?;
        Ok(Self::with_executor(
            owner,
            policy,
            state_root,
            credential_store,
            credential_ref,
            Arc::new(ReqwestExecutor { client }),
        ))
    }

    /// Construct a deployment-managed Grafana federation. The token must already be present in the
    /// caller-owned SecretStore; there is deliberately no hosted Connect Session for this route.
    pub async fn open_hosted(
        tenant_id: String,
        policy: HostedGrafanaConfig,
        operator_groups: Vec<String>,
        state_root: &Path,
        credential_store: Arc<dyn SecretStore>,
        hosted_state: hosted_state::PostgresState,
    ) -> Result<Self, MonitoringError> {
        ensure_owner_directory(state_root)?;
        let owner = PrincipalContext::hosted(
            tenant_id.clone(),
            "service:connectors-monitoring".to_owned(),
            "service:connectors-monitoring".to_owned(),
            None,
            "deployment:monitoring".to_owned(),
            "0".repeat(64),
        )
        .map_err(|_| MonitoringError::new("hosted-principal"))?;
        let client = reqwest::Client::builder()
            .redirect(Policy::none())
            .no_proxy()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .user_agent("b10x-connectors/0.1")
            .build()
            .map_err(|_| MonitoringError::new("http-client"))?;
        Self::open_hosted_with_executor(
            owner,
            policy,
            operator_groups,
            state_root,
            credential_store,
            Arc::new(ReqwestExecutor { client }),
            Some(hosted_state),
        )
        .await
    }

    async fn open_hosted_with_executor<E: HttpExecutor>(
        owner: PrincipalContext,
        hosted: HostedGrafanaConfig,
        operator_groups: Vec<String>,
        state_root: &Path,
        credential_store: Arc<dyn SecretStore>,
        executor: Arc<E>,
        hosted_state: Option<hosted_state::PostgresState>,
    ) -> Result<Self, MonitoringError> {
        let credential_ref = grafana_credential_ref(&owner)?;
        let origin = hosted
            .origin
            .clone()
            .ok_or_else(|| MonitoringError::new("hosted-policy"))?;
        let parent = ParentConnection {
            connection_ref: hosted
                .connection_ref
                .clone()
                .ok_or_else(|| MonitoringError::new("hosted-policy"))?,
            label: hosted
                .label
                .clone()
                .ok_or_else(|| MonitoringError::new("hosted-policy"))?,
        };
        let grant_ref = hosted
            .grant_ref
            .clone()
            .ok_or_else(|| MonitoringError::new("hosted-policy"))?;
        let read_groups = hosted.read_groups.iter().cloned().collect();
        let targets = hosted.targets.clone();
        let interval = hosted.reconcile_interval_seconds;
        let policy = GrafanaIntegrationConfig {
            origin,
            grant_ref,
            initiation: InitiationConfig::B10x,
            target_grants: BTreeMap::new(),
            connect_session_ttl_seconds: 300,
        };
        let backend = Self {
            inner: Arc::new(MonitoringInner {
                owner: owner.clone(),
                access: MonitoringAccess::HostedGroups {
                    tenant_id: owner.tenant_id().to_owned(),
                    read_groups,
                    operator_groups: operator_groups.into_iter().collect(),
                },
                policy,
                hosted_targets: Some(targets),
                state_root: state_root.to_path_buf(),
                state: Mutex::new(MonitoringState {
                    parent: Some(parent),
                    ..MonitoringState::default()
                }),
                sessions: Mutex::new(
                    ConnectSessionLifecycle::new(GRAFANA, MAX_CONNECT_SESSIONS)
                        .expect("static Connect Session policy is valid"),
                ),
                completion: tokio::sync::Mutex::new(()),
                credential_store,
                credential_ref,
                tasks: Mutex::new(Vec::new()),
                executor,
                audit: Mutex::new(()),
                hosted_state,
            }),
        };
        backend
            .inner
            .reconcile_hosted_targets()
            .await
            .map_err(|_| MonitoringError::new("hosted-discovery"))?;
        let inner = Arc::clone(&backend.inner);
        lock(&backend.inner.tasks).push(tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(interval));
            ticker.tick().await;
            loop {
                ticker.tick().await;
                if inner.reconcile_hosted_targets().await.is_err() {
                    inner.degrade_hosted_targets();
                }
            }
        }));
        Ok(backend)
    }

    fn with_executor<E: HttpExecutor>(
        owner: PrincipalContext,
        policy: GrafanaIntegrationConfig,
        state_root: &Path,
        credential_store: Arc<dyn SecretStore>,
        credential_ref: CredentialRef,
        executor: Arc<E>,
    ) -> Self {
        Self {
            inner: Arc::new(MonitoringInner {
                owner,
                access: MonitoringAccess::ExactOwner,
                policy,
                hosted_targets: None,
                state_root: state_root.to_path_buf(),
                state: Mutex::new(MonitoringState::default()),
                sessions: Mutex::new(
                    ConnectSessionLifecycle::new(GRAFANA, MAX_CONNECT_SESSIONS)
                        .expect("static Connect Session policy is valid"),
                ),
                completion: tokio::sync::Mutex::new(()),
                credential_store,
                credential_ref,
                tasks: Mutex::new(Vec::new()),
                executor,
                audit: Mutex::new(()),
                hosted_state: None,
            }),
        }
    }

    /// Number of Grafana and mediated target Connections in this daemon generation.
    #[must_use]
    pub fn connection_count(&self) -> usize {
        let state = lock(&self.inner.state);
        usize::from(state.parent.is_some()) + state.children.len()
    }
}

#[async_trait]
impl ConnectorBackend for MonitoringBackend {
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
            datasources: false,
        }
    }

    fn owns_operation(&self, request: &OperationRequest) -> bool {
        match request {
            OperationRequest::Describe(request) => supported_operation(&request.operation_ref),
            OperationRequest::Invoke(request) => {
                supported_operation(&request.operation_ref)
                    && self.inner.owns_connection_ref(&request.connection_ref)
            }
            OperationRequest::Search(_) => false,
            _ => false,
        }
    }

    fn owns_connection(&self, request: &ConnectionRequest) -> bool {
        match request {
            ConnectionRequest::Describe(request) => {
                self.inner.owns_connection_ref(&request.connection_ref)
            }
            ConnectionRequest::ObservationSearch(request) => {
                self.inner.is_parent(&request.source_connection_ref)
            }
            ConnectionRequest::Materialize(request) => {
                self.inner.has_observation(&request.observation_ref)
            }
            ConnectionRequest::ConnectSessionCreate(request) => {
                self.inner.hosted_targets.is_none() && request.integration_ref == GRAFANA
            }
            ConnectionRequest::ConnectSessionStatus(request) => {
                self.inner.has_session(&request.connect_session_ref)
            }
            ConnectionRequest::Search(_) => false,
            _ => false,
        }
    }

    async fn handle(
        &self,
        context: &PrincipalContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        self.inner.check_operation_context(context)?;
        match request {
            OperationRequest::Search(request) => {
                if !self.inner.admits(context) {
                    return Ok(OperationResult::Search {
                        operations: Vec::new(),
                    });
                }
                let mut operations = self.inner.search_operations(&request.query);
                operations.sort_by(|left, right| left.operation_ref.cmp(&right.operation_ref));
                operations.truncate(usize::from(request.limit));
                Ok(OperationResult::Search { operations })
            }
            OperationRequest::Describe(request) if supported_operation(&request.operation_ref) => {
                self.inner.require_operation_access(context)?;
                self.inner.describe(context, request)
            }
            OperationRequest::Invoke(request) if supported_operation(&request.operation_ref) => {
                self.inner.require_operation_access(context)?;
                self.inner.invoke(context, request).await
            }
            _ => Err(operation_not_found()),
        }
    }

    async fn handle_connection(
        &self,
        context: &PrincipalContext,
        request: ConnectionRequest,
    ) -> Result<ConnectionResult, ConnectionError> {
        self.inner.check_connection_context(context)?;
        match request {
            ConnectionRequest::Search(request) => {
                if !self.inner.admits(context) {
                    return Ok(ConnectionResult::Search {
                        connections: Vec::new(),
                    });
                }
                let mut connections = self.inner.search_connections(&request.query);
                connections.sort_by(|left, right| left.connection_ref.cmp(&right.connection_ref));
                connections.truncate(usize::from(request.limit));
                Ok(ConnectionResult::Search { connections })
            }
            ConnectionRequest::Describe(request) => {
                self.inner.require_connection_access(context)?;
                self.inner
                    .describe_connection(&request.connection_ref)
                    .map(ConnectionResult::Describe)
                    .ok_or_else(connection_not_found)
            }
            ConnectionRequest::ObservationSearch(request)
                if self.inner.is_parent(&request.source_connection_ref) =>
            {
                self.inner.require_connection_access(context)?;
                Ok(ConnectionResult::ObservationSearch {
                    observations: self
                        .inner
                        .search_observations(&request.query, request.limit),
                })
            }
            ConnectionRequest::Materialize(request)
                if self.inner.has_observation(&request.observation_ref) =>
            {
                self.inner.require_connection_access(context)?;
                self.inner
                    .materialize(&request.observation_ref)
                    .map(ConnectionResult::Materialize)
            }
            ConnectionRequest::ConnectSessionCreate(request)
                if request.integration_ref == GRAFANA && self.inner.hosted_targets.is_none() =>
            {
                self.inner.require_connection_access(context)?;
                self.inner
                    .create_session(request.label)
                    .await
                    .map(ConnectionResult::ConnectSessionCreate)
            }
            ConnectionRequest::ConnectSessionStatus(request)
                if self.inner.has_session(&request.connect_session_ref) =>
            {
                self.inner.require_connection_access(context)?;
                self.inner
                    .session_status(&request.connect_session_ref)
                    .map(ConnectionResult::ConnectSessionStatus)
                    .ok_or_else(connection_not_found)
            }
            _ => Err(connection_not_found()),
        }
    }

    async fn shutdown(&self) {
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

impl MonitoringInner {
    fn check_operation_context(&self, actual: &PrincipalContext) -> Result<(), OperationError> {
        if self.same_authority_partition(actual) {
            Ok(())
        } else {
            Err(OperationError::new(
                OperationErrorCode::StaleAuthority,
                "owner authority snapshot is not current",
                false,
            ))
        }
    }

    fn check_connection_context(&self, actual: &PrincipalContext) -> Result<(), ConnectionError> {
        if self.same_authority_partition(actual) {
            Ok(())
        } else {
            Err(ConnectionError::new(
                ConnectionErrorCode::StaleAuthority,
                "owner authority snapshot is not current",
                false,
            ))
        }
    }

    fn same_authority_partition(&self, actual: &PrincipalContext) -> bool {
        match &self.access {
            MonitoringAccess::ExactOwner => actual == &self.owner,
            MonitoringAccess::HostedGroups { tenant_id, .. } => actual.tenant_id() == tenant_id,
        }
    }

    fn require_operation_access(&self, actual: &PrincipalContext) -> Result<(), OperationError> {
        self.admits(actual).then_some(()).ok_or_else(|| {
            OperationError::new(
                OperationErrorCode::NotGranted,
                "monitoring Connection is not granted to this principal",
                false,
            )
        })
    }

    fn require_connection_access(&self, actual: &PrincipalContext) -> Result<(), ConnectionError> {
        self.admits(actual).then_some(()).ok_or_else(|| {
            ConnectionError::new(
                ConnectionErrorCode::NotGranted,
                "monitoring Connection is not granted to this principal",
                false,
            )
        })
    }

    fn admits(&self, actual: &PrincipalContext) -> bool {
        match &self.access {
            MonitoringAccess::ExactOwner => actual == &self.owner,
            MonitoringAccess::HostedGroups {
                tenant_id,
                read_groups,
                operator_groups,
            } => {
                actual.tenant_id() == tenant_id
                    && (!read_groups.is_disjoint(actual.verified_groups())
                        || !operator_groups.is_disjoint(actual.verified_groups()))
            }
        }
    }

    fn search_operations(&self, query: &str) -> Vec<OperationSummary> {
        let needle = query.to_ascii_lowercase();
        operation_ids()
            .into_iter()
            .filter_map(|operation_ref| {
                let connections = self.connections_for_operation(operation_ref);
                if connections.is_empty() {
                    return None;
                }
                let operation = operation_document(operation_ref)?;
                let title = title(operation_ref);
                let haystack = format!(
                    "{operation_ref} {title} {}",
                    operation.contract_description()
                )
                .to_ascii_lowercase();
                (needle.is_empty() || haystack.contains(&needle)).then_some(OperationSummary {
                    operation_ref: operation_ref.to_owned(),
                    title: title.to_owned(),
                    effect: effect(operation.effects()),
                    approval: ApprovalPosture::NotRequired,
                    connections,
                })
            })
            .collect()
    }

    fn describe(
        &self,
        context: &PrincipalContext,
        request: DescribeRequest,
    ) -> Result<OperationResult, OperationError> {
        let operation =
            operation_document(&request.operation_ref).ok_or_else(operation_not_found)?;
        let connections = self.connections_for_operation(&request.operation_ref);
        if connections.is_empty() {
            return Err(operation_not_found());
        }
        Ok(OperationResult::Describe(OperationDescription {
            operation_ref: request.operation_ref.clone(),
            title: title(&request.operation_ref).to_owned(),
            description: operation.contract_description().to_owned(),
            input_schema: operation.input_schema().clone(),
            output_schema: response_schema(
                provider_for_operation(&request.operation_ref),
                &request.operation_ref,
            )?,
            effect: effect(operation.effects()),
            approval: ApprovalPosture::NotRequired,
            connections,
            description_ref: self.description_ref(context, &request.operation_ref),
        }))
    }

    async fn invoke(
        &self,
        context: &PrincipalContext,
        request: InvokeRequest,
    ) -> Result<OperationResult, OperationError> {
        if request.description_ref != self.description_ref(context, &request.operation_ref) {
            return Err(OperationError::new(
                OperationErrorCode::StaleAuthority,
                "operation description lease is stale",
                false,
            ));
        }
        if request.approval_evidence_ref.is_some() {
            return Err(operation_invalid());
        }
        validate_input(&request.operation_ref, &request.input)?;
        let provider = provider_for_operation(&request.operation_ref);
        let operation =
            operation_document(&request.operation_ref).ok_or_else(operation_not_found)?;
        if operation.protocol_driver() != ProtocolDriver::HttpV1 {
            return Err(operation_unavailable());
        }
        let audit_ref = format!(
            "audit:monitoring:{}",
            random_uuid().map_err(|_| operation_unavailable())?
        );
        self.append_audit(AuditEvent {
            audit_ref: &audit_ref,
            operation_ref: &request.operation_ref,
            connection_ref: &request.connection_ref,
            parent_connection_ref: None,
            route_adapter: None,
            tenant_id: context.tenant_id(),
            agent_id: context.actor_subject(),
            outcome: "attempted",
        })
        .map_err(|_| operation_unavailable())?;

        let (result, parent_ref, adapter) = if provider == GRAFANA {
            let parent = lock(&self.state)
                .parent
                .clone()
                .filter(|parent| parent.connection_ref == request.connection_ref)
                .ok_or_else(operation_not_granted)?;
            let token = self.load_credential().await?;
            let value = self
                .execute_direct(context, &parent, &token, operation, request.input)
                .await?;
            if request.operation_ref == GRAFANA_DATASOURCES_LIST {
                if self.hosted_targets.is_some() {
                    self.reconcile_hosted_output(&parent.connection_ref, &value)?;
                } else {
                    self.reconcile_observations(&parent.connection_ref, &value)?;
                }
            }
            (project_output(&request.operation_ref, &value)?, None, None)
        } else {
            let child = lock(&self.state)
                .children
                .get(&request.connection_ref)
                .filter(|child| child.provider == provider)
                .cloned()
                .ok_or_else(operation_not_granted)?;
            if !child_is_current(&lock(&self.state), &child) {
                return Err(operation_not_granted());
            }
            let token = self.load_credential().await?;
            let value = self
                .execute_mediated(context, &child, &token, operation, request.input)
                .await?;
            (
                project_output(&request.operation_ref, &value)?,
                Some(child.parent_connection_ref.clone()),
                Some(DomainRouteAdapter::GrafanaDatasourceProxyV1.as_str()),
            )
        };
        self.append_audit(AuditEvent {
            audit_ref: &audit_ref,
            operation_ref: &request.operation_ref,
            connection_ref: &request.connection_ref,
            parent_connection_ref: parent_ref.as_deref(),
            route_adapter: adapter,
            tenant_id: context.tenant_id(),
            agent_id: context.actor_subject(),
            outcome: "completed",
        })
        .map_err(|_| operation_unavailable())?;
        Ok(OperationResult::Invoke(InvocationResult {
            operation_ref: request.operation_ref,
            output: result,
            connector_audit_ref: audit_ref,
            execution_ref: None,
        }))
    }

    async fn load_credential(&self) -> Result<Secret, OperationError> {
        match self.credential_store.get(&self.credential_ref).await {
            Ok(secret) => Ok(secret),
            Err(error) if error.is_not_found() => Err(operation_not_granted()),
            Err(_) => Err(operation_unavailable()),
        }
    }

    async fn execute_direct(
        &self,
        context: &PrincipalContext,
        parent: &ParentConnection,
        token: &Secret,
        operation: &'static connector_resolve::document::Operation,
        input: Value,
    ) -> Result<Value, OperationError> {
        let connection = ConnectionAuthority::new(
            &parent.connection_ref,
            initiation_policy(self.policy.initiation),
        )
        .map_err(|_| operation_not_granted())?;
        let admission = AdmittedOperation::from_grant_decision(
            GRAFANA,
            &operation.id,
            context.tenant_id(),
            context.actor_subject(),
            &self.policy.grant_ref,
            connection,
        );
        let plan = plan_operation(
            GRAFANA,
            operation,
            admission,
            &PlanningEnvironment {
                available_drivers: BTreeSet::from([DriverId::HttpV1]),
                available_route_adapters: BTreeSet::new(),
                capabilities: BTreeSet::from([Capability::PrivateNetwork]),
                permission_subjects: vec![self.policy.canonical_origin()],
            },
        )
        .map_err(|_| operation_not_granted())?;
        if !matches!(plan.protocol(), ProtocolPlan::HttpV1(_)) {
            return Err(operation_unavailable());
        }
        let credential = grafana_credential(token)?;
        let request = resolve(
            operation,
            &self.policy.canonical_origin(),
            &input,
            &BTreeMap::new(),
            &[credential],
        )
        .map_err(|_| operation_invalid())?;
        let expected = format!("{}/", self.policy.canonical_origin());
        if request.request.url != self.policy.canonical_origin()
            && !request.request.url.starts_with(&expected)
        {
            return Err(operation_not_granted());
        }
        self.executor
            .execute(request.request)
            .await
            .map_err(|_| operation_unavailable())
    }

    async fn execute_mediated(
        &self,
        context: &PrincipalContext,
        child: &ChildConnection,
        token: &Secret,
        operation: &'static connector_resolve::document::Operation,
        input: Value,
    ) -> Result<Value, OperationError> {
        let connection = ConnectionAuthority::mediated(
            &child.connection_ref,
            InitiationPolicy::b10x_only(),
            &child.parent_connection_ref,
            &child.resource_binding,
            DomainRouteAdapter::GrafanaDatasourceProxyV1,
        )
        .map_err(|_| operation_not_granted())?;
        let admission = AdmittedOperation::from_grant_decision(
            &child.provider,
            &operation.id,
            context.tenant_id(),
            context.actor_subject(),
            &child.grant_ref,
            connection,
        );
        let plan = plan_operation(
            &child.provider,
            operation,
            admission,
            &PlanningEnvironment {
                available_drivers: BTreeSet::from([DriverId::HttpV1]),
                available_route_adapters: BTreeSet::from([
                    DomainRouteAdapter::GrafanaDatasourceProxyV1,
                ]),
                capabilities: BTreeSet::from([Capability::PrivateNetwork]),
                permission_subjects: vec![self.policy.canonical_origin()],
            },
        )
        .map_err(|_| operation_not_granted())?;
        let ProtocolPlan::MediatedHttpV1(mediated) = plan.protocol() else {
            return Err(operation_unavailable());
        };
        let resolved = resolve(operation, TARGET_BASE, &input, &BTreeMap::new(), &[])
            .map_err(|_| operation_invalid())?;
        let relative = resolved
            .request
            .url
            .strip_prefix(TARGET_BASE)
            .filter(|relative| relative.starts_with(&mediated.target_path_template))
            .ok_or_else(operation_not_granted)?;
        if mediated.parent_connection != child.parent_connection_ref
            || mediated.resource_binding != child.resource_binding
            || mediated.adapter != DomainRouteAdapter::GrafanaDatasourceProxyV1
            || !safe_datasource_uid(&child.resource_binding)
        {
            return Err(operation_not_granted());
        }
        let mut request = Request {
            method: resolved.request.method,
            url: format!(
                "{}/api/datasources/proxy/uid/{}{}",
                self.policy.canonical_origin(),
                child.resource_binding,
                relative
            ),
            headers: resolved.request.headers,
            body: resolved.request.body,
        };
        let credential = grafana_credential(token)?;
        connector_resolve::auth::place(&operation.id, &credential, &mut request)
            .map_err(|_| operation_unavailable())?;
        self.executor
            .execute(request)
            .await
            .map_err(|_| operation_unavailable())
    }

    fn connections_for_operation(&self, operation_ref: &str) -> Vec<ConnectionSummary> {
        let provider = provider_for_operation(operation_ref);
        let state = lock(&self.state);
        if provider == GRAFANA {
            return state
                .parent
                .as_ref()
                .map(|parent| {
                    vec![ConnectionSummary {
                        connection_ref: parent.connection_ref.clone(),
                        label: parent.label.clone(),
                        provider: provider.to_owned(),
                        audiences: audiences_for_operation(operation_ref),
                    }]
                })
                .unwrap_or_default();
        }
        state
            .children
            .values()
            .filter(|child| child.provider == provider && child_is_current(&state, child))
            .map(|child| ConnectionSummary {
                connection_ref: child.connection_ref.clone(),
                label: child.label.clone(),
                provider: provider.to_owned(),
                audiences: audiences_for_operation(operation_ref),
            })
            .collect()
    }

    fn description_ref(&self, context: &PrincipalContext, operation_ref: &str) -> String {
        let mut digest = Sha256::new();
        digest.update(document_text(provider_for_operation(operation_ref)).as_bytes());
        digest.update(b"\0");
        digest.update(serde_json::to_vec(context).expect("owner context serializes"));
        digest.update(b"\0");
        digest.update(operation_ref.as_bytes());
        for connection in self.connections_for_operation(operation_ref) {
            digest.update(b"\0");
            digest.update(connection.connection_ref.as_bytes());
        }
        format!("description-sha256-{:x}", digest.finalize())
    }

    fn search_connections(&self, query: &str) -> Vec<ControlConnectionSummary> {
        let query = query.to_ascii_lowercase();
        let state = lock(&self.state);
        let mut result = Vec::new();
        if let Some(parent) = &state.parent {
            let summary = self.parent_summary(parent);
            if query.is_empty()
                || summary.label.to_ascii_lowercase().contains(&query)
                || GRAFANA.contains(&query)
            {
                result.push(summary);
            }
        }
        result.extend(state.children.values().filter_map(|child| {
            let summary = self.child_summary(&state, child);
            (query.is_empty()
                || summary.label.to_ascii_lowercase().contains(&query)
                || child.provider.contains(&query))
            .then_some(summary)
        }));
        result
    }

    fn describe_connection(&self, connection_ref: &str) -> Option<ConnectionDescription> {
        let state = lock(&self.state);
        if let Some(parent) = state
            .parent
            .as_ref()
            .filter(|parent| parent.connection_ref == connection_ref)
        {
            return Some(ConnectionDescription {
                summary: self.parent_summary(parent),
                channels: Vec::new(),
            });
        }
        state
            .children
            .get(connection_ref)
            .map(|child| ConnectionDescription {
                summary: self.child_summary(&state, child),
                channels: Vec::new(),
            })
    }

    fn parent_summary(&self, parent: &ParentConnection) -> ControlConnectionSummary {
        ControlConnectionSummary {
            connection_ref: parent.connection_ref.clone(),
            integration_ref: GRAFANA.to_owned(),
            label: parent.label.clone(),
            state: ConnectionState::Callable,
            initiation: initiation(self.policy.initiation),
            route: ConnectionRoute::Direct,
            scope: None,
            actor: None,
            auth_profile: None,
        }
    }

    fn child_summary(
        &self,
        state: &MonitoringState,
        child: &ChildConnection,
    ) -> ControlConnectionSummary {
        ControlConnectionSummary {
            connection_ref: child.connection_ref.clone(),
            integration_ref: child.provider.clone(),
            label: child.label.clone(),
            state: if child_is_current(state, child) {
                ConnectionState::Callable
            } else {
                ConnectionState::Degraded
            },
            initiation: vec![ConnectionInitiator::B10x],
            route: ConnectionRoute::ViaConnection {
                parent_connection_ref: child.parent_connection_ref.clone(),
                route_adapter: RouteAdapter::GrafanaDatasourceProxyV1,
            },
            scope: None,
            actor: None,
            auth_profile: None,
        }
    }

    fn is_parent(&self, connection_ref: &str) -> bool {
        lock(&self.state)
            .parent
            .as_ref()
            .is_some_and(|parent| parent.connection_ref == connection_ref)
    }

    fn owns_connection_ref(&self, connection_ref: &str) -> bool {
        let state = lock(&self.state);
        state
            .parent
            .as_ref()
            .is_some_and(|parent| parent.connection_ref == connection_ref)
            || state.children.contains_key(connection_ref)
    }

    fn has_observation(&self, observation_ref: &str) -> bool {
        lock(&self.state).observations.contains_key(observation_ref)
    }

    fn has_session(&self, session_ref: &str) -> bool {
        lock(&self.sessions).owns(session_ref)
    }

    fn search_observations(&self, query: &str, limit: u16) -> Vec<DiscoveryObservationSummary> {
        let query = query.to_ascii_lowercase();
        lock(&self.state)
            .observations
            .values()
            .filter(|observation| {
                query.is_empty()
                    || observation.title.to_ascii_lowercase().contains(&query)
                    || observation
                        .observed_type
                        .to_ascii_lowercase()
                        .contains(&query)
            })
            .take(usize::from(limit))
            .map(observation_summary)
            .collect()
    }

    fn materialize(&self, observation_ref: &str) -> Result<ConnectionDescription, ConnectionError> {
        let mut state = lock(&self.state);
        let observation = state
            .observations
            .get(observation_ref)
            .cloned()
            .ok_or_else(connection_not_found)?;
        if !observation.active {
            return Err(ConnectionError::new(
                ConnectionErrorCode::Conflict,
                "discovery observation is no longer current",
                false,
            ));
        }
        if let Some(connection_ref) = &observation.connection_ref {
            let child = state
                .children
                .get(connection_ref)
                .ok_or_else(connection_protocol)?;
            if !child_is_current(&state, child) {
                return Err(ConnectionError::new(
                    ConnectionErrorCode::Conflict,
                    "materialized Connection no longer matches current discovery evidence",
                    false,
                ));
            }
            return Ok(ConnectionDescription {
                summary: self.child_summary(&state, child),
                channels: Vec::new(),
            });
        }
        let provider = observation.target_provider.clone().ok_or_else(|| {
            ConnectionError::new(
                ConnectionErrorCode::NotGranted,
                "observed datasource type has no target Provider contract",
                false,
            )
        })?;
        let grant_ref = self
            .policy
            .target_grant(&provider)
            .ok_or_else(|| {
                ConnectionError::new(
                    ConnectionErrorCode::NotGranted,
                    "target Provider has no independent Connector Grant",
                    false,
                )
            })?
            .to_owned();
        let connection_ref = format!(
            "connection:{provider}:{}",
            digest_prefix(observation_ref.as_bytes())
        );
        let child = ChildConnection {
            connection_ref: connection_ref.clone(),
            label: observation.title.clone(),
            provider,
            grant_ref,
            parent_connection_ref: observation.source_connection_ref,
            observation_ref: observation_ref.to_owned(),
            resource_binding: observation.resource_binding,
        };
        state.children.insert(connection_ref.clone(), child.clone());
        state
            .observations
            .get_mut(observation_ref)
            .expect("observation was read from this map")
            .connection_ref = Some(connection_ref);
        Ok(ConnectionDescription {
            summary: self.child_summary(&state, &child),
            channels: Vec::new(),
        })
    }

    async fn reconcile_hosted_targets(&self) -> Result<(), OperationError> {
        let parent = lock(&self.state)
            .parent
            .clone()
            .ok_or_else(operation_not_granted)?;
        let token = self.load_credential().await?;
        let operation =
            operation_document(GRAFANA_DATASOURCES_LIST).ok_or_else(operation_unavailable)?;
        let output = self
            .execute_direct(
                &self.owner,
                &parent,
                &token,
                operation,
                serde_json::json!({}),
            )
            .await?;
        self.reconcile_hosted_output(&parent.connection_ref, &output)
    }

    fn reconcile_hosted_output(
        &self,
        source_connection_ref: &str,
        output: &Value,
    ) -> Result<(), OperationError> {
        let targets = self.hosted_targets.as_ref().ok_or_else(operation_invalid)?;
        let items = output.as_array().ok_or_else(operation_invalid)?;
        if items.len() > 5000 {
            return Err(operation_invalid());
        }
        let mut discovered = BTreeMap::<(String, String), (String, String)>::new();
        for item in items {
            let object = item.as_object().ok_or_else(operation_invalid)?;
            let uid = object
                .get("uid")
                .and_then(Value::as_str)
                .filter(|uid| safe_datasource_uid(uid))
                .ok_or_else(operation_invalid)?;
            let provider = object
                .get("type")
                .and_then(Value::as_str)
                .map(str::to_ascii_lowercase)
                .filter(|kind| valid_observed_type(kind))
                .ok_or_else(operation_invalid)?;
            let title = object
                .get("name")
                .and_then(Value::as_str)
                .filter(|title| valid_title(title))
                .ok_or_else(operation_invalid)?;
            discovered.insert(
                (provider, format!("{:x}", Sha256::digest(uid.as_bytes()))),
                (uid.to_owned(), title.to_owned()),
            );
        }
        let evidence_sha256 = format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(&discovered.iter().collect::<Vec<_>>())
                    .map_err(|_| operation_invalid())?
            )
        );
        let mut state = lock(&self.state);
        state.evidence_generation = state.evidence_generation.saturating_add(1).max(1);
        let generation = state.evidence_generation;
        state.observations.clear();
        state.children.clear();
        for target in targets {
            let observation_ref = format!(
                "observation:grafana:{}",
                digest_prefix(
                    format!("{source_connection_ref}\0{}", target.connection_ref).as_bytes()
                )
            );
            let found = discovered.get(&(target.provider.clone(), target.uid_sha256.clone()));
            let resource_binding = found.map_or_else(String::new, |(uid, _)| uid.clone());
            state.observations.insert(
                observation_ref.clone(),
                StoredObservation {
                    observation_ref: observation_ref.clone(),
                    source_connection_ref: source_connection_ref.to_owned(),
                    observed_type: target.provider.clone(),
                    title: target.label.clone(),
                    resource_binding: resource_binding.clone(),
                    target_provider: Some(target.provider.clone()),
                    active: found.is_some(),
                    evidence_generation: generation,
                    evidence_sha256: evidence_sha256.clone(),
                    connection_ref: Some(target.connection_ref.clone()),
                },
            );
            state.children.insert(
                target.connection_ref.clone(),
                ChildConnection {
                    connection_ref: target.connection_ref.clone(),
                    label: target.label.clone(),
                    provider: target.provider.clone(),
                    grant_ref: target.grant_ref.clone(),
                    parent_connection_ref: source_connection_ref.to_owned(),
                    observation_ref,
                    resource_binding,
                },
            );
        }
        Ok(())
    }

    fn degrade_hosted_targets(&self) {
        let mut state = lock(&self.state);
        for observation in state.observations.values_mut() {
            observation.active = false;
        }
    }

    async fn create_session(
        self: &Arc<Self>,
        label: String,
    ) -> Result<ConnectSessionStatus, ConnectionError> {
        if lock(&self.state).parent.is_some() {
            return Err(ConnectionError::new(
                ConnectionErrorCode::Conflict,
                "the configured Grafana Integration already has a Connection",
                false,
            ));
        }
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
        let task_session = session_ref;
        lock(&self.tasks).push(tokio::spawn(async move {
            inner.serve_completion(endpoint, task_session).await;
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
                MAX_TOKEN_BYTES,
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
        let result = self
            .complete_connection(&session_ref, submission.secret())
            .await;
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
        &self,
        session_ref: &str,
        secret: &Secret,
    ) -> Result<String, MonitoringError> {
        let _completion = self.completion.lock().await;
        if lock(&self.state).parent.is_some() {
            return Err(MonitoringError::new("connection-conflict"));
        }
        let label = lock(&self.sessions)
            .pending_label(session_ref)
            .map_err(|_| MonitoringError::new("connect-session"))?;
        let instance_id = random_uuid()?;
        let parent = ParentConnection {
            connection_ref: format!("connection:grafana:{instance_id}"),
            label,
        };
        let operation = operation_document(GRAFANA_DATASOURCES_LIST)
            .ok_or_else(|| MonitoringError::new("catalog"))?;
        let output = self
            .execute_direct(
                &self.owner,
                &parent,
                secret,
                operation,
                serde_json::json!({}),
            )
            .await
            .map_err(|_| MonitoringError::new("verification"))?;
        let prior = lock(&self.state).clone();
        if self
            .reconcile_observations(&parent.connection_ref, &output)
            .is_err()
        {
            *lock(&self.state) = prior;
            return Err(MonitoringError::new("discovery"));
        }
        if self
            .credential_store
            .put(&self.credential_ref, secret)
            .await
            .is_err()
        {
            *lock(&self.state) = prior;
            return Err(MonitoringError::new("credential-store"));
        }
        let connection_ref = parent.connection_ref.clone();
        lock(&self.state).parent = Some(parent);
        Ok(connection_ref)
    }

    fn reconcile_observations(
        &self,
        source_connection_ref: &str,
        output: &Value,
    ) -> Result<(), OperationError> {
        let items = output.as_array().ok_or_else(operation_invalid)?;
        if items.len() > 5000 {
            return Err(operation_invalid());
        }
        let mut normalized = Vec::new();
        for item in items {
            let object = item.as_object().ok_or_else(operation_invalid)?;
            let uid = object
                .get("uid")
                .and_then(Value::as_str)
                .filter(|uid| safe_datasource_uid(uid))
                .ok_or_else(operation_invalid)?;
            let observed_type = object
                .get("type")
                .and_then(Value::as_str)
                .map(str::to_ascii_lowercase)
                .filter(|kind| valid_observed_type(kind))
                .ok_or_else(operation_invalid)?;
            let title = object
                .get("name")
                .and_then(Value::as_str)
                .filter(|title| valid_title(title))
                .ok_or_else(operation_invalid)?;
            normalized.push((uid.to_owned(), observed_type, title.to_owned()));
        }
        normalized.sort();
        normalized.dedup_by(|left, right| left.0 == right.0);
        let evidence_sha256 = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&normalized).map_err(|_| operation_invalid())?)
        );
        let mut state = lock(&self.state);
        state.evidence_generation = state.evidence_generation.saturating_add(1).max(1);
        let generation = state.evidence_generation;
        for observation in state.observations.values_mut() {
            if observation.source_connection_ref == source_connection_ref {
                observation.active = false;
                observation.evidence_generation = generation;
                observation.evidence_sha256.clone_from(&evidence_sha256);
            }
        }
        for (uid, observed_type, title) in normalized {
            let observation_ref = format!(
                "observation:grafana:{}",
                digest_prefix(format!("{source_connection_ref}\0{uid}").as_bytes())
            );
            let target_provider = target_provider(&observed_type).map(str::to_owned);
            match state.observations.get_mut(&observation_ref) {
                Some(observation) => {
                    observation.observed_type = observed_type;
                    observation.title = title;
                    observation.target_provider = target_provider;
                    observation.active = true;
                    observation.evidence_generation = generation;
                    observation.evidence_sha256.clone_from(&evidence_sha256);
                }
                None => {
                    state.observations.insert(
                        observation_ref.clone(),
                        StoredObservation {
                            observation_ref,
                            source_connection_ref: source_connection_ref.to_owned(),
                            observed_type,
                            title,
                            resource_binding: uid,
                            target_provider,
                            active: true,
                            evidence_generation: generation,
                            evidence_sha256: evidence_sha256.clone(),
                            connection_ref: None,
                        },
                    );
                }
            }
        }
        Ok(())
    }

    fn append_audit(&self, event: AuditEvent<'_>) -> Result<(), MonitoringError> {
        let _guard = lock(&self.audit);
        let mut line = serde_json::to_vec(&serde_json::json!({
            "at_unix_ms": now_ms().ok_or_else(|| MonitoringError::new("clock"))?,
            "event": event,
        }))
        .map_err(|_| MonitoringError::new("audit"))?;
        line.push(b'\n');
        if let Some(hosted_state) = &self.hosted_state {
            return hosted_state
                .append(AUDIT_STATE_KEY, &line, MAX_AUDIT_BYTES as usize)
                .map(|_| ())
                .map_err(|error| match error {
                    hosted_state::StateError::Capacity => MonitoringError::new("audit-bound"),
                    _ => MonitoringError::new("audit"),
                });
        }
        let path = self.state_root.join("monitoring-audit.jsonl");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .mode(0o600)
            .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
            .open(path)
            .map_err(|_| MonitoringError::new("audit"))?;
        let metadata = file.metadata().map_err(|_| MonitoringError::new("audit"))?;
        if !metadata.file_type().is_file()
            || metadata.uid() != rustix::process::geteuid().as_raw()
            || metadata.permissions().mode() & 0o077 != 0
        {
            return Err(MonitoringError::new("audit"));
        }
        if metadata
            .len()
            .checked_add(line.len() as u64)
            .is_none_or(|size| size > MAX_AUDIT_BYTES)
        {
            return Err(MonitoringError::new("audit-bound"));
        }
        file.write_all(&line)
            .and_then(|()| file.sync_data())
            .map_err(|_| MonitoringError::new("audit"))
    }
}

fn grafana_credential(token: &Secret) -> Result<Assembled, OperationError> {
    let provider =
        catalog::provider(catalog::ProviderKey::id(GRAFANA)).ok_or_else(operation_unavailable)?;
    let credential = provider
        .credential(GRAFANA_CREDENTIAL)
        .ok_or_else(operation_unavailable)?;
    Ok(Assembled::new(
        credential.name,
        acquire(credential, token.expose_secret(), None),
        credential.place,
    ))
}

fn grafana_credential_ref(owner: &PrincipalContext) -> Result<CredentialRef, MonitoringError> {
    CredentialRef::new(
        owner.tenant_id(),
        GRAFANA_AUTHORITY,
        DEFAULT_SERVICE,
        GRAFANA_CREDENTIAL_LEAF,
    )
    .map_err(|_| MonitoringError::new("credential-reference"))
}

fn observation_summary(observation: &StoredObservation) -> DiscoveryObservationSummary {
    let state = if !observation.active {
        DiscoveryObservationState::Withdrawn
    } else if observation.connection_ref.is_some() {
        DiscoveryObservationState::Materialized
    } else if observation.target_provider.is_some() {
        DiscoveryObservationState::Observed
    } else {
        DiscoveryObservationState::Unsupported
    };
    DiscoveryObservationSummary {
        observation_ref: observation.observation_ref.clone(),
        discovery_ref: DISCOVERY_REF.to_owned(),
        source_connection_ref: observation.source_connection_ref.clone(),
        observed_type: observation.observed_type.clone(),
        title: observation.title.clone(),
        state,
        evidence_generation: observation.evidence_generation,
        evidence_sha256: observation.evidence_sha256.clone(),
        target_provider_ref: observation.target_provider.clone(),
        connection_ref: if state == DiscoveryObservationState::Materialized {
            observation.connection_ref.clone()
        } else {
            None
        },
    }
}

fn child_is_current(state: &MonitoringState, child: &ChildConnection) -> bool {
    state
        .observations
        .get(&child.observation_ref)
        .is_some_and(|observation| {
            observation.active
                && observation.target_provider.as_deref() == Some(child.provider.as_str())
                && observation.source_connection_ref == child.parent_connection_ref
                && observation.resource_binding == child.resource_binding
        })
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

fn initiation_policy(config: InitiationConfig) -> InitiationPolicy {
    match config {
        InitiationConfig::B10x => InitiationPolicy::b10x_only(),
        InitiationConfig::Provider => InitiationPolicy::provider_only(),
        InitiationConfig::Both => InitiationPolicy::bidirectional(),
    }
}

fn safe_datasource_uid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn valid_observed_type(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn valid_title(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}

fn project_output(operation: &str, raw: &Value) -> Result<Value, OperationError> {
    let projected = match operation {
        monitoring_model::GRAFANA_DATASOURCES_LIST => project_datasources(raw)?,
        monitoring_model::GRAFANA_DASHBOARDS_LIST => project_dashboard_list(raw)?,
        monitoring_model::GRAFANA_DASHBOARD_GET => project_dashboard(raw)?,
        monitoring_model::PROMETHEUS_QUERY_RANGE => project_prometheus(raw)?,
        monitoring_model::LOKI_QUERY_RANGE => project_loki(raw)?,
        monitoring_model::ALERTMANAGER_ALERTS_LIST => project_alerts(raw)?,
        _ => return Err(operation_not_found()),
    };
    if serde_json::to_vec(&projected).map_or(true, |bytes| {
        bytes.len() > protocol::operation::MAX_RESULT_BYTES
    }) {
        return Err(operation_unavailable());
    }
    Ok(projected)
}

fn project_datasources(raw: &Value) -> Result<Value, OperationError> {
    let sources = raw
        .as_array()
        .ok_or_else(operation_invalid)?
        .iter()
        .take(500)
        .filter_map(|source| {
            let object = source.as_object()?;
            let name = bounded_text(object.get("name")?.as_str()?, 256);
            let provider = bounded_text(object.get("type")?.as_str()?, 128);
            (!name.is_empty() && !provider.is_empty()).then(|| {
                serde_json::json!({
                    "name": name,
                    "provider": provider,
                    "status": "available",
                    "callable": matches!(provider.as_str(), "prometheus" | "loki" | "alertmanager"),
                })
            })
        })
        .collect::<Vec<_>>();
    Ok(
        serde_json::json!({"sources": sources, "complete": raw.as_array().is_some_and(|items| items.len() <= 500)}),
    )
}

fn project_dashboard_list(raw: &Value) -> Result<Value, OperationError> {
    let object = raw.as_object().ok_or_else(operation_invalid)?;
    let items = object
        .get("items")
        .and_then(Value::as_array)
        .ok_or_else(operation_invalid)?;
    let dashboards = items
        .iter()
        .take(500)
        .filter_map(project_dashboard_summary)
        .collect::<Vec<_>>();
    let next_cursor = object
        .get("metadata")
        .and_then(Value::as_object)
        .and_then(|metadata| metadata.get("continue"))
        .and_then(Value::as_str)
        .map(|value| bounded_text(value, 4096));
    Ok(serde_json::json!({
        "dashboards": dashboards,
        "next_cursor": next_cursor,
        "complete": items.len() <= 500 && next_cursor.is_none(),
    }))
}

fn project_dashboard_summary(value: &Value) -> Option<Value> {
    let object = value.as_object()?;
    let metadata = object.get("metadata")?.as_object()?;
    let spec = object.get("spec")?.as_object()?;
    let uid = bounded_text(metadata.get("name")?.as_str()?, 256);
    let title = bounded_text(spec.get("title")?.as_str()?, 256);
    let tags = spec
        .get("tags")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .take(32)
        .map(|tag| bounded_text(tag, 128))
        .collect::<Vec<_>>();
    Some(serde_json::json!({"uid": uid, "title": title, "tags": tags}))
}

fn project_dashboard(raw: &Value) -> Result<Value, OperationError> {
    let object = raw.as_object().ok_or_else(operation_invalid)?;
    let metadata = object
        .get("metadata")
        .and_then(Value::as_object)
        .ok_or_else(operation_invalid)?;
    let spec = object
        .get("spec")
        .and_then(Value::as_object)
        .ok_or_else(operation_invalid)?;
    let panels = spec
        .get("panels")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(500)
        .filter_map(|panel| {
            let panel = panel.as_object()?;
            Some(serde_json::json!({
                "title": bounded_text(panel.get("title").and_then(Value::as_str).unwrap_or("Untitled"), 256),
                "kind": bounded_text(panel.get("type").and_then(Value::as_str).unwrap_or("unknown"), 64),
            }))
        })
        .collect::<Vec<_>>();
    let tags = spec
        .get("tags")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .take(32)
        .map(|tag| bounded_text(tag, 128))
        .collect::<Vec<_>>();
    Ok(serde_json::json!({
        "uid": bounded_text(metadata.get("name").and_then(Value::as_str).unwrap_or(""), 256),
        "title": bounded_text(spec.get("title").and_then(Value::as_str).unwrap_or("Untitled"), 256),
        "description": redact_text(spec.get("description").and_then(Value::as_str).unwrap_or(""), 1024).0,
        "tags": tags,
        "panels": panels,
        "panels_truncated": spec.get("panels").and_then(Value::as_array).is_some_and(|items| items.len() > 500),
    }))
}

fn project_prometheus(raw: &Value) -> Result<Value, OperationError> {
    let object = raw.as_object().ok_or_else(operation_invalid)?;
    if object.get("status").and_then(Value::as_str) != Some("success") {
        return Err(operation_unavailable());
    }
    let data = object
        .get("data")
        .and_then(Value::as_object)
        .ok_or_else(operation_invalid)?;
    let result = data
        .get("result")
        .and_then(Value::as_array)
        .ok_or_else(operation_invalid)?;
    let series = result
        .iter()
        .take(500)
        .filter_map(|series| {
            let series = series.as_object()?;
            let labels = project_labels(series.get("metric"));
            let samples = series
                .get("values")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .take(2_000)
                .filter_map(project_sample)
                .collect::<Vec<_>>();
            let instant = series.get("value").and_then(project_sample);
            Some(serde_json::json!({"labels": labels, "samples": samples, "value": instant}))
        })
        .collect::<Vec<_>>();
    Ok(serde_json::json!({
        "status": "success",
        "result_type": bounded_text(data.get("resultType").and_then(Value::as_str).unwrap_or("unknown"), 32),
        "series": series,
        "truncated": result.len() > 500,
    }))
}

fn project_sample(value: &Value) -> Option<Value> {
    let pair = value.as_array()?;
    (pair.len() == 2).then(|| {
        serde_json::json!({
            "timestamp": pair[0].clone(),
            "value": bounded_text(pair[1].as_str().unwrap_or(""), 128),
        })
    })
}

fn project_loki(raw: &Value) -> Result<Value, OperationError> {
    let object = raw.as_object().ok_or_else(operation_invalid)?;
    if object.get("status").and_then(Value::as_str) != Some("success") {
        return Err(operation_unavailable());
    }
    let data = object
        .get("data")
        .and_then(Value::as_object)
        .ok_or_else(operation_invalid)?;
    let streams = data
        .get("result")
        .and_then(Value::as_array)
        .ok_or_else(operation_invalid)?;
    let mut lines = Vec::new();
    for stream in streams.iter().take(500) {
        let Some(stream) = stream.as_object() else {
            continue;
        };
        let labels = project_labels(stream.get("stream"));
        for pair in stream
            .get("values")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if lines.len() == 1_000 {
                break;
            }
            let Some(pair) = pair.as_array().filter(|pair| pair.len() == 2) else {
                continue;
            };
            let (line, redacted) = redact_text(pair[1].as_str().unwrap_or(""), 8 * 1024);
            lines.push(serde_json::json!({
                "timestamp": pair[0].clone(),
                "labels": labels,
                "line": line,
                "redacted": redacted,
                "truncated": pair[1].as_str().is_some_and(|value| value.len() > 8 * 1024),
            }));
        }
        if lines.len() == 1_000 {
            break;
        }
    }
    Ok(serde_json::json!({
        "result_type": bounded_text(data.get("resultType").and_then(Value::as_str).unwrap_or("streams"), 32),
        "lines": lines,
        "truncated": lines.len() == 1_000 || streams.len() > 500,
    }))
}

fn project_alerts(raw: &Value) -> Result<Value, OperationError> {
    let alerts = raw
        .as_array()
        .ok_or_else(operation_invalid)?
        .iter()
        .take(500)
        .filter_map(|alert| {
            let alert = alert.as_object()?;
            let annotations = alert.get("annotations").and_then(Value::as_object);
            let (summary, summary_redacted) = redact_text(
                annotations
                    .and_then(|values| values.get("summary"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                512,
            );
            let status = alert
                .get("status")
                .and_then(Value::as_object)
                .and_then(|status| status.get("state"))
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            Some(serde_json::json!({
                "labels": project_labels(alert.get("labels")),
                "summary": summary,
                "summary_redacted": summary_redacted,
                "state": bounded_text(status, 64),
                "starts_at": bounded_text(alert.get("startsAt").and_then(Value::as_str).unwrap_or(""), 64),
                "ends_at": bounded_text(alert.get("endsAt").and_then(Value::as_str).unwrap_or(""), 64),
            }))
        })
        .collect::<Vec<_>>();
    Ok(serde_json::json!({
        "alerts": alerts,
        "complete": raw.as_array().is_some_and(|items| items.len() <= 500),
    }))
}

fn project_labels(value: Option<&Value>) -> Value {
    const ALLOWED: [&str; 12] = [
        "__name__",
        "alertname",
        "cluster",
        "container",
        "deployment",
        "instance",
        "job",
        "namespace",
        "pod",
        "service",
        "severity",
        "status",
    ];
    let labels = value
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter(|(key, _)| ALLOWED.contains(&key.as_str()))
        .filter_map(|(key, value)| {
            value
                .as_str()
                .map(|value| (key.clone(), Value::String(redact_text(value, 256).0)))
        })
        .collect::<serde_json::Map<_, _>>();
    Value::Object(labels)
}

fn bounded_text(value: &str, maximum: usize) -> String {
    value
        .chars()
        .filter(|character| !character.is_control() || matches!(character, '\n' | '\t'))
        .take(maximum)
        .collect()
}

fn redact_text(value: &str, maximum: usize) -> (String, bool) {
    let lowered = value.to_ascii_lowercase();
    let sensitive = [
        "authorization:",
        "bearer ",
        "password=",
        "password:",
        "secret=",
        "secret:",
        "token=",
        "token:",
        "api_key=",
        "api-key:",
        "-----begin private key-----",
    ]
    .iter()
    .any(|pattern| lowered.contains(pattern))
        || value
            .split('.')
            .filter(|segment| segment.len() > 16)
            .count()
            >= 3
            && value.starts_with("eyJ");
    if sensitive {
        ("[redacted]".to_owned(), true)
    } else {
        (bounded_text(value, maximum), false)
    }
}

fn digest_prefix(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))[..24].to_owned()
}

fn random_uuid() -> Result<String, MonitoringError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| MonitoringError::new("randomness"))?;
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

fn ensure_owner_directory(path: &Path) -> Result<(), MonitoringError> {
    if !path.exists() {
        fs::create_dir_all(path).map_err(|_| MonitoringError::new("owner-state-directory"))?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|_| MonitoringError::new("owner-state-directory"))?;
    }
    let metadata =
        fs::symlink_metadata(path).map_err(|_| MonitoringError::new("owner-state-directory"))?;
    if !metadata.file_type().is_dir()
        || metadata.file_type().is_symlink()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err(MonitoringError::new("owner-state-directory"));
    }
    Ok(())
}

include!("backend_tests.rs");

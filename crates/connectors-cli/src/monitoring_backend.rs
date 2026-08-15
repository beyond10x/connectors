//! Grafana discovery plus mediated Prometheus, Loki, and Alertmanager execution.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::os::unix::fs::{
    FileTypeExt as _, MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _,
};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use connector_resolve::auth::{acquire, Assembled};
use connector_resolve::document::{HostEffect, ProtocolDriver};
use connector_resolve::{resolve, Request};
use connector_secrets::Secret;
use domain::{
    AdmittedOperation, Capability, ConnectionAuthority, DriverId, InitiationPolicy, ProtocolPlan,
    RouteAdapter as DomainRouteAdapter,
};
use protocol::connection::{
    ConnectSessionState, ConnectSessionStatus, ConnectionDescription, ConnectionError,
    ConnectionErrorCode, ConnectionInitiator, ConnectionRequest, ConnectionResult, ConnectionRoute,
    ConnectionState, ConnectionSummary as ControlConnectionSummary, DiscoveryObservationState,
    DiscoveryObservationSummary, RouteAdapter,
};
use protocol::operation::{
    ApprovalPosture, ConnectionSummary, DescribeRequest, EffectClass, InvocationResult,
    InvokeRequest, OperationDescription, OperationError, OperationErrorCode, OperationRequest,
    OperationResult, OperationSummary, OwnerContext,
};
use reqwest::redirect::Policy;
use serde_json::Value;
use server::local::OperationBackend;
use service::{plan_operation, PlanningEnvironment};
use sha2::{Digest as _, Sha256};
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::task::JoinHandle;
use zeroize::Zeroizing;

use crate::{GrafanaIntegrationConfig, InitiationConfig};

pub(crate) const GRAFANA: &str = "grafana";
const GRAFANA_CREDENTIAL: &str = "grafana.service_account_token";
const DISCOVERY_REF: &str = "grafana-data-sources";
const GRAFANA_DATASOURCES_LIST: &str = "grafana-datasources-list";
const GRAFANA_DASHBOARDS_LIST: &str = "grafana-dashboards-list";
const GRAFANA_DASHBOARD_GET: &str = "grafana-dashboard-get";
pub(crate) const PROMETHEUS_QUERY_RANGE: &str = "prometheus-query-range";
pub(crate) const LOKI_QUERY_RANGE: &str = "loki-query-range";
pub(crate) const ALERTMANAGER_ALERTS_LIST: &str = "alertmanager-alerts-list";
const TARGET_BASE: &str = "https://mediated-target.invalid";
const MAX_CONNECT_SESSIONS: usize = 16;
const MAX_TOKEN_BYTES: usize = 8 * 1024;
const MAX_AUDIT_BYTES: u64 = 16 * 1024 * 1024;

const GRAFANA_DOCUMENT: &str = include_str!("../../../catalog/grafana.catalog.json");
const PROMETHEUS_DOCUMENT: &str = include_str!("../../../catalog/prometheus.catalog.json");
const LOKI_DOCUMENT: &str = include_str!("../../../catalog/loki.catalog.json");
const ALERTMANAGER_DOCUMENT: &str = include_str!("../../../catalog/alertmanager.catalog.json");

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

/// Composition backend: monitoring requests terminate here and everything else is delegated.
pub struct MonitoringBackend {
    operation: Arc<dyn OperationBackend>,
    inner: Arc<MonitoringInner>,
}

struct MonitoringInner {
    owner: OwnerContext,
    policy: GrafanaIntegrationConfig,
    state_root: PathBuf,
    state: Mutex<MonitoringState>,
    sessions: Mutex<BTreeMap<String, SessionRecord>>,
    token: Mutex<Option<Secret>>,
    tasks: Mutex<Vec<JoinHandle<()>>>,
    executor: Arc<dyn HttpExecutor>,
    audit: Mutex<()>,
}

#[derive(Default)]
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

#[derive(Clone)]
struct SessionRecord {
    label: String,
    state: ConnectSessionState,
    expires_at_unix_ms: u64,
    endpoint: Option<PathBuf>,
    connection_ref: Option<String>,
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
    /// Construct a Grafana monitoring backend. Credentials remain absent until a Connect Session
    /// completes, and are held in memory until an OS-keychain backend replaces this alpha store.
    pub fn open(
        owner: OwnerContext,
        policy: GrafanaIntegrationConfig,
        state_root: &Path,
        operation: Arc<dyn OperationBackend>,
    ) -> Result<Self, MonitoringError> {
        ensure_owner_directory(state_root)?;
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
            operation,
            Arc::new(ReqwestExecutor { client }),
        ))
    }

    fn with_executor<E: HttpExecutor>(
        owner: OwnerContext,
        policy: GrafanaIntegrationConfig,
        state_root: &Path,
        operation: Arc<dyn OperationBackend>,
        executor: Arc<E>,
    ) -> Self {
        Self {
            operation,
            inner: Arc::new(MonitoringInner {
                owner,
                policy,
                state_root: state_root.to_path_buf(),
                state: Mutex::new(MonitoringState::default()),
                sessions: Mutex::new(BTreeMap::new()),
                token: Mutex::new(None),
                tasks: Mutex::new(Vec::new()),
                executor,
                audit: Mutex::new(()),
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
impl OperationBackend for MonitoringBackend {
    async fn handle(
        &self,
        context: &OwnerContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        self.inner.check_operation_context(context)?;
        match request {
            OperationRequest::Search(request) => {
                let mut operations = match self
                    .operation
                    .handle(context, OperationRequest::Search(request.clone()))
                    .await
                {
                    Ok(OperationResult::Search { operations }) => operations,
                    Err(error)
                        if matches!(
                            error.code,
                            OperationErrorCode::NotFound | OperationErrorCode::Unavailable
                        ) =>
                    {
                        Vec::new()
                    }
                    Ok(_) => return Err(operation_protocol()),
                    Err(error) => return Err(error),
                };
                operations.extend(self.inner.search_operations(&request.query));
                operations.sort_by(|left, right| left.operation_ref.cmp(&right.operation_ref));
                operations.dedup_by(|left, right| left.operation_ref == right.operation_ref);
                operations.truncate(usize::from(request.limit));
                Ok(OperationResult::Search { operations })
            }
            OperationRequest::Describe(request) if supported_operation(&request.operation_ref) => {
                self.inner.describe(context, request)
            }
            OperationRequest::Invoke(request) if supported_operation(&request.operation_ref) => {
                self.inner.invoke(context, request).await
            }
            other => self.operation.handle(context, other).await,
        }
    }

    async fn handle_connection(
        &self,
        context: &OwnerContext,
        request: ConnectionRequest,
    ) -> Result<ConnectionResult, ConnectionError> {
        self.inner.check_connection_context(context)?;
        match request {
            ConnectionRequest::Search(request) => {
                let mut connections = match self
                    .operation
                    .handle_connection(context, ConnectionRequest::Search(request.clone()))
                    .await
                {
                    Ok(ConnectionResult::Search { connections }) => connections,
                    Err(error)
                        if matches!(
                            error.code,
                            ConnectionErrorCode::NotFound | ConnectionErrorCode::Unavailable
                        ) =>
                    {
                        Vec::new()
                    }
                    Ok(_) => return Err(connection_protocol()),
                    Err(error) => return Err(error),
                };
                connections.extend(self.inner.search_connections(&request.query));
                connections.sort_by(|left, right| left.connection_ref.cmp(&right.connection_ref));
                connections.dedup_by(|left, right| left.connection_ref == right.connection_ref);
                connections.truncate(usize::from(request.limit));
                Ok(ConnectionResult::Search { connections })
            }
            ConnectionRequest::Describe(request) => {
                if let Some(description) = self.inner.describe_connection(&request.connection_ref) {
                    Ok(ConnectionResult::Describe(description))
                } else {
                    self.operation
                        .handle_connection(context, ConnectionRequest::Describe(request))
                        .await
                }
            }
            ConnectionRequest::ObservationSearch(request) => {
                if self.inner.is_parent(&request.source_connection_ref) {
                    Ok(ConnectionResult::ObservationSearch {
                        observations: self
                            .inner
                            .search_observations(&request.query, request.limit),
                    })
                } else {
                    self.operation
                        .handle_connection(context, ConnectionRequest::ObservationSearch(request))
                        .await
                }
            }
            ConnectionRequest::Materialize(request) => {
                if self.inner.has_observation(&request.observation_ref) {
                    self.inner
                        .materialize(&request.observation_ref)
                        .map(ConnectionResult::Materialize)
                } else {
                    self.operation
                        .handle_connection(context, ConnectionRequest::Materialize(request))
                        .await
                }
            }
            ConnectionRequest::ConnectSessionCreate(request)
                if request.integration_ref == GRAFANA =>
            {
                self.inner
                    .create_session(request.label)
                    .await
                    .map(ConnectionResult::ConnectSessionCreate)
            }
            ConnectionRequest::ConnectSessionStatus(request)
                if self.inner.has_session(&request.connect_session_ref) =>
            {
                self.inner
                    .session_status(&request.connect_session_ref)
                    .map(ConnectionResult::ConnectSessionStatus)
                    .ok_or_else(connection_not_found)
            }
            other => self.operation.handle_connection(context, other).await,
        }
    }

    async fn handle_event(
        &self,
        context: &OwnerContext,
        request: protocol::event::EventRequest,
    ) -> Result<protocol::event::EventResult, protocol::event::EventError> {
        self.operation.handle_event(context, request).await
    }

    async fn shutdown(&self) {
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
        *lock(&self.inner.token) = None;
        self.operation.shutdown().await;
    }

    fn supports_connections(&self) -> bool {
        true
    }

    fn supports_events(&self) -> bool {
        true
    }
}

impl MonitoringInner {
    fn check_operation_context(&self, actual: &OwnerContext) -> Result<(), OperationError> {
        if actual == &self.owner {
            Ok(())
        } else {
            Err(OperationError::new(
                OperationErrorCode::StaleAuthority,
                "owner authority snapshot is not current",
                false,
            ))
        }
    }

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
        context: &OwnerContext,
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
        context: &OwnerContext,
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

        let (result, parent_ref, adapter) = if provider == GRAFANA {
            let parent = lock(&self.state)
                .parent
                .clone()
                .filter(|parent| parent.connection_ref == request.connection_ref)
                .ok_or_else(operation_not_granted)?;
            let token = lock(&self.token)
                .clone()
                .ok_or_else(operation_not_granted)?;
            let value = self
                .execute_direct(context, &parent, &token, operation, request.input)
                .await?;
            if request.operation_ref == GRAFANA_DATASOURCES_LIST {
                self.reconcile_observations(&parent.connection_ref, &value)?;
            }
            (value, None, None)
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
            let token = lock(&self.token)
                .clone()
                .ok_or_else(operation_not_granted)?;
            let value = self
                .execute_mediated(context, &child, &token, operation, request.input)
                .await?;
            (
                value,
                Some(child.parent_connection_ref.clone()),
                Some(DomainRouteAdapter::GrafanaDatasourceProxyV1.as_str()),
            )
        };
        let audit_ref = format!(
            "audit:monitoring:{}",
            random_uuid().map_err(|_| operation_unavailable())?
        );
        self.append_audit(AuditEvent {
            audit_ref: &audit_ref,
            operation_ref: &request.operation_ref,
            connection_ref: &request.connection_ref,
            parent_connection_ref: parent_ref.as_deref(),
            route_adapter: adapter,
            tenant_id: &context.tenant_id,
            agent_id: &context.agent_id,
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

    async fn execute_direct(
        &self,
        context: &OwnerContext,
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
            &context.tenant_id,
            &context.agent_id,
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
        context: &OwnerContext,
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
            &context.tenant_id,
            &context.agent_id,
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
                .filter(|_| lock(&self.token).is_some())
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

    fn description_ref(&self, context: &OwnerContext, operation_ref: &str) -> String {
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
            state: if lock(&self.token).is_some() {
                ConnectionState::Callable
            } else {
                ConnectionState::Degraded
            },
            initiation: initiation(self.policy.initiation),
            route: ConnectionRoute::Direct,
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
            state: if child_is_current(state, child) && lock(&self.token).is_some() {
                ConnectionState::Callable
            } else {
                ConnectionState::Degraded
            },
            initiation: vec![ConnectionInitiator::B10x],
            route: ConnectionRoute::ViaConnection {
                parent_connection_ref: child.parent_connection_ref.clone(),
                route_adapter: RouteAdapter::GrafanaDatasourceProxyV1,
            },
        }
    }

    fn is_parent(&self, connection_ref: &str) -> bool {
        lock(&self.state)
            .parent
            .as_ref()
            .is_some_and(|parent| parent.connection_ref == connection_ref)
    }

    fn has_observation(&self, observation_ref: &str) -> bool {
        lock(&self.state).observations.contains_key(observation_ref)
    }

    fn has_session(&self, session_ref: &str) -> bool {
        lock(&self.sessions).contains_key(session_ref)
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
        let task_session = session_ref;
        lock(&self.tasks).push(tokio::spawn(async move {
            inner
                .serve_completion(listener, task_session, endpoint)
                .await;
        }));
        Ok(status)
    }

    fn session_status(&self, session_ref: &str) -> Option<ConnectSessionStatus> {
        let session = lock(&self.sessions).get(session_ref)?.clone();
        Some(ConnectSessionStatus {
            connect_session_ref: session_ref.to_owned(),
            integration_ref: GRAFANA.to_owned(),
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
        if let Some(stream) = stream.as_mut() {
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
        &self,
        session_ref: &str,
        secret: Secret,
    ) -> Result<String, MonitoringError> {
        let label = lock(&self.sessions)
            .get(session_ref)
            .filter(|session| session.state == ConnectSessionState::Pending)
            .map(|session| session.label.clone())
            .ok_or_else(|| MonitoringError::new("connect-session"))?;
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
                &secret,
                operation,
                serde_json::json!({}),
            )
            .await
            .map_err(|_| MonitoringError::new("verification"))?;
        self.reconcile_observations(&parent.connection_ref, &output)
            .map_err(|_| MonitoringError::new("discovery"))?;
        *lock(&self.token) = Some(secret);
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
        let mut line = serde_json::to_vec(&serde_json::json!({
            "at_unix_ms": now_ms().ok_or_else(|| MonitoringError::new("clock"))?,
            "event": event,
        }))
        .map_err(|_| MonitoringError::new("audit"))?;
        line.push(b'\n');
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

fn operation_ids() -> [&'static str; 6] {
    [
        GRAFANA_DASHBOARDS_LIST,
        GRAFANA_DASHBOARD_GET,
        GRAFANA_DATASOURCES_LIST,
        PROMETHEUS_QUERY_RANGE,
        LOKI_QUERY_RANGE,
        ALERTMANAGER_ALERTS_LIST,
    ]
}

pub(crate) fn supported_operation(operation: &str) -> bool {
    operation_ids().contains(&operation)
}

pub(crate) fn provider_for_operation(operation: &str) -> &'static str {
    match operation {
        GRAFANA_DASHBOARDS_LIST | GRAFANA_DASHBOARD_GET | GRAFANA_DATASOURCES_LIST => GRAFANA,
        PROMETHEUS_QUERY_RANGE => "prometheus",
        LOKI_QUERY_RANGE => "loki",
        ALERTMANAGER_ALERTS_LIST => "alertmanager",
        _ => "",
    }
}

fn document_text(provider: &str) -> &'static str {
    match provider {
        GRAFANA => GRAFANA_DOCUMENT,
        "prometheus" => PROMETHEUS_DOCUMENT,
        "loki" => LOKI_DOCUMENT,
        "alertmanager" => ALERTMANAGER_DOCUMENT,
        _ => "{}",
    }
}

pub(crate) fn operation_document(
    operation: &str,
) -> Option<&'static connector_resolve::document::Operation> {
    connector_resolve::document::provider(provider_for_operation(operation))?.operation(operation)
}

pub(crate) fn audiences_for_operation(operation: &str) -> Vec<String> {
    let Some(operation) = catalog::operation(catalog::OperationKey::id(operation)) else {
        return Vec::new();
    };
    let Some(provider) = catalog::provider(catalog::ProviderKey::id(operation.provider)) else {
        return Vec::new();
    };
    provider
        .services
        .iter()
        .find(|service| service.name == operation.service)
        .map_or(provider.audiences, |service| service.audiences)
        .iter()
        .map(|audience| audience.as_str().to_owned())
        .collect()
}

pub(crate) fn response_schema(provider: &str, operation: &str) -> Result<Value, OperationError> {
    let value: Value =
        serde_json::from_str(document_text(provider)).map_err(|_| operation_unavailable())?;
    value["operations"]
        .as_array()
        .and_then(|operations| {
            operations
                .iter()
                .find(|candidate| candidate["id"] == operation)
        })
        .and_then(|operation| operation.get("response_schema"))
        .cloned()
        .ok_or_else(operation_unavailable)
}

fn target_provider(observed_type: &str) -> Option<&'static str> {
    let provider = catalog::provider(catalog::ProviderKey::id(GRAFANA))?;
    provider
        .discoveries
        .iter()
        .find(|discovery| discovery.id == DISCOVERY_REF)?
        .mappings
        .iter()
        .find(|mapping| mapping.observed_type == observed_type)
        .map(|mapping| mapping.target_provider)
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

pub(crate) fn validate_input(operation: &str, input: &Value) -> Result<(), OperationError> {
    if serde_json::to_vec(input).map_or(true, |bytes| {
        bytes.len() > protocol::operation::MAX_ARGUMENT_BYTES
    }) {
        return Err(operation_invalid());
    }
    let object = input.as_object().ok_or_else(operation_invalid)?;
    let expected = operation_document(operation)
        .ok_or_else(operation_not_found)?
        .caller_parameters()
        .into_iter()
        .collect::<BTreeSet<_>>();
    if object.len() != expected.len() || object.keys().any(|key| !expected.contains(key.as_str())) {
        return Err(operation_invalid());
    }
    let string = |name: &str, maximum: usize| {
        object
            .get(name)
            .and_then(Value::as_str)
            .is_some_and(|value| {
                !value.is_empty() && value.len() <= maximum && !value.contains('\0')
            })
    };
    let valid = match operation {
        GRAFANA_DASHBOARDS_LIST => {
            string("namespace", 256)
                && object
                    .get("limit")
                    .and_then(Value::as_u64)
                    .is_some_and(|limit| (1..=1000).contains(&limit))
                && object
                    .get("continue")
                    .and_then(Value::as_str)
                    .is_some_and(|value| value.len() <= 4096 && !value.contains('\0'))
        }
        GRAFANA_DASHBOARD_GET => string("namespace", 256) && string("uid", 256),
        GRAFANA_DATASOURCES_LIST | ALERTMANAGER_ALERTS_LIST => object.is_empty(),
        PROMETHEUS_QUERY_RANGE => {
            string("query", 16 * 1024)
                && string("start", 64)
                && string("end", 64)
                && string("step", 64)
        }
        LOKI_QUERY_RANGE => {
            string("query", 16 * 1024)
                && string("start", 64)
                && string("end", 64)
                && object
                    .get("limit")
                    .and_then(Value::as_u64)
                    .is_some_and(|limit| (1..=5000).contains(&limit))
                && object
                    .get("direction")
                    .and_then(Value::as_str)
                    .is_some_and(|direction| matches!(direction, "forward" | "backward"))
        }
        _ => false,
    };
    valid.then_some(()).ok_or_else(operation_invalid)
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

fn effect(effects: &[HostEffect]) -> EffectClass {
    if effects.contains(&HostEffect::Write) {
        EffectClass::Mutating
    } else {
        EffectClass::ReadOnly
    }
}

pub(crate) fn title(operation: &str) -> &'static str {
    match operation {
        GRAFANA_DASHBOARDS_LIST => "List Grafana dashboards",
        GRAFANA_DASHBOARD_GET => "Get a Grafana dashboard",
        GRAFANA_DATASOURCES_LIST => "Refresh Grafana datasource discovery",
        PROMETHEUS_QUERY_RANGE => "Query Prometheus metrics over a time range",
        LOKI_QUERY_RANGE => "Query Loki logs over a time range",
        ALERTMANAGER_ALERTS_LIST => "List Alertmanager alerts",
        _ => "Unknown operation",
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

async fn accept_owner(listener: UnixListener) -> Result<UnixStream, MonitoringError> {
    loop {
        let (stream, _) = listener
            .accept()
            .await
            .map_err(|_| MonitoringError::new("connect-session-accept"))?;
        let credential = stream
            .peer_cred()
            .map_err(|_| MonitoringError::new("connect-session-peer"))?;
        if credential.uid() == rustix::process::geteuid().as_raw() {
            return Ok(stream);
        }
    }
}

async fn read_submitted_secret(
    mut stream: UnixStream,
) -> Result<(Secret, UnixStream), MonitoringError> {
    let mut bytes = Zeroizing::new(Vec::with_capacity(256));
    {
        let reader = BufReader::new(&mut stream);
        let mut bounded = reader.take((MAX_TOKEN_BYTES + 3) as u64);
        tokio::time::timeout(
            Duration::from_secs(30),
            bounded.read_until(b'\n', &mut bytes),
        )
        .await
        .map_err(|_| MonitoringError::new("connect-session-timeout"))?
        .map_err(|_| MonitoringError::new("connect-session-read"))?;
    }
    if bytes.last() != Some(&b'\n') || bytes.len() > MAX_TOKEN_BYTES + 2 {
        return Err(MonitoringError::new("credential-shape"));
    }
    bytes.pop();
    if bytes.last() == Some(&b'\r') {
        bytes.pop();
    }
    let value =
        std::str::from_utf8(&bytes).map_err(|_| MonitoringError::new("credential-shape"))?;
    if value.is_empty()
        || value.len() > MAX_TOKEN_BYTES
        || value
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
    {
        return Err(MonitoringError::new("credential-shape"));
    }
    let bytes = std::mem::take(&mut *bytes);
    let value = String::from_utf8(bytes).expect("credential bytes were validated as UTF-8");
    Ok((Secret::new(value), stream))
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

fn refuse_existing_path(path: &Path) -> Result<(), MonitoringError> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        _ => Err(MonitoringError::new("connect-session-path")),
    }
}

fn remove_owned_socket(path: &Path) -> Result<(), MonitoringError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(MonitoringError::new("connect-session-path")),
    };
    if !metadata.file_type().is_socket() || metadata.uid() != rustix::process::geteuid().as_raw() {
        return Err(MonitoringError::new("connect-session-path"));
    }
    fs::remove_file(path).map_err(|_| MonitoringError::new("connect-session-path"))
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
        "connection, observation, or Connect Session was not found",
        false,
    )
}

fn connection_protocol() -> ConnectionError {
    ConnectionError::new(
        ConnectionErrorCode::Protocol,
        "connection backend returned an incompatible response",
        false,
    )
}

fn operation_unavailable() -> OperationError {
    OperationError::new(
        OperationErrorCode::Unavailable,
        "monitoring connector runtime is unavailable",
        true,
    )
}

fn operation_not_found() -> OperationError {
    OperationError::new(
        OperationErrorCode::NotFound,
        "monitoring operation was not found",
        false,
    )
}

fn operation_not_granted() -> OperationError {
    OperationError::new(
        OperationErrorCode::NotGranted,
        "operation is not granted for this Connection",
        false,
    )
}

fn operation_invalid() -> OperationError {
    OperationError::new(
        OperationErrorCode::InvalidInput,
        "operation input is outside the catalog contract",
        false,
    )
}

fn operation_protocol() -> OperationError {
    OperationError::new(
        OperationErrorCode::Protocol,
        "operation backend returned an incompatible response",
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

    #[derive(Default)]
    struct FakeExecutor {
        requests: Mutex<Vec<Request>>,
    }

    #[async_trait]
    impl HttpExecutor for FakeExecutor {
        async fn execute(&self, request: Request) -> Result<Value, MonitoringError> {
            let output = if request.url.contains("/api/datasources")
                && !request.url.contains("/proxy/")
            {
                serde_json::json!([
                    {"id":1,"uid":"prom-main","name":"Metrics","type":"prometheus"},
                    {"id":2,"uid":"loki-main","name":"Logs","type":"loki"},
                    {"id":3,"uid":"private-main","name":"Private plugin","type":"vendor-private"}
                ])
            } else {
                serde_json::json!({"status":"success","data":{"result":[]}})
            };
            lock(&self.requests).push(request);
            Ok(output)
        }
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

    fn policy() -> GrafanaIntegrationConfig {
        toml::from_str(
            r#"
origin = "https://grafana.example"
grant_ref = "grant:grafana"
initiation = "b10x"
connect_session_ttl_seconds = 300

[target_grants]
prometheus = "grant:prometheus"
loki = "grant:loki"
alertmanager = "grant:alertmanager"
"#,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn discovery_materialization_and_query_stay_on_the_grafana_route() {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let executor = Arc::new(FakeExecutor::default());
        let backend = MonitoringBackend::with_executor(
            owner(),
            policy(),
            root.path(),
            Arc::new(RefusingBackend),
            Arc::clone(&executor),
        );
        let parent = ParentConnection {
            connection_ref: "connection:grafana:test".to_owned(),
            label: "Infrastructure Grafana".to_owned(),
        };
        let token = Secret::new("SENTINEL-NOT-A-REAL-SECRET");
        let output = backend
            .inner
            .execute_direct(
                &owner(),
                &parent,
                &token,
                operation_document(GRAFANA_DATASOURCES_LIST).unwrap(),
                serde_json::json!({}),
            )
            .await
            .unwrap();
        backend
            .inner
            .reconcile_observations(&parent.connection_ref, &output)
            .unwrap();
        lock(&backend.inner.state).parent = Some(parent);
        *lock(&backend.inner.token) = Some(token);

        let observations = backend.inner.search_observations("", 64);
        assert_eq!(observations.len(), 3);
        assert!(observations.iter().any(|observation| {
            observation.observed_type == "vendor-private"
                && observation.state == DiscoveryObservationState::Unsupported
        }));
        let prometheus = observations
            .iter()
            .find(|observation| observation.observed_type == "prometheus")
            .unwrap();
        let child = backend
            .inner
            .materialize(&prometheus.observation_ref)
            .unwrap();
        assert!(matches!(
            child.summary.route,
            ConnectionRoute::ViaConnection {
                route_adapter: RouteAdapter::GrafanaDatasourceProxyV1,
                ..
            }
        ));

        let described = backend
            .handle(
                &owner(),
                OperationRequest::Describe(DescribeRequest {
                    operation_ref: PROMETHEUS_QUERY_RANGE.to_owned(),
                }),
            )
            .await
            .unwrap();
        let OperationResult::Describe(description) = described else {
            panic!("expected description")
        };
        let child_ref = child.summary.connection_ref.clone();
        backend
            .handle(
                &owner(),
                OperationRequest::Invoke(InvokeRequest {
                    operation_ref: PROMETHEUS_QUERY_RANGE.to_owned(),
                    connection_ref: child_ref.clone(),
                    description_ref: description.description_ref,
                    input: serde_json::json!({
                        "query":"up",
                        "start":"now-5m",
                        "end":"now",
                        "step":"30s"
                    }),
                    approval_evidence_ref: None,
                }),
            )
            .await
            .unwrap();

        let requests = lock(&executor.requests);
        let query = requests.last().unwrap();
        assert!(query.url.starts_with(
            "https://grafana.example/api/datasources/proxy/uid/prom-main/api/v1/query_range?"
        ));
        assert!(!query.url.contains("mediated-target.invalid"));
        assert!(!format!("{query:?}").contains("SENTINEL"));

        let observation_ref = prometheus.observation_ref.clone();
        lock(&backend.inner.state)
            .observations
            .get_mut(&observation_ref)
            .unwrap()
            .target_provider = Some("loki".to_owned());
        assert!(backend
            .inner
            .connections_for_operation(PROMETHEUS_QUERY_RANGE)
            .is_empty());
        assert_eq!(
            backend
                .inner
                .describe_connection(&child_ref)
                .unwrap()
                .summary
                .state,
            ConnectionState::Degraded
        );
        assert!(backend.inner.materialize(&observation_ref).is_err());
    }
}

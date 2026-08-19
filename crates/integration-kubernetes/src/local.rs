//! Trusted personal-local kubeconfig discovery and bounded Kubernetes monitoring discovery.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use async_trait::async_trait;
use connector_resolve::document::ProtocolDriver;
use connector_resolve::resolve;
use domain::{
    AdmittedOperation, Capability, ConnectionAuthority, DriverId, InitiationPolicy, ProtocolPlan,
    RouteAdapter as DomainRouteAdapter,
};
use futures_util::AsyncReadExt as _;
use http::{Method, Request as HttpRequest};
use k8s_openapi::api::authentication::v1::SelfSubjectReview;
use k8s_openapi::api::authorization::v1::{
    ResourceAttributes, SelfSubjectAccessReview, SelfSubjectAccessReviewSpec,
};
use k8s_openapi::api::core::v1::{Service, ServicePort};
use kube::api::{ListParams, PostParams};
use kube::config::{KubeConfigOptions, Kubeconfig};
use kube::{Api, Client, Config};
use protocol::connection::{
    CandidateActivateRequest, CandidateSearchRequest, ConnectionCandidateState,
    ConnectionCandidateSummary, ConnectionDescription, ConnectionError, ConnectionErrorCode,
    ConnectionInitiator, ConnectionRequest, ConnectionResult, ConnectionRoute, ConnectionState,
    ConnectionSummary, DiscoveryObservationState, DiscoveryObservationSummary, MaterializeRequest,
    ObservationSearchRequest, RouteAdapter,
};
use protocol::operation::{
    ApprovalPosture, ConnectionSummary as OperationConnectionSummary, DescribeRequest, EffectClass,
    InvocationResult, InvokeRequest, OperationDescription, OperationError, OperationErrorCode,
    OperationRequest, OperationResult, OperationSummary,
};
use serde_json::Value;
use service::{
    plan_operation, BackendCapabilities, ConnectorBackend, PlanningEnvironment, PrincipalContext,
};
use sha2::{Digest as _, Sha256};

use connectors_config::{InitiationConfig, KubernetesIntegrationConfig};

const KUBERNETES: &str = "kubernetes";
const DISCOVERY_REF: &str = "discovery:kubernetes-service-v1";
const TARGET_BASE: &str = "https://mediated-target.invalid";
const MAX_PROXY_RESULT_BYTES: usize = protocol::operation::MAX_RESULT_BYTES;

/// Setup refusal for the personal-local Kubernetes backend. Details deliberately do not include
/// kubeconfig contents, credential helpers, endpoints, or paths.
#[derive(Debug, thiserror::Error)]
pub enum KubernetesLocalError {
    #[error("standard kubeconfig contexts could not be discovered")]
    Kubeconfig,
}

#[derive(Debug, Clone)]
struct CandidateBinding {
    summary: ConnectionCandidateSummary,
    context_name: String,
    evidence_material: String,
}

#[derive(Default)]
struct KubernetesState {
    connections: BTreeMap<String, ConnectionDescription>,
    candidate_connections: BTreeMap<String, String>,
    clients: BTreeMap<String, Client>,
    observations: BTreeMap<String, StoredServiceObservation>,
    children: BTreeMap<String, KubernetesServiceConnection>,
}

#[derive(Debug, Clone)]
struct StoredServiceObservation {
    summary: DiscoveryObservationSummary,
    namespace: String,
    service: String,
    resource_uid: String,
    port: String,
    provider: String,
    resource_binding: String,
}

#[derive(Debug, Clone)]
struct KubernetesServiceConnection {
    connection_ref: String,
    label: String,
    provider: String,
    grant_ref: String,
    parent_connection_ref: String,
    observation_ref: String,
    namespace: String,
    service: String,
    resource_uid: String,
    port: String,
    resource_binding: String,
}

/// Personal-local backend which passively detects kubeconfig contexts, then contacts a cluster
/// only after one opaque candidate is explicitly activated.
pub struct KubernetesLocalBackend {
    owner: PrincipalContext,
    policy: KubernetesIntegrationConfig,
    candidates: BTreeMap<String, CandidateBinding>,
    state: Mutex<KubernetesState>,
    activation: tokio::sync::Mutex<()>,
}

impl KubernetesLocalBackend {
    /// Read context metadata from the standard merged kubeconfig. No cluster request or auth exec
    /// occurs here. Credential-bearing fields remain private to this trusted Connector process.
    pub fn open(
        owner: PrincipalContext,
        policy: KubernetesIntegrationConfig,
        _state_root: &Path,
    ) -> Result<Self, KubernetesLocalError> {
        let kubeconfig = Kubeconfig::read().map_err(|_| KubernetesLocalError::Kubeconfig)?;
        let candidates = candidates(&kubeconfig);
        Ok(Self {
            owner,
            policy,
            candidates,
            state: Mutex::new(KubernetesState::default()),
            activation: tokio::sync::Mutex::new(()),
        })
    }

    /// Number of activated Kubernetes source Connections in this daemon generation.
    #[must_use]
    pub fn connection_count(&self) -> usize {
        lock(&self.state).candidate_connections.len()
    }

    /// Number of passively detected context candidates.
    #[must_use]
    pub fn candidate_count(&self) -> usize {
        self.candidates.len()
    }

    fn check_context(&self, context: &PrincipalContext) -> Result<(), ConnectionError> {
        if context != &self.owner {
            return Err(ConnectionError::new(
                ConnectionErrorCode::StaleAuthority,
                "owner context does not match this Connector generation",
                false,
            ));
        }
        Ok(())
    }

    fn check_operation_context(&self, context: &PrincipalContext) -> Result<(), OperationError> {
        if context == &self.owner {
            Ok(())
        } else {
            Err(OperationError::new(
                OperationErrorCode::StaleAuthority,
                "owner context does not match this Connector generation",
                false,
            ))
        }
    }

    fn search_candidates(
        &self,
        request: &CandidateSearchRequest,
    ) -> Vec<ConnectionCandidateSummary> {
        let query = request.query.to_ascii_lowercase();
        let state = lock(&self.state);
        self.candidates
            .values()
            .filter(|candidate| {
                candidate
                    .summary
                    .title
                    .to_ascii_lowercase()
                    .contains(&query)
            })
            .map(|candidate| {
                let mut summary = candidate.summary.clone();
                if let Some(connection_ref) = state
                    .candidate_connections
                    .get(&candidate.summary.candidate_ref)
                {
                    summary.state = ConnectionCandidateState::Activated;
                    summary.connection_ref = Some(connection_ref.clone());
                }
                summary
            })
            .take(usize::from(request.limit))
            .collect()
    }

    fn search_connections(&self, query: &str) -> Vec<ConnectionSummary> {
        let query = query.to_ascii_lowercase();
        lock(&self.state)
            .connections
            .values()
            .map(|connection| connection.summary.clone())
            .filter(|connection| connection.label.to_ascii_lowercase().contains(&query))
            .collect()
    }

    async fn activate(
        &self,
        request: CandidateActivateRequest,
    ) -> Result<ConnectionDescription, ConnectionError> {
        let candidate = self
            .candidates
            .get(&request.candidate_ref)
            .cloned()
            .ok_or_else(connection_not_found)?;
        // Activation can invoke an external credential helper. Serialize and re-check so repeated
        // calls cannot run that effect more than once for the same candidate in this generation.
        let _activation = self.activation.lock().await;
        if let Some(connection_ref) = lock(&self.state)
            .candidate_connections
            .get(&request.candidate_ref)
            .cloned()
        {
            return lock(&self.state)
                .connections
                .get(&connection_ref)
                .cloned()
                .ok_or_else(connection_protocol);
        }

        // Re-read at the explicit activation boundary so a stale passive candidate cannot silently
        // bind to a different cluster or identity.
        let kubeconfig = Kubeconfig::read().map_err(|_| connection_unavailable())?;
        let fresh = binding_for_context(&kubeconfig, &candidate.context_name)
            .ok_or_else(connection_not_found)?;
        if fresh.evidence_material != candidate.evidence_material {
            return Err(ConnectionError::new(
                ConnectionErrorCode::StaleAuthority,
                "kubeconfig context changed after it was detected",
                false,
            ));
        }
        if context_uses_credential_plugin(&kubeconfig, &candidate.context_name)
            && !self.policy.allow_exec_auth
        {
            return Err(ConnectionError::new(
                ConnectionErrorCode::NotGranted,
                "the selected context uses a credential plugin; allow_exec_auth is required",
                false,
            ));
        }

        let options = KubeConfigOptions {
            context: Some(candidate.context_name.clone()),
            ..KubeConfigOptions::default()
        };
        let mut config = Config::from_custom_kubeconfig(kubeconfig, &options)
            .await
            .map_err(|_| connection_unavailable())?;
        // The kubeconfig-selected API server is the only admitted destination in this slice.
        // Ambient or kubeconfig proxy routing requires its own reviewed route contract.
        config.proxy_url = None;
        let client = Client::try_from(config).map_err(|_| connection_unavailable())?;
        verify_identity(client.clone()).await?;
        let services = discover_services(client.clone(), &self.policy).await?;

        let connection_ref = opaque_ref(
            "connection:kubernetes:",
            &format!(
                "{}\0{}",
                candidate.summary.candidate_ref, candidate.evidence_material
            ),
        );
        let description = ConnectionDescription {
            summary: ConnectionSummary {
                connection_ref: connection_ref.clone(),
                integration_ref: KUBERNETES.to_owned(),
                label: request.label,
                state: ConnectionState::Authorized,
                initiation: initiation(self.policy.initiation),
                route: ConnectionRoute::Direct,
                scope: None,
                actor: None,
                auth_profile: None,
            },
            channels: Vec::new(),
        };
        let observations = normalize_services(&connection_ref, services);
        let mut state = lock(&self.state);
        state
            .candidate_connections
            .insert(request.candidate_ref, connection_ref.clone());
        state.clients.insert(connection_ref.clone(), client);
        for observation in observations {
            state
                .observations
                .insert(observation.summary.observation_ref.clone(), observation);
        }
        state
            .connections
            .insert(connection_ref, description.clone());
        Ok(description)
    }

    fn observations(
        &self,
        request: &ObservationSearchRequest,
    ) -> Option<Vec<DiscoveryObservationSummary>> {
        let query = request.query.to_ascii_lowercase();
        let state = lock(&self.state);
        if !state
            .connections
            .contains_key(&request.source_connection_ref)
        {
            return None;
        }
        Some(
            state
                .observations
                .values()
                .filter(|observation| {
                    observation.summary.source_connection_ref == request.source_connection_ref
                        && observation
                            .summary
                            .title
                            .to_ascii_lowercase()
                            .contains(&query)
                })
                .take(usize::from(request.limit))
                .map(|observation| observation.summary.clone())
                .collect(),
        )
    }

    fn observation(&self, observation_ref: &str) -> Option<StoredServiceObservation> {
        lock(&self.state).observations.get(observation_ref).cloned()
    }

    fn connections_for_operation(&self, operation_ref: &str) -> Vec<OperationConnectionSummary> {
        let provider = monitoring_model::provider_for_operation(operation_ref);
        if !kubernetes_route_operation(operation_ref) {
            return Vec::new();
        }
        let state = lock(&self.state);
        state
            .children
            .values()
            .filter(|child| child.provider == provider && child_is_current(&state, child))
            .map(|child| OperationConnectionSummary {
                connection_ref: child.connection_ref.clone(),
                label: child.label.clone(),
                provider: child.provider.clone(),
                audiences: monitoring_model::audiences_for_operation(operation_ref),
            })
            .collect()
    }

    fn child_is_current(&self, child: &KubernetesServiceConnection) -> bool {
        let state = lock(&self.state);
        child_is_current(&state, child)
    }

    fn operation_summary(&self, operation_ref: &str) -> Option<OperationSummary> {
        let connections = self.connections_for_operation(operation_ref);
        (!connections.is_empty()).then(|| OperationSummary {
            operation_ref: operation_ref.to_owned(),
            title: monitoring_model::title(operation_ref).to_owned(),
            effect: EffectClass::ReadOnly,
            approval: ApprovalPosture::NotRequired,
            connections,
        })
    }

    fn operation_description(
        &self,
        context: &PrincipalContext,
        operation_ref: &str,
    ) -> Result<OperationDescription, OperationError> {
        let operation =
            monitoring_model::operation_document(operation_ref).ok_or_else(operation_not_found)?;
        let connections = self.connections_for_operation(operation_ref);
        if connections.is_empty() {
            return Err(operation_not_found());
        }
        Ok(OperationDescription {
            operation_ref: operation_ref.to_owned(),
            title: monitoring_model::title(operation_ref).to_owned(),
            description: operation.contract_description().to_owned(),
            input_schema: operation.input_schema().clone(),
            output_schema: monitoring_model::response_schema(
                monitoring_model::provider_for_operation(operation_ref),
                operation_ref,
            )?,
            effect: EffectClass::ReadOnly,
            approval: ApprovalPosture::NotRequired,
            connections,
            description_ref: self.operation_description_ref(context, operation_ref),
        })
    }

    fn operation_description_ref(&self, context: &PrincipalContext, operation_ref: &str) -> String {
        let mut hash = Sha256::new();
        hash.update(context.stable_authority_seed());
        hash.update(b"\0kubernetes_service_proxy_v1\0");
        hash.update(operation_ref.as_bytes());
        for connection in self.connections_for_operation(operation_ref) {
            hash.update(b"\0");
            hash.update(connection.connection_ref.as_bytes());
        }
        format!("description-sha256-{:x}", hash.finalize())
    }

    fn materialize(
        &self,
        request: &MaterializeRequest,
    ) -> Result<ConnectionDescription, ConnectionError> {
        let observation = self
            .observation(&request.observation_ref)
            .ok_or_else(connection_not_found)?;
        if observation.provider == monitoring_model::GRAFANA {
            return Err(ConnectionError::new(
                ConnectionErrorCode::NotGranted,
                "the discovered Grafana Service requires an explicit credential source",
                false,
            ));
        }
        let grant_ref = self
            .policy
            .target_grant(&observation.provider)
            .ok_or_else(|| {
                ConnectionError::new(
                    ConnectionErrorCode::NotGranted,
                    "target Provider has no independent Connector Grant",
                    false,
                )
            })?
            .to_owned();
        let mut state = lock(&self.state);
        if let Some(connection_ref) = observation.summary.connection_ref.as_deref() {
            return state
                .connections
                .get(connection_ref)
                .cloned()
                .ok_or_else(connection_protocol);
        }
        if !state
            .clients
            .contains_key(&observation.summary.source_connection_ref)
        {
            return Err(ConnectionError::new(
                ConnectionErrorCode::Conflict,
                "source Kubernetes Connection is no longer active",
                false,
            ));
        }
        let connection_ref = opaque_ref(
            &format!("connection:{}:", observation.provider),
            &format!(
                "{}\0{}",
                observation.summary.observation_ref, observation.resource_binding
            ),
        );
        let child = KubernetesServiceConnection {
            connection_ref: connection_ref.clone(),
            label: observation.summary.title.clone(),
            provider: observation.provider,
            grant_ref,
            parent_connection_ref: observation.summary.source_connection_ref,
            observation_ref: observation.summary.observation_ref.clone(),
            namespace: observation.namespace,
            service: observation.service,
            resource_uid: observation.resource_uid,
            port: observation.port,
            resource_binding: observation.resource_binding,
        };
        let description = ConnectionDescription {
            summary: ConnectionSummary {
                connection_ref: connection_ref.clone(),
                integration_ref: child.provider.clone(),
                label: child.label.clone(),
                state: ConnectionState::Callable,
                initiation: vec![ConnectionInitiator::B10x],
                route: ConnectionRoute::ViaConnection {
                    parent_connection_ref: child.parent_connection_ref.clone(),
                    route_adapter: RouteAdapter::KubernetesServiceProxyV1,
                },
                scope: None,
                actor: None,
                auth_profile: None,
            },
            channels: Vec::new(),
        };
        let stored = state
            .observations
            .get_mut(&request.observation_ref)
            .ok_or_else(connection_not_found)?;
        stored.summary.state = DiscoveryObservationState::Materialized;
        stored.summary.connection_ref = Some(connection_ref.clone());
        state.children.insert(connection_ref.clone(), child);
        state
            .connections
            .insert(connection_ref, description.clone());
        Ok(description)
    }

    async fn invoke_service(
        &self,
        context: &PrincipalContext,
        request: InvokeRequest,
    ) -> Result<OperationResult, OperationError> {
        if !kubernetes_route_operation(&request.operation_ref)
            || request.description_ref
                != self.operation_description_ref(context, &request.operation_ref)
            || request.approval_evidence_ref.is_some()
        {
            return Err(operation_not_granted());
        }
        monitoring_model::validate_input(&request.operation_ref, &request.input)?;
        let child = lock(&self.state)
            .children
            .get(&request.connection_ref)
            .cloned()
            .ok_or_else(operation_not_granted)?;
        if child.provider != monitoring_model::provider_for_operation(&request.operation_ref)
            || !self.child_is_current(&child)
        {
            return Err(operation_not_granted());
        }
        let operation = monitoring_model::operation_document(&request.operation_ref)
            .ok_or_else(operation_not_found)?;
        if operation.protocol_driver() != ProtocolDriver::HttpV1 {
            return Err(operation_unavailable());
        }
        let connection = ConnectionAuthority::mediated(
            &child.connection_ref,
            InitiationPolicy::b10x_only(),
            &child.parent_connection_ref,
            &child.resource_binding,
            DomainRouteAdapter::KubernetesServiceProxyV1,
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
                    DomainRouteAdapter::KubernetesServiceProxyV1,
                ]),
                capabilities: BTreeSet::from([Capability::PrivateNetwork]),
                permission_subjects: vec![child.resource_binding.clone()],
            },
        )
        .map_err(|_| operation_not_granted())?;
        let ProtocolPlan::MediatedHttpV1(mediated) = plan.protocol() else {
            return Err(operation_unavailable());
        };
        if mediated.parent_connection != child.parent_connection_ref
            || mediated.resource_binding != child.resource_binding
            || mediated.adapter != DomainRouteAdapter::KubernetesServiceProxyV1
        {
            return Err(operation_not_granted());
        }
        let resolved = resolve(
            operation,
            TARGET_BASE,
            &request.input,
            &BTreeMap::new(),
            &[],
        )
        .map_err(|_| operation_invalid())?;
        let relative = resolved
            .request
            .url
            .strip_prefix(TARGET_BASE)
            .filter(|path| path.starts_with('/'))
            .ok_or_else(operation_not_granted)?;
        if resolved.request.method != "GET" || resolved.request.body.is_some() {
            return Err(operation_not_granted());
        }
        let client = lock(&self.state)
            .clients
            .get(&child.parent_connection_ref)
            .cloned()
            .ok_or_else(operation_not_granted)?;
        if !can_get_service(client.clone(), &child.namespace, &child.service).await?
            || !service_is_current(client.clone(), &child).await?
        {
            return Err(operation_not_granted());
        }
        if !can_proxy_service(client.clone(), &child.namespace, &child.service).await? {
            return Err(operation_not_granted());
        }
        let output = proxy_json(client, &child, relative).await?;
        Ok(OperationResult::Invoke(InvocationResult {
            operation_ref: request.operation_ref.clone(),
            output,
            connector_audit_ref: opaque_ref(
                "audit:kubernetes-service:",
                &format!(
                    "{}\0{}\0{}\0{}",
                    context.authority_snapshot_sha256(),
                    request.operation_ref,
                    child.connection_ref,
                    child.resource_binding
                ),
            ),
            execution_ref: None,
        }))
    }
}

#[async_trait]
impl ConnectorBackend for KubernetesLocalBackend {
    async fn ready(&self) -> Result<(), service::BackendReadinessError> {
        // Construction validates kubeconfig and local state. Cluster reachability is provider
        // health and remains an operation-level degradation.
        Ok(())
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
            OperationRequest::Describe(request) => {
                kubernetes_route_operation(&request.operation_ref)
                    && !self
                        .connections_for_operation(&request.operation_ref)
                        .is_empty()
            }
            OperationRequest::Invoke(request) => lock(&self.state)
                .children
                .contains_key(&request.connection_ref),
            OperationRequest::Search(_) => false,
            _ => false,
        }
    }

    fn owns_connection(&self, request: &ConnectionRequest) -> bool {
        match request {
            ConnectionRequest::CandidateSearch(request) => request.integration_ref == KUBERNETES,
            ConnectionRequest::CandidateActivate(request) => {
                self.candidates.contains_key(&request.candidate_ref)
            }
            ConnectionRequest::Describe(request) => lock(&self.state)
                .connections
                .contains_key(&request.connection_ref),
            ConnectionRequest::ObservationSearch(request) => self.observations(request).is_some(),
            ConnectionRequest::Materialize(request) => {
                self.observation(&request.observation_ref).is_some()
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
        self.check_operation_context(context)?;
        match request {
            OperationRequest::Search(search) => {
                let query = search.query.to_ascii_lowercase();
                let mut operations = kubernetes_route_operations()
                    .into_iter()
                    .filter(|operation_ref| {
                        query.is_empty()
                            || operation_ref.contains(&query)
                            || monitoring_model::provider_for_operation(operation_ref)
                                .contains(&query)
                    })
                    .filter_map(|operation_ref| self.operation_summary(operation_ref))
                    .collect::<Vec<_>>();
                operations.truncate(usize::from(search.limit));
                Ok(OperationResult::Search { operations })
            }
            OperationRequest::Describe(DescribeRequest { operation_ref })
                if kubernetes_route_operation(&operation_ref)
                    && !self.connections_for_operation(&operation_ref).is_empty() =>
            {
                self.operation_description(context, &operation_ref)
                    .map(OperationResult::Describe)
            }
            OperationRequest::Invoke(request)
                if lock(&self.state)
                    .children
                    .contains_key(&request.connection_ref) =>
            {
                self.invoke_service(context, request).await
            }
            _ => Err(operation_not_found()),
        }
    }

    async fn handle_connection(
        &self,
        context: &PrincipalContext,
        request: ConnectionRequest,
    ) -> Result<ConnectionResult, ConnectionError> {
        self.check_context(context)?;
        match request {
            ConnectionRequest::CandidateSearch(request)
                if request.integration_ref == KUBERNETES =>
            {
                Ok(ConnectionResult::CandidateSearch {
                    candidates: self.search_candidates(&request),
                })
            }
            ConnectionRequest::CandidateActivate(request)
                if self.candidates.contains_key(&request.candidate_ref) =>
            {
                self.activate(request)
                    .await
                    .map(ConnectionResult::CandidateActivate)
            }
            ConnectionRequest::Search(request) => {
                let mut connections = self.search_connections(&request.query);
                connections.truncate(usize::from(request.limit));
                Ok(ConnectionResult::Search { connections })
            }
            ConnectionRequest::Describe(request) => {
                let description = {
                    lock(&self.state)
                        .connections
                        .get(&request.connection_ref)
                        .cloned()
                };
                description
                    .map(ConnectionResult::Describe)
                    .ok_or_else(connection_not_found)
            }
            ConnectionRequest::ObservationSearch(request) => self
                .observations(&request)
                .map(|observations| ConnectionResult::ObservationSearch { observations })
                .ok_or_else(connection_not_found),
            ConnectionRequest::Materialize(request)
                if self.observation(&request.observation_ref).is_some() =>
            {
                self.materialize(&request)
                    .map(ConnectionResult::Materialize)
            }
            _ => Err(connection_not_found()),
        }
    }
}

fn candidates(kubeconfig: &Kubeconfig) -> BTreeMap<String, CandidateBinding> {
    let mut names = kubeconfig
        .contexts
        .iter()
        .map(|context| context.name.as_str())
        .collect::<Vec<_>>();
    names.sort_unstable();
    names
        .into_iter()
        .filter_map(|name| binding_for_context(kubeconfig, name))
        .map(|binding| (binding.summary.candidate_ref.clone(), binding))
        .collect()
}

fn binding_for_context(kubeconfig: &Kubeconfig, context_name: &str) -> Option<CandidateBinding> {
    if context_name.trim().is_empty()
        || context_name.len() > 256
        || context_name.chars().any(char::is_control)
    {
        return None;
    }
    let context = kubeconfig
        .contexts
        .iter()
        .find(|candidate| candidate.name == context_name)?
        .context
        .as_ref()?;
    let cluster = kubeconfig
        .clusters
        .iter()
        .find(|candidate| candidate.name == context.cluster)?
        .cluster
        .as_ref()?;
    let server = cluster.server.as_deref()?;
    let origin = url::Url::parse(server).ok()?;
    if origin.scheme() != "https"
        || origin.host_str().is_none()
        || !origin.username().is_empty()
        || origin.password().is_some()
        || !(origin.path().is_empty() || origin.path() == "/")
        || origin.query().is_some()
        || origin.fragment().is_some()
        || cluster.insecure_skip_tls_verify == Some(true)
    {
        return None;
    }
    let user = context.user.as_deref().unwrap_or("");
    let namespace = context.namespace.as_deref().unwrap_or("default");
    let auth = kubeconfig
        .auth_infos
        .iter()
        .find(|candidate| candidate.name == user)
        .and_then(|candidate| candidate.auth_info.as_ref());
    let auth_material = auth.map_or_else(String::new, |auth| {
        let mut groups = auth.impersonate_groups.clone().unwrap_or_default();
        groups.sort();
        format!(
            "{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}",
            auth.username.as_deref().unwrap_or(""),
            auth.token_file.as_deref().unwrap_or(""),
            auth.client_certificate.as_deref().unwrap_or(""),
            auth.client_key.as_deref().unwrap_or(""),
            auth.token.is_some(),
            auth.password.is_some(),
            auth.client_key_data.is_some(),
            auth.impersonate.as_deref().unwrap_or(""),
            auth.impersonate_uid.as_deref().unwrap_or(""),
            groups.join(","),
            auth.auth_provider
                .as_ref()
                .map_or("", |provider| provider.name.as_str()),
            auth.exec
                .as_ref()
                .and_then(|exec| exec.command.as_deref())
                .unwrap_or(""),
            auth.exec.is_some(),
        )
    });
    let evidence_material = digest(&format!(
        "{context_name}\0{}\0{user}\0{namespace}\0{server}\0{}\0{}\0{}\0{}\0{auth_material}",
        context.cluster,
        cluster.certificate_authority.as_deref().unwrap_or(""),
        cluster
            .certificate_authority_data
            .as_deref()
            .map_or_else(String::new, digest),
        cluster.tls_server_name.as_deref().unwrap_or(""),
        cluster.disable_compression.unwrap_or(false),
    ));
    let evidence_sha256 = evidence_material.clone();
    Some(CandidateBinding {
        summary: ConnectionCandidateSummary {
            candidate_ref: opaque_ref("candidate:kubernetes:", &evidence_material),
            integration_ref: KUBERNETES.to_owned(),
            title: context_name.to_owned(),
            state: ConnectionCandidateState::Detected,
            evidence_sha256,
            connection_ref: None,
        },
        context_name: context_name.to_owned(),
        evidence_material,
    })
}

fn context_uses_credential_plugin(kubeconfig: &Kubeconfig, context_name: &str) -> bool {
    let Some(context) = kubeconfig
        .contexts
        .iter()
        .find(|candidate| candidate.name == context_name)
        .and_then(|candidate| candidate.context.as_ref())
    else {
        return false;
    };
    context.user.as_ref().is_some_and(|user| {
        kubeconfig
            .auth_infos
            .iter()
            .find(|candidate| &candidate.name == user)
            .and_then(|candidate| candidate.auth_info.as_ref())
            .is_some_and(|auth| auth.exec.is_some() || auth.auth_provider.is_some())
    })
}

async fn verify_identity(client: Client) -> Result<(), ConnectionError> {
    let reviews: Api<SelfSubjectReview> = Api::all(client);
    let reviewed = reviews
        .create(&PostParams::default(), &SelfSubjectReview::default())
        .await
        .map_err(|_| connection_unavailable())?;
    let username = reviewed
        .status
        .and_then(|status| status.user_info)
        .and_then(|identity| identity.username);
    if username.as_deref().is_none_or(str::is_empty) {
        return Err(connection_unavailable());
    }
    Ok(())
}

async fn discover_services(
    client: Client,
    policy: &KubernetesIntegrationConfig,
) -> Result<Vec<Service>, ConnectionError> {
    let limit = u32::from(policy.resource_limit);
    if policy.namespaces.is_empty() {
        if !can_list_services(client.clone(), None).await? {
            return Err(ConnectionError::new(
                ConnectionErrorCode::NotGranted,
                "the selected Kubernetes identity cannot list Services cluster-wide",
                false,
            ));
        }
        let services: Api<Service> = Api::all(client);
        let mut items = services
            .list(&ListParams::default().limit(limit))
            .await
            .map(|list| list.items)
            .map_err(|_| connection_unavailable())?;
        items.truncate(usize::from(policy.resource_limit));
        return Ok(items);
    }

    let mut remaining = usize::from(policy.resource_limit);
    let mut discovered = Vec::new();
    for namespace in &policy.namespaces {
        if remaining == 0 {
            break;
        }
        if !can_list_services(client.clone(), Some(namespace)).await? {
            continue;
        }
        let services: Api<Service> = Api::namespaced(client.clone(), namespace);
        let mut items = services
            .list(&ListParams::default().limit(remaining as u32))
            .await
            .map_err(|_| connection_unavailable())?
            .items;
        items.truncate(remaining);
        remaining -= items.len();
        discovered.extend(items);
    }
    Ok(discovered)
}

async fn can_list_services(
    client: Client,
    namespace: Option<&str>,
) -> Result<bool, ConnectionError> {
    let reviews: Api<SelfSubjectAccessReview> = Api::all(client);
    let review = SelfSubjectAccessReview {
        spec: SelfSubjectAccessReviewSpec {
            resource_attributes: Some(ResourceAttributes {
                group: Some(String::new()),
                namespace: namespace.map(str::to_owned),
                resource: Some("services".to_owned()),
                verb: Some("list".to_owned()),
                version: Some("v1".to_owned()),
                ..ResourceAttributes::default()
            }),
            ..SelfSubjectAccessReviewSpec::default()
        },
        ..SelfSubjectAccessReview::default()
    };
    let reviewed = reviews
        .create(&PostParams::default(), &review)
        .await
        .map_err(|_| connection_unavailable())?;
    Ok(reviewed.status.is_some_and(|status| status.allowed))
}

async fn can_proxy_service(
    client: Client,
    namespace: &str,
    service: &str,
) -> Result<bool, OperationError> {
    let reviews: Api<SelfSubjectAccessReview> = Api::all(client);
    let review = SelfSubjectAccessReview {
        spec: SelfSubjectAccessReviewSpec {
            resource_attributes: Some(ResourceAttributes {
                group: Some(String::new()),
                namespace: Some(namespace.to_owned()),
                resource: Some("services".to_owned()),
                subresource: Some("proxy".to_owned()),
                name: Some(service.to_owned()),
                verb: Some("get".to_owned()),
                version: Some("v1".to_owned()),
                ..ResourceAttributes::default()
            }),
            ..SelfSubjectAccessReviewSpec::default()
        },
        ..SelfSubjectAccessReview::default()
    };
    let reviewed = reviews
        .create(&PostParams::default(), &review)
        .await
        .map_err(|_| operation_unavailable())?;
    Ok(reviewed.status.is_some_and(|status| status.allowed))
}

async fn can_get_service(
    client: Client,
    namespace: &str,
    service: &str,
) -> Result<bool, OperationError> {
    let reviews: Api<SelfSubjectAccessReview> = Api::all(client);
    let review = SelfSubjectAccessReview {
        spec: SelfSubjectAccessReviewSpec {
            resource_attributes: Some(ResourceAttributes {
                group: Some(String::new()),
                namespace: Some(namespace.to_owned()),
                resource: Some("services".to_owned()),
                name: Some(service.to_owned()),
                verb: Some("get".to_owned()),
                version: Some("v1".to_owned()),
                ..ResourceAttributes::default()
            }),
            ..SelfSubjectAccessReviewSpec::default()
        },
        ..SelfSubjectAccessReview::default()
    };
    let reviewed = reviews
        .create(&PostParams::default(), &review)
        .await
        .map_err(|_| operation_unavailable())?;
    Ok(reviewed.status.is_some_and(|status| status.allowed))
}

async fn service_is_current(
    client: Client,
    child: &KubernetesServiceConnection,
) -> Result<bool, OperationError> {
    let services: Api<Service> = Api::namespaced(client, &child.namespace);
    let current = services
        .get(&child.service)
        .await
        .map_err(|_| operation_unavailable())?;
    let Some(provider) = recognize_service(&current) else {
        return Ok(false);
    };
    let Some(uid) = current.metadata.uid.as_deref() else {
        return Ok(false);
    };
    let Some(port) = current
        .spec
        .as_ref()
        .and_then(|spec| spec.ports.as_deref())
        .and_then(|ports| select_service_port(provider, ports))
    else {
        return Ok(false);
    };
    let binding = format!(
        "{}\0{}\0{}\0{}\0{}\0{}",
        child.parent_connection_ref, child.namespace, child.service, port, uid, provider
    );
    Ok(provider == child.provider
        && uid == child.resource_uid
        && port == child.port
        && opaque_ref("binding:kubernetes-service:", &binding) == child.resource_binding)
}

async fn proxy_json(
    client: Client,
    child: &KubernetesServiceConnection,
    relative: &str,
) -> Result<Value, OperationError> {
    let route = format!(
        "/api/v1/namespaces/{}/services/{}:{}/proxy{}",
        child.namespace, child.service, child.port, relative
    );
    let request = HttpRequest::builder()
        .method(Method::GET)
        .uri(route)
        .header(http::header::ACCEPT, "application/json")
        .body(Vec::new())
        .map_err(|_| operation_invalid())?;
    let stream = client
        .request_stream(request)
        .await
        .map_err(|_| operation_unavailable())?;
    let mut bytes = Vec::new();
    stream
        .take((MAX_PROXY_RESULT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| operation_unavailable())?;
    if bytes.len() > MAX_PROXY_RESULT_BYTES {
        return Err(OperationError::new(
            OperationErrorCode::ResultTooLarge,
            "Kubernetes Service proxy result exceeded the Connector bound",
            false,
        ));
    }
    serde_json::from_slice(&bytes).map_err(|_| operation_unavailable())
}

fn normalize_services(
    source_connection_ref: &str,
    services: Vec<Service>,
) -> Vec<StoredServiceObservation> {
    let mut seen = BTreeSet::new();
    let mut observations = services
        .into_iter()
        .filter_map(|service| {
            let provider = recognize_service(&service)?;
            let namespace = service.metadata.namespace.as_deref()?;
            let name = service.metadata.name.as_deref()?;
            let uid = service.metadata.uid.as_deref()?;
            let port = select_service_port(provider, service.spec.as_ref()?.ports.as_deref()?)?;
            let binding =
                format!("{source_connection_ref}\0{namespace}\0{name}\0{port}\0{uid}\0{provider}");
            if !seen.insert(binding.clone()) {
                return None;
            }
            let title = format!("{namespace}/{name} ({provider})");
            Some(StoredServiceObservation {
                summary: DiscoveryObservationSummary {
                    observation_ref: opaque_ref("observation:kubernetes:", &binding),
                    discovery_ref: DISCOVERY_REF.to_owned(),
                    source_connection_ref: source_connection_ref.to_owned(),
                    observed_type: "kubernetes_service".to_owned(),
                    title,
                    state: DiscoveryObservationState::Observed,
                    evidence_generation: 1,
                    evidence_sha256: digest(&binding),
                    target_provider_ref: Some(provider.to_owned()),
                    connection_ref: None,
                },
                namespace: namespace.to_owned(),
                service: name.to_owned(),
                resource_uid: uid.to_owned(),
                port,
                provider: provider.to_owned(),
                resource_binding: opaque_ref("binding:kubernetes-service:", &binding),
            })
        })
        .collect::<Vec<_>>();
    observations.sort_by(|left, right| left.summary.title.cmp(&right.summary.title));
    observations
}

fn select_service_port(provider: &str, ports: &[ServicePort]) -> Option<String> {
    let candidates = ports
        .iter()
        .filter(|port| {
            port.protocol
                .as_deref()
                .is_none_or(|protocol| protocol == "TCP")
        })
        .collect::<Vec<_>>();
    let known_number = match provider {
        "grafana" => 3000,
        "prometheus" => 9090,
        "loki" => 3100,
        "alertmanager" => 9093,
        _ => return None,
    };
    let preferred = candidates
        .iter()
        .copied()
        .find(|port| port.port == known_number)
        .or_else(|| {
            candidates.iter().copied().find(|port| {
                port.name.as_deref().is_some_and(|name| {
                    matches!(
                        name,
                        "http"
                            | "http-web"
                            | "web"
                            | "service"
                            | "grafana"
                            | "prometheus"
                            | "loki"
                            | "alertmanager"
                    )
                })
            })
        })
        .or_else(|| (candidates.len() == 1).then_some(candidates[0]))?;
    preferred
        .name
        .as_ref()
        .filter(|name| valid_dns_label(name, 63))
        .cloned()
        .or_else(|| {
            (1..=65_535)
                .contains(&preferred.port)
                .then(|| preferred.port.to_string())
        })
}

fn child_is_current(state: &KubernetesState, child: &KubernetesServiceConnection) -> bool {
    state
        .observations
        .get(&child.observation_ref)
        .is_some_and(|observation| {
            observation.summary.state == DiscoveryObservationState::Materialized
                && observation.summary.connection_ref.as_deref()
                    == Some(child.connection_ref.as_str())
                && observation.resource_binding == child.resource_binding
                && observation.summary.source_connection_ref == child.parent_connection_ref
                && state.clients.contains_key(&child.parent_connection_ref)
        })
}

fn valid_dns_label(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && value
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn recognize_service(service: &Service) -> Option<&'static str> {
    let name = service
        .metadata
        .name
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase();
    let identity_labels = service
        .metadata
        .labels
        .as_ref()
        .into_iter()
        .flat_map(|labels| {
            ["app.kubernetes.io/name", "app", "k8s-app", "name"]
                .into_iter()
                .filter_map(|key| labels.get(key))
                .map(|value| value.to_ascii_lowercase())
        });
    let haystack = std::iter::once(name)
        .chain(identity_labels)
        .collect::<Vec<_>>()
        .join(" ");
    if haystack.contains("grafana") {
        Some("grafana")
    } else if haystack.contains("alertmanager") {
        Some("alertmanager")
    } else if haystack.contains("loki") {
        Some("loki")
    } else if haystack.contains("prometheus") {
        Some("prometheus")
    } else {
        None
    }
}

fn kubernetes_route_operations() -> [&'static str; 3] {
    [
        monitoring_model::PROMETHEUS_QUERY_RANGE,
        monitoring_model::LOKI_QUERY_RANGE,
        monitoring_model::ALERTMANAGER_ALERTS_LIST,
    ]
}

fn kubernetes_route_operation(operation_ref: &str) -> bool {
    kubernetes_route_operations().contains(&operation_ref)
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

fn opaque_ref(prefix: &str, value: &str) -> String {
    format!("{prefix}{}", &digest(value)[..32])
}

fn digest(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn connection_not_found() -> ConnectionError {
    ConnectionError::new(
        ConnectionErrorCode::NotFound,
        "connection candidate was not found",
        false,
    )
}

fn connection_unavailable() -> ConnectionError {
    ConnectionError::new(
        ConnectionErrorCode::Unavailable,
        "the selected Kubernetes context could not be verified",
        true,
    )
}

fn connection_protocol() -> ConnectionError {
    ConnectionError::new(
        ConnectionErrorCode::Protocol,
        "the Connection backend returned an invalid response",
        false,
    )
}

fn operation_not_found() -> OperationError {
    OperationError::new(
        OperationErrorCode::NotFound,
        "Kubernetes-mediated monitoring operation was not found",
        false,
    )
}

fn operation_not_granted() -> OperationError {
    OperationError::new(
        OperationErrorCode::NotGranted,
        "operation is not granted for this Kubernetes Service Connection",
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

fn operation_unavailable() -> OperationError {
    OperationError::new(
        OperationErrorCode::Unavailable,
        "Kubernetes Service route is unavailable",
        true,
    )
}

include!("local_tests.rs");

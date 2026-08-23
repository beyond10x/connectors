//! Trusted personal-local kubeconfig discovery and bounded Kubernetes monitoring discovery.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use crate::local_services::{
    can_get_service, can_proxy_service, discover_services, normalize_services, proxy_json,
    service_is_current, verify_identity,
};
use crate::local_workloads::{KubeconfigReader, WorkloadSurface};
use crate::workloads::{
    restart_operation, status_operation, DeploymentInput, DeploymentReader as _, RestartInput,
    RESTART_OPERATION, STATUS_OPERATION,
};
use async_trait::async_trait;
use connector_resolve::document::ProtocolDriver;
use connector_resolve::resolve;
use domain::{
    AdmittedOperation, Capability, ConnectionAuthority, DriverId, InitiationPolicy, ProtocolPlan,
    RouteAdapter as DomainRouteAdapter,
};
use k8s_openapi::api::core::v1::Service;
use kube::config::{KubeConfigOptions, Kubeconfig};
use kube::{Client, Config};
use protocol::connection::{
    CandidateActivateRequest, CandidateSearchRequest, ConnectionCandidateState,
    ConnectionCandidateSummary, ConnectionDescription, ConnectionError, ConnectionErrorCode,
    ConnectionInitiator, ConnectionRequest, ConnectionResult, ConnectionRoute, ConnectionState,
    ConnectionSummary, DiscoveryObservationState, DiscoveryObservationSummary, MaterializeRequest,
    ObservationSearchRequest, RouteAdapter,
};
use protocol::datasource::{
    DatasourceError, DatasourceErrorCode, DatasourceRequest, DatasourceResult,
};
use protocol::operation::{
    ApprovalPosture, ConnectionSummary as OperationConnectionSummary, DescribeRequest, EffectClass,
    InvocationResult, InvokeRequest, OperationDescription, OperationError, OperationErrorCode,
    OperationRequest, OperationResult, OperationSummary,
};
use service::{
    plan_operation, BackendCapabilities, ConnectorBackend, PlanningEnvironment, PrincipalContext,
};
use sha2::{Digest as _, Sha256};

use connectors_config::{InitiationConfig, KubernetesIntegrationConfig};

const KUBERNETES: &str = "kubernetes";
const ARGOCD: &str = "argocd";
pub(crate) const DISCOVERY_REF: &str = "discovery:kubernetes-service-v1";
const TARGET_BASE: &str = "https://mediated-target.invalid";
pub(crate) const MAX_PROXY_RESULT_BYTES: usize = protocol::operation::MAX_RESULT_BYTES;

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
pub(crate) struct StoredServiceObservation {
    pub(crate) summary: DiscoveryObservationSummary,
    pub(crate) namespace: String,
    pub(crate) service: String,
    pub(crate) resource_uid: String,
    pub(crate) port: String,
    pub(crate) provider: String,
    pub(crate) resource_binding: String,
}

#[derive(Debug, Clone)]
pub(crate) struct KubernetesServiceConnection {
    pub(crate) connection_ref: String,
    pub(crate) label: String,
    pub(crate) provider: String,
    pub(crate) grant_ref: String,
    pub(crate) parent_connection_ref: String,
    pub(crate) observation_ref: String,
    pub(crate) namespace: String,
    pub(crate) service: String,
    pub(crate) resource_uid: String,
    pub(crate) port: String,
    pub(crate) resource_binding: String,
}

/// Personal-local backend which passively detects kubeconfig contexts, then contacts a cluster
/// only after one opaque candidate is explicitly activated.
pub struct KubernetesLocalBackend {
    owner: PrincipalContext,
    policy: KubernetesIntegrationConfig,
    candidates: BTreeMap<String, CandidateBinding>,
    state: Mutex<KubernetesState>,
    activation: tokio::sync::Mutex<()>,
    /// `kubernetes.workloads`, the same projection the deployment publishes. See
    /// `crate::local_workloads`.
    workloads: WorkloadSurface,
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
            workloads: WorkloadSurface::default(),
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

    fn check_datasource_context(&self, context: &PrincipalContext) -> Result<(), DatasourceError> {
        if context == &self.owner {
            Ok(())
        } else {
            Err(DatasourceError::new(
                DatasourceErrorCode::StaleAuthority,
                "owner context does not match this Connector generation",
                false,
            ))
        }
    }

    /// The one cluster this daemon generation has attached, if any.
    ///
    /// Personal-local activation binds one kubeconfig context at a time, so taking the first entry
    /// is the whole of the selection. When a second context is ever activatable this becomes the
    /// place that has to choose, and it will be a choice a person makes rather than a map order.
    fn attached_cluster(&self) -> Option<(String, Client)> {
        let state = lock(&self.state);
        state
            .clients
            .iter()
            .next()
            .map(|(connection_ref, client)| (connection_ref.clone(), client.clone()))
    }

    /// Namespaces this placement offers as datasource bindings.
    ///
    /// Configuration is the only source. An empty `namespaces` means cluster-wide *discovery* for
    /// Services, which is a different question from which namespaces a person may page workloads
    /// in — enumerating every namespace on a shared cluster would offer hundreds of bindings, most
    /// of which the operator's own RBAC would then refuse one read at a time.
    fn readable_namespaces(&self) -> Vec<String> {
        self.policy
            .namespaces
            .iter()
            .filter(|namespace| valid_dns_label(namespace, 253))
            .cloned()
            .collect()
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

    /// The cluster Connection itself, when it is attached.
    ///
    /// Distinct from the child Service Connections below it: those are Prometheus and Loki behind
    /// Kubernetes Services, reached by proxy. This is the cluster, and it is what
    /// `kubernetes.deployment.*` acts on.
    fn cluster_connection(&self) -> Option<(String, String)> {
        let state = lock(&self.state);
        state.clients.keys().next().and_then(|connection_ref| {
            state
                .connections
                .get(connection_ref)
                .map(|description| (connection_ref.clone(), description.summary.label.clone()))
        })
    }

    fn connections_for_operation(&self, operation_ref: &str) -> Vec<OperationConnectionSummary> {
        // The two workload operations belong to the cluster, not to a Service behind it. Publishing
        // them only for child Services is why an activated cluster admitted nothing a person could
        // call: the Connection was attached, readable as a datasource, and had no operation at all.
        if matches!(operation_ref, STATUS_OPERATION | RESTART_OPERATION) {
            return self
                .cluster_connection()
                .map(|(connection_ref, label)| OperationConnectionSummary {
                    connection_ref,
                    label,
                    provider: KUBERNETES.to_owned(),
                    audiences: vec!["operations".to_owned()],
                    purpose: None,
                })
                .into_iter()
                .collect();
        }
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
                purpose: None,
            })
            .collect()
    }

    fn child_is_current(&self, child: &KubernetesServiceConnection) -> bool {
        let state = lock(&self.state);
        child_is_current(&state, child)
    }

    fn operation_summary(&self, operation_ref: &str) -> Option<OperationSummary> {
        let connections = self.connections_for_operation(operation_ref);
        if connections.is_empty() {
            return None;
        }
        // A rollout restart changes a running deployment, so it carries the posture that says so.
        // The monitoring route operations are proxied reads and keep theirs.
        let (title, effect, approval) = match operation_ref {
            STATUS_OPERATION => (
                "Read Kubernetes deployment status",
                EffectClass::ReadOnly,
                ApprovalPosture::NotRequired,
            ),
            RESTART_OPERATION => (
                "Restart a Kubernetes Deployment rollout",
                EffectClass::Mutating,
                ApprovalPosture::Required,
            ),
            _ => (
                monitoring_model::title(operation_ref),
                EffectClass::ReadOnly,
                ApprovalPosture::NotRequired,
            ),
        };
        Some(OperationSummary {
            operation_ref: operation_ref.to_owned(),
            title: title.to_owned(),
            effect,
            approval,
            connections,
        })
    }

    fn operation_description(
        &self,
        context: &PrincipalContext,
        operation_ref: &str,
    ) -> Result<OperationDescription, OperationError> {
        let connections = self.connections_for_operation(operation_ref);
        if connections.is_empty() {
            return Err(operation_not_found());
        }
        // The two workload operations carry the contract every host mode publishes; only the
        // Connections and the lease below are this placement's.
        let description_ref = self.operation_description_ref(context, operation_ref);
        match operation_ref {
            STATUS_OPERATION => return Ok(status_operation(connections, description_ref)),
            RESTART_OPERATION => return Ok(restart_operation(connections, description_ref)),
            _ => {}
        }
        let operation =
            monitoring_model::operation_document(operation_ref).ok_or_else(operation_not_found)?;
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
            description_ref,
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
        if credential_bearing_provider(&observation.provider) {
            // The provider kind is already in the observation the caller is holding, so naming it
            // here costs nothing and is the difference between a refusal someone can act on and one
            // they have to guess at.
            return Err(ConnectionError::new(
                ConnectionErrorCode::NotGranted,
                format!(
                    "the discovered {} Service requires an explicit credential source",
                    observation.provider
                ),
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
                initiation: vec![ConnectionInitiator::Platform],
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

    /// Runs one `kubernetes.deployment.*` operation against the attached cluster.
    ///
    /// The description lease is checked first: a restart approved against one description must not
    /// be dispatched after the surface moved underneath it. That is the whole reason the two
    /// operations carry a lease rather than being callable from a bare reference.
    async fn invoke_workload(
        &self,
        context: &PrincipalContext,
        request: InvokeRequest,
    ) -> Result<OperationResult, OperationError> {
        let Some((connection_ref, _)) = self.cluster_connection() else {
            return Err(operation_not_found());
        };
        if request.connection_ref != connection_ref {
            return Err(operation_not_found());
        }
        if request.description_ref
            != self.operation_description_ref(context, &request.operation_ref)
        {
            return Err(OperationError::new(
                OperationErrorCode::StaleAuthority,
                "the Kubernetes operation description has moved on; describe it again and retry",
                true,
            ));
        }
        let client = {
            let state = lock(&self.state);
            state.clients.get(&connection_ref).cloned()
        }
        .ok_or_else(operation_unavailable)?;
        let reader = KubeconfigReader::new(client);
        let output = match request.operation_ref.as_str() {
            STATUS_OPERATION => {
                let input: DeploymentInput =
                    serde_json::from_value(request.input).map_err(|_| operation_invalid())?;
                serde_json::to_value(reader.read(&input.namespace, &input.name).await?)
                    .map_err(|_| operation_unavailable())?
            }
            RESTART_OPERATION => {
                let input: RestartInput =
                    serde_json::from_value(request.input).map_err(|_| operation_invalid())?;
                serde_json::to_value(
                    reader
                        .restart(
                            &input.namespace,
                            &input.name,
                            &input.uid,
                            &input.resource_version,
                        )
                        .await?,
                )
                .map_err(|_| operation_unavailable())?
            }
            _ => return Err(operation_not_found()),
        };
        Ok(OperationResult::Invoke(InvocationResult {
            operation_ref: request.operation_ref.clone(),
            output,
            connector_audit_ref: opaque_ref(
                "audit:kubernetes-workload:",
                &format!(
                    "{}\0{}\0{}",
                    context.authority_snapshot_sha256(),
                    request.operation_ref,
                    connection_ref
                ),
            ),
            execution_ref: None,
        }))
    }

    async fn invoke_service(
        &self,
        context: &PrincipalContext,
        request: InvokeRequest,
    ) -> Result<OperationResult, OperationError> {
        if !kubernetes_route_operation(&request.operation_ref)
            || request.description_ref
                != self.operation_description_ref(context, &request.operation_ref)
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
            InitiationPolicy::platform_only(),
            &child.parent_connection_ref,
            &child.resource_binding,
            DomainRouteAdapter::KubernetesServiceProxyV1,
        )
        .map_err(|_| operation_not_granted())?;
        let admission = AdmittedOperation::for_local_owner(
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
            // `kubernetes.workloads`, read through whichever kubeconfig context the operator
            // activated. Declared unconditionally: a placement that advertised datasources only
            // once a cluster was attached would make the surface appear and disappear under a
            // person mid-session, and the refusal for "nothing attached yet" says what to do.
            datasources: true,
        }
    }

    fn owns_operation(&self, request: &OperationRequest) -> bool {
        match request {
            OperationRequest::Describe(request) => !self
                .connections_for_operation(&request.operation_ref)
                .is_empty(),
            OperationRequest::Invoke(request) => {
                lock(&self.state)
                    .children
                    .contains_key(&request.connection_ref)
                    || (matches!(
                        request.operation_ref.as_str(),
                        STATUS_OPERATION | RESTART_OPERATION
                    ) && self
                        .cluster_connection()
                        .is_some_and(|(reference, _)| reference == request.connection_ref))
            }
            OperationRequest::Search(_) => false,
            _ => false,
        }
    }

    fn owns_datasource(&self, request: &DatasourceRequest) -> bool {
        WorkloadSurface::owns(request)
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
                let mut operations = local_operations()
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
                if !self.connections_for_operation(&operation_ref).is_empty() =>
            {
                self.operation_description(context, &operation_ref)
                    .map(OperationResult::Describe)
            }
            OperationRequest::Invoke(request)
                if matches!(
                    request.operation_ref.as_str(),
                    STATUS_OPERATION | RESTART_OPERATION
                ) =>
            {
                self.invoke_workload(context, request).await
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

    async fn handle_datasource(
        &self,
        context: &PrincipalContext,
        request: DatasourceRequest,
    ) -> Result<DatasourceResult, DatasourceError> {
        self.check_datasource_context(context)?;
        self.workloads
            .handle(
                context,
                request,
                self.attached_cluster(),
                &self.readable_namespaces(),
            )
            .await
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

pub(crate) fn valid_dns_label(value: &str, maximum: usize) -> bool {
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

/// Every Service name or identity label that names **exactly one** component, so a substring test
/// would recognize its siblings too.
///
/// A default Argo CD install ships eight Services whose names contain `argocd`, and only
/// `argocd-server` is the API. `argocd-repo-server` speaks a private gRPC protocol,
/// `argocd-server-metrics` serves Prometheus text on a different port, and `argocd-redis` is a
/// cache holding session state. `haystack.contains("argocd")` would offer all eight as Argo CD
/// Connection candidates, and `contains("argocd-server")` would still take the metrics Service —
/// which shares `app.kubernetes.io/component: server` — so this arm matches whole tokens and runs
/// before the substring arms below.
///
/// Whole-token rather than name-only because a Helm release renames the Service: `argo-cd` chart
/// installs it as `<release>-argocd-server` while keeping `app.kubernetes.io/name: argocd-server`,
/// so the label is the stable identity and the name is not.
const EXACT_IDENTITIES: [(&str, &str); 1] = [("argocd-server", "argocd")];

pub(crate) fn recognize_service(service: &Service) -> Option<&'static str> {
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
    let identities = std::iter::once(name)
        .chain(identity_labels)
        .collect::<Vec<_>>();
    if let Some((_, provider)) = EXACT_IDENTITIES
        .iter()
        .find(|(token, _)| identities.iter().any(|identity| identity == token))
    {
        return Some(provider);
    }
    let haystack = identities.join(" ");
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

/// Every operation this placement can publish: the two cluster workload operations, then the
/// proxied monitoring routes.
fn local_operations() -> Vec<&'static str> {
    let mut operations = vec![STATUS_OPERATION, RESTART_OPERATION];
    operations.extend(kubernetes_route_operations());
    operations
}

/// Whether a recognized provider's own API refuses an unauthenticated request, so this placement
/// cannot open a Connection to it.
///
/// **A mediated call carries no credential.** `invoke_service` resolves the request with an empty
/// credential slice, because the Kubernetes Service proxy is the whole of the authority it has: the
/// caller's cluster identity gets the request to the Service, and nothing after that speaks for the
/// caller to the Service's own API. That is exactly right for Prometheus, Loki and Alertmanager,
/// which are ordinarily deployed inside a cluster with no authentication of their own.
///
/// It is exactly wrong for the two below. Grafana wants a service-account token; Argo CD wants an
/// account or project-role JWT on every `/api/v1` request and answers 401 without one. Materializing
/// either as a mediated Connection would publish something that looks callable and returns 401 on
/// first use, so discovery stops at the observation: it tells the operator that this cluster runs
/// one, and the Connection is opened directly against `providers/<id>.toml` with a token the
/// operator supplies.
///
/// Giving this placement a credential source is real work rather than an oversight — it needs a
/// custody surface, a config vocabulary, and a credential threaded into the mediated route for every
/// provider at once, which would also let the discovered Grafana finally materialize. Design 10's
/// 2026-08-20 amendment records it as the open follow-up.
fn credential_bearing_provider(provider: &str) -> bool {
    matches!(provider, monitoring_model::GRAFANA | ARGOCD)
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
        InitiationConfig::Platform => vec![ConnectionInitiator::Platform],
        InitiationConfig::Provider => vec![ConnectionInitiator::Provider],
        InitiationConfig::Both => {
            vec![ConnectionInitiator::Platform, ConnectionInitiator::Provider]
        }
    }
}

pub(crate) fn opaque_ref(prefix: &str, value: &str) -> String {
    format!("{prefix}{}", &digest(value)[..32])
}

pub(crate) fn digest(value: &str) -> String {
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

pub(crate) fn connection_unavailable() -> ConnectionError {
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

pub(crate) fn operation_invalid() -> OperationError {
    OperationError::new(
        OperationErrorCode::InvalidInput,
        "operation input is outside the catalog contract",
        false,
    )
}

pub(crate) fn operation_unavailable() -> OperationError {
    OperationError::new(
        OperationErrorCode::Unavailable,
        "Kubernetes Service route is unavailable",
        true,
    )
}

include!("local_tests.rs");

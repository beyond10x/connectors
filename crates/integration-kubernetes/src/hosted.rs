//! Deployment-owned, policy-scoped Kubernetes workload Integration.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use connectors_config::KubernetesNamespaceAccessConfig;
use protocol::connection::{
    ConnectionDescription as ControlConnectionDescription, ConnectionError, ConnectionErrorCode,
    ConnectionInitiator, ConnectionRequest, ConnectionResult, ConnectionRoute, ConnectionState,
    ConnectionSummary as ControlConnectionSummary, DescribeRequest as ConnectionDescribeRequest,
};
use protocol::datasource::{
    AccessMode, BindingSearchRequest, Completeness, DatasourceBinding, DatasourceDescription,
    DatasourceError, DatasourceErrorCode, DatasourcePage, DatasourceProvenance, DatasourceRead,
    DatasourceRecord, DatasourceRequest, DatasourceResult, DatasourceSummary,
    DescribeRequest as DatasourceDescribeRequest, ReadRequest, ReadVerb, RecordView,
    SearchRequest as DatasourceSearchRequest,
};
use protocol::operation::{
    ApprovalPosture, ConnectionSummary, DescribeRequest, EffectClass, InvocationResult,
    InvokeRequest, OperationDescription, OperationError, OperationErrorCode, OperationRequest,
    OperationResult, OperationSummary,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest as _, Sha256};
use url::Url;
use zeroize::Zeroizing;

use service::{BackendCapabilities, ConnectorBackend, PrincipalContext};

mod datasource;

#[cfg(test)]
use datasource::namespace_binding;

const STATUS_OPERATION: &str = "kubernetes.deployment.status";
const RESTART_OPERATION: &str = "kubernetes.deployment.rollout-restart";
const DATASOURCE: &str = "kubernetes.workloads";
const CONNECTION: &str = "connection:kubernetes:in-cluster";
const MAX_KUBERNETES_RESPONSE_BYTES: usize = 256 * 1024;
const MAX_RELATED_RECORDS: usize = 50;
const CURSOR_TTL: Duration = Duration::from_secs(300);

#[derive(Debug, thiserror::Error)]
pub enum KubernetesBackendError {
    #[error("Kubernetes Integration tenant or namespace policy is invalid")]
    InvalidPolicy,
    #[error("Kubernetes in-cluster service environment is unavailable")]
    MissingService,
    #[error("Kubernetes service-account trust material could not be read")]
    TrustMaterial,
    #[error("Kubernetes HTTPS client could not be configured")]
    HttpClient,
}

pub struct KubernetesStatusBackend {
    expected_tenant: String,
    namespace_access: BTreeMap<String, NamespaceAccess>,
    operator_groups: BTreeSet<String>,
    reader: Arc<dyn DeploymentReader>,
    cursors: Mutex<BTreeMap<String, CursorState>>,
}

#[derive(Clone)]
struct NamespaceAccess {
    read_groups: BTreeSet<String>,
    restart_groups: BTreeSet<String>,
}

struct CursorState {
    namespace: String,
    principal_subject: String,
    authority_snapshot_sha256: String,
    provider_cursor: String,
    expires_at: SystemTime,
}

#[async_trait]
trait DeploymentReader: Send + Sync {
    async fn read(&self, namespace: &str, name: &str) -> Result<DeploymentStatus, OperationError>;

    async fn list_workloads(
        &self,
        _namespace: &str,
        _limit: u16,
        _cursor: Option<&str>,
    ) -> Result<WorkloadList, DatasourceError> {
        Err(datasource_unavailable(
            "Kubernetes workload listing is unavailable",
        ))
    }

    async fn workload_detail(
        &self,
        _namespace: &str,
        _name: &str,
    ) -> Result<WorkloadDetail, DatasourceError> {
        Err(datasource_unavailable(
            "Kubernetes workload detail is unavailable",
        ))
    }

    async fn restart(
        &self,
        _namespace: &str,
        _name: &str,
        _uid: &str,
        _resource_version: &str,
    ) -> Result<RestartAccepted, OperationError> {
        Err(unavailable("Kubernetes rollout restart is unavailable"))
    }
}

struct InClusterReader {
    client: reqwest::Client,
    base: Url,
    token_file: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
struct DeploymentStatus {
    namespace: String,
    name: String,
    generation: i64,
    observed_generation: i64,
    desired_replicas: i32,
    ready_replicas: i32,
    available_replicas: i32,
    updated_replicas: i32,
    running: bool,
    available_condition: bool,
}

#[derive(Debug, Clone, Serialize)]
struct WorkloadCompact {
    namespace: String,
    name: String,
    uid: String,
    resource_version: String,
    generation: i64,
    observed_generation: i64,
    desired_replicas: i32,
    updated_replicas: i32,
    ready_replicas: i32,
    available_replicas: i32,
    unavailable_replicas: i32,
    rollout_state: String,
}

struct WorkloadList {
    workloads: Vec<WorkloadCompact>,
    next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct WorkloadDetail {
    #[serde(flatten)]
    workload: WorkloadCompact,
    pods: Vec<PodSummary>,
    warnings: Vec<WarningSummary>,
    related_complete: bool,
}

#[derive(Debug, Clone, Serialize)]
struct PodSummary {
    name: String,
    phase: String,
    ready_containers: u16,
    total_containers: u16,
    restart_count: u32,
    containers: Vec<ContainerSummary>,
}

#[derive(Debug, Clone, Serialize)]
struct ContainerSummary {
    name: String,
    image: String,
    image_id: String,
    ready: bool,
    restart_count: u32,
    state_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct WarningSummary {
    involved_kind: String,
    involved_name: String,
    reason: String,
    count: i32,
    first_observed_at: Option<String>,
    last_observed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct RestartAccepted {
    namespace: String,
    name: String,
    uid: String,
    resource_version: String,
    patch_accepted: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeploymentInput {
    namespace: String,
    name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RestartInput {
    namespace: String,
    name: String,
    uid: String,
    resource_version: String,
}

#[derive(Deserialize)]
struct KubernetesDeployment {
    metadata: KubernetesMetadata,
    #[serde(default)]
    spec: KubernetesDeploymentSpec,
    #[serde(default)]
    status: KubernetesDeploymentState,
}

#[derive(Deserialize)]
struct KubernetesMetadata {
    name: String,
    namespace: String,
    #[serde(default)]
    uid: String,
    #[serde(default, rename = "resourceVersion")]
    resource_version: String,
    #[serde(default)]
    generation: i64,
}

#[derive(Default, Deserialize)]
struct KubernetesDeploymentSpec {
    #[serde(default = "one_replica")]
    replicas: i32,
    #[serde(default)]
    selector: KubernetesLabelSelector,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KubernetesLabelSelector {
    #[serde(default)]
    match_labels: BTreeMap<String, String>,
}

const fn one_replica() -> i32 {
    1
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KubernetesDeploymentState {
    #[serde(default)]
    observed_generation: i64,
    #[serde(default)]
    ready_replicas: i32,
    #[serde(default)]
    available_replicas: i32,
    #[serde(default)]
    updated_replicas: i32,
    #[serde(default)]
    unavailable_replicas: i32,
    #[serde(default)]
    conditions: Vec<KubernetesCondition>,
}

#[derive(Deserialize)]
struct KubernetesList<T> {
    #[serde(default)]
    metadata: KubernetesListMetadata,
    items: Vec<T>,
}

#[derive(Default, Deserialize)]
struct KubernetesListMetadata {
    #[serde(default, rename = "continue")]
    continue_token: String,
}

#[derive(Deserialize)]
struct KubernetesPod {
    metadata: KubernetesMetadata,
    #[serde(default)]
    spec: KubernetesPodSpec,
    #[serde(default)]
    status: KubernetesPodStatus,
}

#[derive(Default, Deserialize)]
struct KubernetesPodSpec {
    #[serde(default)]
    containers: Vec<KubernetesContainer>,
}

#[derive(Deserialize)]
struct KubernetesContainer {
    name: String,
    image: String,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KubernetesPodStatus {
    #[serde(default)]
    phase: String,
    #[serde(default)]
    container_statuses: Vec<KubernetesContainerStatus>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct KubernetesContainerStatus {
    name: String,
    #[serde(default)]
    image: String,
    #[serde(default)]
    image_id: String,
    #[serde(default)]
    ready: bool,
    #[serde(default)]
    restart_count: u32,
    #[serde(default)]
    state: BTreeMap<String, KubernetesContainerState>,
}

#[derive(Default, Deserialize)]
struct KubernetesContainerState {
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct KubernetesEvent {
    involved_object: KubernetesObjectReference,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    count: i32,
    #[serde(default)]
    first_timestamp: Option<String>,
    #[serde(default)]
    last_timestamp: Option<String>,
}

#[derive(Deserialize)]
struct KubernetesObjectReference {
    #[serde(default)]
    kind: String,
    #[serde(default)]
    name: String,
}

#[derive(Deserialize)]
struct KubernetesCondition {
    #[serde(rename = "type")]
    kind: String,
    status: String,
}

impl KubernetesStatusBackend {
    pub fn in_cluster(
        expected_tenant: String,
        namespace_access: Vec<KubernetesNamespaceAccessConfig>,
        operator_groups: Vec<String>,
        token_file: impl Into<PathBuf>,
        ca_file: &Path,
    ) -> Result<Self, KubernetesBackendError> {
        let (namespace_access, operator_groups) =
            validate_policy(&expected_tenant, namespace_access, operator_groups)?;
        let host = env::var("KUBERNETES_SERVICE_HOST")
            .map_err(|_| KubernetesBackendError::MissingService)?;
        let port = env::var("KUBERNETES_SERVICE_PORT_HTTPS")
            .unwrap_or_else(|_| "443".to_owned())
            .parse::<u16>()
            .map_err(|_| KubernetesBackendError::MissingService)?;
        let mut base = Url::parse("https://kubernetes.default.svc/")
            .map_err(|_| KubernetesBackendError::MissingService)?;
        base.set_host(Some(&host))
            .map_err(|_| KubernetesBackendError::MissingService)?;
        base.set_port(Some(port))
            .map_err(|_| KubernetesBackendError::MissingService)?;
        let ca = fs::read(ca_file).map_err(|_| KubernetesBackendError::TrustMaterial)?;
        let certificate = reqwest::Certificate::from_pem(&ca)
            .map_err(|_| KubernetesBackendError::TrustMaterial)?;
        let client = reqwest::Client::builder()
            .https_only(true)
            .add_root_certificate(certificate)
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| KubernetesBackendError::HttpClient)?;
        Ok(Self {
            expected_tenant,
            namespace_access,
            operator_groups,
            reader: Arc::new(InClusterReader {
                client,
                base,
                token_file: token_file.into(),
            }),
            cursors: Mutex::new(BTreeMap::new()),
        })
    }

    #[cfg(test)]
    fn with_reader(
        expected_tenant: String,
        namespace_access: Vec<KubernetesNamespaceAccessConfig>,
        operator_groups: Vec<String>,
        reader: Arc<dyn DeploymentReader>,
    ) -> Result<Self, KubernetesBackendError> {
        let (namespace_access, operator_groups) =
            validate_policy(&expected_tenant, namespace_access, operator_groups)?;
        Ok(Self {
            namespace_access,
            operator_groups,
            expected_tenant,
            reader,
            cursors: Mutex::new(BTreeMap::new()),
        })
    }

    fn require_owner(&self, context: &PrincipalContext) -> Result<(), OperationError> {
        if context.tenant_id() == self.expected_tenant {
            Ok(())
        } else {
            Err(not_granted("Connector tenant binding refused the request"))
        }
    }

    fn require_connection_owner(&self, context: &PrincipalContext) -> Result<(), ConnectionError> {
        if context.tenant_id() == self.expected_tenant {
            Ok(())
        } else {
            Err(ConnectionError::new(
                ConnectionErrorCode::NotGranted,
                "Connector tenant binding refused the request",
                false,
            ))
        }
    }

    fn is_operator(&self, context: &PrincipalContext) -> bool {
        !self.operator_groups.is_empty()
            && !self.operator_groups.is_disjoint(context.verified_groups())
    }

    fn can_read(&self, context: &PrincipalContext, namespace: &str) -> bool {
        self.namespace_access.get(namespace).is_some_and(|access| {
            self.is_operator(context) || !access.read_groups.is_disjoint(context.verified_groups())
        })
    }

    fn can_restart(&self, context: &PrincipalContext, namespace: &str) -> bool {
        self.namespace_access.get(namespace).is_some_and(|access| {
            self.is_operator(context)
                || !access.restart_groups.is_disjoint(context.verified_groups())
        })
    }

    fn readable_namespaces(&self, context: &PrincipalContext) -> Vec<&str> {
        self.namespace_access
            .keys()
            .filter(|namespace| self.can_read(context, namespace))
            .map(String::as_str)
            .collect()
    }

    fn has_read_access(&self, context: &PrincipalContext) -> bool {
        self.namespace_access
            .keys()
            .any(|namespace| self.can_read(context, namespace))
    }

    fn has_restart_access(&self, context: &PrincipalContext) -> bool {
        self.namespace_access
            .keys()
            .any(|namespace| self.can_restart(context, namespace))
    }

    fn description(
        &self,
        context: &PrincipalContext,
        operation_ref: &str,
    ) -> Result<OperationDescription, OperationError> {
        if operation_ref == STATUS_OPERATION && self.has_read_access(context) {
            return Ok(OperationDescription {
            operation_ref: STATUS_OPERATION.to_owned(),
            title: "Read Kubernetes deployment status".to_owned(),
            description: "Reads the observed replica and Available condition of one admitted Kubernetes Deployment. It cannot read Secrets or mutate cluster state.".to_owned(),
            input_schema: json!({
                "type": "object",
                "additionalProperties": false,
                "required": ["namespace", "name"],
                "properties": {
                    "namespace": {"type": "string", "minLength": 1, "maxLength": 63},
                    "name": {"type": "string", "minLength": 1, "maxLength": 253}
                }
            }),
            output_schema: json!({
                "type": "object",
                "additionalProperties": false,
                "required": ["namespace", "name", "generation", "observed_generation", "desired_replicas", "ready_replicas", "available_replicas", "updated_replicas", "running", "available_condition"],
                "properties": {
                    "namespace": {"type": "string"}, "name": {"type": "string"},
                    "generation": {"type": "integer"}, "observed_generation": {"type": "integer"},
                    "desired_replicas": {"type": "integer"}, "ready_replicas": {"type": "integer"},
                    "available_replicas": {"type": "integer"}, "updated_replicas": {"type": "integer"},
                    "running": {"type": "boolean"}, "available_condition": {"type": "boolean"}
                }
            }),
            effect: EffectClass::ReadOnly,
            approval: ApprovalPosture::NotRequired,
            connections: vec![connection()],
            description_ref: description_ref(context, STATUS_OPERATION),
        });
        }
        if operation_ref == RESTART_OPERATION && self.has_restart_access(context) {
            return Ok(OperationDescription {
                operation_ref: RESTART_OPERATION.to_owned(),
                title: "Restart a Kubernetes Deployment rollout".to_owned(),
                description: "Patches the admitted Deployment pod-template restart annotation after exact human approval. It never deletes Pods and reports success when Kubernetes accepts the patch.".to_owned(),
                input_schema: json!({
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["namespace", "name", "uid", "resource_version"],
                    "properties": {
                        "namespace": {"type": "string", "minLength": 1, "maxLength": 63},
                        "name": {"type": "string", "minLength": 1, "maxLength": 253},
                        "uid": {"type": "string", "minLength": 1, "maxLength": 128},
                        "resource_version": {"type": "string", "minLength": 1, "maxLength": 128}
                    }
                }),
                output_schema: json!({
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["namespace", "name", "uid", "resource_version", "patch_accepted"],
                    "properties": {
                        "namespace": {"type": "string"}, "name": {"type": "string"},
                        "uid": {"type": "string"}, "resource_version": {"type": "string"},
                        "patch_accepted": {"const": true}
                    }
                }),
                effect: EffectClass::Mutating,
                approval: ApprovalPosture::Required,
                connections: vec![connection()],
                description_ref: description_ref(context, RESTART_OPERATION),
            });
        }
        Err(not_granted(
            "Kubernetes operation is not granted to this principal",
        ))
    }

    async fn invoke(
        &self,
        context: &PrincipalContext,
        request: InvokeRequest,
    ) -> Result<OperationResult, OperationError> {
        if request.connection_ref != CONNECTION {
            return Err(not_granted(
                "Kubernetes operation authority is stale or not granted",
            ));
        }
        match request.operation_ref.as_str() {
            STATUS_OPERATION => {
                if request.description_ref != description_ref(context, STATUS_OPERATION)
                    || request.approval_evidence_ref.is_some()
                {
                    return Err(stale("Kubernetes status authority is stale"));
                }
                let input: DeploymentInput = serde_json::from_value(request.input)
                    .map_err(|_| invalid("Kubernetes deployment input is invalid"))?;
                if !self.can_read(context, &input.namespace) || !valid_dns_label(&input.name, 253) {
                    return Err(not_granted(
                        "Kubernetes namespace or deployment is not admitted",
                    ));
                }
                let status = self.reader.read(&input.namespace, &input.name).await?;
                let output = serde_json::to_value(status)
                    .map_err(|_| unavailable("Kubernetes status could not be encoded"))?;
                Ok(OperationResult::Invoke(InvocationResult {
                    operation_ref: STATUS_OPERATION.to_owned(),
                    output,
                    connector_audit_ref: audit_ref(
                        context,
                        STATUS_OPERATION,
                        &input.namespace,
                        &input.name,
                    ),
                    execution_ref: None,
                }))
            }
            RESTART_OPERATION => {
                if request.description_ref != description_ref(context, RESTART_OPERATION) {
                    return Err(stale("Kubernetes restart authority is stale"));
                }
                if request.approval_evidence_ref.is_none() {
                    return Err(OperationError::new(
                        OperationErrorCode::ApprovalRequired,
                        "Kubernetes rollout restart requires fresh human approval",
                        false,
                    ));
                }
                let input: RestartInput = serde_json::from_value(request.input)
                    .map_err(|_| invalid("Kubernetes restart input is invalid"))?;
                if !self.can_restart(context, &input.namespace)
                    || !valid_dns_label(&input.name, 253)
                    || !valid_ref(&input.uid, 128)
                    || !valid_ref(&input.resource_version, 128)
                {
                    return Err(not_granted("Kubernetes restart target is not admitted"));
                }
                let accepted = self
                    .reader
                    .restart(
                        &input.namespace,
                        &input.name,
                        &input.uid,
                        &input.resource_version,
                    )
                    .await?;
                let output = serde_json::to_value(accepted)
                    .map_err(|_| unavailable("Kubernetes restart result could not be encoded"))?;
                Ok(OperationResult::Invoke(InvocationResult {
                    operation_ref: RESTART_OPERATION.to_owned(),
                    output,
                    connector_audit_ref: audit_ref(
                        context,
                        RESTART_OPERATION,
                        &input.namespace,
                        &input.name,
                    ),
                    execution_ref: None,
                }))
            }
            _ => Err(OperationError::new(
                OperationErrorCode::NotFound,
                "Kubernetes Integration operation was not found",
                false,
            )),
        }
    }
}

#[async_trait]
impl DeploymentReader for InClusterReader {
    async fn read(&self, namespace: &str, name: &str) -> Result<DeploymentStatus, OperationError> {
        let mut endpoint = self.base.clone();
        endpoint
            .path_segments_mut()
            .map_err(|_| unavailable("Kubernetes API endpoint is invalid"))?
            .extend([
                "apis",
                "apps",
                "v1",
                "namespaces",
                namespace,
                "deployments",
                name,
            ]);
        let token = Zeroizing::new(
            fs::read_to_string(&self.token_file)
                .map_err(|_| unavailable("Kubernetes workload identity is unavailable"))?,
        );
        if token.is_empty() || token.len() > 64 * 1024 {
            return Err(unavailable("Kubernetes workload identity is invalid"));
        }
        let response = self
            .client
            .get(endpoint)
            .bearer_auth(token.trim())
            .send()
            .await
            .map_err(|_| unavailable("Kubernetes API request failed"))?;
        match response.status() {
            reqwest::StatusCode::NOT_FOUND => {
                return Err(OperationError::new(
                    OperationErrorCode::NotFound,
                    "Kubernetes Deployment was not found",
                    false,
                ));
            }
            reqwest::StatusCode::FORBIDDEN | reqwest::StatusCode::UNAUTHORIZED => {
                return Err(not_granted("Kubernetes workload identity refused the read"));
            }
            status if !status.is_success() => {
                return Err(unavailable(
                    "Kubernetes API returned a non-success response",
                ));
            }
            _ => {}
        }
        if response.content_length().is_some_and(|length| {
            length > u64::try_from(MAX_KUBERNETES_RESPONSE_BYTES).expect("bound fits u64")
        }) {
            return Err(OperationError::new(
                OperationErrorCode::ResultTooLarge,
                "Kubernetes status exceeds the result bound",
                false,
            ));
        }
        let body = response
            .bytes()
            .await
            .map_err(|_| unavailable("Kubernetes response could not be read"))?;
        if body.len() > MAX_KUBERNETES_RESPONSE_BYTES {
            return Err(OperationError::new(
                OperationErrorCode::ResultTooLarge,
                "Kubernetes status exceeds the result bound",
                false,
            ));
        }
        let deployment: KubernetesDeployment = serde_json::from_slice(&body)
            .map_err(|_| unavailable("Kubernetes response is malformed"))?;
        if deployment.metadata.namespace != namespace || deployment.metadata.name != name {
            return Err(unavailable("Kubernetes returned a different Deployment"));
        }
        Ok(project(deployment))
    }

    async fn list_workloads(
        &self,
        namespace: &str,
        limit: u16,
        cursor: Option<&str>,
    ) -> Result<WorkloadList, DatasourceError> {
        let mut endpoint =
            self.api_url(&["apis", "apps", "v1", "namespaces", namespace, "deployments"])?;
        {
            let mut query = endpoint.query_pairs_mut();
            query.append_pair("limit", &limit.to_string());
            if let Some(cursor) = cursor {
                query.append_pair("continue", cursor);
            }
        }
        let list: KubernetesList<KubernetesDeployment> = self.get_json(endpoint).await?;
        let workloads = list.items.into_iter().map(project_compact).collect();
        Ok(WorkloadList {
            workloads,
            next_cursor: (!list.metadata.continue_token.is_empty())
                .then_some(list.metadata.continue_token),
        })
    }

    async fn workload_detail(
        &self,
        namespace: &str,
        name: &str,
    ) -> Result<WorkloadDetail, DatasourceError> {
        let deployment_endpoint = self.api_url(&[
            "apis",
            "apps",
            "v1",
            "namespaces",
            namespace,
            "deployments",
            name,
        ])?;
        let deployment: KubernetesDeployment = self.get_json(deployment_endpoint).await?;
        if deployment.metadata.namespace != namespace || deployment.metadata.name != name {
            return Err(datasource_unavailable(
                "Kubernetes returned a different Deployment",
            ));
        }

        let selector = deployment
            .spec
            .selector
            .match_labels
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(",");
        let mut related_complete = !selector.is_empty();
        let pods = if selector.is_empty() {
            Vec::new()
        } else {
            let mut endpoint = self.api_url(&["api", "v1", "namespaces", namespace, "pods"])?;
            {
                let mut query = endpoint.query_pairs_mut();
                query.append_pair("labelSelector", &selector);
                query.append_pair("limit", &(MAX_RELATED_RECORDS + 1).to_string());
            }
            let list: KubernetesList<KubernetesPod> = self.get_json(endpoint).await?;
            related_complete &=
                list.metadata.continue_token.is_empty() && list.items.len() <= MAX_RELATED_RECORDS;
            list.items
                .into_iter()
                .take(MAX_RELATED_RECORDS)
                .map(project_pod)
                .collect()
        };

        let mut event_endpoint = self.api_url(&["api", "v1", "namespaces", namespace, "events"])?;
        {
            let mut query = event_endpoint.query_pairs_mut();
            query.append_pair(
                "fieldSelector",
                &format!("type=Warning,involvedObject.kind=Deployment,involvedObject.name={name}"),
            );
            query.append_pair("limit", &(MAX_RELATED_RECORDS + 1).to_string());
        }
        let events: KubernetesList<KubernetesEvent> = self.get_json(event_endpoint).await?;
        related_complete &=
            events.metadata.continue_token.is_empty() && events.items.len() <= MAX_RELATED_RECORDS;
        let warnings = events
            .items
            .into_iter()
            .take(MAX_RELATED_RECORDS)
            .filter(|event| safe_event_reason(&event.reason))
            .map(|event| WarningSummary {
                involved_kind: event.involved_object.kind,
                involved_name: event.involved_object.name,
                reason: event.reason,
                count: event.count,
                first_observed_at: event.first_timestamp,
                last_observed_at: event.last_timestamp,
            })
            .collect();

        Ok(WorkloadDetail {
            workload: project_compact(deployment),
            pods,
            warnings,
            related_complete,
        })
    }

    async fn restart(
        &self,
        namespace: &str,
        name: &str,
        uid: &str,
        resource_version: &str,
    ) -> Result<RestartAccepted, OperationError> {
        let endpoint = self
            .api_url(&[
                "apis",
                "apps",
                "v1",
                "namespaces",
                namespace,
                "deployments",
                name,
            ])
            .map_err(|_| unavailable("Kubernetes API endpoint is invalid"))?;
        let token = self
            .token()
            .map_err(|_| unavailable("Kubernetes workload identity is unavailable"))?;
        let response = self
            .client
            .patch(endpoint)
            .bearer_auth(token.trim())
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/strategic-merge-patch+json",
            )
            .json(&json!({
                "metadata": {"uid": uid, "resourceVersion": resource_version},
                "spec": {"template": {"metadata": {"annotations": {
                    "kubectl.kubernetes.io/restartedAt": now_unix_ms().to_string()
                }}}}
            }))
            .send()
            .await
            .map_err(|_| {
                OperationError::new(
                    OperationErrorCode::OutcomeUnknown,
                    "Kubernetes rollout restart outcome is unknown",
                    false,
                )
            })?;
        match response.status() {
            reqwest::StatusCode::CONFLICT | reqwest::StatusCode::UNPROCESSABLE_ENTITY => {
                return Err(stale(
                    "Kubernetes Deployment changed before the restart patch",
                ));
            }
            reqwest::StatusCode::NOT_FOUND => {
                return Err(OperationError::new(
                    OperationErrorCode::NotFound,
                    "Kubernetes Deployment was not found",
                    false,
                ));
            }
            reqwest::StatusCode::FORBIDDEN | reqwest::StatusCode::UNAUTHORIZED => {
                return Err(not_granted(
                    "Kubernetes workload identity refused the restart",
                ));
            }
            status if !status.is_success() => {
                return Err(outcome_unknown(
                    "Kubernetes rollout restart outcome is unknown after dispatch",
                ));
            }
            _ => {}
        }
        let body = bounded_body(response).await.map_err(|_| {
            outcome_unknown("Kubernetes restart response is invalid after dispatch")
        })?;
        let deployment: KubernetesDeployment = serde_json::from_slice(&body).map_err(|_| {
            outcome_unknown("Kubernetes restart response is malformed after dispatch")
        })?;
        if deployment.metadata.namespace != namespace
            || deployment.metadata.name != name
            || deployment.metadata.uid != uid
            || deployment.metadata.resource_version.is_empty()
        {
            return Err(OperationError::new(
                OperationErrorCode::OutcomeUnknown,
                "Kubernetes accepted a restart but returned unexpected authority",
                false,
            ));
        }
        Ok(RestartAccepted {
            namespace: namespace.to_owned(),
            name: name.to_owned(),
            uid: uid.to_owned(),
            resource_version: deployment.metadata.resource_version,
            patch_accepted: true,
        })
    }
}

impl InClusterReader {
    fn api_url(&self, segments: &[&str]) -> Result<Url, DatasourceError> {
        let mut endpoint = self.base.clone();
        endpoint
            .path_segments_mut()
            .map_err(|_| datasource_unavailable("Kubernetes API endpoint is invalid"))?
            .extend(segments);
        Ok(endpoint)
    }

    fn token(&self) -> Result<Zeroizing<String>, DatasourceError> {
        let token =
            Zeroizing::new(fs::read_to_string(&self.token_file).map_err(|_| {
                datasource_unavailable("Kubernetes workload identity is unavailable")
            })?);
        if token.is_empty() || token.len() > 64 * 1024 {
            return Err(datasource_unavailable(
                "Kubernetes workload identity is invalid",
            ));
        }
        Ok(token)
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(
        &self,
        endpoint: Url,
    ) -> Result<T, DatasourceError> {
        let token = self.token()?;
        let response = self
            .client
            .get(endpoint)
            .bearer_auth(token.trim())
            .send()
            .await
            .map_err(|_| datasource_unavailable("Kubernetes API request failed"))?;
        match response.status() {
            reqwest::StatusCode::NOT_FOUND => {
                return Err(datasource_not_found("Kubernetes workload was not found"));
            }
            reqwest::StatusCode::FORBIDDEN | reqwest::StatusCode::UNAUTHORIZED => {
                return Err(datasource_not_granted(
                    "Kubernetes workload identity refused the read",
                ));
            }
            status if !status.is_success() => {
                return Err(datasource_unavailable(
                    "Kubernetes API returned a non-success response",
                ));
            }
            _ => {}
        }
        let body = bounded_body(response).await?;
        serde_json::from_slice(&body)
            .map_err(|_| datasource_unavailable("Kubernetes response is malformed"))
    }
}

async fn bounded_body(response: reqwest::Response) -> Result<Vec<u8>, DatasourceError> {
    if response.content_length().is_some_and(|length| {
        length > u64::try_from(MAX_KUBERNETES_RESPONSE_BYTES).expect("bound fits u64")
    }) {
        return Err(DatasourceError::new(
            DatasourceErrorCode::ResultTooLarge,
            "Kubernetes response exceeds the result bound",
            false,
        ));
    }
    let body = response
        .bytes()
        .await
        .map_err(|_| datasource_unavailable("Kubernetes response could not be read"))?;
    if body.len() > MAX_KUBERNETES_RESPONSE_BYTES {
        return Err(DatasourceError::new(
            DatasourceErrorCode::ResultTooLarge,
            "Kubernetes response exceeds the result bound",
            false,
        ));
    }
    Ok(body.to_vec())
}

fn project(deployment: KubernetesDeployment) -> DeploymentStatus {
    let available_condition = deployment
        .status
        .conditions
        .iter()
        .any(|condition| condition.kind == "Available" && condition.status == "True");
    let running = deployment.status.observed_generation >= deployment.metadata.generation
        && deployment.status.ready_replicas >= deployment.spec.replicas
        && deployment.status.available_replicas >= deployment.spec.replicas
        && available_condition;
    DeploymentStatus {
        namespace: deployment.metadata.namespace,
        name: deployment.metadata.name,
        generation: deployment.metadata.generation,
        observed_generation: deployment.status.observed_generation,
        desired_replicas: deployment.spec.replicas,
        ready_replicas: deployment.status.ready_replicas,
        available_replicas: deployment.status.available_replicas,
        updated_replicas: deployment.status.updated_replicas,
        running,
        available_condition,
    }
}

fn project_compact(deployment: KubernetesDeployment) -> WorkloadCompact {
    let available = deployment
        .status
        .conditions
        .iter()
        .any(|condition| condition.kind == "Available" && condition.status == "True");
    let progressing = deployment
        .status
        .conditions
        .iter()
        .any(|condition| condition.kind == "Progressing" && condition.status == "True");
    let rollout_state = if deployment.status.observed_generation < deployment.metadata.generation
        || deployment.status.updated_replicas < deployment.spec.replicas
    {
        "progressing"
    } else if available
        && deployment.status.ready_replicas >= deployment.spec.replicas
        && deployment.status.available_replicas >= deployment.spec.replicas
    {
        "available"
    } else if progressing {
        "progressing"
    } else {
        "degraded"
    };
    WorkloadCompact {
        namespace: deployment.metadata.namespace,
        name: deployment.metadata.name,
        uid: deployment.metadata.uid,
        resource_version: deployment.metadata.resource_version,
        generation: deployment.metadata.generation,
        observed_generation: deployment.status.observed_generation,
        desired_replicas: deployment.spec.replicas,
        updated_replicas: deployment.status.updated_replicas,
        ready_replicas: deployment.status.ready_replicas,
        available_replicas: deployment.status.available_replicas,
        unavailable_replicas: deployment.status.unavailable_replicas,
        rollout_state: rollout_state.to_owned(),
    }
}

fn project_pod(pod: KubernetesPod) -> PodSummary {
    let mut statuses = pod
        .status
        .container_statuses
        .into_iter()
        .map(|status| (status.name.clone(), status))
        .collect::<BTreeMap<_, _>>();
    let mut containers = Vec::new();
    for container in pod.spec.containers {
        let status = statuses.remove(&container.name);
        containers.push(ContainerSummary {
            name: container.name,
            image: status
                .as_ref()
                .filter(|status| !status.image.is_empty())
                .map_or(container.image, |status| status.image.clone()),
            image_id: status
                .as_ref()
                .map_or_else(String::new, |status| status.image_id.clone()),
            ready: status.as_ref().is_some_and(|status| status.ready),
            restart_count: status.as_ref().map_or(0, |status| status.restart_count),
            state_reason: status.as_ref().and_then(|status| {
                status
                    .state
                    .values()
                    .filter_map(|state| state.reason.as_ref())
                    .find(|reason| safe_event_reason(reason))
                    .cloned()
            }),
        });
    }
    for (_, status) in statuses {
        containers.push(ContainerSummary {
            name: status.name,
            image: status.image,
            image_id: status.image_id,
            ready: status.ready,
            restart_count: status.restart_count,
            state_reason: status
                .state
                .values()
                .filter_map(|state| state.reason.as_ref())
                .find(|reason| safe_event_reason(reason))
                .cloned(),
        });
    }
    containers.sort_by(|left, right| left.name.cmp(&right.name));
    let ready_containers = containers
        .iter()
        .filter(|container| container.ready)
        .count();
    let restart_count = containers
        .iter()
        .map(|container| container.restart_count)
        .sum();
    PodSummary {
        name: pod.metadata.name,
        phase: pod.status.phase,
        ready_containers: u16::try_from(ready_containers).unwrap_or(u16::MAX),
        total_containers: u16::try_from(containers.len()).unwrap_or(u16::MAX),
        restart_count,
        containers,
    }
}

fn safe_event_reason(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn validate_policy(
    expected_tenant: &str,
    namespace_access: Vec<KubernetesNamespaceAccessConfig>,
    operator_groups: Vec<String>,
) -> Result<(BTreeMap<String, NamespaceAccess>, BTreeSet<String>), KubernetesBackendError> {
    if !valid_ref(expected_tenant, 256) || namespace_access.is_empty() {
        return Err(KubernetesBackendError::InvalidPolicy);
    }
    let operator_groups = operator_groups.into_iter().collect::<BTreeSet<_>>();
    if operator_groups.iter().any(|group| !valid_group(group)) {
        return Err(KubernetesBackendError::InvalidPolicy);
    }
    let mut access = BTreeMap::new();
    for policy in namespace_access {
        let read_groups = policy.read_groups.into_iter().collect::<BTreeSet<_>>();
        let restart_groups = policy.restart_groups.into_iter().collect::<BTreeSet<_>>();
        if !valid_dns_label(&policy.namespace, 63)
            || read_groups.iter().any(|group| !valid_group(group))
            || restart_groups.iter().any(|group| !valid_group(group))
            || !restart_groups.is_subset(&read_groups)
            || access
                .insert(
                    policy.namespace,
                    NamespaceAccess {
                        read_groups,
                        restart_groups,
                    },
                )
                .is_some()
        {
            return Err(KubernetesBackendError::InvalidPolicy);
        }
    }
    if operator_groups.is_empty() && access.values().all(|policy| policy.read_groups.is_empty()) {
        return Err(KubernetesBackendError::InvalidPolicy);
    }
    Ok((access, operator_groups))
}

fn valid_group(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit() && index > 0
                || matches!(byte, b'-' | b'_') && index > 0
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
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'.')
        })
}

fn valid_ref(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn status_summary() -> OperationSummary {
    OperationSummary {
        operation_ref: STATUS_OPERATION.to_owned(),
        title: "Read Kubernetes deployment status".to_owned(),
        effect: EffectClass::ReadOnly,
        approval: ApprovalPosture::NotRequired,
        connections: vec![connection()],
    }
}

fn restart_summary() -> OperationSummary {
    OperationSummary {
        operation_ref: RESTART_OPERATION.to_owned(),
        title: "Restart a Kubernetes Deployment rollout".to_owned(),
        effect: EffectClass::Mutating,
        approval: ApprovalPosture::Required,
        connections: vec![connection()],
    }
}

fn connection() -> ConnectionSummary {
    ConnectionSummary {
        connection_ref: CONNECTION.to_owned(),
        label: "Development cluster".to_owned(),
        provider: "kubernetes".to_owned(),
        audiences: vec!["operations".to_owned()],
    }
}

fn control_connection() -> ControlConnectionSummary {
    ControlConnectionSummary {
        connection_ref: CONNECTION.to_owned(),
        integration_ref: "kubernetes".to_owned(),
        label: "Development cluster".to_owned(),
        state: ConnectionState::Callable,
        initiation: vec![ConnectionInitiator::B10x],
        route: ConnectionRoute::Direct,
        scope: None,
        actor: None,
        auth_profile: None,
    }
}

fn description_ref(context: &PrincipalContext, operation_ref: &str) -> String {
    let digest = Sha256::digest(format!(
        "{operation_ref}\0{}\0{}\0v2",
        context.authority_snapshot_id(),
        context.authority_snapshot_sha256()
    ));
    format!("description:kubernetes:{}", hex::encode(&digest[..16]))
}

fn audit_ref(
    context: &PrincipalContext,
    capability_ref: &str,
    namespace: &str,
    name: &str,
) -> String {
    let digest = Sha256::digest(format!(
        "{}\0{}\0{capability_ref}\0{namespace}\0{name}",
        context.tenant_id(),
        context.actor_subject(),
    ));
    format!("audit:kubernetes:{}", hex::encode(&digest[..16]))
}

fn now_unix_ms() -> u64 {
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    u64::try_from(milliseconds).unwrap_or(u64::MAX)
}

fn invalid(message: &'static str) -> OperationError {
    OperationError::new(OperationErrorCode::InvalidInput, message, false)
}

fn not_granted(message: &'static str) -> OperationError {
    OperationError::new(OperationErrorCode::NotGranted, message, false)
}

fn unavailable(message: &'static str) -> OperationError {
    OperationError::new(OperationErrorCode::Unavailable, message, true)
}

fn stale(message: &'static str) -> OperationError {
    OperationError::new(OperationErrorCode::StaleAuthority, message, false)
}

fn outcome_unknown(message: &'static str) -> OperationError {
    OperationError::new(OperationErrorCode::OutcomeUnknown, message, false)
}

fn datasource_unavailable(message: &'static str) -> DatasourceError {
    DatasourceError::new(DatasourceErrorCode::Unavailable, message, true)
}

fn datasource_not_found(message: &'static str) -> DatasourceError {
    DatasourceError::new(DatasourceErrorCode::NotFound, message, false)
}

fn datasource_not_granted(message: &'static str) -> DatasourceError {
    DatasourceError::new(DatasourceErrorCode::NotGranted, message, false)
}

#[cfg(test)]
include!("hosted_tests.rs");

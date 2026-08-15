//! Deployment-owned, read-only Kubernetes status Integration.

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use protocol::connection::{
    ConnectionDescription as ControlConnectionDescription, ConnectionError, ConnectionErrorCode,
    ConnectionInitiator, ConnectionRequest, ConnectionResult, ConnectionRoute, ConnectionState,
    ConnectionSummary as ControlConnectionSummary, DescribeRequest as ConnectionDescribeRequest,
};
use protocol::operation::{
    ApprovalPosture, ConnectionSummary, DescribeRequest, EffectClass, InvocationResult,
    InvokeRequest, OperationDescription, OperationError, OperationErrorCode, OperationRequest,
    OperationResult, OperationSummary, OwnerContext,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest as _, Sha256};
use url::Url;
use zeroize::Zeroizing;

use server::local::OperationBackend;

const OPERATION: &str = "kubernetes.deployment.status";
const CONNECTION: &str = "connection:kubernetes:in-cluster";
const MAX_KUBERNETES_RESPONSE_BYTES: usize = 256 * 1024;

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
    allowed_namespaces: BTreeSet<String>,
    reader: Arc<dyn DeploymentReader>,
}

#[async_trait]
trait DeploymentReader: Send + Sync {
    async fn read(&self, namespace: &str, name: &str) -> Result<DeploymentStatus, OperationError>;
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeploymentInput {
    namespace: String,
    name: String,
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
    generation: i64,
}

#[derive(Default, Deserialize)]
struct KubernetesDeploymentSpec {
    #[serde(default = "one_replica")]
    replicas: i32,
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
    conditions: Vec<KubernetesCondition>,
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
        allowed_namespaces: Vec<String>,
        token_file: impl Into<PathBuf>,
        ca_file: &Path,
    ) -> Result<Self, KubernetesBackendError> {
        let allowed_namespaces = validate_policy(&expected_tenant, allowed_namespaces)?;
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
            allowed_namespaces,
            reader: Arc::new(InClusterReader {
                client,
                base,
                token_file: token_file.into(),
            }),
        })
    }

    #[cfg(test)]
    fn with_reader(
        expected_tenant: String,
        allowed_namespaces: Vec<String>,
        reader: Arc<dyn DeploymentReader>,
    ) -> Result<Self, KubernetesBackendError> {
        Ok(Self {
            allowed_namespaces: validate_policy(&expected_tenant, allowed_namespaces)?,
            expected_tenant,
            reader,
        })
    }

    fn require_owner(&self, context: &OwnerContext) -> Result<(), OperationError> {
        if context.tenant_id == self.expected_tenant {
            Ok(())
        } else {
            Err(not_granted("Connector tenant binding refused the request"))
        }
    }

    fn require_connection_owner(&self, context: &OwnerContext) -> Result<(), ConnectionError> {
        if context.tenant_id == self.expected_tenant {
            Ok(())
        } else {
            Err(ConnectionError::new(
                ConnectionErrorCode::NotGranted,
                "Connector tenant binding refused the request",
                false,
            ))
        }
    }

    fn description(&self, context: &OwnerContext) -> OperationDescription {
        OperationDescription {
            operation_ref: OPERATION.to_owned(),
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
            description_ref: description_ref(context),
        }
    }

    async fn invoke(
        &self,
        context: &OwnerContext,
        request: InvokeRequest,
    ) -> Result<OperationResult, OperationError> {
        if request.operation_ref != OPERATION
            || request.connection_ref != CONNECTION
            || request.description_ref != description_ref(context)
            || request.approval_evidence_ref.is_some()
        {
            return Err(not_granted(
                "Kubernetes operation authority is stale or not granted",
            ));
        }
        let input: DeploymentInput = serde_json::from_value(request.input)
            .map_err(|_| invalid("Kubernetes deployment input is invalid"))?;
        if !self.allowed_namespaces.contains(&input.namespace)
            || !valid_dns_label(&input.namespace, 63)
            || !valid_dns_label(&input.name, 253)
        {
            return Err(not_granted(
                "Kubernetes namespace or deployment is not admitted",
            ));
        }
        let status = self.reader.read(&input.namespace, &input.name).await?;
        let output = serde_json::to_value(status)
            .map_err(|_| unavailable("Kubernetes status could not be encoded"))?;
        Ok(OperationResult::Invoke(InvocationResult {
            operation_ref: OPERATION.to_owned(),
            output,
            connector_audit_ref: audit_ref(context, &input),
            execution_ref: None,
        }))
    }
}

#[async_trait]
impl OperationBackend for KubernetesStatusBackend {
    async fn handle(
        &self,
        context: &OwnerContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        self.require_owner(context)?;
        match request {
            OperationRequest::Search(search) => {
                let query = search.query.to_ascii_lowercase();
                let operations = (query.is_empty()
                    || ["kubernetes", "deployment", "backend", "running", "status"]
                        .iter()
                        .any(|term| query.contains(term)))
                .then(summary)
                .into_iter()
                .take(usize::from(search.limit))
                .collect();
                Ok(OperationResult::Search { operations })
            }
            OperationRequest::Describe(DescribeRequest { operation_ref })
                if operation_ref == OPERATION =>
            {
                Ok(OperationResult::Describe(self.description(context)))
            }
            OperationRequest::Invoke(request) => self.invoke(context, request).await,
            _ => Err(OperationError::new(
                OperationErrorCode::NotFound,
                "Kubernetes Integration operation was not found",
                false,
            )),
        }
    }

    async fn handle_connection(
        &self,
        context: &OwnerContext,
        request: ConnectionRequest,
    ) -> Result<ConnectionResult, ConnectionError> {
        self.require_connection_owner(context)?;
        match request {
            ConnectionRequest::Search(search) => {
                let query = search.query.to_ascii_lowercase();
                let connections = (query.is_empty()
                    || ["kubernetes", "development", "cluster", "read-only"]
                        .iter()
                        .any(|term| query.contains(term)))
                .then(control_connection)
                .into_iter()
                .take(usize::from(search.limit))
                .collect();
                Ok(ConnectionResult::Search { connections })
            }
            ConnectionRequest::Describe(ConnectionDescribeRequest { connection_ref })
                if connection_ref == CONNECTION =>
            {
                Ok(ConnectionResult::Describe(ControlConnectionDescription {
                    summary: control_connection(),
                    channels: Vec::new(),
                }))
            }
            _ => Err(ConnectionError::new(
                ConnectionErrorCode::NotFound,
                "Kubernetes Integration Connection was not found",
                false,
            )),
        }
    }

    fn supports_connections(&self) -> bool {
        true
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

fn validate_policy(
    expected_tenant: &str,
    namespaces: Vec<String>,
) -> Result<BTreeSet<String>, KubernetesBackendError> {
    let namespaces = namespaces.into_iter().collect::<BTreeSet<_>>();
    if !valid_ref(expected_tenant, 256)
        || namespaces.is_empty()
        || namespaces
            .iter()
            .any(|namespace| !valid_dns_label(namespace, 63))
    {
        return Err(KubernetesBackendError::InvalidPolicy);
    }
    Ok(namespaces)
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

fn summary() -> OperationSummary {
    OperationSummary {
        operation_ref: OPERATION.to_owned(),
        title: "Read Kubernetes deployment status".to_owned(),
        effect: EffectClass::ReadOnly,
        approval: ApprovalPosture::NotRequired,
        connections: vec![connection()],
    }
}

fn connection() -> ConnectionSummary {
    ConnectionSummary {
        connection_ref: CONNECTION.to_owned(),
        label: "Development cluster (read-only)".to_owned(),
        provider: "kubernetes".to_owned(),
        audiences: vec!["operations".to_owned()],
    }
}

fn control_connection() -> ControlConnectionSummary {
    ControlConnectionSummary {
        connection_ref: CONNECTION.to_owned(),
        integration_ref: "kubernetes".to_owned(),
        label: "Development cluster (read-only)".to_owned(),
        state: ConnectionState::Callable,
        initiation: vec![ConnectionInitiator::B10x],
        route: ConnectionRoute::Direct,
    }
}

fn description_ref(context: &OwnerContext) -> String {
    let digest = Sha256::digest(format!(
        "{OPERATION}\0{}\0{}\0v1",
        context.authority_snapshot_id, context.authority_snapshot_sha256
    ));
    format!("description:kubernetes:{}", hex::encode(&digest[..16]))
}

fn audit_ref(context: &OwnerContext, input: &DeploymentInput) -> String {
    let digest = Sha256::digest(format!(
        "{}\0{}\0{}\0{}",
        context.tenant_id, context.agent_id, input.namespace, input.name
    ));
    format!("audit:kubernetes:{}", hex::encode(&digest[..16]))
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

#[cfg(test)]
mod tests {
    use super::*;

    struct Reader;

    #[async_trait]
    impl DeploymentReader for Reader {
        async fn read(
            &self,
            namespace: &str,
            name: &str,
        ) -> Result<DeploymentStatus, OperationError> {
            Ok(DeploymentStatus {
                namespace: namespace.to_owned(),
                name: name.to_owned(),
                generation: 3,
                observed_generation: 3,
                desired_replicas: 2,
                ready_replicas: 2,
                available_replicas: 2,
                updated_replicas: 2,
                running: true,
                available_condition: true,
            })
        }
    }

    fn owner(tenant: &str) -> OwnerContext {
        OwnerContext {
            tenant_id: tenant.to_owned(),
            agent_id: "agent-dev".to_owned(),
            agent_revision: 1,
            authority_snapshot_id: "snapshot-dev".to_owned(),
            authority_snapshot_sha256: "a".repeat(64),
        }
    }

    #[tokio::test]
    async fn read_only_status_is_description_bound_and_namespace_scoped() {
        let backend = KubernetesStatusBackend::with_reader(
            "tenant-dev".to_owned(),
            vec!["b10x".to_owned()],
            Arc::new(Reader),
        )
        .unwrap();
        let context = owner("tenant-dev");
        let OperationResult::Describe(description) = backend
            .handle(
                &context,
                OperationRequest::Describe(DescribeRequest {
                    operation_ref: OPERATION.to_owned(),
                }),
            )
            .await
            .unwrap()
        else {
            panic!("description result expected");
        };
        let OperationResult::Invoke(result) = backend
            .handle(
                &context,
                OperationRequest::Invoke(InvokeRequest {
                    operation_ref: OPERATION.to_owned(),
                    connection_ref: CONNECTION.to_owned(),
                    description_ref: description.description_ref,
                    input: json!({"namespace": "b10x", "name": "backend"}),
                    approval_evidence_ref: None,
                }),
            )
            .await
            .unwrap()
        else {
            panic!("invoke result expected");
        };
        assert_eq!(result.output["running"], true);

        let refused = backend
            .handle(
                &context,
                OperationRequest::Invoke(InvokeRequest {
                    operation_ref: OPERATION.to_owned(),
                    connection_ref: CONNECTION.to_owned(),
                    description_ref: description_ref(&context),
                    input: json!({"namespace": "kube-system", "name": "backend"}),
                    approval_evidence_ref: None,
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(refused.code, OperationErrorCode::NotGranted);
    }

    #[tokio::test]
    async fn hosted_connection_projection_is_value_free_and_tenant_bound() {
        let backend = KubernetesStatusBackend::with_reader(
            "tenant-dev".to_owned(),
            vec!["b10x".to_owned()],
            Arc::new(Reader),
        )
        .unwrap();
        let ConnectionResult::Search { connections } = backend
            .handle_connection(
                &owner("tenant-dev"),
                ConnectionRequest::Search(protocol::connection::SearchRequest {
                    query: String::new(),
                    limit: 16,
                }),
            )
            .await
            .unwrap()
        else {
            panic!("Connection search result expected");
        };
        assert_eq!(connections, vec![control_connection()]);
        assert!(backend
            .handle_connection(
                &owner("tenant-other"),
                ConnectionRequest::Search(protocol::connection::SearchRequest {
                    query: String::new(),
                    limit: 16,
                }),
            )
            .await
            .is_err());
    }

    #[test]
    fn deployment_projection_requires_observed_available_replicas() {
        let deployment: KubernetesDeployment = serde_json::from_value(json!({
            "metadata": {"namespace": "b10x", "name": "backend", "generation": 2},
            "spec": {"replicas": 1},
            "status": {
                "observedGeneration": 1,
                "readyReplicas": 1,
                "availableReplicas": 1,
                "updatedReplicas": 1,
                "conditions": [{"type": "Available", "status": "True"}]
            }
        }))
        .unwrap();
        assert!(!project(deployment).running);
    }
}

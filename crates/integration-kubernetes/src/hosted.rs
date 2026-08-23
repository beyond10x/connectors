//! Deployment-owned, policy-scoped Kubernetes workload Integration.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use connectors_config::KubernetesNamespaceAccessConfig;
use protocol::connection::{
    ConnectionDescription as ControlConnectionDescription, ConnectionError, ConnectionErrorCode,
    ConnectionInitiator, ConnectionRequest, ConnectionResult, ConnectionRoute, ConnectionState,
    ConnectionSummary as ControlConnectionSummary, DescribeRequest as ConnectionDescribeRequest,
};
use protocol::datasource::{
    BindingSearchRequest, DatasourceError, DatasourceErrorCode, DatasourceRequest,
    DatasourceResult, DescribeRequest as DatasourceDescribeRequest, ReadRequest,
    SearchRequest as DatasourceSearchRequest,
};
use protocol::operation::{
    ApprovalPosture, ConnectionSummary, DescribeRequest, EffectClass, InvocationResult,
    InvokeRequest, OperationDescription, OperationError, OperationErrorCode, OperationRequest,
    OperationResult, OperationSummary,
};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest as _, Sha256};
use url::Url;
use zeroize::Zeroizing;

use service::{BackendCapabilities, ConnectorBackend, PrincipalContext};

// The workload projection is shared with every other host mode, so a Deployment read
// through a workstation kubeconfig and one read through this ServiceAccount are the same
// record. See `crate::workloads`.
use crate::workloads::*;

mod datasource;

const CONNECTION: &str = "connection:kubernetes:in-cluster";

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
    cursors: CursorStore,
}

#[derive(Clone)]
struct NamespaceAccess {
    read_groups: BTreeSet<String>,
    restart_groups: BTreeSet<String>,
}

struct InClusterReader {
    client: reqwest::Client,
    base: Url,
    token_file: PathBuf,
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
            cursors: CursorStore::default(),
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
            cursors: CursorStore::default(),
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
                if request.description_ref != description_ref(context, STATUS_OPERATION) {
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
                // The description declares `ApprovalPosture::Required`, and the demanded
                // approval is verified and spent upstream by the sealed proof chain (S-045,
                // S-046) before this Integration is reachable; no local reading of the
                // caller-supplied evidence reference decides admission here (S-047).
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
        purpose: None,
    }
}

fn control_connection() -> ControlConnectionSummary {
    ControlConnectionSummary {
        connection_ref: CONNECTION.to_owned(),
        integration_ref: "kubernetes".to_owned(),
        label: "Development cluster".to_owned(),
        state: ConnectionState::Callable,
        initiation: vec![ConnectionInitiator::Platform],
        route: ConnectionRoute::Direct,
        scope: None,
        actor: None,
        auth_profile: None,
    }
}

/// The description lease for one Kubernetes operation.
///
/// Seeded from the stable authority, not from the authority snapshot id or sha. Those are the
/// access-token id and the introspection-envelope hash, and describe and invoke never share a
/// token: describe travels on `connectors.catalog.read`, invoke on `connectors.invoke`, and the
/// client caches one token per scope. Deriving the lease from them refused every hosted
/// invocation of these operations as stale.
fn description_ref(context: &PrincipalContext, operation_ref: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(b"b10x/kubernetes-operation-description/v3\0");
    digest.update(context.stable_authority_seed());
    digest.update(b"\0");
    digest.update(operation_ref.as_bytes());
    format!(
        "description:kubernetes:{}",
        hex::encode(&digest.finalize()[..16])
    )
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

#[cfg(test)]
include!("hosted_tests.rs");

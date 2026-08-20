//! The Kubernetes workload projection, shared by every host mode.
//!
//! What a placement may read is a policy question and stays with the placement. *What a workload
//! looks like once it is read* is not: a Deployment seen through the operator's own kubeconfig and
//! the same Deployment seen through an in-cluster ServiceAccount are the same object, and a person
//! moving between the two workbenches must not find a different product. Before this module the
//! projection lived only in `hosted.rs`, so the personal-local placement published
//! `datasources: false` and the local Kubernetes Connection sat at `authorized` with nothing to
//! call — the cluster was attached and unusable.
//!
//! Everything here is therefore host-mode-neutral: the wire types, the projection into
//! `WorkloadCompact`/`WorkloadDetail`, the JSON Schemas, the projection digest, the description
//! lease, the binding refs and the opaque cursor store. `DeploymentReader` is the one seam — a
//! placement supplies the transport that carries its own credential, and nothing else differs.
//! `datasource_projection_sha256()` is a single function for exactly this reason. Note what that
//! buys and what it does not: equality between the two placements is *structural*, because there is
//! only one definition to disagree with — no test asserts it, and none can, since there is nothing
//! left to compare. The thing worth guarding is a second copy appearing.
//!
//! Admission deliberately does *not* live here. The hosted receiver checks Identity groups against
//! configured namespace grants; the personal-local one is owner-only behind a Unix socket. Sharing
//! those would move a policy decision away from the placement that owns it.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use protocol::datasource::{
    AccessMode, Completeness, DatasourceBinding, DatasourceDescription, DatasourceError,
    DatasourceErrorCode, DatasourcePage, DatasourceProvenance, DatasourceRead, DatasourceRecord,
    DatasourceResult, DatasourceSummary, ReadRequest, ReadVerb, RecordView,
};
use protocol::operation::{
    ApprovalPosture, ConnectionSummary, EffectClass, OperationDescription, OperationError,
    OperationErrorCode,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest as _, Sha256};

use service::PrincipalContext;

pub(crate) const STATUS_OPERATION: &str = "kubernetes.deployment.status";
pub(crate) const RESTART_OPERATION: &str = "kubernetes.deployment.rollout-restart";
pub(crate) const DATASOURCE: &str = "kubernetes.workloads";
pub(crate) const MAX_KUBERNETES_RESPONSE_BYTES: usize = 256 * 1024;
pub(crate) const MAX_RELATED_RECORDS: usize = 50;
pub(crate) const CURSOR_TTL: Duration = Duration::from_secs(300);

#[async_trait]
pub(crate) trait DeploymentReader: Send + Sync {
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

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DeploymentStatus {
    pub(crate) namespace: String,
    pub(crate) name: String,
    pub(crate) generation: i64,
    pub(crate) observed_generation: i64,
    pub(crate) desired_replicas: i32,
    pub(crate) ready_replicas: i32,
    pub(crate) available_replicas: i32,
    pub(crate) updated_replicas: i32,
    pub(crate) running: bool,
    pub(crate) available_condition: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct WorkloadCompact {
    pub(crate) namespace: String,
    pub(crate) name: String,
    pub(crate) uid: String,
    pub(crate) resource_version: String,
    pub(crate) generation: i64,
    pub(crate) observed_generation: i64,
    pub(crate) desired_replicas: i32,
    pub(crate) updated_replicas: i32,
    pub(crate) ready_replicas: i32,
    pub(crate) available_replicas: i32,
    pub(crate) unavailable_replicas: i32,
    pub(crate) rollout_state: String,
}

pub(crate) struct WorkloadList {
    pub(crate) workloads: Vec<WorkloadCompact>,
    pub(crate) next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct WorkloadDetail {
    #[serde(flatten)]
    pub(crate) workload: WorkloadCompact,
    pub(crate) pods: Vec<PodSummary>,
    pub(crate) warnings: Vec<WarningSummary>,
    pub(crate) related_complete: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PodSummary {
    pub(crate) name: String,
    pub(crate) phase: String,
    pub(crate) ready_containers: u16,
    pub(crate) total_containers: u16,
    pub(crate) restart_count: u32,
    pub(crate) containers: Vec<ContainerSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ContainerSummary {
    pub(crate) name: String,
    pub(crate) image: String,
    pub(crate) image_id: String,
    pub(crate) ready: bool,
    pub(crate) restart_count: u32,
    pub(crate) state_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct WarningSummary {
    pub(crate) involved_kind: String,
    pub(crate) involved_name: String,
    pub(crate) reason: String,
    pub(crate) count: i32,
    pub(crate) first_observed_at: Option<String>,
    pub(crate) last_observed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct RestartAccepted {
    pub(crate) namespace: String,
    pub(crate) name: String,
    pub(crate) uid: String,
    pub(crate) resource_version: String,
    pub(crate) patch_accepted: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeploymentInput {
    pub(crate) namespace: String,
    pub(crate) name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RestartInput {
    pub(crate) namespace: String,
    pub(crate) name: String,
    pub(crate) uid: String,
    pub(crate) resource_version: String,
}

#[derive(Deserialize)]
pub(crate) struct KubernetesDeployment {
    pub(crate) metadata: KubernetesMetadata,
    #[serde(default)]
    pub(crate) spec: KubernetesDeploymentSpec,
    #[serde(default)]
    pub(crate) status: KubernetesDeploymentState,
}

#[derive(Deserialize)]
pub(crate) struct KubernetesMetadata {
    pub(crate) name: String,
    pub(crate) namespace: String,
    #[serde(default)]
    pub(crate) uid: String,
    #[serde(default, rename = "resourceVersion")]
    pub(crate) resource_version: String,
    #[serde(default)]
    pub(crate) generation: i64,
}

#[derive(Default, Deserialize)]
pub(crate) struct KubernetesDeploymentSpec {
    #[serde(default = "one_replica")]
    pub(crate) replicas: i32,
    #[serde(default)]
    pub(crate) selector: KubernetesLabelSelector,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KubernetesLabelSelector {
    #[serde(default)]
    pub(crate) match_labels: BTreeMap<String, String>,
}

pub(crate) const fn one_replica() -> i32 {
    1
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KubernetesDeploymentState {
    #[serde(default)]
    pub(crate) observed_generation: i64,
    #[serde(default)]
    pub(crate) ready_replicas: i32,
    #[serde(default)]
    pub(crate) available_replicas: i32,
    #[serde(default)]
    pub(crate) updated_replicas: i32,
    #[serde(default)]
    pub(crate) unavailable_replicas: i32,
    #[serde(default)]
    pub(crate) conditions: Vec<KubernetesCondition>,
}

#[derive(Deserialize)]
pub(crate) struct KubernetesList<T> {
    #[serde(default)]
    pub(crate) metadata: KubernetesListMetadata,
    pub(crate) items: Vec<T>,
}

#[derive(Default, Deserialize)]
pub(crate) struct KubernetesListMetadata {
    #[serde(default, rename = "continue")]
    pub(crate) continue_token: String,
}

#[derive(Deserialize)]
pub(crate) struct KubernetesPod {
    pub(crate) metadata: KubernetesMetadata,
    #[serde(default)]
    pub(crate) spec: KubernetesPodSpec,
    #[serde(default)]
    pub(crate) status: KubernetesPodStatus,
}

#[derive(Default, Deserialize)]
pub(crate) struct KubernetesPodSpec {
    #[serde(default)]
    pub(crate) containers: Vec<KubernetesContainer>,
}

#[derive(Deserialize)]
pub(crate) struct KubernetesContainer {
    pub(crate) name: String,
    pub(crate) image: String,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KubernetesPodStatus {
    #[serde(default)]
    pub(crate) phase: String,
    #[serde(default)]
    pub(crate) container_statuses: Vec<KubernetesContainerStatus>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KubernetesContainerStatus {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) image: String,
    #[serde(default)]
    pub(crate) image_id: String,
    #[serde(default)]
    pub(crate) ready: bool,
    #[serde(default)]
    pub(crate) restart_count: u32,
    #[serde(default)]
    pub(crate) state: BTreeMap<String, KubernetesContainerState>,
}

#[derive(Default, Deserialize)]
pub(crate) struct KubernetesContainerState {
    #[serde(default)]
    pub(crate) reason: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KubernetesEvent {
    pub(crate) involved_object: KubernetesObjectReference,
    #[serde(default)]
    pub(crate) reason: String,
    #[serde(default)]
    pub(crate) count: i32,
    #[serde(default)]
    pub(crate) first_timestamp: Option<String>,
    #[serde(default)]
    pub(crate) last_timestamp: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct KubernetesObjectReference {
    #[serde(default)]
    pub(crate) kind: String,
    #[serde(default)]
    pub(crate) name: String,
}

#[derive(Deserialize)]
pub(crate) struct KubernetesCondition {
    #[serde(rename = "type")]
    pub(crate) kind: String,
    pub(crate) status: String,
}

pub(crate) fn project(deployment: KubernetesDeployment) -> DeploymentStatus {
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

pub(crate) fn project_compact(deployment: KubernetesDeployment) -> WorkloadCompact {
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

pub(crate) fn project_pod(pod: KubernetesPod) -> PodSummary {
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

pub(crate) fn safe_event_reason(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

pub(crate) fn now_unix_ms() -> u64 {
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    u64::try_from(milliseconds).unwrap_or(u64::MAX)
}

pub(crate) fn invalid(message: &'static str) -> OperationError {
    OperationError::new(OperationErrorCode::InvalidInput, message, false)
}

pub(crate) fn not_granted(message: &'static str) -> OperationError {
    OperationError::new(OperationErrorCode::NotGranted, message, false)
}

pub(crate) fn unavailable(message: &'static str) -> OperationError {
    OperationError::new(OperationErrorCode::Unavailable, message, true)
}

pub(crate) fn stale(message: &'static str) -> OperationError {
    OperationError::new(OperationErrorCode::StaleAuthority, message, false)
}

pub(crate) fn outcome_unknown(message: &'static str) -> OperationError {
    OperationError::new(OperationErrorCode::OutcomeUnknown, message, false)
}

pub(crate) fn datasource_unavailable(message: &'static str) -> DatasourceError {
    DatasourceError::new(DatasourceErrorCode::Unavailable, message, true)
}

pub(crate) fn datasource_not_found(message: &'static str) -> DatasourceError {
    DatasourceError::new(DatasourceErrorCode::NotFound, message, false)
}

pub(crate) fn datasource_not_granted(message: &'static str) -> DatasourceError {
    DatasourceError::new(DatasourceErrorCode::NotGranted, message, false)
}


pub(crate) fn datasource_summary() -> DatasourceSummary {
    DatasourceSummary {
        datasource_ref: DATASOURCE.to_owned(),
        title: "Kubernetes workloads".to_owned(),
        access_mode: AccessMode::Live,
        verbs: vec![ReadVerb::List, ReadVerb::Get],
    }
}

pub(crate) fn workload_key_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["name"],
        "properties": {"name": {"type": "string", "minLength": 1, "maxLength": 253}}
    })
}

pub(crate) fn compact_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["namespace", "name", "uid", "resource_version", "generation", "observed_generation", "desired_replicas", "updated_replicas", "ready_replicas", "available_replicas", "unavailable_replicas", "rollout_state"],
        "properties": {
            "namespace": {"type": "string"}, "name": {"type": "string"},
            "uid": {"type": "string"}, "resource_version": {"type": "string"},
            "generation": {"type": "integer"}, "observed_generation": {"type": "integer"},
            "desired_replicas": {"type": "integer"}, "updated_replicas": {"type": "integer"},
            "ready_replicas": {"type": "integer"}, "available_replicas": {"type": "integer"},
            "unavailable_replicas": {"type": "integer"},
            "rollout_state": {"enum": ["available", "progressing", "degraded"]}
        }
    })
}

pub(crate) fn detail_schema() -> serde_json::Value {
    let mut schema = compact_schema();
    let object = schema.as_object_mut().expect("static object schema");
    object
        .get_mut("required")
        .and_then(serde_json::Value::as_array_mut)
        .expect("static required schema")
        .extend([json!("pods"), json!("warnings"), json!("related_complete")]);
    let properties = object
        .get_mut("properties")
        .and_then(serde_json::Value::as_object_mut)
        .expect("static properties schema");
    properties.insert(
        "pods".to_owned(),
        json!({
            "type": "array",
            "maxItems": MAX_RELATED_RECORDS,
            "items": {
                "type": "object",
                "additionalProperties": false,
                "required": ["name", "phase", "ready_containers", "total_containers", "restart_count", "containers"],
                "properties": {
                    "name": {"type": "string"},
                    "phase": {"type": "string"},
                    "ready_containers": {"type": "integer", "minimum": 0},
                    "total_containers": {"type": "integer", "minimum": 0},
                    "restart_count": {"type": "integer", "minimum": 0},
                    "containers": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "additionalProperties": false,
                            "required": ["name", "image", "image_id", "ready", "restart_count", "state_reason"],
                            "properties": {
                                "name": {"type": "string"},
                                "image": {"type": "string"},
                                "image_id": {"type": "string"},
                                "ready": {"type": "boolean"},
                                "restart_count": {"type": "integer", "minimum": 0},
                                "state_reason": {"type": ["string", "null"]}
                            }
                        }
                    }
                }
            }
        }),
    );
    properties.insert(
        "warnings".to_owned(),
        json!({
            "type": "array",
            "maxItems": MAX_RELATED_RECORDS,
            "items": {
                "type": "object",
                "additionalProperties": false,
                "required": ["involved_kind", "involved_name", "reason", "count", "first_observed_at", "last_observed_at"],
                "properties": {
                    "involved_kind": {"type": "string"},
                    "involved_name": {"type": "string"},
                    "reason": {"type": "string"},
                    "count": {"type": "integer", "minimum": 0},
                    "first_observed_at": {"type": ["string", "null"]},
                    "last_observed_at": {"type": ["string", "null"]}
                }
            }
        }),
    );
    properties.insert("related_complete".to_owned(), json!({"type": "boolean"}));
    schema
}

pub(crate) fn datasource_projection_sha256() -> String {
    let declaration = json!({
        "protocol": "b10x.value-projection.v1",
        "datasource_ref": DATASOURCE,
        "version": 1,
        "key_schema": workload_key_schema(),
        "compact_schema": compact_schema(),
        "detail_schema": detail_schema(),
        "excluded": ["annotations", "labels", "environment", "secret_references", "event_messages", "raw_objects"]
    });
    hex::encode(Sha256::digest(
        serde_json::to_vec(&declaration).expect("static projection declaration"),
    ))
}

pub(crate) fn datasource_description(context: &PrincipalContext) -> DatasourceDescription {
    DatasourceDescription {
        summary: datasource_summary(),
        description: "Live, namespace-scoped Deployment status with bounded Pod, image digest, restart-count, and warning-reason summaries. Raw Kubernetes objects, event messages, labels, annotations, environment values, and Secret data are never returned.".to_owned(),
        key_schema: workload_key_schema(),
        compact_schema: compact_schema(),
        detail_schema: detail_schema(),
        projection_protocol: "b10x.value-projection.v1".to_owned(),
        projection_sha256: datasource_projection_sha256(),
        description_ref: datasource_description_ref(context),
    }
}

/// The description lease for the workload datasource.
///
/// Seeded from the stable authority and never from the authority snapshot id or sha: the hosted
/// receiver fills those from the access token, every datasource request travels on the cached
/// `connectors.catalog.read` token, and that token is refreshed inside five minutes. A describe
/// and the read that follows can therefore straddle a refresh, and seeding the lease from the
/// token made that ordinary event look like an authority change.
pub(crate) fn datasource_description_ref(context: &PrincipalContext) -> String {
    let mut digest = Sha256::new();
    digest.update(b"b10x/kubernetes-datasource-description/v2\0");
    digest.update(context.stable_authority_seed());
    digest.update(b"\0");
    digest.update(datasource_projection_sha256().as_bytes());
    format!(
        "description:kubernetes:datasource:{}",
        hex::encode(&digest.finalize()[..16])
    )
}

/// One namespace, offered as a binding of the workload datasource.
///
/// The Connection is a parameter because it is the one thing that legitimately differs by host
/// mode: in-cluster the binding hangs off the deployment's own Connection, on a workstation off
/// the Connection the operator activated from a kubeconfig context. The binding ref itself does
/// not carry it — a namespace names the same records whichever way the cluster was reached.
pub(crate) fn namespace_binding(connection_ref: &str, namespace: &str) -> DatasourceBinding {
    let digest = Sha256::digest(format!("{DATASOURCE}\0{namespace}\0v1"));
    let mut generation_bytes = [0_u8; 8];
    generation_bytes.copy_from_slice(&digest[..8]);
    DatasourceBinding {
        datasource_ref: DATASOURCE.to_owned(),
        binding_ref: namespace_binding_ref(namespace),
        connection_ref: connection_ref.to_owned(),
        label: namespace.to_owned(),
        purpose: None,
        generation: u64::from_be_bytes(generation_bytes),
    }
}

pub(crate) fn namespace_binding_ref(namespace: &str) -> String {
    let digest = Sha256::digest(format!("{DATASOURCE}\0{namespace}\0binding-v1"));
    format!("binding:kubernetes:{}", hex::encode(&digest[..16]))
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
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'.')
        })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkloadKey {
    pub(crate) name: String,
}

struct CursorState {
    namespace: String,
    principal_subject: String,
    /// Bound to the stable authority, never to the access token: page two of a listing arrives on
    /// whatever token is current then, and an expired cursor for a rotated token is a refusal a
    /// caller cannot act on.
    authority_seed: Vec<u8>,
    provider_cursor: String,
    expires_at: SystemTime,
}

/// Opaque, authority-bound paging cursors for the workload datasource.
///
/// The provider's own `continue` token never leaves this process. A cursor handed to a caller is a
/// digest bound to that caller's stable authority and to the namespace it was issued for, so a
/// cursor cannot be replayed by a different principal or against a namespace the issuer never
/// granted. Placement-neutral: paging a listing is the same act on a workstation and in a cluster.
#[derive(Default)]
pub(crate) struct CursorStore {
    cursors: Mutex<BTreeMap<String, CursorState>>,
}

impl CursorStore {
    fn resolve(
        &self,
        context: &PrincipalContext,
        namespace: &str,
        cursor: Option<&str>,
    ) -> Result<Option<String>, DatasourceError> {
        let Some(cursor) = cursor else {
            return Ok(None);
        };
        let mut cursors = self
            .cursors
            .lock()
            .map_err(|_| datasource_unavailable("Kubernetes cursor state is unavailable"))?;
        let now = SystemTime::now();
        cursors.retain(|_, state| state.expires_at > now);
        let state = cursors.remove(cursor).ok_or_else(|| {
            DatasourceError::new(
                DatasourceErrorCode::CursorExpired,
                "Kubernetes datasource cursor is expired or unknown",
                false,
            )
        })?;
        if state.namespace != namespace
            || state.principal_subject != context.subject()
            || state.authority_seed != context.stable_authority_seed()
        {
            return Err(datasource_not_granted(
                "Kubernetes datasource cursor belongs to different authority",
            ));
        }
        Ok(Some(state.provider_cursor))
    }

    fn store(
        &self,
        context: &PrincipalContext,
        namespace: &str,
        provider_cursor: Option<String>,
    ) -> Result<Option<String>, DatasourceError> {
        let Some(provider_cursor) = provider_cursor.filter(|value| !value.is_empty()) else {
            return Ok(None);
        };
        let now = SystemTime::now();
        let nonce = now
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let mut digest = Sha256::new();
        digest.update(b"b10x/kubernetes-datasource-cursor/v2\0");
        digest.update(context.stable_authority_seed());
        digest.update(format!("\0{namespace}\0{provider_cursor}\0{nonce}").as_bytes());
        let cursor_ref = format!(
            "cursor:kubernetes:{}",
            hex::encode(&digest.finalize()[..20])
        );
        let mut cursors = self
            .cursors
            .lock()
            .map_err(|_| datasource_unavailable("Kubernetes cursor state is unavailable"))?;
        cursors.retain(|_, state| state.expires_at > now);
        if cursors.len() >= 256 {
            return Err(DatasourceError::new(
                DatasourceErrorCode::ResultTooLarge,
                "Kubernetes cursor capacity is exhausted",
                true,
            ));
        }
        cursors.insert(
            cursor_ref.clone(),
            CursorState {
                namespace: namespace.to_owned(),
                principal_subject: context.subject().to_owned(),
                authority_seed: context.stable_authority_seed(),
                provider_cursor,
                expires_at: now + CURSOR_TTL,
            },
        );
        Ok(Some(cursor_ref))
    }
}

/// Serves one read of `kubernetes.workloads`, once the placement has decided the caller may read
/// this namespace.
///
/// Admission is the caller's: the namespace arrives already authorised, because who may read what
/// is the one thing the two host modes legitimately disagree about. Everything from here down —
/// the datasource ref, the description lease, cursor handling, the record views, the provenance —
/// is identical wherever it runs, which is what makes the two workbenches the same product.
pub(crate) async fn read_workloads(
    reader: &dyn DeploymentReader,
    cursors: &CursorStore,
    context: &PrincipalContext,
    namespace: &str,
    request: ReadRequest,
    connector_audit_ref: String,
) -> Result<DatasourceResult, DatasourceError> {
    if request.datasource_ref != DATASOURCE {
        return Err(datasource_not_found("Kubernetes datasource was not found"));
    }
    if request.description_ref != datasource_description_ref(context) {
        // Recoverable in one call, and the caller is told which one. It used to say only
        // "authority is stale", which reads like something an operator has to repair.
        return Err(DatasourceError::new(
            DatasourceErrorCode::StaleAuthority,
            "the Kubernetes datasource description lease has moved on; describe it again and retry the read",
            true,
        ));
    }
    let (records, next_cursor, completeness) = match request.read {
        DatasourceRead::List { limit, cursor } => {
            let provider_cursor = cursors.resolve(context, namespace, cursor.as_deref())?;
            let list = reader
                .list_workloads(namespace, limit, provider_cursor.as_deref())
                .await?;
            let next_cursor = cursors.store(context, namespace, list.next_cursor)?;
            let records = list
                .workloads
                .into_iter()
                .map(|workload| {
                    let name = workload.name.clone();
                    let value = serde_json::to_value(workload).map_err(|_| {
                        datasource_unavailable("Kubernetes workload could not be encoded")
                    })?;
                    Ok(DatasourceRecord {
                        key: json!({"name": name}),
                        view: RecordView::Compact,
                        value,
                    })
                })
                .collect::<Result<Vec<_>, DatasourceError>>()?;
            (records, next_cursor, Completeness::Complete)
        }
        DatasourceRead::Get { key } => {
            let key: WorkloadKey = serde_json::from_value(key).map_err(|_| {
                DatasourceError::new(
                    DatasourceErrorCode::InvalidInput,
                    "Kubernetes workload key is invalid",
                    false,
                )
            })?;
            if !valid_dns_label(&key.name, 253) {
                return Err(DatasourceError::new(
                    DatasourceErrorCode::InvalidInput,
                    "Kubernetes workload name is invalid",
                    false,
                ));
            }
            let detail = reader.workload_detail(namespace, &key.name).await?;
            let completeness = if detail.related_complete {
                Completeness::Complete
            } else {
                Completeness::Partial
            };
            let value = serde_json::to_value(detail).map_err(|_| {
                datasource_unavailable("Kubernetes workload detail could not be encoded")
            })?;
            (
                vec![DatasourceRecord {
                    key: json!({"name": key.name}),
                    view: RecordView::Detail,
                    value,
                }],
                None,
                completeness,
            )
        }
    };
    Ok(DatasourceResult::Read(DatasourcePage {
        datasource_ref: DATASOURCE.to_owned(),
        records,
        next_cursor,
        completeness,
        observed_at_unix_ms: now_unix_ms(),
        provenance: DatasourceProvenance {
            binding_ref: request.binding_ref,
            projection_sha256: datasource_projection_sha256(),
            connector_audit_ref,
        },
    }))
}

/// The two workload operations, as every host mode publishes them.
///
/// Shared for the same reason the datasource projection is: a Deployment read through a
/// workstation kubeconfig and one read through an in-cluster ServiceAccount are the same object,
/// so the contract a caller is handed must not depend on which placement answered. Only the
/// Connections and the description lease belong to the placement.
pub(crate) fn status_operation(
    connections: Vec<ConnectionSummary>,
    description_ref: String,
) -> OperationDescription {
    OperationDescription {
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
        connections,
        description_ref,
    }
}

pub(crate) fn restart_operation(
    connections: Vec<ConnectionSummary>,
    description_ref: String,
) -> OperationDescription {
    OperationDescription {
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
        connections,
        description_ref,
    }
}

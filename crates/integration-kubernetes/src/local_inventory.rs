//! Bounded inventory reads through an explicitly selected, activated local Connection.
//!
//! These output values implement `ess/system/domains/inventory.yaml`. They observe the Deployment
//! template, not Pods, and do not change the existing compact workload datasource projection.

use protocol::operation::{
    ApprovalPosture, ConnectionSummary, EffectClass, InvokeRequest, OperationDescription,
    OperationError, OperationErrorCode, MAX_RESULT_BYTES,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use service::PrincipalContext;

use super::{operation_invalid, operation_unavailable, KubernetesLocalBackend};
use crate::local_workloads::{operation_from_datasource, KubeconfigReader};
use crate::workloads::valid_dns_label;

pub(super) const NAMESPACE_OPERATION: &str = "kubernetes.namespace.list";
pub(super) const WORKLOAD_OPERATION: &str = "kubernetes.workload.list";
const MAX_LIST_LIMIT: u16 = 100;
const RESULT_HEADROOM: usize = 4096;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NamespaceInput {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkloadInput {
    namespace: String,
    #[serde(default = "default_limit")]
    limit: u16,
    #[serde(default)]
    cursor: Option<String>,
}

const fn default_limit() -> u16 {
    25
}

#[derive(Serialize)]
struct ContainerImage {
    name: String,
    image: String,
}

#[derive(Serialize)]
struct DeploymentSummary {
    name: String,
    containers: Vec<ContainerImage>,
    desired_replicas: i32,
    ready_replicas: i32,
}

#[derive(Serialize)]
struct NamespaceInventory<'a> {
    connection_ref: &'a str,
    namespaces: Vec<String>,
}

#[derive(Serialize)]
struct WorkloadInventory<'a> {
    connection_ref: &'a str,
    namespace: &'a str,
    deployments: Vec<DeploymentSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_cursor: Option<String>,
}

fn valid_namespace(namespace: &str) -> bool {
    valid_dns_label(namespace, 63) && !namespace.contains('.')
}

fn bounded_value(value: impl Serialize) -> Result<Value, OperationError> {
    let value = serde_json::to_value(value).map_err(|_| operation_unavailable())?;
    if serde_json::to_vec(&value)
        .map_err(|_| operation_unavailable())?
        .len()
        > MAX_RESULT_BYTES - RESULT_HEADROOM
    {
        return Err(OperationError::new(
            OperationErrorCode::ResultTooLarge,
            "Kubernetes inventory exceeds the result bound; request a smaller workload limit",
            false,
        ));
    }
    Ok(value)
}

impl KubernetesLocalBackend {
    /// The caller has already passed owner, activation and description-lease admission.
    pub(super) async fn inventory_output(
        &self,
        context: &PrincipalContext,
        request: &InvokeRequest,
        reader: &KubeconfigReader,
    ) -> Result<Value, OperationError> {
        let namespaces = self
            .policy
            .namespaces
            .iter()
            .filter(|namespace| valid_namespace(namespace))
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        if request.operation_ref == NAMESPACE_OPERATION {
            let _: NamespaceInput =
                serde_json::from_value(request.input.clone()).map_err(|_| operation_invalid())?;
            return bounded_value(NamespaceInventory {
                connection_ref: &request.connection_ref,
                namespaces: namespaces.into_iter().collect(),
            });
        }
        let input: WorkloadInput =
            serde_json::from_value(request.input.clone()).map_err(|_| operation_invalid())?;
        if !valid_namespace(&input.namespace)
            || !(1..=MAX_LIST_LIMIT).contains(&input.limit)
            || input
                .cursor
                .as_deref()
                .is_some_and(|cursor| cursor.is_empty() || cursor.len() > 512)
        {
            return Err(operation_invalid());
        }
        if !namespaces.contains(&input.namespace) {
            return Err(OperationError::new(
                OperationErrorCode::NotGranted,
                "namespace is not admitted by this local Kubernetes configuration; list admitted namespaces first",
                false,
            ));
        }
        // Namespace alone is insufficient when a caller has several activated clusters. The
        // domain-separated key also prevents replay through the separate datasource protocol.
        let cursor_scope = format!(
            "{WORKLOAD_OPERATION}\0{}\0{}",
            request.connection_ref, input.namespace
        );
        let cursors = &self.workloads.inventory_cursors;
        let provider_cursor = cursors
            .resolve(context, &cursor_scope, input.cursor.as_deref())
            .map_err(operation_from_datasource)?;
        let (deployments, provider_cursor) = reader
            .list_deployments(&input.namespace, input.limit, provider_cursor.as_deref())
            .await
            .map_err(operation_from_datasource)?;
        let deployments = deployments
            .into_iter()
            .map(|deployment| DeploymentSummary {
                name: deployment.metadata.name,
                containers: deployment
                    .spec
                    .template
                    .spec
                    .containers
                    .into_iter()
                    .map(|container| ContainerImage {
                        name: container.name,
                        image: container.image,
                    })
                    .collect(),
                desired_replicas: deployment.spec.replicas,
                ready_replicas: deployment.status.ready_replicas,
            })
            .collect();
        let next_cursor = cursors
            .store(context, &cursor_scope, provider_cursor)
            .map_err(operation_from_datasource)?;
        bounded_value(WorkloadInventory {
            connection_ref: &request.connection_ref,
            namespace: &input.namespace,
            deployments,
            next_cursor,
        })
    }
}

pub(super) fn namespace_operation(
    connections: Vec<ConnectionSummary>,
    description_ref: String,
) -> OperationDescription {
    OperationDescription {
        operation_ref: NAMESPACE_OPERATION.to_owned(),
        title: "List admitted Kubernetes namespaces".to_owned(),
        description: "Lists namespaces admitted by this local configuration for the selected activated Connection. An empty list admits none; Kubernetes RBAC is checked when workloads are read. Does not enumerate the cluster.".to_owned(),
        input_schema: json!({"type": "object", "additionalProperties": false, "properties": {}}),
        output_schema: json!({
            "type": "object", "additionalProperties": false,
            "required": ["connection_ref", "namespaces"],
            "properties": {
                "connection_ref": {"type": "string"},
                "namespaces": {"type": "array", "items": {"type": "string"}}
            }
        }),
        effect: EffectClass::ReadOnly,
        approval: ApprovalPosture::NotRequired,
        connections,
        description_ref,
    }
}

pub(super) fn workload_operation(
    connections: Vec<ConnectionSummary>,
    description_ref: String,
) -> OperationDescription {
    OperationDescription {
        operation_ref: WORKLOAD_OPERATION.to_owned(),
        title: "List Kubernetes deployment inventory".to_owned(),
        description: "Lists Deployment names, template container images, and desired/ready replicas in one admitted namespace of the selected activated Connection. Bounded pages may be short; use next_cursor for the next page. No Pod or Secret reads.".to_owned(),
        input_schema: json!({
            "type": "object", "additionalProperties": false, "required": ["namespace"],
            "properties": {
                "namespace": {"type": "string", "minLength": 1, "maxLength": 63, "pattern": "^[a-z0-9]([-a-z0-9]*[a-z0-9])?$"},
                "limit": {"type": "integer", "minimum": 1, "maximum": MAX_LIST_LIMIT, "default": default_limit()},
                "cursor": {"type": "string", "minLength": 1, "maxLength": 512}
            }
        }),
        output_schema: json!({
            "type": "object", "additionalProperties": false,
            "required": ["connection_ref", "namespace", "deployments"],
            "properties": {
                "connection_ref": {"type": "string"}, "namespace": {"type": "string"},
                "next_cursor": {"type": "string"},
                "deployments": {"type": "array", "maxItems": MAX_LIST_LIMIT, "items": {
                    "type": "object", "additionalProperties": false,
                    "required": ["name", "containers", "desired_replicas", "ready_replicas"],
                    "properties": {
                        "name": {"type": "string"}, "desired_replicas": {"type": "integer"}, "ready_replicas": {"type": "integer"},
                        "containers": {"type": "array", "items": {
                            "type": "object", "additionalProperties": false, "required": ["name", "image"],
                            "properties": {"name": {"type": "string"}, "image": {"type": "string"}}
                        }}
                    }
                }}
            }
        }),
        effect: EffectClass::ReadOnly,
        approval: ApprovalPosture::NotRequired,
        connections,
        description_ref,
    }
}

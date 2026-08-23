//! Toolset v1: the static, versioned table behind the three MCP meta-tools (design 14, S-053).
//!
//! `tools/list` never grows: the projected names below are data — returned by `tool_search`,
//! described by `tool_describe`, accepted by `tool_invoke`. Role projection is derived, never
//! re-implemented: an entry exists for a caller only while that caller's own seam results
//! satisfy its requirement, so a renamed or withdrawn underlying operation degrades to a
//! hidden tool, never to a bypass. Every read and invocation goes back through the decided
//! admission seams in `hosted.rs`, and description leases stay server-side: `tool_describe`
//! never returns one, and `tool_invoke` re-describes before dispatch, retrying exactly once
//! when the seam answers `stale_authority`.

use std::collections::BTreeSet;

use protocol::datasource::{
    BindingSearchRequest, DatasourceBinding, DatasourceDescription, DatasourceError,
    DatasourceErrorCode, DatasourcePage, DatasourceRead, DatasourceRequest, DatasourceResult,
    DescribeRequest as DatasourceDescribeRequest, ReadRequest,
    RequestEnvelope as DatasourceRequestEnvelope, ResponseEnvelope as DatasourceResponseEnvelope,
};
use protocol::operation::{
    ApprovalPosture, DescribeRequest, InvokeRequest, OperationDescription, OperationError,
    OperationErrorCode, OperationRequest, OperationResult, OwnerContext, RequestEnvelope,
    ResponseEnvelope, SearchRequest,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::hosted::{HostedPrincipal, HostedState};

/// Published toolset identity, returned beside every `tool_search` result.
const TOOLSET_VERSION: &str = "v1";

/// The datasource every datasource-backed entry reads through the caller's own bindings.
const WORKLOADS_DATASOURCE: &str = "kubernetes.workloads";
/// The deployment's one in-cluster Connection, named by every op-backed entry.
const KUBERNETES_CONNECTION: &str = "connection:kubernetes:in-cluster";
/// The seam search that discovers the caller's admitted kubernetes operations.
const REQUIREMENT_QUERY: &str = "kubernetes";

/// What must hold in the caller's own seam results for a table entry to exist for that caller.
enum Requirement {
    /// The named operation appears in the caller's own operation search.
    Operation(&'static str),
    /// The caller's own `kubernetes.workloads` binding list is non-empty.
    WorkloadsBinding,
}

/// Where an entry routes when invoked — always back through the decided seams in `hosted.rs`.
enum Target {
    /// Server-side describe, then invoke, of one hosted operation with the args as input.
    OperationInvoke {
        operation_ref: &'static str,
        connection_ref: &'static str,
    },
    /// Project the caller's admitted workload bindings as namespaces.
    DatasourceBindings,
    /// Bounded compact list through the named namespace's binding.
    DatasourceList,
    /// Exact detail read through the named namespace's binding.
    DatasourceGet,
}

/// One projected tool.
struct McpTool {
    name: &'static str,
    title: &'static str,
    description: &'static str,
    requires: Requirement,
    target: Target,
    input_schema: fn() -> Value,
}

/// Toolset v1 (design 14). `k8s_pod_logs` is deliberately present before S-054 lands
/// `kubernetes.pod.logs`: its requirement hides it until the operation exists, which keeps the
/// two stories order-independent.
const TOOLSET: &[McpTool] = &[
    McpTool {
        name: "k8s_namespace_list",
        title: "List admitted namespaces",
        description: "List the Kubernetes namespaces the caller's authority admits reading.",
        requires: Requirement::WorkloadsBinding,
        target: Target::DatasourceBindings,
        input_schema: no_args_schema,
    },
    McpTool {
        name: "k8s_deployment_list",
        title: "List deployments in a namespace",
        description: "List deployment rollout summaries in one admitted Kubernetes namespace.",
        requires: Requirement::WorkloadsBinding,
        target: Target::DatasourceList,
        input_schema: deployment_list_schema,
    },
    McpTool {
        name: "k8s_deployment_status",
        title: "Read one deployment's status",
        description: "Read rollout status for one deployment in an admitted namespace.",
        requires: Requirement::Operation("kubernetes.deployment.status"),
        target: Target::OperationInvoke {
            operation_ref: "kubernetes.deployment.status",
            connection_ref: KUBERNETES_CONNECTION,
        },
        input_schema: deployment_status_schema,
    },
    McpTool {
        name: "k8s_pod_status",
        title: "Read pod status for one deployment",
        description: "Read per-pod and per-container status for one deployment in an admitted \
                      namespace.",
        requires: Requirement::WorkloadsBinding,
        target: Target::DatasourceGet,
        input_schema: pod_status_schema,
    },
    McpTool {
        name: "k8s_pod_logs",
        title: "Read one pod's log tail",
        description: "Read the tail of one pod's logs in an admitted namespace.",
        requires: Requirement::Operation("kubernetes.pod.logs"),
        target: Target::OperationInvoke {
            operation_ref: "kubernetes.pod.logs",
            connection_ref: KUBERNETES_CONNECTION,
        },
        input_schema: pod_logs_schema,
    },
];

fn no_args_schema() -> Value {
    json!({ "type": "object", "additionalProperties": false, "properties": {} })
}

fn deployment_list_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["namespace"],
        "properties": {
            "namespace": { "type": "string", "minLength": 1 },
            "limit": { "type": "integer", "minimum": 1, "maximum": 25 },
            "cursor": { "type": "string", "minLength": 1 }
        }
    })
}

fn deployment_status_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["namespace", "name"],
        "properties": {
            "namespace": { "type": "string", "minLength": 1 },
            "name": { "type": "string", "minLength": 1 }
        }
    })
}

fn pod_status_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["namespace", "deployment"],
        "properties": {
            "namespace": { "type": "string", "minLength": 1 },
            "deployment": { "type": "string", "minLength": 1, "maxLength": 253 }
        }
    })
}

fn pod_logs_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["namespace", "pod"],
        "properties": {
            "namespace": { "type": "string", "minLength": 1 },
            "pod": { "type": "string", "minLength": 1 },
            "container": { "type": "string", "minLength": 1 },
            "tail_lines": { "type": "integer", "minimum": 1, "maximum": 1000, "default": 200 },
            "since_seconds": { "type": "integer", "minimum": 1 }
        }
    })
}

/// The whole `tools/list` surface: exactly the three meta-tools, with the projected toolset
/// behind them as data.
pub(super) fn meta_tools() -> Value {
    json!([
        {
            "name": "tool_search",
            "title": "Search the projected toolset",
            "description": "Search the tools the caller's own authority currently supports. \
                            Returns projected tool names for tool_describe and tool_invoke.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "query": { "type": "string", "maxLength": 512 },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 25 }
                }
            }
        },
        {
            "name": "tool_describe",
            "title": "Describe one projected tool",
            "description": "Describe one projected tool: input and output schemas, effect \
                            class, and approval posture.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "required": ["name"],
                "properties": { "name": { "type": "string", "minLength": 1 } }
            }
        },
        {
            "name": "tool_invoke",
            "title": "Invoke one projected tool",
            "description": "Invoke one projected tool by name. A tool demanding approval takes \
                            the approval evidence reference beside the args.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "required": ["name", "args"],
                "properties": {
                    "name": { "type": "string", "minLength": 1 },
                    "args": { "type": "object" },
                    "approval_evidence_ref": {
                        "type": "string", "minLength": 1, "maxLength": 512
                    }
                }
            }
        }
    ])
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SearchArgs {
    #[serde(default)]
    query: Option<String>,
    #[serde(default)]
    limit: Option<u16>,
}

/// `tool_search`: the caller's own view of the toolset, filtered by their query.
pub(super) async fn tool_search(
    state: &HostedState,
    principal: &HostedPrincipal,
    request_id: &str,
    arguments: &Value,
) -> Result<Value, String> {
    let args: SearchArgs = meta_args(arguments)?;
    let limit = args.limit.unwrap_or(25);
    if !(1..=25).contains(&limit) {
        return Err("limit must be between 1 and 25".to_owned());
    }
    let view = match seam_view(state, principal, request_id).await {
        Ok(view) => view,
        Err(refused) => return Ok(refused),
    };
    let query = args.query.unwrap_or_default().to_ascii_lowercase();
    let tools: Vec<Value> = TOOLSET
        .iter()
        .filter(|tool| view.admits(&tool.requires))
        .filter(|tool| {
            query.is_empty()
                || tool.name.contains(&query)
                || tool.title.to_ascii_lowercase().contains(&query)
                || tool.description.to_ascii_lowercase().contains(&query)
        })
        .take(usize::from(limit))
        .map(|tool| {
            json!({ "name": tool.name, "title": tool.title, "description": tool.description })
        })
        .collect();
    Ok(success(&json!({
        "toolset_version": TOOLSET_VERSION,
        "tools": tools,
    })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DescribeArgs {
    name: String,
}

/// `tool_describe`: the projection of one entry — schemas, effect, approval — with the
/// description lease deliberately absent. A hidden entry answers exactly as an absent one.
pub(super) async fn tool_describe(
    state: &HostedState,
    principal: &HostedPrincipal,
    request_id: &str,
    arguments: &Value,
) -> Result<Value, String> {
    let args: DescribeArgs = meta_args(arguments)?;
    let Some(tool) = TOOLSET.iter().find(|tool| tool.name == args.name) else {
        return Ok(not_found("no such tool"));
    };
    let view = match seam_view(state, principal, request_id).await {
        Ok(view) => view,
        Err(refused) => return Ok(refused),
    };
    if !view.admits(&tool.requires) {
        return Ok(not_found("no such tool"));
    }
    let (effect, approval, output_schema) = match &tool.target {
        Target::OperationInvoke { operation_ref, .. } => {
            let description =
                match describe_operation(state, principal, request_id, operation_ref).await {
                    Ok(description) => description,
                    Err(error) => return Ok(operation_refusal(&error)),
                };
            (
                serde_json::to_value(description.effect).expect("closed effect vocabulary"),
                serde_json::to_value(description.approval).expect("closed approval vocabulary"),
                description.output_schema,
            )
        }
        Target::DatasourceBindings => (
            json!("read_only"),
            json!("not_required"),
            json!({
                "type": "object",
                "additionalProperties": false,
                "required": ["namespaces"],
                "properties": {
                    "namespaces": { "type": "array", "items": { "type": "string" } }
                }
            }),
        ),
        Target::DatasourceList => {
            let description = match describe_workloads(state, principal, request_id).await {
                Ok(description) => description,
                Err(error) => return Ok(datasource_refusal(&error)),
            };
            (
                json!("read_only"),
                json!("not_required"),
                json!({
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["deployments", "completeness"],
                    "properties": {
                        "deployments": { "type": "array", "items": description.compact_schema },
                        "next_cursor": { "type": "string" },
                        "completeness": { "enum": ["complete", "partial"] }
                    }
                }),
            )
        }
        Target::DatasourceGet => {
            let description = match describe_workloads(state, principal, request_id).await {
                Ok(description) => description,
                Err(error) => return Ok(datasource_refusal(&error)),
            };
            (
                json!("read_only"),
                json!("not_required"),
                description.detail_schema,
            )
        }
    };
    Ok(success(&json!({
        "name": tool.name,
        "title": tool.title,
        "description": tool.description,
        "input_schema": (tool.input_schema)(),
        "output_schema": output_schema,
        "effect": effect,
        "approval": approval,
    })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InvokeArgs {
    name: String,
    args: Value,
    #[serde(default)]
    approval_evidence_ref: Option<String>,
}

/// `tool_invoke`: route one projected tool back through the decided seams. Op-backed entries
/// re-describe server-side and invoke under the fresh lease; datasource-backed entries resolve
/// the caller's own binding and lease the same way.
pub(super) async fn tool_invoke(
    state: &HostedState,
    principal: &HostedPrincipal,
    request_id: &str,
    arguments: &Value,
) -> Result<Value, String> {
    let invoke: InvokeArgs = meta_args(arguments)?;
    if !invoke.args.is_object() {
        return Err("args must be an object".to_owned());
    }
    let Some(tool) = TOOLSET.iter().find(|tool| tool.name == invoke.name) else {
        return Ok(not_found("no such tool"));
    };
    Ok(match &tool.target {
        Target::OperationInvoke {
            operation_ref,
            connection_ref,
        } => {
            invoke_operation(
                state,
                principal,
                request_id,
                operation_ref,
                connection_ref,
                &invoke.args,
                invoke.approval_evidence_ref.as_deref(),
            )
            .await
        }
        Target::DatasourceBindings => list_namespaces(state, principal, request_id).await,
        Target::DatasourceList => {
            list_deployments(state, principal, request_id, &invoke.args).await
        }
        Target::DatasourceGet => pod_status(state, principal, request_id, &invoke.args).await,
    })
}

/// Describe, then invoke, one hosted operation under the fresh server-side lease. On
/// `stale_authority` the description is silently refreshed exactly once before the refusal
/// surfaces.
async fn invoke_operation(
    state: &HostedState,
    principal: &HostedPrincipal,
    request_id: &str,
    operation_ref: &str,
    connection_ref: &str,
    input: &Value,
    evidence: Option<&str>,
) -> Value {
    let mut description =
        match describe_operation(state, principal, request_id, operation_ref).await {
            Ok(description) => description,
            Err(error) => return operation_refusal(&error),
        };
    let mut retried = false;
    loop {
        if description.approval == ApprovalPosture::Required && evidence.is_none() {
            // Refused here with the honest axis so the caller can obtain evidence; the seam
            // itself would refuse the same invocation without naming the axis.
            return refusal(
                &json!("approval_required"),
                "the operation demands an approval; pass approval_evidence_ref",
                false,
            );
        }
        let attempt = operation_seam(
            state,
            principal,
            request_id,
            OperationRequest::Invoke(InvokeRequest {
                operation_ref: operation_ref.to_owned(),
                connection_ref: connection_ref.to_owned(),
                description_ref: description.description_ref.clone(),
                input: input.clone(),
                approval_evidence_ref: evidence.map(str::to_owned),
            }),
        )
        .await;
        match attempt {
            Ok(OperationResult::Invoke(result)) => return success(&result.output),
            Ok(_) => return operation_refusal(&operation_seam_gap()),
            Err(error) if error.code == OperationErrorCode::StaleAuthority && !retried => {
                retried = true;
                description =
                    match describe_operation(state, principal, request_id, operation_ref).await {
                        Ok(description) => description,
                        Err(error) => return operation_refusal(&error),
                    };
            }
            Err(error) => return operation_refusal(&error),
        }
    }
}

/// `k8s_namespace_list`: the caller's own admitted workload bindings, projected as namespaces.
async fn list_namespaces(
    state: &HostedState,
    principal: &HostedPrincipal,
    request_id: &str,
) -> Value {
    match workload_bindings(state, principal, request_id, "").await {
        Ok(bindings) => {
            let namespaces: Vec<&str> = bindings
                .iter()
                .map(|binding| binding.label.as_str())
                .collect();
            success(&json!({ "namespaces": namespaces }))
        }
        Err(error) => datasource_refusal(&error),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeploymentListArgs {
    namespace: String,
    #[serde(default)]
    limit: Option<u16>,
    #[serde(default)]
    cursor: Option<String>,
}

/// `k8s_deployment_list`: one bounded compact read through the namespace's binding.
async fn list_deployments(
    state: &HostedState,
    principal: &HostedPrincipal,
    request_id: &str,
    args: &Value,
) -> Value {
    let args: DeploymentListArgs = match tool_args(args) {
        Ok(args) => args,
        Err(refused) => return refused,
    };
    let limit = args.limit.unwrap_or(25);
    if !(1..=25).contains(&limit) {
        return invalid_args("limit must be between 1 and 25".to_owned());
    }
    let binding = match namespace_binding(state, principal, request_id, &args.namespace).await {
        Ok(binding) => binding,
        Err(refused) => return refused,
    };
    let page = match workloads_read(
        state,
        principal,
        request_id,
        &binding,
        DatasourceRead::List {
            limit,
            cursor: args.cursor,
        },
    )
    .await
    {
        Ok(page) => page,
        Err(refused) => return refused,
    };
    let deployments: Vec<Value> = page
        .records
        .into_iter()
        .map(|record| record.value)
        .collect();
    let mut listed = json!({
        "deployments": deployments,
        "completeness": serde_json::to_value(page.completeness)
            .expect("closed completeness vocabulary"),
    });
    if let Some(cursor) = page.next_cursor {
        listed["next_cursor"] = Value::String(cursor);
    }
    success(&listed)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PodStatusArgs {
    namespace: String,
    deployment: String,
}

/// `k8s_pod_status`: one exact detail read through the namespace's binding, keyed by the
/// workload key schema's `{"name": <deployment>}`.
async fn pod_status(
    state: &HostedState,
    principal: &HostedPrincipal,
    request_id: &str,
    args: &Value,
) -> Value {
    let args: PodStatusArgs = match tool_args(args) {
        Ok(args) => args,
        Err(refused) => return refused,
    };
    let binding = match namespace_binding(state, principal, request_id, &args.namespace).await {
        Ok(binding) => binding,
        Err(refused) => return refused,
    };
    // Exactly the shape `workload_key_schema()` declares in the kubernetes Integration: one
    // required `name` string and nothing else.
    let page = match workloads_read(
        state,
        principal,
        request_id,
        &binding,
        DatasourceRead::Get {
            key: json!({ "name": args.deployment }),
        },
    )
    .await
    {
        Ok(page) => page,
        Err(refused) => return refused,
    };
    match page.records.into_iter().next() {
        Some(record) => success(&record.value),
        None => not_found("the deployment is not in the namespace's workload records"),
    }
}

/// The caller's own seam results: which operations their search returns and whether they hold
/// a workload binding. Both drive requirement checks; nothing here is a policy decision.
struct SeamView {
    operations: BTreeSet<String>,
    has_binding: bool,
}

impl SeamView {
    fn admits(&self, requirement: &Requirement) -> bool {
        match requirement {
            Requirement::Operation(operation_ref) => self.operations.contains(*operation_ref),
            Requirement::WorkloadsBinding => self.has_binding,
        }
    }
}

async fn seam_view(
    state: &HostedState,
    principal: &HostedPrincipal,
    request_id: &str,
) -> Result<SeamView, Value> {
    let searched = operation_seam(
        state,
        principal,
        request_id,
        OperationRequest::Search(SearchRequest {
            query: REQUIREMENT_QUERY.to_owned(),
            limit: 25,
        }),
    )
    .await;
    let operations = match searched {
        Ok(OperationResult::Search { operations }) => operations
            .into_iter()
            .map(|operation| operation.operation_ref)
            .collect(),
        Ok(_) => return Err(operation_refusal(&operation_seam_gap())),
        Err(error) => return Err(operation_refusal(&error)),
    };
    let has_binding = match workload_bindings(state, principal, request_id, "").await {
        Ok(bindings) => !bindings.is_empty(),
        // A deployment without the workloads datasource hides the entries; it never errors
        // the projection.
        Err(error) if error.code == DatasourceErrorCode::NotFound => false,
        Err(error) => return Err(datasource_refusal(&error)),
    };
    Ok(SeamView {
        operations,
        has_binding,
    })
}

/// The caller's admitted `kubernetes.workloads` bindings, through the datasource seam.
async fn workload_bindings(
    state: &HostedState,
    principal: &HostedPrincipal,
    request_id: &str,
    query: &str,
) -> Result<Vec<DatasourceBinding>, DatasourceError> {
    match datasource_seam(
        state,
        principal,
        request_id,
        DatasourceRequest::Bindings(BindingSearchRequest {
            datasource_ref: WORKLOADS_DATASOURCE.to_owned(),
            query: query.to_owned(),
            limit: 25,
        }),
    )
    .await?
    {
        DatasourceResult::Bindings { bindings } => Ok(bindings),
        _ => Err(datasource_seam_gap()),
    }
}

/// Resolve the caller's binding for one namespace, or the honest absence: a namespace outside
/// the caller's bindings is indistinguishable from one that does not exist.
async fn namespace_binding(
    state: &HostedState,
    principal: &HostedPrincipal,
    request_id: &str,
    namespace: &str,
) -> Result<DatasourceBinding, Value> {
    let bindings = workload_bindings(state, principal, request_id, namespace)
        .await
        .map_err(|error| datasource_refusal(&error))?;
    bindings
        .into_iter()
        .find(|binding| binding.label == namespace)
        .ok_or_else(|| not_found("the namespace is not among the caller's admitted bindings"))
}

/// One read through the caller's binding under a server-side description lease, re-described
/// exactly once when the seam answers `stale_authority`.
async fn workloads_read(
    state: &HostedState,
    principal: &HostedPrincipal,
    request_id: &str,
    binding: &DatasourceBinding,
    read: DatasourceRead,
) -> Result<DatasourcePage, Value> {
    let mut description = describe_workloads(state, principal, request_id)
        .await
        .map_err(|error| datasource_refusal(&error))?;
    let mut retried = false;
    loop {
        let attempt = datasource_seam(
            state,
            principal,
            request_id,
            DatasourceRequest::Read(ReadRequest {
                datasource_ref: WORKLOADS_DATASOURCE.to_owned(),
                binding_ref: binding.binding_ref.clone(),
                description_ref: description.description_ref.clone(),
                read: read.clone(),
            }),
        )
        .await;
        match attempt {
            Ok(DatasourceResult::Read(page)) => return Ok(page),
            Ok(_) => return Err(datasource_refusal(&datasource_seam_gap())),
            Err(error) if error.code == DatasourceErrorCode::StaleAuthority && !retried => {
                retried = true;
                description = describe_workloads(state, principal, request_id)
                    .await
                    .map_err(|error| datasource_refusal(&error))?;
            }
            Err(error) => return Err(datasource_refusal(&error)),
        }
    }
}

async fn describe_operation(
    state: &HostedState,
    principal: &HostedPrincipal,
    request_id: &str,
    operation_ref: &str,
) -> Result<OperationDescription, OperationError> {
    match operation_seam(
        state,
        principal,
        request_id,
        OperationRequest::Describe(DescribeRequest {
            operation_ref: operation_ref.to_owned(),
        }),
    )
    .await?
    {
        OperationResult::Describe(description) => Ok(description),
        _ => Err(operation_seam_gap()),
    }
}

async fn describe_workloads(
    state: &HostedState,
    principal: &HostedPrincipal,
    request_id: &str,
) -> Result<DatasourceDescription, DatasourceError> {
    match datasource_seam(
        state,
        principal,
        request_id,
        DatasourceRequest::Describe(DatasourceDescribeRequest {
            datasource_ref: WORKLOADS_DATASOURCE.to_owned(),
        }),
    )
    .await?
    {
        DatasourceResult::Describe(description) => Ok(description),
        _ => Err(datasource_seam_gap()),
    }
}

/// The owner context every synthesized envelope carries: the verified principal's own facts,
/// with the MCP entry marked in the agent identity.
fn owner_context(principal: &HostedPrincipal) -> OwnerContext {
    OwnerContext {
        tenant_id: principal.tenant_id.clone(),
        agent_id: format!("mcp:{}", principal.subject),
        agent_revision: 1,
        authority_snapshot_id: principal.token_id.clone(),
        authority_snapshot_sha256: principal.authority_snapshot_sha256.clone(),
    }
}

/// One operation request through the decided admission seam, parsed back out of its HTTP
/// response. The seam is the same `hosted.rs` code the `/operations` route runs after bearer
/// verification — the scope map, the receiver policy, re-description, and Grant/approval
/// admission all apply unchanged.
async fn operation_seam(
    state: &HostedState,
    principal: &HostedPrincipal,
    request_id: &str,
    request: OperationRequest,
) -> Result<OperationResult, OperationError> {
    let envelope = RequestEnvelope {
        protocol: protocol::operation::CONTRACT.to_owned(),
        request_id: request_id.to_owned(),
        context: owner_context(principal),
        request,
    };
    envelope.validate()?;
    let response = crate::hosted::operation_decided(state, principal, envelope).await;
    let body = axum::body::to_bytes(response.into_body(), protocol::operation::MAX_FRAME_BYTES)
        .await
        .map_err(|_| operation_seam_gap())?;
    let envelope: ResponseEnvelope =
        serde_json::from_slice(&body).map_err(|_| operation_seam_gap())?;
    match (envelope.response, envelope.error) {
        (Some(result), None) => Ok(result),
        (None, Some(refusal)) => Err(refusal),
        _ => Err(operation_seam_gap()),
    }
}

fn operation_seam_gap() -> OperationError {
    OperationError::new(
        OperationErrorCode::Unavailable,
        "the admission seam answered outside the operation contract",
        false,
    )
}

/// The datasource twin of [`operation_seam`].
async fn datasource_seam(
    state: &HostedState,
    principal: &HostedPrincipal,
    request_id: &str,
    request: DatasourceRequest,
) -> Result<DatasourceResult, DatasourceError> {
    let envelope = DatasourceRequestEnvelope {
        protocol: protocol::datasource::CONTRACT.to_owned(),
        request_id: request_id.to_owned(),
        context: owner_context(principal),
        request,
    };
    envelope.validate()?;
    let response = crate::hosted::datasource_decided(state, principal, envelope).await;
    let body = axum::body::to_bytes(response.into_body(), protocol::datasource::MAX_FRAME_BYTES)
        .await
        .map_err(|_| datasource_seam_gap())?;
    let envelope: DatasourceResponseEnvelope =
        serde_json::from_slice(&body).map_err(|_| datasource_seam_gap())?;
    match (envelope.response, envelope.error) {
        (Some(result), None) => Ok(result),
        (None, Some(refusal)) => Err(refusal),
        _ => Err(datasource_seam_gap()),
    }
}

fn datasource_seam_gap() -> DatasourceError {
    DatasourceError::new(
        DatasourceErrorCode::Unavailable,
        "the admission seam answered outside the datasource contract",
        false,
    )
}

/// Parse one meta-tool's own arguments; a failure here is the JSON-RPC caller's `-32602`.
fn meta_args<T: serde::de::DeserializeOwned>(arguments: &Value) -> Result<T, String> {
    serde_json::from_value(arguments.clone()).map_err(|parse| format!("invalid arguments: {parse}"))
}

/// Parse one projected tool's args; a failure here is a tool-level `invalid_input` result.
fn tool_args<T: serde::de::DeserializeOwned>(args: &Value) -> Result<T, Value> {
    serde_json::from_value(args.clone())
        .map_err(|parse| invalid_args(format!("invalid args: {parse}")))
}

/// One successful tools/call result: `structuredContent` plus the spec-recommended text block
/// carrying the same JSON.
fn success(output: &Value) -> Value {
    json!({
        "content": [{ "type": "text", "text": output.to_string() }],
        "structuredContent": output,
        "isError": false,
    })
}

/// One refused tools/call result: the protocol's own error code, snake_case, in
/// `structuredContent`, and the message as the text block.
fn refusal(code: &Value, message: &str, retriable: bool) -> Value {
    json!({
        "content": [{ "type": "text", "text": message }],
        "structuredContent": { "code": code, "message": message, "retriable": retriable },
        "isError": true,
    })
}

fn operation_refusal(error: &OperationError) -> Value {
    refusal(
        &serde_json::to_value(error.code).expect("closed error vocabulary serializes"),
        &error.message,
        error.retriable,
    )
}

fn datasource_refusal(error: &DatasourceError) -> Value {
    refusal(
        &serde_json::to_value(error.code).expect("closed error vocabulary serializes"),
        &error.message,
        error.retriable,
    )
}

fn not_found(message: &str) -> Value {
    refusal(&json!("not_found"), message, false)
}

fn invalid_args(message: String) -> Value {
    refusal(&json!("invalid_input"), &message, false)
}

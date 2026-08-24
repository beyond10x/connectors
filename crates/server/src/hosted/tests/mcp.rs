//! S-053: the hosted server speaks MCP through the admission seam (design 14).
//!
//! These tests drive the stateless `POST /mcp` JSON-RPC surface end to end: bearer
//! verification before any JSON-RPC processing, the three-meta-tool surface, role projection
//! derived from the caller's own seam results, the server-side describe→invoke lease flow with
//! one silent stale-authority retry, datasource-backed routing, and the designed transport
//! refusals. The backend is a fake advertising exactly the kubernetes operations and workload
//! bindings the toolset targets, filtered by the verified groups the seam hands it.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use connector_state::{MemoryState, StateStore};
use domain::{ApprovalRecord, Grant, GrantSet};
use protocol::datasource::{
    AccessMode, Completeness, DatasourceBinding, DatasourceDescription, DatasourcePage,
    DatasourceProvenance, DatasourceRead, DatasourceRecord, DatasourceSummary, ReadVerb,
    RecordView,
};
use serde_json::{json, Value};

use super::*;

const K8S_OPERATION: &str = "kubernetes.deployment.status";
const K8S_CONNECTION: &str = "connection:kubernetes:in-cluster";
const WORKLOADS: &str = "kubernetes.workloads";
const WORKLOADS_BINDING: &str = "binding:kubernetes:dev";
const WORKLOADS_LEASE: &str = "ds-lease-1";
const OPERATION_LEASE: &str = "lease-1";
const ISSUER: &str = "https://identity.example.test";

/// Verifies three fixed principals, distinguished by bearer: a kubernetes read-group caller
/// with both self-service scopes, the same caller without any group, and a read-group caller
/// holding only the catalog scope.
struct McpVerifier;

#[async_trait]
impl IdentityVerifier for McpVerifier {
    async fn ready(&self) -> Result<(), IdentityVerificationError> {
        Ok(())
    }

    async fn verify(
        &self,
        credential: &str,
        audience: &str,
    ) -> Result<HostedPrincipal, IdentityVerificationError> {
        if audience != CONNECTORS_AUDIENCE {
            return Err(IdentityVerificationError::Refused);
        }
        let (groups, scopes): (&[&str], &[&str]) = match credential {
            "sre-token" => (&["sre"], &["connectors.catalog.read", "connectors.invoke"]),
            "groupless-token" => (&[], &["connectors.catalog.read", "connectors.invoke"]),
            "reader-token" => (&["sre"], &["connectors.catalog.read"]),
            _ => return Err(IdentityVerificationError::Refused),
        };
        Ok(HostedPrincipal {
            issuer: ISSUER.to_owned(),
            tenant_id: "tenant-dev".to_owned(),
            subject: "person:test".to_owned(),
            email: None,
            actor_subject: "person:test".to_owned(),
            token_id: "token-mcp".to_owned(),
            scopes: scopes.iter().map(|scope| (*scope).to_owned()).collect(),
            groups: groups.iter().map(|group| (*group).to_owned()).collect(),
            authority_snapshot_sha256: "b".repeat(64),
            deployment_id: None,
        })
    }
}

/// A backend owning one kubernetes operation and one workload binding, both visible only to
/// the read groups — exactly what the toolset's requirement mechanism consumes. The invoke arm
/// proves the seam flow: it refuses any lease other than the one its describe served, and
/// optionally answers `stale_authority` a configured number of times first.
struct McpBackend {
    /// Describe the operation as `Mutating` + `Required` instead of the read shape.
    demand_approval: bool,
    /// How many leading invokes answer `stale_authority` despite a fresh lease.
    stale_invokes: AtomicUsize,
    describes: AtomicUsize,
    invokes: AtomicUsize,
    /// The `approval_evidence_ref` the last dispatched invoke carried.
    evidence: Mutex<Option<Option<String>>>,
    /// Every datasource get key that reached the read arm.
    get_keys: Mutex<Vec<Value>>,
    /// How many workload records the list arm serves, across protocol pages of `limit`.
    deployments: usize,
}

impl Default for McpBackend {
    fn default() -> Self {
        Self {
            demand_approval: false,
            stale_invokes: AtomicUsize::new(0),
            describes: AtomicUsize::new(0),
            invokes: AtomicUsize::new(0),
            evidence: Mutex::new(None),
            get_keys: Mutex::new(Vec::new()),
            deployments: 1,
        }
    }
}

impl McpBackend {
    fn visible(context: &PrincipalContext) -> bool {
        context
            .verified_groups()
            .iter()
            .any(|group| group == "sre" || group == "operator")
    }

    fn posture(&self) -> (EffectClass, ApprovalPosture) {
        if self.demand_approval {
            (EffectClass::Mutating, ApprovalPosture::Required)
        } else {
            (EffectClass::ReadOnly, ApprovalPosture::NotRequired)
        }
    }

    fn connection() -> protocol::operation::ConnectionSummary {
        protocol::operation::ConnectionSummary {
            connection_ref: K8S_CONNECTION.to_owned(),
            label: "in-cluster".to_owned(),
            provider: "kubernetes".to_owned(),
            audiences: Vec::new(),
            purpose: None,
        }
    }
}

#[async_trait]
impl ConnectorBackend for McpBackend {
    async fn ready(&self) -> Result<(), service::BackendReadinessError> {
        Ok(())
    }

    async fn handle(
        &self,
        context: &PrincipalContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        assert_eq!(context.subject(), "person:test");
        match request {
            OperationRequest::Search(_) => {
                let operations = if Self::visible(context) {
                    let (effect, approval) = self.posture();
                    vec![protocol::operation::OperationSummary {
                        operation_ref: K8S_OPERATION.to_owned(),
                        title: "Read one deployment's status".to_owned(),
                        effect,
                        approval,
                        connections: vec![Self::connection()],
                    }]
                } else {
                    Vec::new()
                };
                Ok(OperationResult::Search { operations })
            }
            OperationRequest::Describe(request) if request.operation_ref == K8S_OPERATION => {
                self.describes.fetch_add(1, Ordering::SeqCst);
                let (effect, approval) = self.posture();
                Ok(OperationResult::Describe(OperationDescription {
                    operation_ref: K8S_OPERATION.to_owned(),
                    title: "Read one deployment's status".to_owned(),
                    description: "Rollout status for one deployment.".to_owned(),
                    input_schema: json!({"type": "object"}),
                    output_schema: json!({"type": "object"}),
                    effect,
                    approval,
                    connections: vec![Self::connection()],
                    description_ref: OPERATION_LEASE.to_owned(),
                }))
            }
            OperationRequest::Describe(_) => Err(OperationError::new(
                OperationErrorCode::NotFound,
                "no such operation",
                false,
            )),
            OperationRequest::Invoke(request) => {
                self.invokes.fetch_add(1, Ordering::SeqCst);
                assert_eq!(request.operation_ref, K8S_OPERATION);
                assert_eq!(request.connection_ref, K8S_CONNECTION);
                if request.description_ref != OPERATION_LEASE {
                    return Err(OperationError::new(
                        OperationErrorCode::StaleAuthority,
                        "the description lease is not the served one",
                        false,
                    ));
                }
                if self.stale_invokes.load(Ordering::SeqCst) > 0 {
                    self.stale_invokes.fetch_sub(1, Ordering::SeqCst);
                    return Err(OperationError::new(
                        OperationErrorCode::StaleAuthority,
                        "authority rotated during the invocation",
                        false,
                    ));
                }
                *self.evidence.lock().expect("evidence lock") =
                    Some(request.approval_evidence_ref.clone());
                Ok(OperationResult::Invoke(InvocationResult {
                    operation_ref: K8S_OPERATION.to_owned(),
                    output: json!({"namespace": "dev", "name": "web", "ready": true}),
                    connector_audit_ref: "audit:test".to_owned(),
                    execution_ref: None,
                }))
            }
            _ => unreachable!("mcp tests send only search, describe and invoke"),
        }
    }

    async fn handle_datasource(
        &self,
        context: &PrincipalContext,
        request: DatasourceRequest,
    ) -> Result<DatasourceResult, DatasourceError> {
        match request {
            DatasourceRequest::Bindings(request) => {
                assert_eq!(request.datasource_ref, WORKLOADS);
                let bindings = if Self::visible(context) {
                    vec![DatasourceBinding {
                        datasource_ref: WORKLOADS.to_owned(),
                        binding_ref: WORKLOADS_BINDING.to_owned(),
                        connection_ref: K8S_CONNECTION.to_owned(),
                        label: "dev".to_owned(),
                        purpose: None,
                        generation: 1,
                    }]
                } else {
                    Vec::new()
                };
                Ok(DatasourceResult::Bindings { bindings })
            }
            DatasourceRequest::Describe(request) => {
                assert_eq!(request.datasource_ref, WORKLOADS);
                Ok(DatasourceResult::Describe(DatasourceDescription {
                    summary: DatasourceSummary {
                        datasource_ref: WORKLOADS.to_owned(),
                        title: "Kubernetes workloads".to_owned(),
                        access_mode: AccessMode::Live,
                        verbs: vec![ReadVerb::List, ReadVerb::Get],
                    },
                    description: "Deployment rollout state per admitted namespace.".to_owned(),
                    // The exact shape `workload_key_schema()` declares in
                    // `crates/integration-kubernetes/src/workloads.rs`: one required `name`
                    // string and nothing else.
                    key_schema: json!({
                        "type": "object",
                        "additionalProperties": false,
                        "required": ["name"],
                        "properties": {
                            "name": {"type": "string", "minLength": 1, "maxLength": 253}
                        }
                    }),
                    compact_schema: json!({"type": "object"}),
                    detail_schema: json!({"type": "object"}),
                    projection_protocol: "b10x.workloads.v1".to_owned(),
                    projection_sha256: "a".repeat(64),
                    description_ref: WORKLOADS_LEASE.to_owned(),
                }))
            }
            DatasourceRequest::Read(request) => {
                assert_eq!(request.datasource_ref, WORKLOADS);
                assert_eq!(request.binding_ref, WORKLOADS_BINDING);
                if request.description_ref != WORKLOADS_LEASE {
                    return Err(DatasourceError::new(
                        DatasourceErrorCode::StaleAuthority,
                        "the datasource lease is not the served one",
                        false,
                    ));
                }
                let (view, key, value) = match request.read {
                    DatasourceRead::List { limit, cursor } => {
                        // The toolset's server-side aggregation drives this arm: full
                        // protocol pages, an opaque offset cursor, `deployments` records in
                        // all. The first record keeps the classic name `web`.
                        assert!((1..=25).contains(&limit));
                        let start: usize = cursor.as_deref().map_or(0, |token| {
                            token
                                .strip_prefix("page-")
                                .expect("the cursor is the one this arm issued")
                                .parse()
                                .expect("the cursor offset is numeric")
                        });
                        let end = self.deployments.min(start + usize::from(limit));
                        let records = (start..end)
                            .map(|index| {
                                let name = if index == 0 {
                                    "web".to_owned()
                                } else {
                                    format!("web-{index:03}")
                                };
                                DatasourceRecord {
                                    key: json!({"name": name}),
                                    view: RecordView::Compact,
                                    value: json!({
                                        "name": name,
                                        "desired_replicas": 2,
                                        "ready_replicas": 2,
                                        "rollout_state": "available",
                                    }),
                                }
                            })
                            .collect();
                        return Ok(DatasourceResult::Read(DatasourcePage {
                            datasource_ref: WORKLOADS.to_owned(),
                            records,
                            next_cursor: (end < self.deployments).then(|| format!("page-{end}")),
                            completeness: Completeness::Complete,
                            observed_at_unix_ms: 1,
                            provenance: DatasourceProvenance {
                                binding_ref: WORKLOADS_BINDING.to_owned(),
                                projection_sha256: "a".repeat(64),
                                connector_audit_ref: "audit:workloads".to_owned(),
                            },
                        }));
                    }
                    DatasourceRead::Get { key } => {
                        // The real `workload_key_schema()` admits exactly `{"name": <string>}`.
                        let object = key.as_object().expect("the get key is an object");
                        assert_eq!(
                            object.keys().collect::<Vec<_>>(),
                            ["name"],
                            "the get key must carry exactly the workload key schema's property"
                        );
                        let name = object["name"].as_str().expect("the key name is a string");
                        assert!(!name.is_empty() && name.len() <= 253);
                        self.get_keys.lock().expect("key lock").push(key.clone());
                        let value = json!({"namespace": "dev", "name": name, "pods": []});
                        (RecordView::Detail, key, value)
                    }
                };
                Ok(DatasourceResult::Read(DatasourcePage {
                    datasource_ref: WORKLOADS.to_owned(),
                    records: vec![DatasourceRecord { key, view, value }],
                    next_cursor: None,
                    completeness: Completeness::Complete,
                    observed_at_unix_ms: 1,
                    provenance: DatasourceProvenance {
                        binding_ref: WORKLOADS_BINDING.to_owned(),
                        projection_sha256: "a".repeat(64),
                        connector_audit_ref: "audit:workloads".to_owned(),
                    },
                }))
            }
            DatasourceRequest::Search(_) => unreachable!("mcp tests never search datasources"),
        }
    }
}

fn app(backend: Arc<McpBackend>, authority: HostedAuthority) -> Router {
    router(
        Arc::new(McpVerifier),
        backend,
        HostedAdmissionPolicy::new(["operator".to_owned()])
            .with_kubernetes_groups(["sre".to_owned()], Vec::<String>::new()),
        authority,
    )
}

fn rpc_frame(id: Value, method: &str, params: Value) -> Vec<u8> {
    serde_json::to_vec(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))
        .expect("frame serializes")
}

fn mcp_request(token: &str, body: Vec<u8>) -> Request<Body> {
    Request::post("/mcp")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(body))
        .unwrap()
}

async fn body_json(response: Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), 2 * OPERATION_MAX_FRAME_BYTES)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// One JSON-RPC exchange expected to answer HTTP 200 with a parsed message.
async fn rpc(app: Router, token: &str, method: &str, params: Value) -> Value {
    let response = app
        .oneshot(mcp_request(token, rpc_frame(json!(1), method, params)))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    body_json(response).await
}

/// One `tools/call`, returning the JSON-RPC result (the MCP tool result object). Shared with
/// the monitoring toolset tests (`tests/mcp_monitoring.rs`), which drive the same surface.
pub(super) async fn call_tool(app: Router, token: &str, name: &str, arguments: Value) -> Value {
    let message = rpc(
        app,
        token,
        "tools/call",
        json!({"name": name, "arguments": arguments}),
    )
    .await;
    assert!(
        message.get("error").is_none(),
        "tools/call answered a protocol error: {message}"
    );
    message["result"].clone()
}

#[tokio::test]
async fn initialize_echoes_admitted_revisions_and_answers_ping() {
    let backend = Arc::new(McpBackend::default());
    for (proposed, answered) in [
        ("2025-03-26", "2025-03-26"),
        ("2025-06-18", "2025-06-18"),
        ("1999-01-01", "2025-06-18"),
    ] {
        let message = rpc(
            app(backend.clone(), HostedAuthority::unbound()),
            "sre-token",
            "initialize",
            json!({
                "protocolVersion": proposed,
                "capabilities": {},
                "clientInfo": {"name": "test-client", "version": "0"}
            }),
        )
        .await;
        assert_eq!(message["jsonrpc"], "2.0");
        assert_eq!(message["id"], json!(1));
        assert_eq!(message["result"]["protocolVersion"], answered);
        assert_eq!(
            message["result"]["capabilities"]["tools"]["listChanged"],
            json!(false)
        );
        assert_eq!(message["result"]["serverInfo"]["name"], "b10x-connectors");
    }
    let pong = rpc(
        app(backend, HostedAuthority::unbound()),
        "sre-token",
        "ping",
        json!({}),
    )
    .await;
    assert_eq!(pong["result"], json!({}));
}

#[tokio::test]
async fn the_mcp_route_is_stateless_post_only() {
    let backend = Arc::new(McpBackend::default());
    // A notification expects no response body: acknowledged and dropped.
    let notification =
        serde_json::to_vec(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
            .unwrap();
    let response = app(backend.clone(), HostedAuthority::unbound())
        .oneshot(mcp_request("sre-token", notification))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    // No SSE stream and no session to delete: both verbs answer 405.
    for method in ["GET", "DELETE"] {
        let request = Request::builder()
            .method(method)
            .uri("/mcp")
            .header(header::AUTHORIZATION, "Bearer sre-token")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            app(backend.clone(), HostedAuthority::unbound())
                .oneshot(request)
                .await
                .unwrap()
                .status(),
            StatusCode::METHOD_NOT_ALLOWED
        );
    }
}

#[tokio::test]
async fn tools_list_returns_exactly_the_three_meta_tools() {
    let backend = Arc::new(McpBackend::default());
    let message = rpc(
        app(backend, HostedAuthority::unbound()),
        "sre-token",
        "tools/list",
        json!({}),
    )
    .await;
    let tools = message["result"]["tools"].as_array().expect("tools array");
    let names: Vec<&str> = tools
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["tool_search", "tool_describe", "tool_invoke"]);
    for tool in tools {
        assert_eq!(tool["inputSchema"]["type"], "object", "{tool}");
        assert!(tool["description"].as_str().is_some_and(|d| !d.is_empty()));
    }
}

#[tokio::test]
async fn tool_search_projects_only_the_entries_the_callers_seam_results_support() {
    let backend = Arc::new(McpBackend::default());
    let listed = call_tool(
        app(backend.clone(), HostedAuthority::unbound()),
        "sre-token",
        "tool_search",
        json!({}),
    )
    .await;
    assert_eq!(listed["isError"], json!(false), "{listed}");
    assert_eq!(listed["structuredContent"]["toolset_version"], "v1");
    let names: Vec<&str> = listed["structuredContent"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    // k8s_pod_logs stays hidden: `kubernetes.pod.logs` is not among the caller's seam search
    // results until S-054 lands the operation.
    assert_eq!(
        names,
        [
            "k8s_namespace_list",
            "k8s_deployment_list",
            "k8s_deployment_status",
            "k8s_pod_status"
        ]
    );

    // A group-less principal's own seam results carry nothing, so the projection is empty.
    let empty = call_tool(
        app(backend, HostedAuthority::unbound()),
        "groupless-token",
        "tool_search",
        json!({}),
    )
    .await;
    assert_eq!(empty["isError"], json!(false), "{empty}");
    assert_eq!(empty["structuredContent"]["tools"], json!([]));
}

#[tokio::test]
async fn tool_describe_projects_the_underlying_description_without_a_lease() {
    let backend = Arc::new(McpBackend::default());
    let described = call_tool(
        app(backend.clone(), HostedAuthority::unbound()),
        "sre-token",
        "tool_describe",
        json!({"name": "k8s_deployment_status"}),
    )
    .await;
    assert_eq!(described["isError"], json!(false), "{described}");
    let projection = &described["structuredContent"];
    assert_eq!(projection["name"], "k8s_deployment_status");
    assert_eq!(projection["effect"], "read_only");
    assert_eq!(projection["approval"], "not_required");
    assert_eq!(projection["output_schema"], json!({"type": "object"}));
    assert!(
        !described.to_string().contains(OPERATION_LEASE),
        "the description lease must never reach an MCP caller: {described}"
    );

    // A tool whose requirement the caller's seam results do not support does not exist.
    let hidden = call_tool(
        app(backend, HostedAuthority::unbound()),
        "sre-token",
        "tool_describe",
        json!({"name": "k8s_pod_logs"}),
    )
    .await;
    assert_eq!(hidden["isError"], json!(true));
    assert_eq!(hidden["structuredContent"]["code"], "not_found");
}

#[tokio::test]
async fn an_op_backed_invoke_describes_then_invokes_with_the_fresh_lease() {
    let backend = Arc::new(McpBackend::default());
    let result = call_tool(
        app(backend.clone(), HostedAuthority::unbound()),
        "sre-token",
        "tool_invoke",
        json!({"name": "k8s_deployment_status", "args": {"namespace": "dev", "name": "web"}}),
    )
    .await;
    assert_eq!(result["isError"], json!(false), "{result}");
    assert_eq!(
        result["structuredContent"],
        json!({"namespace": "dev", "name": "web", "ready": true})
    );
    // The spec-recommended text block carries the same JSON.
    let text = result["content"][0]["text"].as_str().unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(text).unwrap(),
        result["structuredContent"]
    );
    // The toolset described, the route re-described for admission, and exactly one invoke
    // reached the backend — carrying the served lease, or the backend would have refused it.
    assert_eq!(backend.describes.load(Ordering::SeqCst), 2);
    assert_eq!(backend.invokes.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn datasource_backed_tools_route_through_the_datasource_seam() {
    let backend = Arc::new(McpBackend::default());
    let namespaces = call_tool(
        app(backend.clone(), HostedAuthority::unbound()),
        "sre-token",
        "tool_invoke",
        json!({"name": "k8s_namespace_list", "args": {}}),
    )
    .await;
    assert_eq!(namespaces["isError"], json!(false), "{namespaces}");
    assert_eq!(
        namespaces["structuredContent"],
        json!({"namespaces": ["dev"]})
    );

    let deployments = call_tool(
        app(backend.clone(), HostedAuthority::unbound()),
        "sre-token",
        "tool_invoke",
        json!({"name": "k8s_deployment_list", "args": {"namespace": "dev"}}),
    )
    .await;
    assert_eq!(deployments["isError"], json!(false), "{deployments}");
    assert_eq!(
        deployments["structuredContent"]["deployments"][0]["name"],
        "web"
    );
    assert_eq!(deployments["structuredContent"]["truncated"], json!(false));

    let pods = call_tool(
        app(backend.clone(), HostedAuthority::unbound()),
        "sre-token",
        "tool_invoke",
        json!({"name": "k8s_pod_status", "args": {"namespace": "dev", "deployment": "web"}}),
    )
    .await;
    assert_eq!(pods["isError"], json!(false), "{pods}");
    assert_eq!(pods["structuredContent"]["name"], "web");
    // The read reached the seam as the workload key schema's exact `{"name": …}` key.
    assert_eq!(
        backend.get_keys.lock().unwrap().clone(),
        vec![json!({"name": "web"})]
    );
    // No invoke arm was touched: every read routed through the datasource seam.
    assert_eq!(backend.invokes.load(Ordering::SeqCst), 0);

    // A namespace outside the caller's bindings is indistinguishable from an absent one.
    let missing = call_tool(
        app(backend, HostedAuthority::unbound()),
        "sre-token",
        "tool_invoke",
        json!({"name": "k8s_deployment_list", "args": {"namespace": "prod"}}),
    )
    .await;
    assert_eq!(missing["isError"], json!(true));
    assert_eq!(missing["structuredContent"]["code"], "not_found");
}

/// S-063: the live `latest` namespace shape — over a hundred deployments — must come back
/// whole from one arg-less call, the seam's pages and cursors aggregated server-side.
#[tokio::test]
async fn a_busy_namespace_is_listed_whole_without_a_paging_surface() {
    let backend = Arc::new(McpBackend {
        deployments: 120,
        ..McpBackend::default()
    });
    let listed = call_tool(
        app(backend.clone(), HostedAuthority::unbound()),
        "sre-token",
        "tool_invoke",
        json!({"name": "k8s_deployment_list", "args": {"namespace": "dev"}}),
    )
    .await;
    assert_eq!(listed["isError"], json!(false), "{listed}");
    let records = listed["structuredContent"]["deployments"]
        .as_array()
        .expect("deployments array");
    assert_eq!(records.len(), 120, "the whole namespace is one listing");
    assert_eq!(records[0]["name"], "web");
    assert_eq!(records[119]["name"], "web-119");
    assert_eq!(listed["structuredContent"]["truncated"], json!(false));

    // The dropped paging surface is really gone: paging args refuse instead of steering.
    let refused = call_tool(
        app(backend, HostedAuthority::unbound()),
        "sre-token",
        "tool_invoke",
        json!({"name": "k8s_deployment_list", "args": {"namespace": "dev", "limit": 5}}),
    )
    .await;
    assert_eq!(refused["isError"], json!(true), "{refused}");
    assert_eq!(refused["structuredContent"]["code"], "invalid_input");
}

/// The aggregation is never unbounded: past the record cap the listing is cut and says so.
#[tokio::test]
async fn a_pathological_namespace_is_cut_with_an_explicit_truncation_marker() {
    let backend = Arc::new(McpBackend {
        deployments: 620,
        ..McpBackend::default()
    });
    let listed = call_tool(
        app(backend, HostedAuthority::unbound()),
        "sre-token",
        "tool_invoke",
        json!({"name": "k8s_deployment_list", "args": {"namespace": "dev"}}),
    )
    .await;
    assert_eq!(listed["isError"], json!(false), "{listed}");
    let records = listed["structuredContent"]["deployments"]
        .as_array()
        .expect("deployments array");
    assert_eq!(records.len(), 500, "the record cap cuts the walk");
    assert_eq!(listed["structuredContent"]["truncated"], json!(true));
}

#[tokio::test]
async fn an_invoke_without_the_invoke_scope_surfaces_not_granted() {
    let backend = Arc::new(McpBackend::default());
    let result = call_tool(
        app(backend.clone(), HostedAuthority::unbound()),
        "reader-token",
        "tool_invoke",
        json!({"name": "k8s_deployment_status", "args": {"namespace": "dev", "name": "web"}}),
    )
    .await;
    assert_eq!(result["isError"], json!(true), "{result}");
    assert_eq!(result["structuredContent"]["code"], "not_granted");
    assert_eq!(result["structuredContent"]["retriable"], json!(false));
    assert_eq!(backend.invokes.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn approval_demand_and_evidence_pass_through_the_admission_seam() {
    let store = Arc::new(MemoryState::new());
    let backend = Arc::new(McpBackend {
        demand_approval: true,
        ..Default::default()
    });
    let bound: Arc<dyn StateStore> = store.clone();
    let authority = HostedAuthority::bound(bound, ISSUER);

    // Without evidence the toolset refuses with the honest axis before anything is spent.
    let refused = call_tool(
        app(backend.clone(), authority.clone()),
        "sre-token",
        "tool_invoke",
        json!({"name": "k8s_deployment_status", "args": {"namespace": "dev", "name": "web"}}),
    )
    .await;
    assert_eq!(refused["isError"], json!(true), "{refused}");
    assert_eq!(refused["structuredContent"]["code"], "approval_required");
    assert_eq!(backend.invokes.load(Ordering::SeqCst), 0);

    // With a grant, an issued approval, and the evidence beside the args, the reference
    // travels through the decided seam to the dispatched invocation verbatim.
    GrantSet {
        revision: 1,
        grants: vec![Grant {
            grant: "grant:test".to_owned(),
            provider: "kubernetes".to_owned(),
            connection: K8S_CONNECTION.to_owned(),
            selector: None,
            allow: BTreeSet::from([K8S_OPERATION.to_owned()]),
            deny: BTreeSet::new(),
            inbound_events: BTreeSet::new(),
        }],
    }
    .write(&*store, "tenant-dev")
    .expect("seed grants");
    issue_approval(
        &*store,
        &ApprovalRecord {
            reference: "approval:mcp".to_owned(),
            issuer: ISSUER.to_owned(),
            subject: "person:test".to_owned(),
            operation: K8S_OPERATION.to_owned(),
            connection: K8S_CONNECTION.to_owned(),
            input_digest: canonical_input_digest(&json!({"namespace": "dev", "name": "web"})),
            expires_at_seconds: u64::MAX,
        },
    )
    .expect("seed approval");
    let admitted = call_tool(
        app(backend.clone(), authority),
        "sre-token",
        "tool_invoke",
        json!({
            "name": "k8s_deployment_status",
            "args": {"namespace": "dev", "name": "web"},
            "approval_evidence_ref": "approval:mcp"
        }),
    )
    .await;
    assert_eq!(admitted["isError"], json!(false), "{admitted}");
    assert_eq!(
        backend.evidence.lock().unwrap().clone(),
        Some(Some("approval:mcp".to_owned()))
    );
}

#[tokio::test]
async fn a_stale_authority_refusal_is_retried_exactly_once_with_a_fresh_lease() {
    let once = Arc::new(McpBackend {
        stale_invokes: AtomicUsize::new(1),
        ..Default::default()
    });
    let result = call_tool(
        app(once.clone(), HostedAuthority::unbound()),
        "sre-token",
        "tool_invoke",
        json!({"name": "k8s_deployment_status", "args": {"namespace": "dev", "name": "web"}}),
    )
    .await;
    assert_eq!(result["isError"], json!(false), "{result}");
    assert_eq!(once.invokes.load(Ordering::SeqCst), 2);

    let always = Arc::new(McpBackend {
        stale_invokes: AtomicUsize::new(usize::MAX),
        ..Default::default()
    });
    let refused = call_tool(
        app(always.clone(), HostedAuthority::unbound()),
        "sre-token",
        "tool_invoke",
        json!({"name": "k8s_deployment_status", "args": {"namespace": "dev", "name": "web"}}),
    )
    .await;
    assert_eq!(refused["isError"], json!(true), "{refused}");
    assert_eq!(refused["structuredContent"]["code"], "stale_authority");
    assert_eq!(
        always.invokes.load(Ordering::SeqCst),
        2,
        "stale authority is retried exactly once"
    );
}

#[tokio::test]
async fn transport_refusals_carry_the_designed_statuses_and_codes() {
    let backend = Arc::new(McpBackend::default());

    // A missing or refused bearer answers before any JSON-RPC processing.
    let unauthenticated = Request::post("/mcp")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(rpc_frame(json!(1), "ping", json!({}))))
        .unwrap();
    let response = app(backend.clone(), HostedAuthority::unbound())
        .oneshot(unauthenticated)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let refused = app(backend.clone(), HostedAuthority::unbound())
        .oneshot(mcp_request(
            "wrong-token",
            rpc_frame(json!(1), "ping", json!({})),
        ))
        .await
        .unwrap();
    assert_eq!(refused.status(), StatusCode::UNAUTHORIZED);

    // Malformed JSON refuses as a parse error.
    let parse = app(backend.clone(), HostedAuthority::unbound())
        .oneshot(mcp_request("sre-token", b"{not json".to_vec()))
        .await
        .unwrap();
    assert_eq!(parse.status(), StatusCode::BAD_REQUEST);
    assert_eq!(body_json(parse).await["error"]["code"], json!(-32700));

    // A batch array is refused whole: batching left the protocol in 2025-06-18.
    let batch = app(backend.clone(), HostedAuthority::unbound())
        .oneshot(mcp_request(
            "sre-token",
            serde_json::to_vec(&json!([{"jsonrpc": "2.0", "id": 1, "method": "ping"}])).unwrap(),
        ))
        .await
        .unwrap();
    assert_eq!(batch.status(), StatusCode::BAD_REQUEST);
    assert_eq!(body_json(batch).await["error"]["code"], json!(-32600));

    // A method outside the subset answers method-not-found.
    let unknown = rpc(
        app(backend.clone(), HostedAuthority::unbound()),
        "sre-token",
        "resources/list",
        json!({}),
    )
    .await;
    assert_eq!(unknown["error"]["code"], json!(-32601));

    // A tools/call naming anything but the three meta-tools is invalid params.
    let tool = rpc(
        app(backend.clone(), HostedAuthority::unbound()),
        "sre-token",
        "tools/call",
        json!({"name": "rm_rf", "arguments": {}}),
    )
    .await;
    assert_eq!(tool["error"]["code"], json!(-32602));

    // An oversized frame refuses at the transport bound.
    let oversized = mcp_request("sre-token", vec![b' '; OPERATION_MAX_FRAME_BYTES + 1]);
    assert_eq!(
        app(backend, HostedAuthority::unbound())
            .oneshot(oversized)
            .await
            .unwrap()
            .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
}

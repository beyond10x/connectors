//! S-060: monitoring joins the MCP toolset (designs 14 and 15).
//!
//! These tests drive the same stateless `POST /mcp` surface as `tests/mcp.rs`, against a fake
//! backend advertising exactly the six catalogued read-only monitoring operations — with the
//! deployed shape design 15 measures: one central Grafana connection and several configured
//! Prometheus/Loki/Alertmanager targets per operation. They prove the requirement-driven
//! projection (a monitoring-read principal sees the tools, a group-less one sees none), the
//! honest `target` argument (enumerated from the caller's own described connections, resolved
//! to the chosen `connection_ref` at invoke, never hardcoded), and the design-14 refusal
//! patterns on the invoke path.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use serde_json::{json, Value};

use super::mcp::call_tool;
use super::*;

const MONITORING_LEASE: &str = "mon-lease-1";
const ISSUER: &str = "https://identity.example.test";

/// One advertised operation: `(operation_ref, title, [(connection_ref, label), …])`.
type FakeOperation = (
    &'static str,
    &'static str,
    &'static [(&'static str, &'static str)],
);

/// The six catalogued monitoring operations with the deployed connection shape: Grafana rides
/// the one central connection; every target-backed provider carries two configured targets.
const MONITORING_OPERATIONS: &[FakeOperation] = &[
    (
        "grafana-dashboards-list",
        "List Grafana dashboards",
        &[("connection:grafana:central", "central-grafana")],
    ),
    (
        "grafana-dashboard-get",
        "Get a Grafana dashboard",
        &[("connection:grafana:central", "central-grafana")],
    ),
    (
        "grafana-datasources-list",
        "Refresh Grafana datasource discovery",
        &[("connection:grafana:central", "central-grafana")],
    ),
    (
        "prometheus-query-range",
        "Query Prometheus metrics over a time range",
        &[
            ("connection:prometheus:dev-eu-central-1", "dev-eu-central-1"),
            (
                "connection:prometheus:prod-eu-central-1",
                "prod-eu-central-1",
            ),
        ],
    ),
    (
        "loki-query-range",
        "Query Loki logs over a time range",
        &[
            ("connection:loki:dev-eu-central-1", "dev-eu-central-1"),
            ("connection:loki:prod-eu-central-1", "prod-eu-central-1"),
        ],
    ),
    (
        "alertmanager-alerts-list",
        "List Alertmanager alerts",
        &[
            (
                "connection:alertmanager:dev-eu-central-1",
                "dev-eu-central-1",
            ),
            (
                "connection:alertmanager:prod-eu-central-1",
                "prod-eu-central-1",
            ),
        ],
    ),
];

/// Verifies three fixed principals: a monitoring read-group caller with both self-service
/// scopes, the same caller without any group, and a read-group caller holding only the
/// catalog scope.
struct MonitoringVerifier;

#[async_trait]
impl IdentityVerifier for MonitoringVerifier {
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
            "obs-token" => (&["obs"], &["connectors.catalog.read", "connectors.invoke"]),
            "groupless-token" => (&[], &["connectors.catalog.read", "connectors.invoke"]),
            "reader-token" => (&["obs"], &["connectors.catalog.read"]),
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

/// A backend owning the six monitoring operations, visible only to the read groups — the
/// monitoring twin of `tests/mcp.rs`'s kubernetes fake. Search filters by the query the seam
/// hands it, exactly like the real monitoring backend's haystack, so the toolset's
/// requirement queries are exercised rather than assumed. The invoke arm records the
/// dispatched `connection_ref` and input, refuses any lease other than the served one, and
/// optionally answers `stale_authority` a configured number of times first. The kubernetes
/// workloads datasource deliberately does not exist here.
#[derive(Default)]
struct MonitoringBackend {
    /// How many leading invokes answer `stale_authority` despite a fresh lease.
    stale_invokes: AtomicUsize,
    describes: AtomicUsize,
    invokes: AtomicUsize,
    /// Every `(operation_ref, connection_ref, input)` that reached the invoke arm.
    dispatched: Mutex<Vec<(String, String, Value)>>,
}

impl MonitoringBackend {
    fn visible(context: &PrincipalContext) -> bool {
        context
            .verified_groups()
            .iter()
            .any(|group| group == "obs" || group == "operator")
    }

    fn operation(operation_ref: &str) -> Option<&'static FakeOperation> {
        MONITORING_OPERATIONS
            .iter()
            .find(|(candidate, _, _)| *candidate == operation_ref)
    }

    fn connections(
        connections: &[(&str, &str)],
        provider: &str,
    ) -> Vec<protocol::operation::ConnectionSummary> {
        connections
            .iter()
            .map(
                |(connection_ref, label)| protocol::operation::ConnectionSummary {
                    connection_ref: (*connection_ref).to_owned(),
                    label: (*label).to_owned(),
                    provider: provider.to_owned(),
                    audiences: Vec::new(),
                    purpose: None,
                },
            )
            .collect()
    }
}

#[async_trait]
impl ConnectorBackend for MonitoringBackend {
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
            OperationRequest::Search(request) => {
                let needle = request.query.to_ascii_lowercase();
                let operations = if Self::visible(context) {
                    MONITORING_OPERATIONS
                        .iter()
                        .filter(|(operation_ref, title, _)| {
                            let haystack = format!("{operation_ref} {title}").to_ascii_lowercase();
                            needle.is_empty() || haystack.contains(&needle)
                        })
                        .map(|(operation_ref, title, connections)| {
                            protocol::operation::OperationSummary {
                                operation_ref: (*operation_ref).to_owned(),
                                title: (*title).to_owned(),
                                effect: EffectClass::ReadOnly,
                                approval: ApprovalPosture::NotRequired,
                                connections: Self::connections(
                                    connections,
                                    provider_of(operation_ref),
                                ),
                            }
                        })
                        .collect()
                } else {
                    Vec::new()
                };
                Ok(OperationResult::Search { operations })
            }
            OperationRequest::Describe(request) => {
                self.describes.fetch_add(1, Ordering::SeqCst);
                let described = Self::operation(&request.operation_ref)
                    .filter(|_| Self::visible(context))
                    .ok_or_else(|| {
                        OperationError::new(
                            OperationErrorCode::NotFound,
                            "no such operation",
                            false,
                        )
                    })?;
                let (operation_ref, title, connections) = described;
                Ok(OperationResult::Describe(OperationDescription {
                    operation_ref: (*operation_ref).to_owned(),
                    title: (*title).to_owned(),
                    description: "One catalogued read-only monitoring operation.".to_owned(),
                    input_schema: json!({"type": "object"}),
                    output_schema: json!({"type": "object"}),
                    effect: EffectClass::ReadOnly,
                    approval: ApprovalPosture::NotRequired,
                    connections: Self::connections(connections, provider_of(operation_ref)),
                    description_ref: MONITORING_LEASE.to_owned(),
                }))
            }
            OperationRequest::Invoke(request) => {
                self.invokes.fetch_add(1, Ordering::SeqCst);
                let (_, _, connections) = Self::operation(&request.operation_ref)
                    .expect("only advertised operations are invoked");
                assert!(
                    connections
                        .iter()
                        .any(|(connection_ref, _)| *connection_ref == request.connection_ref),
                    "the dispatched connection must be one the describe advertised: {}",
                    request.connection_ref
                );
                if request.description_ref != MONITORING_LEASE {
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
                self.dispatched.lock().expect("dispatch lock").push((
                    request.operation_ref.clone(),
                    request.connection_ref.clone(),
                    request.input.clone(),
                ));
                Ok(OperationResult::Invoke(InvocationResult {
                    operation_ref: request.operation_ref,
                    output: json!({"observed": true}),
                    connector_audit_ref: "audit:monitoring".to_owned(),
                    execution_ref: None,
                }))
            }
            _ => unreachable!("monitoring mcp tests send only search, describe and invoke"),
        }
    }

    async fn handle_datasource(
        &self,
        _context: &PrincipalContext,
        request: DatasourceRequest,
    ) -> Result<DatasourceResult, DatasourceError> {
        // This deployment carries no kubernetes workloads datasource; the projection must
        // tolerate the honest absence rather than erroring.
        match request {
            DatasourceRequest::Bindings(_) | DatasourceRequest::Describe(_) => Err(
                DatasourceError::new(DatasourceErrorCode::NotFound, "no such datasource", false),
            ),
            _ => unreachable!("monitoring mcp tests never read datasources"),
        }
    }
}

fn provider_of(operation_ref: &str) -> &'static str {
    match operation_ref.split('-').next() {
        Some("grafana") => "grafana",
        Some("prometheus") => "prometheus",
        Some("loki") => "loki",
        _ => "alertmanager",
    }
}

fn app(backend: Arc<MonitoringBackend>) -> Router {
    router(
        Arc::new(MonitoringVerifier),
        backend,
        HostedAdmissionPolicy::new(["operator".to_owned()])
            .with_monitoring_groups(["obs".to_owned()]),
        HostedAuthority::unbound(),
    )
}

#[tokio::test]
async fn tool_search_lists_the_monitoring_tools_for_a_monitoring_read_principal() {
    let backend = Arc::new(MonitoringBackend::default());
    let listed = call_tool(app(backend.clone()), "obs-token", "tool_search", json!({})).await;
    assert_eq!(listed["isError"], json!(false), "{listed}");
    let names: Vec<&str> = listed["structuredContent"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    // Every kubernetes entry stays hidden: this deployment's seam results carry neither the
    // kubernetes operations nor a workloads binding.
    assert_eq!(
        names,
        [
            "grafana_dashboards_list",
            "grafana_dashboard_get",
            "grafana_datasources_list",
            "prometheus_query_range",
            "loki_query_range",
            "alertmanager_alerts"
        ]
    );

    // The caller's query narrows the projection without changing what exists.
    let narrowed = call_tool(
        app(backend.clone()),
        "obs-token",
        "tool_search",
        json!({"query": "loki"}),
    )
    .await;
    let narrowed_names: Vec<&str> = narrowed["structuredContent"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    assert_eq!(narrowed_names, ["loki_query_range"]);

    // A group-less principal's own seam results carry nothing, so the projection is empty.
    let empty = call_tool(app(backend), "groupless-token", "tool_search", json!({})).await;
    assert_eq!(empty["isError"], json!(false), "{empty}");
    assert_eq!(empty["structuredContent"]["tools"], json!([]));
}

#[tokio::test]
async fn tool_describe_enumerates_the_callers_configured_targets_without_a_lease() {
    let backend = Arc::new(MonitoringBackend::default());
    let described = call_tool(
        app(backend.clone()),
        "obs-token",
        "tool_describe",
        json!({"name": "prometheus_query_range"}),
    )
    .await;
    assert_eq!(described["isError"], json!(false), "{described}");
    let projection = &described["structuredContent"];
    assert_eq!(projection["name"], "prometheus_query_range");
    assert_eq!(projection["effect"], "read_only");
    assert_eq!(projection["approval"], "not_required");
    // The `target` argument enumerates exactly the caller's own described connections —
    // never a hardcoded cluster list.
    assert_eq!(
        projection["input_schema"]["properties"]["target"]["enum"],
        json!(["dev-eu-central-1", "prod-eu-central-1"])
    );
    assert!(
        !described.to_string().contains(MONITORING_LEASE),
        "the description lease must never reach an MCP caller: {described}"
    );

    // The single central Grafana connection enumerates the same way.
    let grafana = call_tool(
        app(backend),
        "obs-token",
        "tool_describe",
        json!({"name": "grafana_dashboards_list"}),
    )
    .await;
    assert_eq!(
        grafana["structuredContent"]["input_schema"]["properties"]["target"]["enum"],
        json!(["central-grafana"])
    );
}

#[tokio::test]
async fn a_monitoring_invoke_routes_the_chosen_target_through_the_decided_seam() {
    let backend = Arc::new(MonitoringBackend::default());
    let result = call_tool(
        app(backend.clone()),
        "obs-token",
        "tool_invoke",
        json!({
            "name": "prometheus_query_range",
            "args": {
                "target": "prod-eu-central-1",
                "query": "up",
                "start": "2026-08-24T00:00:00Z",
                "end": "2026-08-24T01:00:00Z",
                "step": "60s"
            }
        }),
    )
    .await;
    assert_eq!(result["isError"], json!(false), "{result}");
    assert_eq!(result["structuredContent"], json!({"observed": true}));
    // The chosen target resolved to its connection_ref, and `target` itself never travelled
    // into the operation input.
    assert_eq!(
        backend.dispatched.lock().unwrap().clone(),
        vec![(
            "prometheus-query-range".to_owned(),
            "connection:prometheus:prod-eu-central-1".to_owned(),
            json!({
                "query": "up",
                "start": "2026-08-24T00:00:00Z",
                "end": "2026-08-24T01:00:00Z",
                "step": "60s"
            }),
        )]
    );
    // The toolset described to resolve the target, the route re-described for admission, and
    // exactly one invoke reached the backend.
    assert_eq!(backend.describes.load(Ordering::SeqCst), 2);
    assert_eq!(backend.invokes.load(Ordering::SeqCst), 1);

    // A lone configured connection needs no target at all.
    let sole = Arc::new(MonitoringBackend::default());
    let listed = call_tool(
        app(sole.clone()),
        "obs-token",
        "tool_invoke",
        json!({"name": "grafana_datasources_list", "args": {}}),
    )
    .await;
    assert_eq!(listed["isError"], json!(false), "{listed}");
    assert_eq!(
        sole.dispatched.lock().unwrap().clone(),
        vec![(
            "grafana-datasources-list".to_owned(),
            "connection:grafana:central".to_owned(),
            json!({}),
        )]
    );
}

#[tokio::test]
async fn a_monitoring_invoke_refuses_dishonest_targets_before_any_dispatch() {
    let backend = Arc::new(MonitoringBackend::default());
    // Several configured targets and no choice: refused with the honest axis, nothing spent.
    let ambiguous = call_tool(
        app(backend.clone()),
        "obs-token",
        "tool_invoke",
        json!({
            "name": "loki_query_range",
            "args": {
                "query": "{app=\"web\"}",
                "start": "2026-08-24T00:00:00Z",
                "end": "2026-08-24T01:00:00Z",
                "limit": 100,
                "direction": "backward"
            }
        }),
    )
    .await;
    assert_eq!(ambiguous["isError"], json!(true), "{ambiguous}");
    assert_eq!(ambiguous["structuredContent"]["code"], "invalid_input");

    // A target outside the caller's own described connections does not exist.
    let unknown = call_tool(
        app(backend.clone()),
        "obs-token",
        "tool_invoke",
        json!({
            "name": "prometheus_query_range",
            "args": {
                "target": "staging-eu-central-1",
                "query": "up",
                "start": "2026-08-24T00:00:00Z",
                "end": "2026-08-24T01:00:00Z",
                "step": "60s"
            }
        }),
    )
    .await;
    assert_eq!(unknown["isError"], json!(true), "{unknown}");
    assert_eq!(unknown["structuredContent"]["code"], "invalid_input");
    assert_eq!(backend.invokes.load(Ordering::SeqCst), 0);

    // Without the invoke scope the decided seam refuses exactly as design 14 patterns it.
    let refused = call_tool(
        app(backend.clone()),
        "reader-token",
        "tool_invoke",
        json!({
            "name": "grafana_dashboard_get",
            "args": {"target": "central-grafana", "namespace": "default", "uid": "abc"}
        }),
    )
    .await;
    assert_eq!(refused["isError"], json!(true), "{refused}");
    assert_eq!(refused["structuredContent"]["code"], "not_granted");
    assert_eq!(backend.invokes.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn a_stale_monitoring_invoke_re_resolves_the_same_target_exactly_once() {
    let backend = Arc::new(MonitoringBackend {
        stale_invokes: AtomicUsize::new(1),
        ..Default::default()
    });
    let result = call_tool(
        app(backend.clone()),
        "obs-token",
        "tool_invoke",
        json!({
            "name": "alertmanager_alerts",
            "args": {"target": "dev-eu-central-1"}
        }),
    )
    .await;
    assert_eq!(result["isError"], json!(false), "{result}");
    assert_eq!(backend.invokes.load(Ordering::SeqCst), 2);
    assert_eq!(
        backend.dispatched.lock().unwrap().clone(),
        vec![(
            "alertmanager-alerts-list".to_owned(),
            "connection:alertmanager:dev-eu-central-1".to_owned(),
            json!({}),
        )]
    );
}

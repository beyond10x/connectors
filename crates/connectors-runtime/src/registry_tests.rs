use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use protocol::connection::{
    ConnectionInitiator, ConnectionRoute, ConnectionState,
    ConnectionSummary as ResourceConnectionSummary, DescribeRequest as ConnectionDescribe,
    SearchRequest as ConnectionSearch,
};
use protocol::event::{
    ChannelSummary as EventChannelSummary, ReceiveRequest, SearchRequest as EventSearch,
};
use protocol::operation::{
    ApprovalPosture, ConnectionSummary as OperationConnectionSummary, DescribeRequest, EffectClass,
    InvocationResult, InvokeRequest, SearchRequest, SessionRequest,
};
use serde_json::json;

use super::*;

#[derive(Default)]
struct Calls {
    operation_search: AtomicUsize,
    operation_direct: AtomicUsize,
    connection_search: AtomicUsize,
    connection_direct: AtomicUsize,
    event_search: AtomicUsize,
    event_direct: AtomicUsize,
}

struct SyntheticBackend {
    capabilities: BackendCapabilities,
    operations: Vec<OperationSummary>,
    description: Option<OperationDescription>,
    invoke_connection: Option<String>,
    claims_direct_operation: bool,
    connections: Vec<ResourceConnectionSummary>,
    claims_connection: bool,
    channels: Vec<EventChannelSummary>,
    claims_event: bool,
    calls: Calls,
    invocation_leases: Mutex<Vec<String>>,
}

struct UnavailableBackend;

#[async_trait]
impl ConnectorBackend for UnavailableBackend {
    async fn ready(&self) -> Result<(), BackendReadinessError> {
        Err(BackendReadinessError)
    }

    async fn handle(
        &self,
        _context: &PrincipalContext,
        _request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        unreachable!("readiness never dispatches an operation")
    }
}

impl SyntheticBackend {
    fn empty(capabilities: BackendCapabilities) -> Arc<Self> {
        Arc::new(Self {
            capabilities,
            operations: Vec::new(),
            description: None,
            invoke_connection: None,
            claims_direct_operation: false,
            connections: Vec::new(),
            claims_connection: false,
            channels: Vec::new(),
            claims_event: false,
            calls: Calls::default(),
            invocation_leases: Mutex::new(Vec::new()),
        })
    }

    fn with_operations(operations: Vec<OperationSummary>) -> Arc<Self> {
        Arc::new(Self {
            capabilities: BackendCapabilities::OPERATIONS,
            operations,
            description: None,
            invoke_connection: None,
            claims_direct_operation: false,
            connections: Vec::new(),
            claims_connection: false,
            channels: Vec::new(),
            claims_event: false,
            calls: Calls::default(),
            invocation_leases: Mutex::new(Vec::new()),
        })
    }

    fn contributor(connection_ref: &str, local_lease: &str) -> Arc<Self> {
        Arc::new(Self {
            capabilities: BackendCapabilities::OPERATIONS,
            operations: vec![operation_summary("tickets.read", connection_ref)],
            description: Some(operation_description(
                "tickets.read",
                connection_ref,
                local_lease,
            )),
            invoke_connection: Some(connection_ref.to_owned()),
            claims_direct_operation: false,
            connections: Vec::new(),
            claims_connection: false,
            channels: Vec::new(),
            claims_event: false,
            calls: Calls::default(),
            invocation_leases: Mutex::new(Vec::new()),
        })
    }

    fn with_connections(connections: Vec<ResourceConnectionSummary>) -> Arc<Self> {
        Arc::new(Self {
            capabilities: BackendCapabilities {
                operations: false,
                connections: true,
                events: false,
                datasources: false,
            },
            operations: Vec::new(),
            description: None,
            invoke_connection: None,
            claims_direct_operation: false,
            connections,
            claims_connection: false,
            channels: Vec::new(),
            claims_event: false,
            calls: Calls::default(),
            invocation_leases: Mutex::new(Vec::new()),
        })
    }

    fn with_channels(channels: Vec<EventChannelSummary>) -> Arc<Self> {
        Arc::new(Self {
            capabilities: BackendCapabilities {
                operations: false,
                connections: false,
                events: true,
                datasources: false,
            },
            operations: Vec::new(),
            description: None,
            invoke_connection: None,
            claims_direct_operation: false,
            connections: Vec::new(),
            claims_connection: false,
            channels,
            claims_event: false,
            calls: Calls::default(),
            invocation_leases: Mutex::new(Vec::new()),
        })
    }

    fn exclusive_claims() -> Arc<Self> {
        Arc::new(Self {
            capabilities: BackendCapabilities {
                operations: true,
                connections: true,
                events: true,
                datasources: false,
            },
            operations: Vec::new(),
            description: None,
            invoke_connection: None,
            claims_direct_operation: true,
            connections: Vec::new(),
            claims_connection: true,
            channels: Vec::new(),
            claims_event: true,
            calls: Calls::default(),
            invocation_leases: Mutex::new(Vec::new()),
        })
    }
}

#[async_trait]
impl ConnectorBackend for SyntheticBackend {
    async fn ready(&self) -> Result<(), BackendReadinessError> {
        // Registry routing fixtures carry no configured runtime dependency.
        Ok(())
    }

    fn capabilities(&self) -> BackendCapabilities {
        self.capabilities
    }

    fn owns_operation(&self, request: &OperationRequest) -> bool {
        match request {
            OperationRequest::Describe(request) => self
                .description
                .as_ref()
                .is_some_and(|description| description.operation_ref == request.operation_ref),
            OperationRequest::Invoke(request) => {
                self.description
                    .as_ref()
                    .is_some_and(|description| description.operation_ref == request.operation_ref)
                    && self.invoke_connection.as_deref() == Some(&request.connection_ref)
            }
            OperationRequest::SessionStatus(_)
            | OperationRequest::SessionTerminate(_)
            | OperationRequest::SessionReconcile(_)
            | OperationRequest::SessionSignal(_) => self.claims_direct_operation,
            OperationRequest::Search(_) => false,
        }
    }

    fn owns_connection(&self, request: &ConnectionRequest) -> bool {
        !matches!(request, ConnectionRequest::Search(_)) && self.claims_connection
    }

    fn owns_event(&self, request: &EventRequest) -> bool {
        !matches!(request, EventRequest::Search(_)) && self.claims_event
    }

    async fn handle(
        &self,
        _context: &PrincipalContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        match request {
            OperationRequest::Search(_) => {
                self.calls.operation_search.fetch_add(1, Ordering::SeqCst);
                Ok(OperationResult::Search {
                    operations: self.operations.clone(),
                })
            }
            OperationRequest::Describe(request) => self
                .description
                .clone()
                .filter(|description| description.operation_ref == request.operation_ref)
                .map(OperationResult::Describe)
                .ok_or_else(|| operation_not_found("synthetic operation was not found")),
            OperationRequest::Invoke(request) => {
                self.calls.operation_direct.fetch_add(1, Ordering::SeqCst);
                self.invocation_leases
                    .lock()
                    .unwrap()
                    .push(request.description_ref);
                Ok(OperationResult::Invoke(InvocationResult {
                    operation_ref: request.operation_ref,
                    output: json!({"selected_connection": request.connection_ref}),
                    connector_audit_ref: "audit:test".to_owned(),
                    execution_ref: None,
                }))
            }
            OperationRequest::SessionStatus(_)
            | OperationRequest::SessionTerminate(_)
            | OperationRequest::SessionReconcile(_)
            | OperationRequest::SessionSignal(_) => {
                self.calls.operation_direct.fetch_add(1, Ordering::SeqCst);
                Err(operation_not_found(
                    "synthetic runtime reference was not found",
                ))
            }
        }
    }

    async fn handle_connection(
        &self,
        _context: &PrincipalContext,
        request: ConnectionRequest,
    ) -> Result<ConnectionResult, ConnectionError> {
        match request {
            ConnectionRequest::Search(_) => {
                self.calls.connection_search.fetch_add(1, Ordering::SeqCst);
                Ok(ConnectionResult::Search {
                    connections: self.connections.clone(),
                })
            }
            _ => {
                self.calls.connection_direct.fetch_add(1, Ordering::SeqCst);
                Err(ConnectionError::new(
                    ConnectionErrorCode::NotFound,
                    "synthetic Connection was not found",
                    false,
                ))
            }
        }
    }

    async fn handle_event(
        &self,
        _context: &PrincipalContext,
        request: EventRequest,
    ) -> Result<EventResult, EventError> {
        match request {
            EventRequest::Search(_) => {
                self.calls.event_search.fetch_add(1, Ordering::SeqCst);
                Ok(EventResult::Search {
                    channels: self.channels.clone(),
                })
            }
            _ => {
                self.calls.event_direct.fetch_add(1, Ordering::SeqCst);
                Err(EventError::new(
                    EventErrorCode::NotFound,
                    "synthetic event was not found",
                    false,
                ))
            }
        }
    }
}

fn context() -> PrincipalContext {
    PrincipalContext::hosted(
        "tenant-test".to_owned(),
        "principal-test".to_owned(),
        "actor-test".to_owned(),
        None,
        "snapshot-test".to_owned(),
        "a".repeat(64),
    )
    .unwrap()
}

/// A backend that publishes one datasource and filters search the way the real ones do.
struct SearchableDatasourceBackend;

#[async_trait]
impl ConnectorBackend for SearchableDatasourceBackend {
    async fn ready(&self) -> Result<(), BackendReadinessError> {
        Ok(())
    }

    async fn handle(
        &self,
        _context: &PrincipalContext,
        _request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        unreachable!("this fixture publishes datasources only")
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            datasources: true,
            ..BackendCapabilities::default()
        }
    }

    async fn handle_datasource(
        &self,
        _context: &PrincipalContext,
        request: DatasourceRequest,
    ) -> Result<DatasourceResult, DatasourceError> {
        let DatasourceRequest::Search(search) = request else {
            unreachable!("this fixture answers search only");
        };
        let summary = DatasourceSummary {
            datasource_ref: "slack.conversations".to_owned(),
            title: "Slack conversations".to_owned(),
            access_mode: protocol::datasource::AccessMode::Live,
            verbs: vec![protocol::datasource::ReadVerb::List],
        };
        let matches = search.query.is_empty()
            || summary
                .title
                .to_ascii_lowercase()
                .contains(&search.query.to_ascii_lowercase());
        Ok(DatasourceResult::Search {
            definitions: if matches { vec![summary] } else { Vec::new() },
        })
    }
}

#[tokio::test]
async fn a_topical_datasource_query_that_matches_nothing_returns_the_admitted_set() {
    // Two review agents read an empty search result as "this deployment has no datasources"
    // and stopped there. The query narrows; it never hides the surface.
    let registry = BackendRegistry::new(vec![Arc::new(SearchableDatasourceBackend)]);
    let found = registry
        .handle_datasource(
            &context(),
            DatasourceRequest::Search(DatasourceSearchRequest {
                query: "connector".to_owned(),
                limit: 10,
            }),
        )
        .await
        .unwrap();
    let DatasourceResult::Search { definitions } = found else {
        panic!("search returns definitions");
    };
    assert_eq!(definitions.len(), 1);
    assert_eq!(definitions[0].datasource_ref, "slack.conversations");
}

#[test]
fn the_registry_lease_ignores_request_scoped_provenance() {
    let provenanced = context()
        .with_hosted_provenance(
            "https://identity.example.test".to_owned(),
            "token-after-rotation".to_owned(),
            None,
            "request-2".to_owned(),
            "trace-2".to_owned(),
        )
        .unwrap();
    // A lease minted while serving describe must admit the invoke on the NEXT
    // authenticated request: fresh request/trace ids and a rotated access token are
    // not an authority change.
    let mut refs_a = ["local-b".to_owned(), "local-a".to_owned()];
    let mut refs_b = ["local-a".to_owned(), "local-b".to_owned()];
    assert_eq!(
        registry_description_ref(&context(), "tickets-list", &mut refs_a).unwrap(),
        registry_description_ref(&provenanced, "tickets-list", &mut refs_b).unwrap(),
    );
    assert_ne!(
        registry_description_ref(&context(), "tickets-list", &mut refs_a).unwrap(),
        registry_description_ref(&context(), "tickets-close", &mut refs_a).unwrap(),
    );
}

fn operation_connection(connection_ref: &str) -> OperationConnectionSummary {
    OperationConnectionSummary {
        connection_ref: connection_ref.to_owned(),
        label: connection_ref.to_owned(),
        provider: "tickets".to_owned(),
        audiences: vec!["operations".to_owned()],
        purpose: None,
    }
}

#[test]
fn rate_stage2_advice_changes_refuse_merging_and_invalidate_the_registry_lease() {
    let left = operation_description("tickets-list", "connection:a", "same-local-ref");
    let mut right = left.clone();
    right.rate_advice = Some(protocol::operation::OperationRateAdvice {
        fixed: Some(protocol::operation::FixedRateLimit {
            requests: 1,
            per_seconds: 60,
            bucket: None,
        }),
        alternatives: Vec::new(),
    });
    assert_eq!(
        ensure_compatible_description(&left, &right)
            .unwrap_err()
            .code,
        OperationErrorCode::Protocol
    );
    let mut before = [registry_contributor_ref(&left)];
    let mut after = [registry_contributor_ref(&right)];
    assert_ne!(
        registry_description_ref(&context(), "tickets-list", &mut before).unwrap(),
        registry_description_ref(&context(), "tickets-list", &mut after).unwrap()
    );
}

fn operation_summary(operation_ref: &str, connection_ref: &str) -> OperationSummary {
    OperationSummary {
        operation_ref: operation_ref.to_owned(),
        title: format!("Operation {operation_ref}"),
        effect: EffectClass::ReadOnly,
        approval: ApprovalPosture::NotRequired,
        connections: vec![operation_connection(connection_ref)],
    }
}

fn operation_description(
    operation_ref: &str,
    connection_ref: &str,
    lease: &str,
) -> OperationDescription {
    OperationDescription {
        rate_advice: None,
        operation_ref: operation_ref.to_owned(),
        title: format!("Operation {operation_ref}"),
        description: "Reads one ticket".to_owned(),
        input_schema: json!({"type": "object"}),
        output_schema: json!({"type": "object"}),
        effect: EffectClass::ReadOnly,
        approval: ApprovalPosture::NotRequired,
        connections: vec![operation_connection(connection_ref)],
        description_ref: lease.to_owned(),
    }
}

fn resource_connection(connection_ref: &str) -> ResourceConnectionSummary {
    ResourceConnectionSummary {
        connection_ref: connection_ref.to_owned(),
        integration_ref: "tickets".to_owned(),
        label: connection_ref.to_owned(),
        state: ConnectionState::Callable,
        initiation: vec![ConnectionInitiator::Platform],
        route: ConnectionRoute::Direct,
        scope: None,
        actor: None,
        auth_profile: None,
    }
}

fn event_channel(channel_ref: &str) -> EventChannelSummary {
    EventChannelSummary {
        channel_ref: channel_ref.to_owned(),
        connection_ref: "connection:a".to_owned(),
        integration_ref: "tickets".to_owned(),
        binding_ref: "binding:a".to_owned(),
        events: vec!["ticket.updated".to_owned()],
    }
}

fn registry(backends: Vec<Arc<SyntheticBackend>>) -> BackendRegistry {
    BackendRegistry::new(
        backends
            .into_iter()
            .map(|backend| backend as Arc<dyn ConnectorBackend>)
            .collect(),
    )
}

#[tokio::test]
async fn readiness_requires_every_configured_backend() {
    let ready: Arc<dyn ConnectorBackend> = SyntheticBackend::empty(BackendCapabilities::OPERATIONS);
    let unavailable: Arc<dyn ConnectorBackend> = Arc::new(UnavailableBackend);
    let registry = BackendRegistry::new(vec![ready, unavailable]);
    assert_eq!(registry.ready().await, Err(BackendReadinessError));
}

#[tokio::test]
async fn search_aggregates_compatible_operations_and_deduplicates_the_operation() {
    let first = SyntheticBackend::with_operations(vec![
        operation_summary("tickets.alpha", "connection:alpha"),
        operation_summary("tickets.read", "connection:b"),
    ]);
    let second =
        SyntheticBackend::with_operations(vec![operation_summary("tickets.read", "connection:a")]);
    let connection_only = SyntheticBackend::empty(BackendCapabilities {
        operations: false,
        connections: true,
        events: false,
        datasources: false,
    });
    let result = registry(vec![
        Arc::clone(&first),
        Arc::clone(&second),
        Arc::clone(&connection_only),
    ])
    .handle(
        &context(),
        OperationRequest::Search(SearchRequest {
            query: "tickets".to_owned(),
            limit: 10,
        }),
    )
    .await
    .unwrap();
    let OperationResult::Search { operations } = result else {
        panic!("registry returned the wrong result");
    };
    assert_eq!(
        operations
            .iter()
            .map(|operation| operation.operation_ref.as_str())
            .collect::<Vec<_>>(),
        ["tickets.alpha", "tickets.read"]
    );
    assert_eq!(
        operations[1]
            .connections
            .iter()
            .map(|connection| connection.connection_ref.as_str())
            .collect::<Vec<_>>(),
        ["connection:a", "connection:b"]
    );
    assert_eq!(first.calls.operation_search.load(Ordering::SeqCst), 1);
    assert_eq!(second.calls.operation_search.load(Ordering::SeqCst), 1);
    assert_eq!(
        connection_only
            .calls
            .operation_search
            .load(Ordering::SeqCst),
        0,
        "operation Search must honor the advertised route family"
    );
}

#[tokio::test]
async fn direct_dispatch_selects_the_unique_claim_without_not_found_probing() {
    let owner = SyntheticBackend::exclusive_claims();
    let non_owner = SyntheticBackend::empty(BackendCapabilities::OPERATIONS);
    let error = registry(vec![Arc::clone(&owner), Arc::clone(&non_owner)])
        .handle(
            &context(),
            OperationRequest::SessionStatus(SessionRequest {
                execution_ref: "execution:missing".to_owned(),
            }),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, OperationErrorCode::NotFound);
    assert_eq!(owner.calls.operation_direct.load(Ordering::SeqCst), 1);
    assert_eq!(non_owner.calls.operation_direct.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn ambiguous_exclusive_claims_fail_with_typed_protocol_errors_before_dispatch() {
    let first = SyntheticBackend::exclusive_claims();
    let second = SyntheticBackend::exclusive_claims();
    let registry = registry(vec![Arc::clone(&first), Arc::clone(&second)]);

    let operation = registry
        .handle(
            &context(),
            OperationRequest::SessionStatus(SessionRequest {
                execution_ref: "execution:1".to_owned(),
            }),
        )
        .await
        .unwrap_err();
    assert_eq!(operation.code, OperationErrorCode::Protocol);

    let connection = registry
        .handle_connection(
            &context(),
            ConnectionRequest::Describe(ConnectionDescribe {
                connection_ref: "connection:1".to_owned(),
            }),
        )
        .await
        .unwrap_err();
    assert_eq!(connection.code, ConnectionErrorCode::Protocol);

    let event = registry
        .handle_event(
            &context(),
            EventRequest::Receive(ReceiveRequest {
                channel_ref: "channel:1".to_owned(),
                after: None,
                limit: 1,
                wait_ms: 0,
            }),
        )
        .await
        .unwrap_err();
    assert_eq!(event.code, EventErrorCode::Protocol);
    assert_eq!(first.calls.operation_direct.load(Ordering::SeqCst), 0);
    assert_eq!(first.calls.connection_direct.load(Ordering::SeqCst), 0);
    assert_eq!(first.calls.event_direct.load(Ordering::SeqCst), 0);
    assert_eq!(second.calls.operation_direct.load(Ordering::SeqCst), 0);
    assert_eq!(second.calls.connection_direct.load(Ordering::SeqCst), 0);
    assert_eq!(second.calls.event_direct.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn describe_merges_connections_and_invoke_receives_the_selected_local_lease() {
    let first = SyntheticBackend::contributor("connection:a", "lease:a");
    let second = SyntheticBackend::contributor("connection:b", "lease:b");
    let registry = registry(vec![Arc::clone(&first), Arc::clone(&second)]);
    let described = registry
        .handle(
            &context(),
            OperationRequest::Describe(DescribeRequest {
                operation_ref: "tickets.read".to_owned(),
            }),
        )
        .await
        .unwrap();
    let OperationResult::Describe(description) = described else {
        panic!("registry returned the wrong result");
    };
    assert_eq!(
        description
            .connections
            .iter()
            .map(|connection| connection.connection_ref.as_str())
            .collect::<Vec<_>>(),
        ["connection:a", "connection:b"]
    );
    assert!(description
        .description_ref
        .starts_with("description:registry:"));
    assert_ne!(description.description_ref, "lease:a");
    assert_ne!(description.description_ref, "lease:b");

    let invoked = registry
        .handle(
            &context(),
            OperationRequest::Invoke(InvokeRequest {
                operation_ref: "tickets.read".to_owned(),
                connection_ref: "connection:a".to_owned(),
                description_ref: description.description_ref,
                input: json!({}),
                approval_evidence_ref: None,
            }),
        )
        .await
        .unwrap();
    let OperationResult::Invoke(result) = invoked else {
        panic!("registry returned the wrong result");
    };
    assert_eq!(
        result.output,
        json!({"selected_connection": "connection:a"})
    );
    assert_eq!(
        first.invocation_leases.lock().unwrap().as_slice(),
        ["lease:a"]
    );
    assert!(second.invocation_leases.lock().unwrap().is_empty());
}

#[tokio::test]
async fn duplicate_connection_references_fail_search() {
    let first =
        SyntheticBackend::with_connections(vec![resource_connection("connection:duplicate")]);
    let second =
        SyntheticBackend::with_connections(vec![resource_connection("connection:duplicate")]);
    let error = registry(vec![first, second])
        .handle_connection(
            &context(),
            ConnectionRequest::Search(ConnectionSearch {
                query: String::new(),
                limit: 10,
            }),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ConnectionErrorCode::Protocol);
}

#[tokio::test]
async fn duplicate_channel_references_fail_search() {
    let error = registry(vec![
        SyntheticBackend::with_channels(vec![event_channel("channel:duplicate")]),
        SyntheticBackend::with_channels(vec![event_channel("channel:duplicate")]),
    ])
    .handle_event(
        &context(),
        EventRequest::Search(EventSearch {
            query: String::new(),
            limit: 10,
        }),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, EventErrorCode::Protocol);
}

// The S-048 local claim-seam tests, textually included so they share this module's fixtures
// while the file itself stays under the module size fence — the same shape
// `integration-slack/src/backend.rs` uses for its test sibling.
include!("registry_claims_tests.rs");

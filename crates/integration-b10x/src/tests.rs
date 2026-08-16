use super::*;
use connectors_config::{B10xConnectionConfig, InitiationConfig};
use protocol::operation::{DescribeRequest, InvokeRequest, OwnerContext, SearchRequest};
use std::fs;
use std::io::Write as _;
use std::net::TcpListener;
use std::thread;
use std::time::{Duration, Instant};

fn config(root: &Path) -> B10xIntegrationConfig {
    let signing_key = root.join("module-signing.key");
    if !signing_key.exists() {
        fs::write(
            &signing_key,
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\n",
        )
        .unwrap();
        fs::set_permissions(&signing_key, fs::Permissions::from_mode(0o600)).unwrap();
    }
    B10xIntegrationConfig {
        connection: B10xConnectionConfig {
            connection_ref: "connection-b10x".to_owned(),
            label: "B10x local".to_owned(),
            grant_ref: "grant-b10x".to_owned(),
            initiation: InitiationConfig::B10x,
        },
        tenant_member_modules: None,
        work_origin: Some("http://127.0.0.1:4180".to_owned()),
        ontology_origin: None,
        ontology_bearer_file: None,
        module_signing_key_file: Some(signing_key),
        module_signing_key_id: Some("test-1".to_owned()),
        module_signing_issuer: Some("urn:b10x:connectors:test".to_owned()),
        audio: None,
        browser: None,
    }
}

fn principal() -> PrincipalContext {
    PrincipalContext::local(&OwnerContext {
        tenant_id: "tenant-test".to_owned(),
        agent_id: "agent-test".to_owned(),
        agent_revision: 1,
        authority_snapshot_id: "snapshot-test".to_owned(),
        authority_snapshot_sha256: "a".repeat(64),
    })
    .unwrap()
}

fn fake_http(response_body: &'static str) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let task = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut request = Vec::new();
        let expected = loop {
            let mut buffer = [0_u8; 2048];
            let read = stream.read(&mut buffer).unwrap();
            assert_ne!(read, 0, "HTTP request ended before its declared body");
            request.extend_from_slice(&buffer[..read]);
            assert!(request.len() <= 64 * 1024);
            let Some(header_end) = request.windows(4).position(|part| part == b"\r\n\r\n") else {
                continue;
            };
            let headers = std::str::from_utf8(&request[..header_end]).unwrap();
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            break header_end + 4 + content_length;
        };
        while request.len() < expected {
            let mut buffer = [0_u8; 2048];
            let read = stream.read(&mut buffer).unwrap();
            assert_ne!(read, 0, "HTTP request ended before its declared body");
            request.extend_from_slice(&buffer[..read]);
        }
        let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response_body}",
                response_body.len()
            );
        stream.write_all(response.as_bytes()).unwrap();
        String::from_utf8(request).unwrap()
    });
    (origin, task)
}

fn stalling_http(delay: Duration) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let task = thread::spawn(move || {
        let (_stream, _) = listener.accept().unwrap();
        thread::sleep(delay);
    });
    (origin, task)
}

async fn invoke_operation(
    backend: &B10xBackend,
    operation_ref: &str,
    input: Value,
    approval_evidence_ref: Option<&str>,
) -> Result<OperationResult, OperationError> {
    let OperationResult::Describe(description) = backend
        .handle(
            &principal(),
            OperationRequest::Describe(DescribeRequest {
                operation_ref: operation_ref.to_owned(),
            }),
        )
        .await
        .unwrap()
    else {
        panic!("description expected")
    };
    backend
        .handle(
            &principal(),
            OperationRequest::Invoke(InvokeRequest {
                operation_ref: operation_ref.to_owned(),
                connection_ref: "connection-b10x".to_owned(),
                description_ref: description.description_ref,
                input,
                approval_evidence_ref: approval_evidence_ref.map(ToOwned::to_owned),
            }),
        )
        .await
}

async fn invoke_read(backend: &B10xBackend, operation_ref: &str, input: Value) -> Value {
    let OperationResult::Invoke(result) = invoke_operation(backend, operation_ref, input, None)
        .await
        .unwrap()
    else {
        panic!("invocation expected")
    };
    result.output
}

fn audit_outcomes(root: &Path) -> Vec<String> {
    fs::read_to_string(root.join("b10x-operation-audit.jsonl"))
        .unwrap()
        .lines()
        .map(|line| {
            serde_json::from_str::<Value>(line).unwrap()["event"]["outcome"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect()
}

#[tokio::test]
async fn search_projects_only_configured_capabilities() {
    let temporary = tempfile::tempdir().unwrap();
    fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let backend =
        B10xBackend::personal(config(temporary.path()), principal(), temporary.path())
            .unwrap();
    let OperationResult::Search { operations } = backend
        .handle(
            &principal(),
            OperationRequest::Search(SearchRequest {
                query: String::new(),
                limit: 25,
            }),
        )
        .await
        .unwrap()
    else {
        panic!("search result expected")
    };
    assert_eq!(operations.len(), 7);
    assert!(operations
        .iter()
        .all(|operation| operation.operation_ref.starts_with("work.")));
    assert!(operations
        .iter()
        .all(|operation| operation.connections.len() == 1));

    let ConnectionResult::Search { connections } = backend
        .handle_connection(
            &principal(),
            ConnectionRequest::Search(protocol::connection::SearchRequest {
                query: "b10x".to_owned(),
                limit: 64,
            }),
        )
        .await
        .unwrap()
    else {
        panic!("Connection search result expected")
    };
    assert_eq!(connections.len(), 1);
    assert_eq!(connections[0].integration_ref, PROVIDER);
    assert_eq!(connections[0].state, ConnectionState::Callable);
    assert_eq!(connections[0].route, ConnectionRoute::Direct);
}

#[test]
fn hosted_tenant_member_defaults_are_an_explicit_module_ceiling() {
    let temporary = tempfile::tempdir().unwrap();
    fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let mut configured = config(temporary.path());
    configured.tenant_member_modules = Some(Vec::new());
    let backend =
        B10xBackend::hosted(configured, vec!["tenant-test".to_owned()], temporary.path())
            .unwrap();
    assert!(!backend.configured("work-request-list"));

    let mut configured = config(temporary.path());
    configured.tenant_member_modules = Some(vec!["work".to_owned()]);
    let backend =
        B10xBackend::hosted(configured, vec!["tenant-test".to_owned()], temporary.path())
            .unwrap();
    assert!(backend.configured("work-request-list"));
}

#[tokio::test]
async fn module_global_ids_resolve_for_declarative_ui_requirements() {
    let temporary = tempfile::tempdir().unwrap();
    fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let backend =
        B10xBackend::personal(config(temporary.path()), principal(), temporary.path())
            .unwrap();
    let described = backend
        .handle(
            &principal(),
            OperationRequest::Describe(DescribeRequest {
                operation_ref: "work/request.list".to_owned(),
            }),
        )
        .await
        .unwrap();
    let OperationResult::Describe(description) = described else {
        panic!("description expected")
    };
    assert_eq!(description.operation_ref, "work/request.list");
}

#[tokio::test]
async fn work_owner_events_are_checkpointed_into_connector_sequence_space() {
    let (origin, server) = fake_http(
        r#"{"events":[{"protocol":"b10x.module-event.v1","id":"owner-event-1","module":"work","key":"task.created","schema_version":1,"occurred_at":"2026-08-16T12:00:00Z","cursor":"work:deployment:1","data":{"task":{"id":"task-1"}}}],"next_cursor":"work:deployment:1","has_more":false}"#,
    );
    let temporary = tempfile::tempdir().unwrap();
    fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let mut configured = config(temporary.path());
    configured.work_origin = Some(origin);
    let backend = B10xBackend::personal(configured, principal(), temporary.path()).unwrap();
    let result = backend
        .handle_event(
            &principal(),
            EventRequest::Receive(protocol::event::ReceiveRequest {
                channel_ref: WORK_EVENT_CHANNEL.to_owned(),
                after: None,
                limit: 10,
                wait_ms: 0,
            }),
        )
        .await
        .unwrap();
    let EventResult::Receive { events, next } = result else {
        panic!("receive result expected")
    };
    assert_eq!(next, "1");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "task.created");
    assert_eq!(events[0].provenance, EventProvenance::Polled);
    assert_eq!(events[0].event_ref, "event:b10x:work:1");
    assert_eq!(
        events[0].payload["cursor"],
        serde_json::json!("work:deployment:1")
    );
    let request = server.join().unwrap();
    assert!(request.starts_with("GET /api/work/v2/events?"));
    assert!(temporary
        .path()
        .join("b10x-work-events.json")
        .exists());
}

#[test]
fn browser_catalog_symbol_is_translated_into_the_closed_driver_input() {
    let document = Document::parse(DOCUMENT).unwrap();
    let operation = document.operation(BROWSER_OPEN_OPERATION).unwrap();
    assert_eq!(
        operation.input_schema()["required"],
        serde_json::json!(["url_2"])
    );
    assert_eq!(
        browser_open_input(serde_json::json!({"url_2":"https://example.com"}))
            .unwrap()
            .url
            .as_deref(),
        Some("https://example.com")
    );
    assert!(browser_open_input(serde_json::json!({"url":"https://example.com"})).is_err());
    assert!(browser_goto_input(serde_json::json!({"url_2":7})).is_err());
}

#[test]
fn a_mutating_post_dispatch_failure_is_not_declared_retriable() {
    let document = Document::parse(DOCUMENT).unwrap();
    let operation = document.operation("work-request-create").unwrap();
    let error = post_dispatch_error(operation, unavailable());
    assert_eq!(error.code, OperationErrorCode::OutcomeUnknown);
    assert!(!error.retriable);
}

#[tokio::test]
async fn work_invocation_crosses_the_private_http_boundary_with_signed_authority() {
    let (origin, server) = fake_http(r#"{"items":[],"next_cursor":null}"#);
    let temporary = tempfile::tempdir().unwrap();
    fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let mut configured = config(temporary.path());
    configured.work_origin = Some(origin);
    let backend = B10xBackend::personal(configured, principal(), temporary.path()).unwrap();
    let output = invoke_read(
        &backend,
        "work.requests.list",
        serde_json::json!({"cursor":"", "limit":1}),
    )
    .await;
    assert_eq!(output, serde_json::json!({"items":[], "next_cursor":null}));
    let request = server.join().unwrap();
    assert!(request.starts_with("GET /api/work/v2/requests?"));
    assert!(request.contains("authorization: DLModule "));
    assert_eq!(audit_outcomes(temporary.path()), ["attempted", "completed"]);
}

#[tokio::test]
async fn copied_static_approval_text_cannot_authorize_an_effect() {
    let temporary = tempfile::tempdir().unwrap();
    fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let backend =
        B10xBackend::personal(config(temporary.path()), principal(), temporary.path())
            .unwrap();

    let missing = invoke_operation(
        &backend,
        "work.requests.create",
        serde_json::json!({}),
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(missing.code, OperationErrorCode::ApprovalRequired);

    let copied = invoke_operation(
        &backend,
        "work.requests.create",
        serde_json::json!({}),
        Some("approval-policy:deployment:copied-from-config"),
    )
    .await
    .unwrap_err();
    assert_eq!(copied.code, OperationErrorCode::ApprovalDenied);
    assert!(!temporary
        .path()
        .join("b10x-operation-audit.jsonl")
        .exists());
}

#[tokio::test]
async fn invalid_post_dispatch_output_is_audited_as_indeterminate() {
    let (origin, server) = fake_http(r#"{}"#);
    let temporary = tempfile::tempdir().unwrap();
    fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let mut configured = config(temporary.path());
    configured.work_origin = Some(origin);
    let backend = B10xBackend::personal(configured, principal(), temporary.path()).unwrap();

    let error = invoke_operation(
        &backend,
        "work.requests.list",
        serde_json::json!({"cursor":"", "limit":1}),
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, OperationErrorCode::Unavailable);
    server.join().unwrap();
    assert_eq!(
        audit_outcomes(temporary.path()),
        ["attempted", "indeterminate"]
    );
}

#[tokio::test]
async fn total_http_deadline_bounds_a_stalled_private_service() {
    let (origin, server) = stalling_http(Duration::from_millis(250));
    let temporary = tempfile::tempdir().unwrap();
    fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let mut configured = config(temporary.path());
    configured.work_origin = Some(origin);
    let mut backend =
        B10xBackend::personal(configured, principal(), temporary.path()).unwrap();
    backend.client = http_client(Duration::from_millis(25), Duration::from_millis(50)).unwrap();
    backend.http_total_timeout = Duration::from_millis(50);

    let operation = backend.document.operation("work-request-list").unwrap();
    let started = Instant::now();
    let error = backend
        .invoke_http(
            &principal(),
            "work-request-list",
            operation,
            serde_json::json!({"cursor":"", "limit":1}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, OperationErrorCode::Unavailable);
    let elapsed = started.elapsed();
    assert!(elapsed < Duration::from_millis(200), "elapsed {elapsed:?}");
    server.join().unwrap();
}

#[tokio::test]
async fn ontology_invocation_carries_request_bound_signed_authority() {
    let (origin, server) = fake_http(r#"{"claims":[],"truncated":false}"#);
    let temporary = tempfile::tempdir().unwrap();
    fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let mut configured = config(temporary.path());
    configured.work_origin = None;
    configured.ontology_origin = Some(origin);
    let backend = B10xBackend::personal(configured, principal(), temporary.path()).unwrap();
    let output = invoke_read(
        &backend,
        "knowledge.query",
        serde_json::json!({
            "branches": ["main"],
            "limit": 10,
            "predicate": null,
            "subject": null
        }),
    )
    .await;
    assert_eq!(output, serde_json::json!({"claims":[], "truncated":false}));
    let request = server.join().unwrap();
    assert!(request.starts_with("POST /v1/query HTTP/1.1\r\n"));
    assert!(request.contains("authorization: DLModule "));
    let audit =
        fs::read_to_string(temporary.path().join("b10x-operation-audit.jsonl")).unwrap();
    assert!(!audit.contains("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"));
}

#[test]
fn every_projected_operation_has_a_response_schema() {
    let catalog: Value = serde_json::from_str(DOCUMENT).unwrap();
    for (canonical, _, _) in all_operation_rows() {
        assert!(response_schema(&catalog, canonical).is_ok(), "{canonical}");
    }
}

#[test]
fn ontology_nullable_fields_are_still_strict_after_catalog_lowering() {
    assert!(validate_semantic_input(
        "knowledge-query",
        &serde_json::json!({
            "branches": ["main"],
            "limit": 10,
            "predicate": null,
            "subject": "entity:one"
        }),
    )
    .is_ok());
    for invalid in [
        serde_json::json!({
            "branches": ["main"], "limit": 10, "predicate": {}, "subject": null
        }),
        serde_json::json!({
            "branches": ["main", "main"], "limit": 10, "predicate": null, "subject": null
        }),
        serde_json::json!({
            "branches": [], "limit": 1.5, "predicate": null, "subject": null
        }),
        serde_json::json!({
            "branches": [], "limit": 10, "predicate": null, "subject": null, "origin": "caller-selected"
        }),
    ] {
        assert!(validate_semantic_input("knowledge-query", &invalid).is_err());
    }
}

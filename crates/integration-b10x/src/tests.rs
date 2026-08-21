use super::*;
use connector_resolve::document::HostEffect;
use connectors_config::{B10xConnectionConfig, InitiationConfig};
use protocol::datasource::{
    BindingSearchRequest, DatasourceRead, DatasourceRequest, DatasourceResult,
    DescribeRequest as DatasourceDescribeRequest, ReadRequest,
};
use protocol::operation::{
    ApprovalPosture, DescribeRequest, InvokeRequest, OwnerContext, SearchRequest,
};
use std::fs;
use std::io::{Read as _, Write as _};
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt as _;
use std::os::unix::net::UnixListener;
use std::path::Path;
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
        planner_origin: None,
        workspaces_origin: None,
        colab_origin: None,
        module_sockets: BTreeMap::new(),
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

fn fake_unix_http(socket: &Path, response_body: &'static str) -> thread::JoinHandle<String> {
    let listener = UnixListener::bind(socket).unwrap();
    thread::spawn(move || {
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
                .unwrap_or_default();
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
    })
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

/// Configure every module origin so search projects the whole module surface at once.
fn every_module_config(root: &Path) -> B10xIntegrationConfig {
    let mut configured = config(root);
    configured.ontology_origin = Some("http://127.0.0.1:4181".to_owned());
    configured.planner_origin = Some("http://127.0.0.1:4182".to_owned());
    configured.workspaces_origin = Some("http://127.0.0.1:4183".to_owned());
    configured.colab_origin = Some("http://127.0.0.1:4184".to_owned());
    configured
}

#[tokio::test]
async fn search_names_each_operation_once_and_never_by_its_second_name() {
    let temporary = tempfile::tempdir().unwrap();
    fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let backend = B10xBackend::personal(
        every_module_config(temporary.path()),
        principal(),
        temporary.path(),
    )
    .unwrap();
    let OperationResult::Search { operations } = backend
        .handle(
            &principal(),
            OperationRequest::Search(SearchRequest {
                query: String::new(),
                limit: 256,
            }),
        )
        .await
        .unwrap()
    else {
        panic!("search result expected")
    };
    assert!(operations.len() > 60, "the module surface is projected");

    // Two results are aliases of one operation exactly when they resolve to the same catalog id.
    let mut seen: BTreeMap<&'static str, String> = BTreeMap::new();
    for summary in &operations {
        let (canonical, published_ref, _) = operation_row(&summary.operation_ref)
            .unwrap_or_else(|| panic!("{} resolves", summary.operation_ref));
        assert_eq!(
            summary.operation_ref, published_ref,
            "search published the catalog id instead of the published name"
        );
        if let Some(first) = seen.insert(canonical, summary.operation_ref.clone()) {
            panic!(
                "{canonical} is published twice: {first} and {}",
                summary.operation_ref
            );
        }
    }
    assert_eq!(seen.len(), operations.len());

    // The catalog id is not a second published name, but it must stay findable and resolvable.
    let OperationResult::Search { operations: found } = backend
        .handle(
            &principal(),
            OperationRequest::Search(SearchRequest {
                query: "colab-workspace-list".to_owned(),
                limit: 8,
            }),
        )
        .await
        .unwrap()
    else {
        panic!("search result expected")
    };
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].operation_ref, "colab.workspaces.list");
}

#[tokio::test]
async fn every_name_of_an_operation_describes_one_operation() {
    let temporary = tempfile::tempdir().unwrap();
    fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let backend = B10xBackend::personal(
        every_module_config(temporary.path()),
        principal(),
        temporary.path(),
    )
    .unwrap();
    let mut leases = Vec::new();
    // Describe answers under the name it was asked about, because the Agent's Connector client
    // refuses any other answer. What must hold is that the names are not two capabilities: one
    // title, one contract, one description lease.
    for asked in [
        "ontology-branch-list",
        "ontology.branches.list",
        "ontology/branch.list",
    ] {
        let OperationResult::Describe(description) = backend
            .handle(
                &principal(),
                OperationRequest::Describe(DescribeRequest {
                    operation_ref: asked.to_owned(),
                }),
            )
            .await
            .unwrap()
        else {
            panic!("description expected")
        };
        assert_eq!(description.operation_ref, asked);
        assert_eq!(description.title, "List Ontology branches");
        leases.push(description.description_ref);
    }
    assert!(
        leases.windows(2).all(|pair| pair[0] == pair[1]),
        "one operation must hold one description lease however it is named"
    );
}

#[tokio::test]
async fn workspace_datasource_projects_only_the_logical_read_model() {
    let (origin, server) = fake_http(
        r#"{"api_version":"workspaces.b10x.io/v2","request_id":"owner-request","result":{"items":[{"id":"wsp_example","tenant":"tenant:secret","owner":"user:secret","name":"Example","retention":"managed","source":{"forge":"git-hub","canonical_url":"https://github.com/b10x/b10x","repository":"b10x/b10x","connection":"connection:forge:secret","default_branch":"main"},"state":"active","created_at_ms":1,"updated_at_ms":2,"expires_at_ms":null}]}}"#,
    );
    let temporary = tempfile::tempdir().unwrap();
    fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let mut configured = config(temporary.path());
    configured.workspaces_origin = Some(origin);
    let backend = B10xBackend::personal(configured, principal(), temporary.path()).unwrap();
    assert!(backend.capabilities().datasources);

    let DatasourceResult::Describe(description) = backend
        .handle_datasource(
            &principal(),
            DatasourceRequest::Describe(DatasourceDescribeRequest {
                datasource_ref: WORKSPACES_DATASOURCE.to_owned(),
            }),
        )
        .await
        .unwrap()
    else {
        panic!("datasource description expected")
    };
    let DatasourceResult::Bindings { bindings } = backend
        .handle_datasource(
            &principal(),
            DatasourceRequest::Bindings(BindingSearchRequest {
                datasource_ref: WORKSPACES_DATASOURCE.to_owned(),
                query: String::new(),
                limit: 1,
            }),
        )
        .await
        .unwrap()
    else {
        panic!("datasource binding expected")
    };
    let DatasourceResult::Read(page) = backend
        .handle_datasource(
            &principal(),
            DatasourceRequest::Read(ReadRequest {
                datasource_ref: WORKSPACES_DATASOURCE.to_owned(),
                binding_ref: bindings[0].binding_ref.clone(),
                description_ref: description.description_ref,
                read: DatasourceRead::List {
                    limit: 10,
                    cursor: None,
                },
            }),
        )
        .await
        .unwrap()
    else {
        panic!("datasource page expected")
    };
    assert_eq!(page.records.len(), 1);
    assert_eq!(
        page.records[0].key,
        serde_json::json!({"id": "wsp_example"})
    );
    assert!(page.records[0].value.get("tenant").is_none());
    assert!(page.records[0].value.get("owner").is_none());
    assert!(page.records[0].value["source"].get("connection").is_none());
    let request = server.join().unwrap();
    assert!(request.starts_with("GET /api/workspaces/v2/workspaces HTTP/1.1"));
    assert!(request
        .to_ascii_lowercase()
        .contains("authorization: dlmodule "));
}

fn hosted_principal(token_id: &str, envelope_sha: char) -> PrincipalContext {
    // Shaped exactly as the hosted receiver builds it: the snapshot id is the access-token id
    // and the snapshot sha hashes the introspection envelope, so both differ on the next token
    // while the admitted authority is identical (`server::hosted::principal`).
    PrincipalContext::hosted_with_groups(
        "tenant-test".to_owned(),
        "person:owner".to_owned(),
        "person:owner".to_owned(),
        Some("owner@example.test".to_owned()),
        token_id.to_owned(),
        envelope_sha.to_string().repeat(64),
        BTreeSet::from(["dev".to_owned()]),
    )
    .unwrap()
}

/// The workspace read failed in the deployed build with
/// `StaleAuthority: workspace datasource definition or binding is stale`, minutes before the
/// identical read succeeded. Datasource requests all travel on the cached `connectors.catalog.read`
/// token, which lives at most five minutes; a describe and a read that straddle its refresh arrive
/// on different tokens. The description lease must not notice — the operation lease in this same
/// backend already derives from the stable authority seed and does not.
#[tokio::test]
async fn a_workspace_read_survives_the_access_token_rotation_between_describe_and_read() {
    let (origin, server) = fake_http(
        r#"{"api_version":"workspaces.b10x.io/v2","request_id":"owner-request","result":{"items":[{"id":"wsp_example","tenant":"tenant:secret","owner":"user:secret","name":"Example","retention":"managed","source":null,"state":"active","created_at_ms":1,"updated_at_ms":2,"expires_at_ms":null}]}}"#,
    );
    let temporary = tempfile::tempdir().unwrap();
    fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let mut configured = config(temporary.path());
    configured.workspaces_origin = Some(origin);
    let backend =
        B10xBackend::hosted(configured, vec!["tenant-test".to_owned()], temporary.path())
            .unwrap();

    let describing = hosted_principal("token-catalog-1", 'c');
    let DatasourceResult::Describe(description) = backend
        .handle_datasource(
            &describing,
            DatasourceRequest::Describe(DatasourceDescribeRequest {
                datasource_ref: WORKSPACES_DATASOURCE.to_owned(),
            }),
        )
        .await
        .unwrap()
    else {
        panic!("datasource description expected")
    };
    let DatasourceResult::Bindings { bindings } = backend
        .handle_datasource(
            &describing,
            DatasourceRequest::Bindings(BindingSearchRequest {
                datasource_ref: WORKSPACES_DATASOURCE.to_owned(),
                query: String::new(),
                limit: 1,
            }),
        )
        .await
        .unwrap()
    else {
        panic!("datasource binding expected")
    };

    let reading = hosted_principal("token-catalog-2", 'd');
    let DatasourceResult::Read(page) = backend
        .handle_datasource(
            &reading,
            DatasourceRequest::Read(ReadRequest {
                datasource_ref: WORKSPACES_DATASOURCE.to_owned(),
                binding_ref: bindings[0].binding_ref.clone(),
                description_ref: description.description_ref,
                read: DatasourceRead::List {
                    limit: 10,
                    cursor: None,
                },
            }),
        )
        .await
        .unwrap()
    else {
        panic!("a read on the next access token must not be refused as stale")
    };
    assert_eq!(page.records.len(), 1);
    server.join().unwrap();
}

/// An unrecognised binding ref is not a stale lease. Reporting it as one taught a reviewing agent
/// to ask an operator to re-authorize a Connector lease that was current.
#[tokio::test]
async fn an_unknown_workspace_binding_is_named_rather_than_reported_as_stale() {
    let temporary = tempfile::tempdir().unwrap();
    fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let mut configured = config(temporary.path());
    configured.workspaces_origin = Some("http://127.0.0.1:1".to_owned());
    let backend =
        B10xBackend::hosted(configured, vec!["tenant-test".to_owned()], temporary.path())
            .unwrap();
    let context = hosted_principal("token-catalog-1", 'c');
    let DatasourceResult::Describe(description) = backend
        .handle_datasource(
            &context,
            DatasourceRequest::Describe(DatasourceDescribeRequest {
                datasource_ref: WORKSPACES_DATASOURCE.to_owned(),
            }),
        )
        .await
        .unwrap()
    else {
        panic!("datasource description expected")
    };
    let refused = backend
        .handle_datasource(
            &context,
            DatasourceRequest::Read(ReadRequest {
                datasource_ref: WORKSPACES_DATASOURCE.to_owned(),
                binding_ref: WORKSPACES_DATASOURCE.to_owned(),
                description_ref: description.description_ref,
                read: DatasourceRead::List {
                    limit: 10,
                    cursor: None,
                },
            }),
        )
        .await
        .unwrap_err();
    assert_eq!(refused.code, DatasourceErrorCode::InvalidInput);
    assert!(refused.message.contains("binding"), "{refused:?}");
    assert!(!refused.message.contains("stale"), "{refused:?}");
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

#[tokio::test]
async fn planner_owner_events_are_checkpointed_into_connector_sequence_space() {
    let (origin, server) = fake_http(
        r#"{"events":[{"protocol":"b10x.module-event.v1","id":"planner-event-1","module":"planner","key":"entity.created","schema_version":1,"occurred_at":"2026-08-17T12:00:00Z","cursor":"planner:deployment:1","data":{"project":"demo","kind":"story","id":"S-001"}}],"next_cursor":"planner:deployment:1","has_more":false}"#,
    );
    let temporary = tempfile::tempdir().unwrap();
    fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let mut configured = config(temporary.path());
    configured.work_origin = None;
    configured.planner_origin = Some(origin);
    let backend = B10xBackend::personal(configured, principal(), temporary.path()).unwrap();
    let result = backend
        .handle_event(
            &principal(),
            EventRequest::Receive(protocol::event::ReceiveRequest {
                channel_ref: PLANNER_EVENT_CHANNEL.to_owned(),
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
    assert_eq!(events[0].event_type, "entity.created");
    assert_eq!(events[0].event_ref, "event:b10x:planner:1");
    let request = server.join().unwrap();
    assert!(request.starts_with("GET /api/planner/v1/events?"));
    assert!(temporary
        .path()
        .join("b10x-planner-events.json")
        .exists());
}

#[test]
fn browser_catalog_symbol_is_translated_into_the_closed_driver_input() {
    let document = Document::parse(DOCUMENT).unwrap();
    let operation = document.operation(BROWSER_OPEN_OPERATION).unwrap();
    // **`url` is declared optional, and `browser.open` means it.** Opening with no URL is a
    // blank browser, which is why `browser_open_input` returns an `Option`. This asserted the
    // opposite because the contract projection marked every parameter required regardless of what
    // the provider declared -- so a model was told it had to supply a URL it may not have.
    assert_eq!(operation.input_schema()["required"], serde_json::json!([]));
    assert!(operation.input_schema()["properties"]
        .as_object()
        .is_some_and(|properties| properties.contains_key("url_2")));
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

#[test]
fn every_declared_write_requires_external_approval() {
    let document = Document::parse(DOCUMENT).unwrap();
    for (canonical, operation_ref, _) in all_operation_rows() {
        let operation = document
            .operation(canonical)
            .unwrap_or_else(|| panic!("{operation_ref} resolves to a catalog operation"));
        if operation.effects().contains(&HostEffect::Write) {
            assert_eq!(
                approval(canonical, operation.effects()),
                ApprovalPosture::Required,
                "{operation_ref} publishes a write effect without required approval"
            );
        }
    }
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
async fn local_work_invocation_is_constrained_to_the_configured_unix_socket() {
    let temporary = tempfile::tempdir().unwrap();
    fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let socket = temporary.path().join("work.sock");
    let server = fake_unix_http(&socket, r#"{"items":[],"next_cursor":null}"#);
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).unwrap();
    let mut configured = config(temporary.path());
    configured.work_origin = None;
    configured.module_sockets.insert("work".to_owned(), socket);
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
    assert!(!request.to_ascii_lowercase().contains("authorization:"));
    assert_eq!(audit_outcomes(temporary.path()), ["attempted", "completed"]);
}

#[tokio::test]
async fn planner_invocation_crosses_the_private_http_boundary_with_signed_authority() {
    let (origin, server) = fake_http(r#"{"items":[],"next_cursor":null}"#);
    let temporary = tempfile::tempdir().unwrap();
    fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let mut configured = config(temporary.path());
    configured.work_origin = None;
    configured.planner_origin = Some(origin);
    let backend = B10xBackend::personal(configured, principal(), temporary.path()).unwrap();
    let output = invoke_read(
        &backend,
        "planner/project.list",
        serde_json::json!({"cursor":"", "limit":"1"}),
    )
    .await;
    assert_eq!(output, serde_json::json!({"items":[], "next_cursor":null}));
    let request = server.join().unwrap();
    assert!(request.starts_with("GET /api/planner/v1/projects?"));
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

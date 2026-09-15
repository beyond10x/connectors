use super::*;
use std::net::TcpStream;
use std::sync::atomic::{AtomicU64, Ordering};

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse as _;
use axum::routing::{get, post};
use axum::{Json, Router};

#[derive(Default)]
struct MemoryStore(Mutex<BTreeMap<String, String>>);

impl SecretStore for MemoryStore {
    fn save(&self, account: &str, secret: &str) -> Result<(), IdentityError> {
        self.0
            .lock()
            .map_err(|_| IdentityError::Keyring)?
            .insert(account.to_owned(), secret.to_owned());
        Ok(())
    }

    fn load(&self, account: &str) -> Result<Zeroizing<String>, IdentityError> {
        self.0
            .lock()
            .map_err(|_| IdentityError::Keyring)?
            .get(account)
            .cloned()
            .map(Zeroizing::new)
            .ok_or(IdentityError::Keyring)
    }

    fn delete(&self, account: &str) -> Result<(), IdentityError> {
        self.0
            .lock()
            .map_err(|_| IdentityError::Keyring)?
            .remove(account);
        Ok(())
    }
}

struct FakeState {
    origin: String,
    connectors_base: String,
    access_issues: Mutex<BTreeMap<String, u64>>,
    mcp_bearers: Mutex<Vec<String>>,
}

async fn discovery(State(state): State<Arc<FakeState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "protocol": DISCOVERY_PROTOCOL,
        "identity_origin": state.origin,
        "identity_audience": CONNECTORS_AUDIENCE,
    }))
}

async fn login_metadata(State(state): State<Arc<FakeState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "issuer": state.origin,
        "authorization_endpoint": format!("{}/authorize", state.origin),
        "token_endpoint": format!("{}/oauth/token", state.origin),
        "access_token_endpoint": format!("{}/v1/access-token", state.origin),
        "cli_client_id": "connectors-test-client",
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code"],
        "code_challenge_methods_supported": ["S256"],
    }))
}

async fn exchange(body: Bytes) -> impl axum::response::IntoResponse {
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains("grant_type=authorization_code"));
    assert!(body.contains("code=controlled-code"));
    (
        [("cache-control", "no-store"), ("pragma", "no-cache")],
        Json(serde_json::json!({
            "session": "controlled-opaque-identity-session",
            "session_type": "opaque_server_session",
            "expires_in": 86400,
            "tenant_id": "tenant-test",
            "subject": "person:test",
            "email": "test@example.test",
        })),
    )
}

async fn access_token(
    State(state): State<Arc<FakeState>>,
    headers: HeaderMap,
    Json(request): Json<serde_json::Value>,
) -> impl axum::response::IntoResponse {
    assert_eq!(
        headers.get("authorization").unwrap(),
        "Bearer controlled-opaque-identity-session"
    );
    assert_eq!(request["audience"], CONNECTORS_AUDIENCE);
    let scope = request["scope"].as_str().unwrap().to_owned();
    let issue = {
        let mut issues = state.access_issues.lock().unwrap();
        let issue = issues.entry(scope.clone()).or_default();
        *issue += 1;
        *issue
    };
    (
        [("cache-control", "no-store"), ("pragma", "no-cache")],
        Json(serde_json::json!({
            "access_token": format!("access-{scope}-{issue}"),
            "token_type": "Bearer",
            "expires_in": 300,
            "audience": CONNECTORS_AUDIENCE,
            "scope": scope,
        })),
    )
}

async fn mcp(
    State(state): State<Arc<FakeState>>,
    headers: HeaderMap,
    Json(request): Json<serde_json::Value>,
) -> axum::response::Response {
    state.mcp_bearers.lock().unwrap().push(
        headers
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned(),
    );
    let id = request["id"].clone();
    let result = if request["method"] == "tools/list" {
        serde_json::json!({"tools": []})
    } else {
        serde_json::json!({"content": [], "isError": false})
    };
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result,
        })),
    )
        .into_response()
}

struct FakeIdentity {
    state: Arc<FakeState>,
    task: tokio::task::JoinHandle<()>,
}

impl FakeIdentity {
    async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let state = Arc::new(FakeState {
            connectors_base: format!("{origin}/connectors/v1"),
            origin,
            access_issues: Mutex::new(BTreeMap::new()),
            mcp_bearers: Mutex::new(Vec::new()),
        });
        let application = Router::new()
            .route(
                "/connectors/v1/.well-known/connectors-client",
                get(discovery),
            )
            .route("/.well-known/identity-cli-login", get(login_metadata))
            .route("/oauth/token", post(exchange))
            .route("/v1/access-token", post(access_token))
            .route("/connectors/v1/mcp", post(mcp))
            .with_state(state.clone());
        let task = tokio::spawn(async move {
            axum::serve(listener, application).await.unwrap();
        });
        Self { state, task }
    }
}

impl Drop for FakeIdentity {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn complete_browser(url: &Url, _no_browser: bool) -> Result<(), IdentityError> {
    let query: BTreeMap<_, _> = url.query_pairs().into_owned().collect();
    assert_eq!(query.get("response_type").map(String::as_str), Some("code"));
    assert_eq!(
        query.get("code_challenge_method").map(String::as_str),
        Some("S256")
    );
    assert!(query.get("nonce").is_some_and(|nonce| !nonce.is_empty()));
    let redirect = query.get("redirect_uri").unwrap().clone();
    let state = query.get("state").unwrap().clone();
    std::thread::spawn(move || {
        let callback = Url::parse(&redirect).unwrap();
        let mut stream = TcpStream::connect(("127.0.0.1", callback.port().unwrap())).unwrap();
        write!(
            stream,
            "GET /callback?code=controlled-code&state={state} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        let mut answer = String::new();
        std::io::Read::read_to_string(&mut stream, &mut answer).unwrap();
        assert!(answer.contains("200 OK"));
    });
    Ok(())
}

#[test]
fn mcp_invocation_uses_only_the_invoke_scope() {
    assert_eq!(
        mcp_scope(br#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#),
        CATALOG_SCOPE
    );
    assert_eq!(
        mcp_scope(br#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"tool_invoke","arguments":{}}}"#),
        INVOKE_SCOPE
    );
    assert_eq!(
        mcp_scope(br#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"tool_search","arguments":{}}}"#),
        CATALOG_SCOPE
    );
}

#[test]
fn hosted_request_families_select_the_smallest_available_scope() {
    assert_eq!(
        connection_scope(&connection::EndpointRequest::Search(
            connection::SearchRequest {
                query: String::new(),
                limit: 1,
            },
        )),
        CATALOG_SCOPE
    );
    assert_eq!(
        connection_scope(&connection::EndpointRequest::ConnectSessionCreate(
            connection::ConnectSessionCreateRequest {
                integration_ref: "slack".to_owned(),
                label: "mine".to_owned(),
                auth_profile: None,
            },
        )),
        CONNECTION_SELF_SCOPE
    );
    assert_eq!(
        event_scope(&event::EventRequest::Search(event::SearchRequest {
            query: "slack".to_owned(),
            limit: 1,
        })),
        EVENT_SELF_SCOPE
    );
    assert_eq!(
        event_scope(&event::EventRequest::Search(event::SearchRequest {
            query: String::new(),
            limit: 1,
        })),
        EVENT_READ_SCOPE
    );
}

#[test]
fn keyring_account_contains_no_endpoint_or_principal() {
    let session = SessionMetadata {
        connectors_base: "https://connectors.example.test/api/connectors/v1".to_owned(),
        identity_origin: "https://identity.example.test".to_owned(),
        tenant_id: "tenant-test".to_owned(),
        subject: "person:test".to_owned(),
        email: Some("test@example.test".to_owned()),
        obtained_at: 1,
        idle_expires_at: 2,
    };
    let account = keyring_account(&session);
    assert!(account.starts_with("v1-"));
    assert!(!account.contains("example"));
    assert!(!account.contains("person"));
}

#[tokio::test]
async fn login_separates_the_session_and_refreshes_exact_scope_tokens() {
    let fake = FakeIdentity::start().await;
    let temporary = tempfile::tempdir().unwrap();
    let metadata = temporary.path().join("identity-sessions.json");
    let store = Arc::new(MemoryStore::default());
    let session = login_with(
        &LoginOptions {
            connectors_base: fake.state.connectors_base.clone(),
            no_browser: true,
            timeout: Duration::from_secs(5),
        },
        &metadata,
        store.clone(),
        complete_browser,
    )
    .await
    .unwrap();

    let state_bytes = fs::read_to_string(&metadata).unwrap();
    assert!(!state_bytes.contains("controlled-opaque-identity-session"));
    assert_eq!(active_session_at(&metadata).unwrap(), Some(session.clone()));
    assert_eq!(store.0.lock().unwrap().len(), 1);

    let now = Arc::new(AtomicU64::new(1_000));
    let mut source = IdentityAccessTokenSource::new(session.clone(), store.clone()).unwrap();
    let clock = now.clone();
    source.clock = Arc::new(move || Ok(clock.load(Ordering::SeqCst)));
    assert_eq!(
        source.access_token(CATALOG_SCOPE).await.unwrap().as_str(),
        "access-connectors.catalog.read-1"
    );
    assert_eq!(
        source.access_token(CATALOG_SCOPE).await.unwrap().as_str(),
        "access-connectors.catalog.read-1",
        "a live token is reused"
    );
    now.store(1_271, Ordering::SeqCst);
    assert_eq!(
        source.access_token(CATALOG_SCOPE).await.unwrap().as_str(),
        "access-connectors.catalog.read-2",
        "the token is replaced inside its thirty-second refresh margin"
    );
    assert_eq!(
        source.access_token(INVOKE_SCOPE).await.unwrap().as_str(),
        "access-connectors.invoke-1",
        "a different family receives a separate exact-scope token"
    );
    assert_eq!(
        fake.state.access_issues.lock().unwrap().clone(),
        BTreeMap::from([(CATALOG_SCOPE.to_owned(), 2), (INVOKE_SCOPE.to_owned(), 1)])
    );

    let client = AuthenticatedHostedClient::from_session(session, store).unwrap();
    let (mut input_writer, input_reader) = tokio::io::duplex(4096);
    let (output_writer, mut output_reader) = tokio::io::duplex(4096);
    let bridge = tokio::spawn(bridge(client, input_reader, output_writer));
    input_writer
        .write_all(
            br#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"tool_invoke","arguments":{}}}
"#,
        )
        .await
        .unwrap();
    input_writer.shutdown().await.unwrap();
    let mut output = String::new();
    output_reader.read_to_string(&mut output).await.unwrap();
    bridge.await.unwrap().unwrap();
    assert_eq!(output.lines().count(), 2);
    assert_eq!(
        fake.state.mcp_bearers.lock().unwrap().as_slice(),
        [
            "Bearer access-connectors.catalog.read-3",
            "Bearer access-connectors.invoke-2",
        ]
    );
}

#[tokio::test]
async fn auth_stage2_identity_409_does_not_renew_or_resend() {
    for identity_expired in [false, true] {
        let fake = FakeIdentity::start().await;
        let temporary = tempfile::tempdir().unwrap();
        let store = Arc::new(MemoryStore::default());
        let session = login_with(
            &LoginOptions {
                connectors_base: fake.state.connectors_base.clone(),
                no_browser: true,
                timeout: Duration::from_secs(5),
            },
            &temporary.path().join("identity-sessions.json"),
            store.clone(),
            complete_browser,
        )
        .await
        .unwrap();
        let calls = Arc::new(AtomicU64::new(0));
        let observed = calls.clone();
        let app = Router::new().route("/operations", post(move |Json(request): Json<serde_json::Value>| {
        let observed = observed.clone();
        async move {
            let attempt = observed.fetch_add(1, Ordering::SeqCst);
            if identity_expired && attempt == 0 {
                return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"message":"SYNTHETIC_PRIVATE_INSTRUCTION"})));
            }
            (StatusCode::CONFLICT, Json(serde_json::json!({
                "protocol": operation::v3::CONTRACT, "request_id": request["request_id"],
                "status": "error", "error": {"code": "authentication_required", "message": "fixture refusal", "retriable": false,
                "authentication": {"operation_ref": "fixture.write", "endpoint_ref": "connection:fixture", "integration_ref": "fixture", "auth_profile": "fixture.user", "need": "reauthorize_existing", "attempt": "not_attempted", "next_action": "start_trusted_remediation"}}
            })))
        }
    }));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = Url::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
        let serving = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let mut client = AuthenticatedHostedClient::from_session(session, store).unwrap();
        client.hosted = HostedClient::from_parts(url, reqwest::Client::new());
        client.tokens.access_token(INVOKE_SCOPE).await.unwrap();
        let reply = client
            .operation(operation::OperationRequest::Invoke(
                operation::InvokeRequest {
                    operation_ref: "fixture.write".into(),
                    connection_ref: "connection:fixture".into(),
                    description_ref: "description:fixture".into(),
                    input: serde_json::json!({}),
                    approval_evidence_ref: None,
                },
            ))
            .await;
        serving.abort();
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1 + u64::from(identity_expired),
            "only the first Identity 401 may resend; provider authentication never does"
        );
        assert_eq!(
            fake.state.access_issues.lock().unwrap().get(INVOKE_SCOPE),
            Some(&(1 + u64::from(identity_expired))),
            "only Identity 401 renews authority; HTTP 409 does not"
        );
        let reply =
            serde_json::to_value(reply.expect("409 retains the typed authentication result"))
                .unwrap();
        assert_eq!(reply["error"]["code"], "authentication_required");
        assert_eq!(reply["error"]["authentication"]["attempt"], "not_attempted");
    }
}

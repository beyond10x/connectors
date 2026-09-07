//! OAuth configuration, custody and exact Connection routing through the actual runtime owners.

use std::collections::{BTreeMap, VecDeque};
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use connector_secrets::{MemoryStore, Secret, SecretStore};
use connectors_config::CatalogIntegrationConfig;
use connectors_runtime::{BackendRegistry, PersonalRuntime};
use integration_catalog::{CatalogBackend, PersonalOAuthBackend};
use protocol::{connection, operation};
use service::{ConnectorBackend, EgressHttpRequest, EgressTransport, PrincipalContext};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

fn owner() -> operation::OwnerContext {
    operation::OwnerContext {
        tenant_id: "fixture".into(),
        agent_id: "fixture".into(),
        agent_revision: 1,
        authority_snapshot_id: "fixture".into(),
        authority_snapshot_sha256: "a".repeat(64),
    }
}
fn principal() -> PrincipalContext {
    PrincipalContext::local(&owner()).unwrap()
}
fn registration() -> CatalogIntegrationConfig {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    serde_json::from_value(serde_json::json!({
        "provider": "gitlab", "instance": "oauth", "grant_ref": "grant:oauth", "initiation": "platform",
        "operator_approved": true, "credential": "gitlab.oauth_token", "endpoints": {"origin": "https://gitlab.example"},
        "oauth": {"auth_profile": "gitlab.oauth_token", "flow": "authorization_code_pkce", "client_authentication": "public",
        "client_id": "fixture-client", "redirect_uri": format!("http://{}/oauth/callback", listener.local_addr().unwrap()),
        "browser_placement": "same_machine", "registration_use": "development_only", "custody": "development_file",
        "allowed_scopes": ["read_api"]}
    })).unwrap()
}
fn config_file(root: &Path, extra: &str) -> std::path::PathBuf {
    let path = root.join("connectors.toml");
    let source = format!(
        r#"[owner]
tenant_id = "fixture"
agent_id = "fixture"
agent_revision = 1
authority_snapshot_id = "fixture"
authority_snapshot_sha256 = "{}"
[[catalog]]
provider = "gitlab"
instance = "oauth"
grant_ref = "grant:oauth"
initiation = "platform"
operator_approved = true
credential = "gitlab.oauth_token"
[catalog.endpoints]
origin = "https://gitlab.example"
[catalog.oauth]
auth_profile = "gitlab.oauth_token"
flow = "authorization_code_pkce"
client_authentication = "public"
client_id = "fixture-client"
redirect_uri = "http://127.0.0.1:47193/oauth/callback"
browser_placement = "same_machine"
registration_use = "development_only"
custody = "development_file"
allowed_scopes = ["read_api"]
{extra}
"#,
        "a".repeat(64)
    );
    std::fs::write(&path, source).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    path
}
fn create() -> connection::ConnectionRequest {
    connection::ConnectionRequest::ConnectSessionCreate(connection::ConnectSessionCreateRequest {
        integration_ref: "gitlab".into(),
        label: "Display label".into(),
        auth_profile: Some("gitlab.oauth_token".into()),
    })
}

#[tokio::test]
async fn composition_opens_only_the_explicit_oauth_custody_and_reports_unsealed_readiness() {
    let root = tempfile::tempdir().unwrap();
    let config = config_file(root.path(), "");
    let state = root.path().join("state");
    let runtime = PersonalRuntime::bind(Some(&config), &state).await.unwrap();
    assert_eq!(runtime.readiness()["personal_oauth_connections"], 1);
    assert_eq!(
        runtime.readiness()["personal_oauth_custody"],
        "development_file_unsealed"
    );
    assert_eq!(runtime.readiness()["credential_store"], "file");
    assert!(state.join("oauth/credentials.store").is_file());
    assert!(state.join("oauth/transactions.sqlite").is_file());
    assert!(!state.join("credentials.store").exists());
    assert_eq!(
        std::fs::metadata(state.join("oauth/credentials.store"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    runtime.serve_until(std::future::ready(())).await.unwrap();
    let reopened = PersonalRuntime::bind(Some(&config), &state).await.unwrap();
    reopened.serve_until(std::future::ready(())).await.unwrap();
    assert!(!state.join("connectors.sock").exists());
}

#[tokio::test]
async fn unsupported_production_registration_refuses_before_readiness_or_oauth_store() {
    let root = tempfile::tempdir().unwrap();
    let config = config_file(root.path(), "");
    let source = std::fs::read_to_string(&config)
        .unwrap()
        .replace("development_only", "production_allowed");
    std::fs::write(&config, source).unwrap();
    let state = root.path().join("state");
    assert!(PersonalRuntime::bind(Some(&config), &state).await.is_err());
    assert!(!state.join("connectors.sock").exists());
    assert!(!state.join("oauth").exists());
}

#[derive(Default)]
struct Egress {
    replies: Mutex<VecDeque<serde_json::Value>>,
    requests: Mutex<Vec<(String, EgressHttpRequest)>>,
}
impl Egress {
    fn reply(&self, value: serde_json::Value) {
        self.replies.lock().unwrap().push_back(value);
    }
    fn count(&self) -> usize {
        self.requests.lock().unwrap().len()
    }
}
#[async_trait::async_trait]
impl EgressTransport for Egress {
    async fn execute(
        &self,
        connection: &str,
        request: EgressHttpRequest,
    ) -> Result<service::EgressHttpResponse, service::EgressTransportError> {
        self.requests
            .lock()
            .unwrap()
            .push((connection.into(), request));
        Ok(service::EgressHttpResponse {
            status: 200,
            headers: BTreeMap::new(),
            body: self
                .replies
                .lock()
                .unwrap()
                .pop_front()
                .expect("queued synthetic response")
                .to_string()
                .into_bytes(),
        })
    }
    async fn connect_websocket(
        &self,
        _: &str,
        _: String,
        _: usize,
    ) -> Result<Box<dyn service::EgressWebSocket>, service::EgressTransportError> {
        unreachable!("no OAuth websocket")
    }
}
async fn http(authority: &str, path: &str, extra: &str) -> String {
    let mut stream = tokio::net::TcpStream::connect(authority).await.unwrap();
    stream
        .write_all(format!("GET {path} HTTP/1.1\r\nHost: {authority}\r\n{extra}\r\n").as_bytes())
        .await
        .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    response
}
async fn authorize(backend: &dyn ConnectorBackend) -> String {
    let connection::ConnectionResult::ConnectSessionCreate(status) = backend
        .handle_connection(&principal(), create())
        .await
        .unwrap()
    else {
        panic!("session")
    };
    let url = url::Url::parse(status.browser_completion_url.as_deref().unwrap()).unwrap();
    let authority = format!("127.0.0.1:{}", url.port().unwrap());
    let capability = url.fragment().unwrap().strip_prefix("token=").unwrap();
    let private = http(
        &authority,
        "/instructions",
        &format!("X-Connect-Session: {capability}\r\n"),
    )
    .await;
    let private: serde_json::Value =
        serde_json::from_str(private.split_once("\r\n\r\n").unwrap().1).unwrap();
    let authorization = url::Url::parse(private["authorization_url"].as_str().unwrap()).unwrap();
    let state = authorization
        .query_pairs()
        .find(|(key, _)| key == "state")
        .unwrap()
        .1
        .into_owned();
    assert!(http(
        &authority,
        &format!("/oauth/callback?state={state}&code=fixture-code"),
        ""
    )
    .await
    .starts_with("HTTP/1.1 200"));
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let result = backend
                .handle_connection(
                    &principal(),
                    connection::ConnectionRequest::ConnectSessionStatus(
                        connection::ConnectSessionStatusRequest {
                            connect_session_ref: status.connect_session_ref.clone(),
                        },
                    ),
                )
                .await;
            if let Ok(connection::ConnectionResult::ConnectSessionStatus(status)) = result {
                if status.state == connection::ConnectSessionState::Completed {
                    break status.connection_ref.unwrap();
                }
                assert_eq!(status.state, connection::ConnectSessionState::Pending);
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn mixed_raw_and_oauth_bindings_have_exactly_one_invoke_owner_and_keep_aggregation() {
    let root = tempfile::tempdir().unwrap();
    let configured = registration();
    let egress = Arc::new(Egress::default());
    egress.reply(serde_json::json!({"access_token": "oauth-access", "refresh_token": "oauth-refresh", "token_type": "Bearer", "expires_in": 60}));
    egress.reply(serde_json::json!({"resource_owner_id": 42, "scope": ["read_api"], "application": {"uid": "fixture-client"}}));
    let oauth = Arc::new(
        PersonalOAuthBackend::open(
            principal(),
            std::slice::from_ref(&configured),
            root.path(),
            egress.clone(),
            true,
        )
        .await
        .unwrap(),
    );
    let mut raw = configured.clone();
    raw.oauth = None;
    raw.instance = Some("raw".into());
    raw.credential = Some("gitlab.token".into());
    let store = Arc::new(MemoryStore::new());
    let address =
        integration_catalog::credential_address("fixture", "com.gitlab.api", &raw, "token")
            .unwrap();
    store
        .put(&address, &Secret::new("raw-access"))
        .await
        .unwrap();
    let raw_backend =
        Arc::new(CatalogBackend::bind_stored(principal(), &[raw], store, egress.clone()).unwrap());
    // The public raw constructor refuses OAuth registrations; the private delegate is owned by the OAuth backend.
    assert!(CatalogBackend::bind_stored(
        principal(),
        &[configured],
        Arc::new(MemoryStore::new()),
        egress.clone()
    )
    .is_err());
    let connection::ConnectionResult::Search {
        connections: raw_connections,
    } = raw_backend
        .handle_connection(
            &principal(),
            connection::ConnectionRequest::Search(connection::SearchRequest {
                query: String::new(),
                limit: 1,
            }),
        )
        .await
        .unwrap()
    else {
        panic!("raw connection metadata")
    };
    let raw_ref = raw_connections[0].connection_ref.clone();
    let registry = BackendRegistry::new(vec![raw_backend.clone(), oauth.clone()]);
    let oauth_ref = authorize(&registry).await;
    let operation::OperationResult::Describe(description) = registry
        .handle(
            &principal(),
            operation::OperationRequest::Describe(operation::DescribeRequest {
                operation_ref: "gitlab-project-list".into(),
            }),
        )
        .await
        .unwrap()
    else {
        panic!("description")
    };
    assert_eq!(
        description.connections.len(),
        2,
        "both independent configured bindings remain described"
    );
    for reference in [oauth_ref.clone(), raw_ref] {
        let request = operation::OperationRequest::Invoke(operation::InvokeRequest {
            operation_ref: description.operation_ref.clone(),
            connection_ref: reference.clone(),
            description_ref: description.description_ref.clone(),
            input: serde_json::json!({}),
            approval_evidence_ref: None,
        });
        assert_eq!(
            [
                raw_backend.owns_operation(&request),
                oauth.owns_operation(&request)
            ]
            .into_iter()
            .filter(|owns| *owns)
            .count(),
            1
        );
        egress.reply(serde_json::json!([]));
        registry.handle(&principal(), request).await.unwrap();
        assert_eq!(egress.requests.lock().unwrap().last().unwrap().0, reference);
    }
    for reference in ["connection:gitlab:unknown", "connection:slack:other"] {
        let before = egress.count();
        let request = operation::OperationRequest::Invoke(operation::InvokeRequest {
            operation_ref: description.operation_ref.clone(),
            connection_ref: reference.into(),
            description_ref: description.description_ref.clone(),
            input: serde_json::json!({}),
            approval_evidence_ref: None,
        });
        assert!(!raw_backend.owns_operation(&request));
        assert!(!oauth.owns_operation(&request));
        assert!(registry.handle(&principal(), request).await.is_err());
        assert_eq!(egress.count(), before);
    }
    {
        let requests = egress.requests.lock().unwrap();
        for (index, expected) in [(2, "Bearer oauth-access"), (3, "Bearer raw-access")] {
            let headers = &requests[index].1.request.headers;
            assert_eq!(
                headers
                    .get("Authorization")
                    .or_else(|| headers.get("authorization"))
                    .map(String::as_str),
                Some(expected)
            );
        }
    }
    registry.shutdown().await;
}

#[tokio::test]
async fn oauth_pass1_daemon_refuses_ambiguous_v1_profile_without_using_label_as_target() {
    let root = tempfile::tempdir().unwrap();
    let path = config_file(root.path(), "");
    let source = std::fs::read_to_string(&path).unwrap();
    let second = source
        .split_once("[[catalog]]")
        .unwrap()
        .1
        .replace("instance = \"oauth\"", "instance = \"another\"")
        .replace(
            "grant_ref = \"grant:oauth\"",
            "grant_ref = \"grant:another\"",
        );
    std::fs::write(&path, format!("{source}\n[[catalog]]{second}")).unwrap();
    let state = root.path().join("state");
    let runtime = PersonalRuntime::bind(Some(&path), &state).await.unwrap();
    assert_eq!(runtime.readiness()["personal_oauth_connections"], 2);
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let serving = tokio::spawn(runtime.serve_until(async {
        let _ = stopped.await;
    }));
    for profile in [None, Some("gitlab.oauth_token"), Some("gitlab.token")] {
        let request = connection::RequestEnvelope {
            protocol: connection::CONTRACT.into(),
            request_id: "request:adversary".into(),
            context: owner(),
            request: connection::ConnectionRequest::ConnectSessionCreate(
                connection::ConnectSessionCreateRequest {
                    integration_ref: "gitlab".into(),
                    label: "another".into(),
                    auth_profile: profile.map(str::to_owned),
                },
            ),
        };
        let mut stream = tokio::net::UnixStream::connect(state.join("connectors.sock"))
            .await
            .unwrap();
        stream
            .write_all(format!("{}\n", serde_json::to_string(&request).unwrap()).as_bytes())
            .await
            .unwrap();
        stream.shutdown().await.unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();
        let response: connection::ResponseEnvelope = serde_json::from_str(&response).unwrap();
        response.validate().unwrap();
        assert_eq!(response.status, connection::ResponseStatus::Error);
        assert!(response.response.is_none());
        assert!(!serde_json::to_string(&response).unwrap().contains("http"));
    }
    stop.send(()).unwrap();
    serving.await.unwrap().unwrap();
    assert!(!state.join("connectors.sock").exists());
}

async fn connection_v2_frame(
    socket: &Path,
    request: &protocol::connection_v2::RequestEnvelope,
) -> Vec<u8> {
    let mut stream = tokio::net::UnixStream::connect(socket).await.unwrap();
    let mut bytes = serde_json::to_vec(request).unwrap();
    bytes.push(b'\n');
    stream.write_all(&bytes).await.unwrap();
    stream.shutdown().await.unwrap();
    let mut response = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut response))
        .await
        .unwrap()
        .unwrap();
    response
}

#[tokio::test]
async fn remediation_local_v2_routes_a_created_binding_and_joins_its_endpoint() {
    use protocol::connection_v2 as v2;
    let root = tempfile::tempdir().unwrap();
    let configured = registration();
    let target =
        integration_catalog::personal_oauth_admitted_connection_ref(&principal(), &configured)
            .unwrap();
    let egress = Arc::new(Egress::default());
    let backend = Arc::new(
        PersonalOAuthBackend::open(
            principal(),
            &[configured],
            root.path(),
            egress.clone(),
            true,
        )
        .await
        .unwrap(),
    );
    let registry = Arc::new(BackendRegistry::new(vec![backend]));
    let socket = root.path().join("local/connectors.sock");
    let daemon = server::local::LocalOperationDaemon::bind(&socket, registry)
        .await
        .unwrap();
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let serving = tokio::spawn(daemon.serve_until(async {
        let _ = stopped.await;
    }));
    let start = |connection_ref: &str, input| v2::RequestEnvelope {
        protocol: v2::CONTRACT.into(),
        request_id: "remediation:local".into(),
        context: owner(),
        request: v2::ConnectionRequest::RemediationStart(v2::RemediationStartRequest {
            operation_ref: "gitlab-project-list".into(),
            connection_ref: connection_ref.into(),
            input,
        }),
    };
    let unknown =
        connection_v2_frame(&socket, &start("connection:absent", serde_json::json!({}))).await;
    let invalid = connection_v2_frame(&socket, &start(&target, serde_json::json!(42))).await;
    let pending = connection_v2_frame(&socket, &start(&target, serde_json::json!({}))).await;
    let decoded = v2::decode_response(&pending);
    let status = if let Ok((_, envelope)) = &decoded {
        if let Some(v2::ConnectionResult::RemediationStart(pending)) = &envelope.response {
            let request = v2::RequestEnvelope {
                protocol: v2::CONTRACT.into(),
                request_id: "remediation:status".into(),
                context: owner(),
                request: v2::ConnectionRequest::RemediationStatus(v2::RemediationStatusRequest {
                    connect_session_ref: pending.connect_session_ref.clone(),
                }),
            };
            Some(connection_v2_frame(&socket, &request).await)
        } else {
            None
        }
    } else {
        None
    };
    stop.send(()).unwrap();
    serving.await.unwrap().unwrap();
    assert!(!socket.exists());
    assert_eq!(
        egress.count(),
        0,
        "trusted start/status do not dispatch an operation or contact the provider"
    );
    let (_, unknown) = v2::decode_response(&unknown).expect("selected v2 refusal");
    assert_eq!(
        unknown.error.unwrap().code,
        connection::ConnectionErrorCode::NotGranted
    );
    let (_, invalid) = v2::decode_response(&invalid).expect("selected v2 invalid input");
    assert_eq!(
        invalid.error.unwrap().code,
        connection::ConnectionErrorCode::InvalidInput
    );
    let (_, envelope) = decoded.expect("selected v2 bound start");
    let Some(v2::ConnectionResult::RemediationStart(pending)) = envelope.response else {
        panic!("bound pending session")
    };
    assert_eq!(pending.resume_state, v2::RemediationResumeState::Pending);
    assert_eq!(pending.connection_ref, target);
    assert_eq!(pending.integration_ref, "gitlab");
    assert_eq!(pending.auth_profile, "gitlab.oauth_token");
    let (_, status) = v2::decode_response(status.as_deref().expect("status requested")).unwrap();
    assert!(matches!(
        status.response,
        Some(v2::ConnectionResult::RemediationStatus(_))
    ));
    let endpoint =
        url::Url::parse(pending.session.browser_completion_url.as_deref().unwrap()).unwrap();
    assert!(
        tokio::net::TcpStream::connect(("127.0.0.1", endpoint.port().unwrap()))
            .await
            .is_err()
    );
}

async fn operation_v3_frame(
    socket: &Path,
    request: operation::OperationRequest,
) -> operation::v3::ResponseEnvelope {
    let request = operation::v3::RequestEnvelope {
        protocol: operation::v3::CONTRACT.into(),
        request_id: "remediation:explicit-operation".into(),
        context: owner(),
        request,
    };
    let mut stream = tokio::net::UnixStream::connect(socket).await.unwrap();
    stream
        .write_all(format!("{}\n", serde_json::to_string(&request).unwrap()).as_bytes())
        .await
        .unwrap();
    stream.shutdown().await.unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).await.unwrap();
    let (version, response) = operation::versions::decode_response(&response).unwrap();
    assert_eq!(version, operation::versions::Version::V0Alpha3);
    assert_eq!(response.request_id, request.request_id);
    response
}

#[tokio::test]
async fn remediation_local_completion_dispatches_only_a_later_explicit_invocation() {
    use protocol::connection_v2 as v2;
    let root = tempfile::tempdir().unwrap();
    let configured = registration();
    let connection =
        integration_catalog::personal_oauth_admitted_connection_ref(&principal(), &configured)
            .unwrap();
    let egress = Arc::new(Egress::default());
    egress.reply(serde_json::json!({"access_token":"bound-local-access", "refresh_token":"bound-local-refresh", "token_type":"Bearer", "expires_in":60}));
    egress.reply(serde_json::json!({"resource_owner_id":42,"scope":["read_api"],"application":{"uid":"fixture-client"}}));
    let backend = Arc::new(
        PersonalOAuthBackend::open(
            principal(),
            &[configured],
            root.path(),
            egress.clone(),
            true,
        )
        .await
        .unwrap(),
    );
    let registry = Arc::new(BackendRegistry::new(vec![backend]));
    let socket = root.path().join("local/connectors.sock");
    let daemon = server::local::LocalOperationDaemon::bind(&socket, registry)
        .await
        .unwrap();
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let serving = tokio::spawn(daemon.serve_until(async {
        let _ = stopped.await;
    }));
    let wire_socket = socket.clone();
    // Always join daemon shutdown, including a failed scenario assertion or timeout.
    let mut scenario = tokio::spawn(async move {
        let socket = wire_socket.as_path();
        let frame = |request| v2::RequestEnvelope {
            protocol: v2::CONTRACT.into(),
            request_id: "remediation:paired".into(),
            context: owner(),
            request,
        };
        let start = frame(v2::ConnectionRequest::RemediationStart(
            v2::RemediationStartRequest {
                operation_ref: "gitlab-project-list".into(),
                connection_ref: connection.clone(),
                input: serde_json::json!({}),
            },
        ));
        let (_, response) =
            v2::decode_response(&connection_v2_frame(socket, &start).await).unwrap();
        let Some(v2::ConnectionResult::RemediationStart(start)) = response.response else {
            panic!("bound start: {:?}", response.error)
        };
        assert_eq!(start.resume_state, v2::RemediationResumeState::Pending);
        assert_eq!(egress.count(), 0);
        let url =
            url::Url::parse(start.session.browser_completion_url.as_deref().unwrap()).unwrap();
        let authority = format!("127.0.0.1:{}", url.port().unwrap());
        let capability = url.fragment().unwrap().strip_prefix("token=").unwrap();
        let private = http(
            &authority,
            "/instructions",
            &format!("X-Connect-Session: {capability}\r\n"),
        )
        .await;
        let private: serde_json::Value =
            serde_json::from_str(private.split_once("\r\n\r\n").unwrap().1).unwrap();
        let authorization =
            url::Url::parse(private["authorization_url"].as_str().unwrap()).unwrap();
        let state = authorization
            .query_pairs()
            .find(|(key, _)| key == "state")
            .unwrap()
            .1
            .into_owned();
        assert!(http(
            &authority,
            &format!("/oauth/callback?state={state}&code=fixture-code"),
            ""
        )
        .await
        .starts_with("HTTP/1.1 200"));
        let status_request = frame(v2::ConnectionRequest::RemediationStatus(
            v2::RemediationStatusRequest {
                connect_session_ref: start.connect_session_ref.clone(),
            },
        ));
        loop {
            let (_, response) =
                v2::decode_response(&connection_v2_frame(socket, &status_request).await).unwrap();
            let Some(v2::ConnectionResult::RemediationStatus(status)) = response.response else {
                panic!("bound status: {:?}", response.error)
            };
            if status.resume_state == v2::RemediationResumeState::Ready {
                break;
            }
            assert_eq!(status.resume_state, v2::RemediationResumeState::Pending);
            tokio::task::yield_now().await;
        }
        assert_eq!(
            egress.count(),
            2,
            "completion made only token and evidence requests"
        );
        let acknowledgement = frame(v2::ConnectionRequest::RemediationAcknowledge(
            v2::RemediationAcknowledgeRequest {
                connect_session_ref: start.connect_session_ref.clone(),
                operation_ref: "gitlab-project-list".into(),
                connection_ref: connection.clone(),
            },
        ));
        let (_, response) =
            v2::decode_response(&connection_v2_frame(socket, &acknowledgement).await).unwrap();
        assert!(matches!(
            response.response,
            Some(v2::ConnectionResult::RemediationAcknowledge(_))
        ));
        let (_, response) =
            v2::decode_response(&connection_v2_frame(socket, &status_request).await).unwrap();
        let Some(v2::ConnectionResult::RemediationStatus(status)) = response.response else {
            panic!("consumed status")
        };
        assert_eq!(status.resume_state, v2::RemediationResumeState::Consumed);
        assert_eq!(
            egress.count(),
            2,
            "acknowledgement/status never dispatch the intended operation"
        );
        let response = operation_v3_frame(
            socket,
            operation::OperationRequest::Describe(operation::DescribeRequest {
                operation_ref: "gitlab-project-list".into(),
            }),
        )
        .await;
        let Some(operation::OperationResult::Describe(description)) = response.response else {
            panic!("fresh description: {:?}", response.error)
        };
        assert!(description
            .connections
            .iter()
            .any(|candidate| candidate.connection_ref == connection));
        assert_eq!(
            egress.count(),
            2,
            "fresh description still never dispatches"
        );
        egress.reply(serde_json::json!([]));
        let response = operation_v3_frame(
            socket,
            operation::OperationRequest::Invoke(operation::InvokeRequest {
                operation_ref: description.operation_ref,
                connection_ref: connection.clone(),
                description_ref: description.description_ref,
                input: serde_json::json!({}),
                approval_evidence_ref: None,
            }),
        )
        .await;
        assert!(
            matches!(
                response.response,
                Some(operation::OperationResult::Invoke(_))
            ),
            "{:?}",
            response.error
        );
        assert_eq!(
            egress.count(),
            3,
            "one separately submitted invocation dispatches exactly once"
        );
        assert_eq!(
            egress.requests.lock().unwrap().last().unwrap().0,
            connection
        );
        assert!(tokio::net::TcpStream::connect(authority).await.is_err());
    });
    let completed = tokio::time::timeout(Duration::from_secs(10), &mut scenario).await;
    if completed.is_err() {
        scenario.abort();
        let _ = scenario.await;
    }
    stop.send(()).unwrap();
    serving.await.unwrap().unwrap();
    assert!(!socket.exists());
    completed.unwrap().unwrap();
}

#[tokio::test]
async fn auth_adversary_local_same_profile_bindings_keep_completion_and_ack_exact() {
    use protocol::connection_v2 as v2;
    let root = tempfile::tempdir().unwrap();
    let other = registration();
    let other_connection =
        integration_catalog::personal_oauth_admitted_connection_ref(&principal(), &other).unwrap();
    let mut configured = registration();
    configured.instance = Some("adversary-second".into());
    configured.grant_ref = "grant:adversary-second".into();
    let connection =
        integration_catalog::personal_oauth_admitted_connection_ref(&principal(), &configured)
            .unwrap();
    let egress = Arc::new(Egress::default());
    egress.reply(serde_json::json!({"access_token":"bound-local-access", "refresh_token":"bound-local-refresh", "token_type":"Bearer", "expires_in":60}));
    egress.reply(serde_json::json!({"resource_owner_id":42,"scope":["read_api"],"application":{"uid":"fixture-client"}}));
    let backend = Arc::new(
        PersonalOAuthBackend::open(
            principal(),
            &[other, configured],
            root.path(),
            egress.clone(),
            true,
        )
        .await
        .unwrap(),
    );
    let registry = Arc::new(BackendRegistry::new(vec![backend]));
    let socket = root.path().join("local/connectors.sock");
    let daemon = server::local::LocalOperationDaemon::bind(&socket, registry)
        .await
        .unwrap();
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let serving = tokio::spawn(daemon.serve_until(async {
        let _ = stopped.await;
    }));
    let wire_socket = socket.clone();
    // Always join daemon shutdown, including a failed scenario assertion or timeout.
    let mut scenario = tokio::spawn(async move {
        let socket = wire_socket.as_path();
        let frame = |request| v2::RequestEnvelope {
            protocol: v2::CONTRACT.into(),
            request_id: "remediation:paired".into(),
            context: owner(),
            request,
        };
        let start = frame(v2::ConnectionRequest::RemediationStart(
            v2::RemediationStartRequest {
                operation_ref: "gitlab-project-list".into(),
                connection_ref: connection.clone(),
                input: serde_json::json!({}),
            },
        ));
        let (_, response) =
            v2::decode_response(&connection_v2_frame(socket, &start).await).unwrap();
        let Some(v2::ConnectionResult::RemediationStart(start)) = response.response else {
            panic!("bound start: {:?}", response.error)
        };
        assert_eq!(start.resume_state, v2::RemediationResumeState::Pending);
        assert_eq!(egress.count(), 0);
        let url =
            url::Url::parse(start.session.browser_completion_url.as_deref().unwrap()).unwrap();
        let authority = format!("127.0.0.1:{}", url.port().unwrap());
        let capability = url.fragment().unwrap().strip_prefix("token=").unwrap();
        let private = http(
            &authority,
            "/instructions",
            &format!("X-Connect-Session: {capability}\r\n"),
        )
        .await;
        let private: serde_json::Value =
            serde_json::from_str(private.split_once("\r\n\r\n").unwrap().1).unwrap();
        let authorization =
            url::Url::parse(private["authorization_url"].as_str().unwrap()).unwrap();
        let state = authorization
            .query_pairs()
            .find(|(key, _)| key == "state")
            .unwrap()
            .1
            .into_owned();
        assert!(http(
            &authority,
            &format!("/oauth/callback?state={state}&code=fixture-code"),
            ""
        )
        .await
        .starts_with("HTTP/1.1 200"));
        let status_request = frame(v2::ConnectionRequest::RemediationStatus(
            v2::RemediationStatusRequest {
                connect_session_ref: start.connect_session_ref.clone(),
            },
        ));
        loop {
            let (_, response) =
                v2::decode_response(&connection_v2_frame(socket, &status_request).await).unwrap();
            let Some(v2::ConnectionResult::RemediationStatus(status)) = response.response else {
                panic!("bound status: {:?}", response.error)
            };
            if status.resume_state == v2::RemediationResumeState::Ready {
                break;
            }
            assert_eq!(status.resume_state, v2::RemediationResumeState::Pending);
            tokio::task::yield_now().await;
        }
        assert_eq!(
            egress.count(),
            2,
            "completion made only token and evidence requests"
        );
        let mut wrong_owner = status_request.clone();
        wrong_owner.context.agent_id = "other-owner".into();
        let (_, refused) =
            v2::decode_response(&connection_v2_frame(socket, &wrong_owner).await).unwrap();
        assert!(refused.response.is_none());
        let wrong_ack = frame(v2::ConnectionRequest::RemediationAcknowledge(
            v2::RemediationAcknowledgeRequest {
                connect_session_ref: start.connect_session_ref.clone(),
                operation_ref: "gitlab-project-list".into(),
                connection_ref: other_connection.clone(),
            },
        ));
        let (_, refused) =
            v2::decode_response(&connection_v2_frame(socket, &wrong_ack).await).unwrap();
        assert!(
            refused.response.is_none(),
            "another configured same-profile binding cannot acknowledge this session"
        );
        let acknowledgement = frame(v2::ConnectionRequest::RemediationAcknowledge(
            v2::RemediationAcknowledgeRequest {
                connect_session_ref: start.connect_session_ref.clone(),
                operation_ref: "gitlab-project-list".into(),
                connection_ref: connection.clone(),
            },
        ));
        let (_, response) =
            v2::decode_response(&connection_v2_frame(socket, &acknowledgement).await).unwrap();
        assert!(matches!(
            response.response,
            Some(v2::ConnectionResult::RemediationAcknowledge(_))
        ));
        let (_, response) =
            v2::decode_response(&connection_v2_frame(socket, &status_request).await).unwrap();
        let Some(v2::ConnectionResult::RemediationStatus(status)) = response.response else {
            panic!("consumed status")
        };
        assert_eq!(status.resume_state, v2::RemediationResumeState::Consumed);
        assert_eq!(
            egress.count(),
            2,
            "acknowledgement/status never dispatch the intended operation"
        );
        let response = operation_v3_frame(
            socket,
            operation::OperationRequest::Describe(operation::DescribeRequest {
                operation_ref: "gitlab-project-list".into(),
            }),
        )
        .await;
        let Some(operation::OperationResult::Describe(description)) = response.response else {
            panic!("fresh description: {:?}", response.error)
        };
        assert!(description
            .connections
            .iter()
            .any(|candidate| candidate.connection_ref == connection));
        assert_eq!(
            egress.count(),
            2,
            "fresh description still never dispatches"
        );
        assert!(
            !description
                .connections
                .iter()
                .any(|candidate| candidate.connection_ref == other_connection),
            "the uncompleted configured peer stays out of callable discovery"
        );
        assert_eq!(
            egress.count(),
            2,
            "only the exact bound credential was acquired; no invocation"
        );
        assert!(egress
            .requests
            .lock()
            .unwrap()
            .iter()
            .all(|request| request.0 == connection));
        assert!(tokio::net::TcpStream::connect(authority).await.is_err());
    });
    let completed = tokio::time::timeout(Duration::from_secs(10), &mut scenario).await;
    if completed.is_err() {
        scenario.abort();
        let _ = scenario.await;
    }
    stop.send(()).unwrap();
    serving.await.unwrap().unwrap();
    assert!(!socket.exists());
    completed.unwrap().unwrap();
}

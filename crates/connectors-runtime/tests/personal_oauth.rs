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

#[tokio::test]
async fn one_shot_create_refuses_before_configuration_custody_and_listener() {
    let root = tempfile::tempdir().unwrap();
    let config = config_file(root.path(), "");
    let state = root.path().join("state");
    let response = PersonalRuntime::one_shot_connection(&config, &state, owner(), create())
        .await
        .unwrap();
    assert_eq!(response.status, connection::ResponseStatus::Error);
    assert!(!state.exists());
    assert!(response.response.is_none());
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

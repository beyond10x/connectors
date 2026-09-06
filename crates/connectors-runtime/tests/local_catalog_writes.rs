//! Regression coverage for catalog writes through the owner-authenticated local socket.
//! Only egress is fake: description, selected grants, credential resolution and framing are real.

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use connector_secrets::MemoryStore;
use connectors_config::{CatalogIntegrationConfig, InitiationConfig, NetworkScopeConfig};
use connectors_runtime::BackendRegistry;
use integration_catalog::CatalogBackend;
use protocol::operation::{
    ApprovalPosture, DescribeRequest, EffectClass, InvokeRequest, OperationDescription,
    OperationError, OperationErrorCode, OperationRequest, OperationResult, OwnerContext,
    RequestEnvelope, ResponseEnvelope, SearchRequest, CONTRACT,
};
use server::local::LocalOperationDaemon;
use service::{
    EgressHttpRequest, EgressHttpResponse, EgressTransport, EgressTransportError, EgressWebSocket,
    PrincipalContext,
};
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};
use tokio::net::UnixStream;
use tokio::sync::oneshot;

const POST: &str = "slack-chat-post-message";
const READ: &str = "slack-conversations-list";

#[derive(Default)]
struct FixtureEgress {
    status: AtomicU16,
    calls: Mutex<Vec<String>>,
    body_override: Mutex<Option<Vec<u8>>>,
}

#[async_trait]
impl EgressTransport for FixtureEgress {
    async fn execute(
        &self,
        authority_ref: &str,
        request: EgressHttpRequest,
    ) -> Result<EgressHttpResponse, EgressTransportError> {
        assert_eq!(request.request.method, "POST");
        assert_eq!(
            request.request.url,
            "https://slack.com/api/chat.postMessage"
        );
        assert_eq!(
            request
                .request
                .headers
                .get("Authorization")
                .map(String::as_str),
            Some("Bearer fixture-writer")
        );
        let body: serde_json::Value =
            serde_json::from_str(request.request.body.as_deref().unwrap()).unwrap();
        assert_eq!(body["channel"], "C-FIXTURE");
        assert_eq!(body["text"], "fixture post");
        self.calls.lock().unwrap().push(authority_ref.to_owned());
        let status = self.status.load(Ordering::Relaxed);
        Ok(EgressHttpResponse {
            status,
            headers: BTreeMap::new(),
            body: self
                .body_override
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_else(|| {
                    if status == 200 {
                        br#"{"ok":true,"channel":"C-FIXTURE","ts":"1.000001"}"#.to_vec()
                    } else {
                        br#"{"ok":false,"error":"fixture-provider-body-must-stay-private"}"#
                            .to_vec()
                    }
                }),
        })
    }

    async fn connect_websocket(
        &self,
        _authority_ref: &str,
        _url: String,
        _maximum_message_bytes: usize,
    ) -> Result<Box<dyn EgressWebSocket>, EgressTransportError> {
        panic!("a catalog HTTP fixture must never open a WebSocket")
    }
}

struct Fixture {
    root: tempfile::TempDir,
    socket: PathBuf,
    egress: Arc<FixtureEgress>,
    stop: oneshot::Sender<()>,
    serving: tokio::task::JoinHandle<()>,
}

impl Fixture {
    async fn start(writable_first: bool, include_writer: bool) -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let egress = Arc::new(FixtureEgress::default());
        egress.status.store(200, Ordering::Relaxed);
        let mut configured = vec![connection(root.path(), "reader", false)];
        if include_writer {
            configured.push(connection(root.path(), "writer", true));
        }
        if writable_first {
            configured.reverse();
        }
        let backend = CatalogBackend::open(
            PrincipalContext::local(&context()).unwrap(),
            &configured,
            &root.path().join("catalog"),
            Arc::new(MemoryStore::new()),
            egress.clone(),
        )
        .await
        .unwrap();
        let registry = Arc::new(BackendRegistry::new(vec![Arc::new(backend)]));
        let socket = root.path().join("s");
        let daemon = LocalOperationDaemon::bind(&socket, registry).await.unwrap();
        let (stop, stopped) = oneshot::channel();
        let serving = tokio::spawn(async move {
            daemon
                .serve_until(async {
                    let _ = stopped.await;
                })
                .await
                .unwrap();
        });
        Self {
            root,
            socket,
            egress,
            stop,
            serving,
        }
    }

    async fn request(&self, request: OperationRequest) -> Result<OperationResult, OperationError> {
        let envelope = RequestEnvelope {
            protocol: CONTRACT.to_owned(),
            request_id: "fixture-request".to_owned(),
            context: context(),
            request,
        };
        envelope.validate().unwrap();
        let mut stream = UnixStream::connect(&self.socket).await.unwrap();
        let mut bytes = serde_json::to_vec(&envelope).unwrap();
        bytes.push(b'\n');
        stream.write_all(&bytes).await.unwrap();
        let mut response = String::new();
        tokio::time::timeout(
            Duration::from_secs(5),
            BufReader::new(stream).read_line(&mut response),
        )
        .await
        .expect("the daemon must answer the bounded request")
        .unwrap();
        let response: ResponseEnvelope = serde_json::from_str(&response).unwrap();
        response
            .validate()
            .expect("the ordinary client can read the response");
        assert_eq!(response.request_id, envelope.request_id);
        match response.error {
            Some(error) => Err(error),
            None => Ok(response.response.unwrap()),
        }
    }

    async fn describe(&self, operation: &str) -> Result<OperationDescription, OperationError> {
        match self
            .request(OperationRequest::Describe(DescribeRequest {
                operation_ref: operation.to_owned(),
            }))
            .await?
        {
            OperationResult::Describe(description) => Ok(description),
            other => panic!("describe returned {other:?}"),
        }
    }

    async fn finish(self) {
        self.stop.send(()).unwrap();
        self.serving.await.unwrap();
        assert!(!self.socket.exists(), "shutdown must remove the socket");
        self.root.close().unwrap();
    }
}

fn context() -> OwnerContext {
    OwnerContext {
        tenant_id: "fixture-local".to_owned(),
        agent_id: "fixture-agent".to_owned(),
        agent_revision: 1,
        authority_snapshot_id: "fixture-authority".to_owned(),
        authority_snapshot_sha256: "a".repeat(64),
    }
}

fn connection(root: &Path, name: &str, allow_writes: bool) -> CatalogIntegrationConfig {
    let credential_file = root.join(name);
    std::fs::write(&credential_file, format!("fixture-{name}")).unwrap();
    std::fs::set_permissions(&credential_file, std::fs::Permissions::from_mode(0o600)).unwrap();
    CatalogIntegrationConfig {
        provider: "slack".to_owned(),
        instance: Some(name.to_owned()),
        label: Some(name.to_owned()),
        grant_ref: format!("grant:fixture:{name}"),
        initiation: InitiationConfig::Platform,
        allow_writes,
        endpoints: BTreeMap::new(),
        usernames: BTreeMap::new(),
        operator_approved: true,
        network: NetworkScopeConfig::Public,
        credential: Some("slack.bot_token".to_owned()),
        credential_file: Some(credential_file),
        oauth: None,
    }
}

fn invoke(description: &OperationDescription, connection_ref: &str) -> OperationRequest {
    OperationRequest::Invoke(InvokeRequest {
        operation_ref: POST.to_owned(),
        connection_ref: connection_ref.to_owned(),
        description_ref: description.description_ref.clone(),
        input: serde_json::json!({"channel":"C-FIXTURE", "text":"fixture post"}),
        // Local owner authority and the selected grant admit this fixture. Required metadata is
        // not a reason to invent a hosted approval record or an `event:` reference.
        approval_evidence_ref: None,
    })
}

async fn mixed_connections(writable_first: bool) {
    let fixture = Fixture::start(writable_first, true).await;
    let searched = fixture
        .request(OperationRequest::Search(SearchRequest {
            query: POST.to_owned(),
            limit: 25,
        }))
        .await
        .expect("mixed grants must not produce a protocol refusal");
    let OperationResult::Search { operations } = searched else {
        panic!("search must return operations")
    };
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].operation_ref, POST);
    assert_eq!(operations[0].effect, EffectClass::Mutating);
    assert_eq!(operations[0].approval, ApprovalPosture::Required);
    assert_eq!(operations[0].connections.len(), 1);
    assert_eq!(operations[0].connections[0].label, "writer");

    let description = fixture
        .describe(POST)
        .await
        .expect("an admitted write is describable");
    assert_eq!(description.effect, EffectClass::Mutating);
    assert_eq!(description.approval, ApprovalPosture::Required);
    assert_eq!(description.connections, operations[0].connections);
    let writer = &description.connections[0].connection_ref;
    let result = fixture.request(invoke(&description, writer)).await.unwrap();
    let OperationResult::Invoke(result) = result else {
        panic!("invoke must return its fixture result")
    };
    assert_eq!(result.operation_ref, POST);
    assert_eq!(result.output["ok"], true);
    assert!(!result.connector_audit_ref.is_empty());
    assert_eq!(
        fixture.egress.calls.lock().unwrap().as_slice(),
        std::slice::from_ref(writer)
    );

    let reads = fixture.describe(READ).await.unwrap();
    let reader = &reads
        .connections
        .iter()
        .find(|row| row.label == "reader")
        .unwrap()
        .connection_ref;
    let refused = fixture
        .request(invoke(&description, reader))
        .await
        .unwrap_err();
    assert_eq!(refused.code, OperationErrorCode::NotGranted);
    assert!(refused.message.contains("--allow writes"));
    assert!(refused.message.contains("allow_writes = true"));
    assert!(refused.message.contains("restart"));
    assert_eq!(
        fixture.egress.calls.lock().unwrap().len(),
        1,
        "a read-only selection cannot borrow the writer's grant"
    );
    fixture.finish().await;
}

#[tokio::test]
async fn read_only_first_still_discovers_describes_and_posts_through_the_writer() {
    mixed_connections(false).await;
}

#[tokio::test]
async fn writable_first_still_discovers_describes_and_posts_through_the_writer() {
    mixed_connections(true).await;
}

#[tokio::test]
async fn describing_a_write_directly_skips_the_first_read_only_connection() {
    let fixture = Fixture::start(false, true).await;
    let description = fixture.describe(POST).await.unwrap();
    assert_eq!(description.approval, ApprovalPosture::Required);
    assert_eq!(description.connections.len(), 1);
    assert_eq!(description.connections[0].label, "writer");
    assert!(fixture.egress.calls.lock().unwrap().is_empty());
    fixture.finish().await;
}

#[tokio::test]
async fn only_read_only_connections_hide_the_write_and_describe_its_missing_grant() {
    let fixture = Fixture::start(false, false).await;
    let result = fixture
        .request(OperationRequest::Search(SearchRequest {
            query: POST.to_owned(),
            limit: 25,
        }))
        .await
        .unwrap();
    assert_eq!(
        result,
        OperationResult::Search {
            operations: Vec::new()
        }
    );
    let refused = fixture.describe(POST).await.unwrap_err();
    assert_eq!(refused.code, OperationErrorCode::NotGranted);
    assert!(refused.message.contains("--allow writes"));
    assert!(refused.message.contains("allow_writes = true"));
    assert!(refused.message.contains("restart"));
    assert!(fixture.egress.calls.lock().unwrap().is_empty());
    fixture.finish().await;
}

#[tokio::test]
async fn stale_description_and_provider_refusal_have_distinct_actionable_results() {
    let fixture = Fixture::start(true, true).await;
    let description = fixture.describe(POST).await.unwrap();
    let writer = &description.connections[0].connection_ref;
    let mut stale = description.clone();
    stale.description_ref = "description:obsolete-fixture".to_owned();
    let refused = fixture.request(invoke(&stale, writer)).await.unwrap_err();
    assert_eq!(refused.code, OperationErrorCode::StaleAuthority);
    assert!(fixture.egress.calls.lock().unwrap().is_empty());

    fixture.egress.status.store(403, Ordering::Relaxed);
    let refused = fixture
        .request(invoke(&description, writer))
        .await
        .unwrap_err();
    assert_eq!(refused.code, OperationErrorCode::Unavailable);
    assert!(refused.message.contains("HTTP 403"));
    assert!(refused.message.contains("lacks permission"));
    assert!(!refused
        .message
        .contains("fixture-provider-body-must-stay-private"));
    assert_eq!(
        fixture.egress.calls.lock().unwrap().as_slice(),
        std::slice::from_ref(writer)
    );
    fixture.finish().await;
}

#[tokio::test]
async fn adversary_http_200_application_refusal_survives_the_documented_socket_path() {
    let fixture = Fixture::start(false, true).await;
    let provider_refusal = serde_json::json!({"ok": false, "error": "missing_scope"});
    *fixture.egress.body_override.lock().unwrap() =
        Some(serde_json::to_vec(&provider_refusal).unwrap());
    let description = fixture.describe(POST).await.unwrap();
    let writer = &description.connections[0].connection_ref;

    let result = fixture.request(invoke(&description, writer)).await.unwrap();
    let OperationResult::Invoke(result) = result else {
        panic!("an HTTP 200 response must retain its provider result")
    };
    assert_eq!(result.output, provider_refusal);
    assert_eq!(result.output["ok"], false);
    assert_eq!(fixture.egress.status.load(Ordering::Relaxed), 200);
    assert_eq!(fixture.egress.calls.lock().unwrap().len(), 1);
    fixture.finish().await;
}

#[tokio::test]
async fn adversary_a_read_description_cannot_authorize_a_different_write_operation() {
    let fixture = Fixture::start(false, true).await;
    let reads = fixture.describe(READ).await.unwrap();
    let writes = fixture.describe(POST).await.unwrap();
    let writer = &writes.connections[0].connection_ref;

    let refused = fixture.request(invoke(&reads, writer)).await.unwrap_err();
    assert_eq!(refused.code, OperationErrorCode::StaleAuthority);
    assert!(fixture.egress.calls.lock().unwrap().is_empty());
    fixture.finish().await;
}

#[tokio::test]
async fn adversary_an_approval_reference_cannot_raise_the_selected_read_only_grant() {
    let fixture = Fixture::start(false, true).await;
    let reads = fixture.describe(READ).await.unwrap();
    let writes = fixture.describe(POST).await.unwrap();
    let reader = &reads
        .connections
        .iter()
        .find(|connection| connection.label == "reader")
        .unwrap()
        .connection_ref;
    let OperationRequest::Invoke(mut request) = invoke(&writes, reader) else {
        unreachable!()
    };
    request.approval_evidence_ref = Some("approval:adversary-untrusted-fixture".to_owned());

    let refused = fixture
        .request(OperationRequest::Invoke(request))
        .await
        .unwrap_err();
    assert_eq!(refused.code, OperationErrorCode::NotGranted);
    assert!(refused.message.contains("allow_writes = true"));
    assert!(fixture.egress.calls.lock().unwrap().is_empty());
    fixture.finish().await;
}

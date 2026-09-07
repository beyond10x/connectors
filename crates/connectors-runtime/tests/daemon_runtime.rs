//! State ownership precedes every personal runtime side effect.

use std::os::unix::fs::PermissionsExt as _;
use std::sync::Arc;

use connectors_runtime::{BackendRegistry, PersonalRuntime};
use protocol::operation::{self, OperationRequest, OperationResult, RequestEnvelope};
use server::local::LocalOperationDaemon;
use server::local::LocalStateOwnership;
use service::{ConnectorBackend, PrincipalContext};
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};

fn context() -> operation::OwnerContext {
    operation::OwnerContext {
        tenant_id: "fixture".into(),
        agent_id: "fixture".into(),
        agent_revision: 1,
        authority_snapshot_id: "fixture".into(),
        authority_snapshot_sha256: "a".repeat(64),
    }
}

fn envelope(request: OperationRequest) -> RequestEnvelope {
    RequestEnvelope {
        protocol: operation::CONTRACT.into(),
        request_id: "fixture-request".into(),
        context: context(),
        request,
    }
}

#[tokio::test]
async fn an_existing_owner_refuses_before_opening_the_reply_claim_journal() {
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let daemon = LocalOperationDaemon::bind(
        root.path().join("connectors.sock"),
        Arc::new(BackendRegistry::new(Vec::new())),
    )
    .await
    .unwrap();
    assert!(PersonalRuntime::bind(None, root.path()).await.is_err());
    assert!(
        !root.path().join("event-reply-claims.sqlite").exists(),
        "contending composition must not mutate state owned by the daemon"
    );
    daemon.serve_until(std::future::ready(())).await.unwrap();
}

#[tokio::test]
async fn an_unsafe_socket_refuses_before_opening_the_reply_claim_journal() {
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let socket = root.path().join("connectors.sock");
    std::fs::write(&socket, "retain").unwrap();
    assert!(PersonalRuntime::bind(None, root.path()).await.is_err());
    assert_eq!(std::fs::read_to_string(socket).unwrap(), "retain");
    assert!(
        !root.path().join("event-reply-claims.sqlite").exists(),
        "an unsafe socket must refuse before journal or adapter initialization"
    );
}

#[derive(Default)]
struct FixtureEgress(std::sync::Mutex<Vec<service::EgressHttpRequest>>);

#[async_trait::async_trait]
impl service::EgressTransport for FixtureEgress {
    async fn execute(
        &self,
        _: &str,
        request: service::EgressHttpRequest,
    ) -> Result<service::EgressHttpResponse, service::EgressTransportError> {
        assert!(request
            .request
            .url
            .starts_with("https://slack.com/api/conversations.list"));
        assert_eq!(
            request
                .request
                .headers
                .get("Authorization")
                .map(String::as_str),
            Some("Bearer fixture-reader")
        );
        self.0.lock().unwrap().push(request);
        Ok(service::EgressHttpResponse {
            status: 200,
            headers: Default::default(),
            body: br#"{"ok":true,"channels":[]}"#.to_vec(),
        })
    }

    async fn connect_websocket(
        &self,
        _: &str,
        _: String,
        _: usize,
    ) -> Result<Box<dyn service::EgressWebSocket>, service::EgressTransportError> {
        panic!("a catalog read must not start a WebSocket")
    }
}

async fn catalog_request(
    root: &std::path::Path,
    egress: Arc<FixtureEgress>,
    request: OperationRequest,
) -> operation::ResponseEnvelope {
    let ownership = LocalStateOwnership::acquire(root.join("connectors.sock")).unwrap();
    let config = connectors_runtime::PersonalConfig::read(&root.join("connectors.toml")).unwrap();
    let backend = integration_catalog::CatalogBackend::open(
        PrincipalContext::local(&context()).unwrap(),
        &config.catalog,
        root,
        Arc::new(connector_secrets::MemoryStore::new()),
        egress,
    )
    .await
    .unwrap();
    let daemon = LocalOperationDaemon::bind_owned(
        ownership,
        Arc::new(BackendRegistry::new(vec![Arc::new(backend)])),
    )
    .await
    .unwrap();
    daemon_request(daemon, root.join("connectors.sock"), request).await
}

#[tokio::test]
async fn daemon_catalog_uses_the_described_credential_and_rejects_read_only_writes_before_egress() {
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut configuration = format!("[owner]\ntenant_id = 'fixture'\nagent_id = 'fixture'\nagent_revision = 1\nauthority_snapshot_id = 'fixture'\nauthority_snapshot_sha256 = '{}'\n", "a".repeat(64));
    for (label, writable) in [("reader", false), ("writer", true)] {
        let file = root.path().join(label);
        std::fs::write(&file, format!("fixture-{label}")).unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
        configuration.push_str(&format!("[[catalog]]\nprovider = 'slack'\ninstance = '{label}'\nlabel = '{label}'\ngrant_ref = 'grant:{label}'\ninitiation = 'platform'\nallow_writes = {writable}\ncredential_file = '{}'\n", file.display()));
    }
    let file = root.path().join("connectors.toml");
    std::fs::write(&file, configuration).unwrap();
    std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o600)).unwrap();
    let egress = Arc::new(FixtureEgress::default());
    let describe = |operation: &str| {
        OperationRequest::Describe(operation::DescribeRequest {
            operation_ref: operation.into(),
        })
    };
    let read = catalog_request(
        root.path(),
        egress.clone(),
        describe("slack-conversations-list"),
    )
    .await;
    let Some(OperationResult::Describe(read)) = read.response else {
        panic!("{read:?}")
    };
    let reader = read
        .connections
        .iter()
        .find(|connection| connection.label == "reader")
        .unwrap()
        .connection_ref
        .clone();
    let writer = read
        .connections
        .iter()
        .find(|connection| connection.label == "writer")
        .unwrap()
        .connection_ref
        .clone();
    assert_eq!(read.connections.len(), 2);
    assert_ne!(
        reader, writer,
        "named instances must have distinct Connection identities"
    );
    let invocation = catalog_request(
        root.path(),
        egress.clone(),
        OperationRequest::Invoke(operation::InvokeRequest {
            operation_ref: read.operation_ref,
            connection_ref: reader.clone(),
            description_ref: read.description_ref,
            input: serde_json::json!({"limit":1}),
            approval_evidence_ref: None,
        }),
    )
    .await;
    assert!(invocation.error.is_none(), "{invocation:?}");
    assert_eq!(egress.0.lock().unwrap().len(), 1);
    let write = catalog_request(
        root.path(),
        egress.clone(),
        describe("slack-chat-post-message"),
    )
    .await;
    let Some(OperationResult::Describe(write)) = write.response else {
        panic!("{write:?}")
    };
    let refused = catalog_request(
        root.path(),
        egress.clone(),
        OperationRequest::Invoke(operation::InvokeRequest {
            operation_ref: write.operation_ref,
            connection_ref: reader,
            description_ref: write.description_ref,
            input: serde_json::json!({"channel":"C-FIXTURE", "text":"fixture"}),
            approval_evidence_ref: None,
        }),
    )
    .await;
    assert_eq!(
        refused.error.unwrap().code,
        operation::OperationErrorCode::NotGranted
    );
    assert_eq!(
        egress.0.lock().unwrap().len(),
        1,
        "write refusal occurs before credential placement/egress"
    );
    assert!(!root.path().join("connectors.sock").exists());
}

struct ShutdownBarrierBackend {
    started: tokio::sync::Notify,
    release: tokio::sync::Notify,
    refuse: bool,
    dispatched: std::sync::atomic::AtomicUsize,
}

#[async_trait::async_trait]
impl ConnectorBackend for ShutdownBarrierBackend {
    async fn ready(&self) -> Result<(), service::BackendReadinessError> {
        Ok(())
    }
    async fn handle(
        &self,
        _: &PrincipalContext,
        _: OperationRequest,
    ) -> Result<OperationResult, operation::OperationError> {
        self.dispatched
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if self.refuse {
            Err(operation::OperationError::new(
                operation::OperationErrorCode::NotGranted,
                "fixture policy refusal",
                false,
            ))
        } else {
            Ok(OperationResult::Search {
                operations: Vec::new(),
            })
        }
    }
    async fn shutdown(&self) {
        self.started.notify_one();
        self.release.notified().await;
    }
}

#[tokio::test]
async fn adversary_state_lock_outlives_async_shutdown_on_success_and_refusal() {
    for refuse in [false, true] {
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let socket = root.path().join("connectors.sock");
        let backend = Arc::new(ShutdownBarrierBackend {
            started: tokio::sync::Notify::new(),
            release: tokio::sync::Notify::new(),
            refuse,
            dispatched: std::sync::atomic::AtomicUsize::new(0),
        });
        let daemon = LocalOperationDaemon::bind(&socket, backend.clone())
            .await
            .unwrap();
        let task = tokio::spawn(daemon_request(
            daemon,
            socket.clone(),
            OperationRequest::Search(operation::SearchRequest {
                query: String::new(),
                limit: 1,
            }),
        ));
        tokio::time::timeout(
            std::time::Duration::from_secs(3),
            backend.started.notified(),
        )
        .await
        .unwrap();
        assert!(
            LocalStateOwnership::acquire(&socket).is_err(),
            "lock released during shutdown"
        );
        assert_eq!(
            backend.dispatched.load(std::sync::atomic::Ordering::SeqCst),
            1
        );
        assert!(
            !task.is_finished(),
            "daemon returned before backend teardown"
        );
        backend.release.notify_one();
        let response = task.await.unwrap();
        assert_eq!(response.error.is_some(), refuse);
        assert!(LocalStateOwnership::acquire(&socket).is_ok());
        assert!(!socket.exists());
    }
}

#[derive(Default)]
struct MalformedReplyBackend {
    shutdown: std::sync::atomic::AtomicUsize,
}

#[async_trait::async_trait]
impl ConnectorBackend for MalformedReplyBackend {
    async fn ready(&self) -> Result<(), service::BackendReadinessError> {
        Ok(())
    }
    async fn handle(
        &self,
        _: &PrincipalContext,
        _: OperationRequest,
    ) -> Result<OperationResult, operation::OperationError> {
        Ok(OperationResult::Invoke(operation::InvocationResult {
            operation_ref: String::new(),
            output: serde_json::json!({}),
            connector_audit_ref: String::new(),
            execution_ref: None,
        }))
    }
    async fn shutdown(&self) {
        self.shutdown
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

#[tokio::test]
async fn final_adversary_malformed_backend_reply_is_reduced_before_shutdown_and_lock_release() {
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let socket = root.path().join("connectors.sock");
    let backend = Arc::new(MalformedReplyBackend::default());
    let daemon = LocalOperationDaemon::bind(&socket, backend.clone())
        .await
        .unwrap();
    let response = daemon_request(
        daemon,
        socket.clone(),
        OperationRequest::Search(operation::SearchRequest {
            query: String::new(),
            limit: 1,
        }),
    )
    .await;
    assert!(
        response.error.is_some(),
        "malformed backend response escaped: {response:?}"
    );
    assert!(response.response.is_none());
    assert!(response.validate().is_ok());
    assert_eq!(
        backend.shutdown.load(std::sync::atomic::Ordering::SeqCst),
        1
    );
    assert!(LocalStateOwnership::acquire(socket).is_ok());
}

async fn daemon_request<B: ConnectorBackend + ?Sized + 'static>(
    daemon: LocalOperationDaemon<B>,
    socket: std::path::PathBuf,
    request: OperationRequest,
) -> operation::ResponseEnvelope {
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let serving = tokio::spawn(daemon.serve_until(async {
        let _ = stopped.await;
    }));
    let mut stream = tokio::net::UnixStream::connect(socket).await.unwrap();
    let mut bytes = serde_json::to_vec(&envelope(request)).unwrap();
    bytes.push(b'\n');
    stream.write_all(&bytes).await.unwrap();
    let mut response = String::new();
    BufReader::new(stream)
        .read_line(&mut response)
        .await
        .unwrap();
    let response = serde_json::from_str(&response).unwrap();
    stop.send(()).unwrap();
    serving.await.unwrap().unwrap();
    response
}

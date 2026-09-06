//! State ownership precedes every personal runtime side effect.

use std::os::unix::fs::PermissionsExt as _;
use std::sync::Arc;

use connectors_runtime::{BackendRegistry, PersonalRuntime};
use protocol::operation::{self, OperationRequest, OperationResult, RequestEnvelope};
use server::local::LocalOperationDaemon;
use server::local::{LocalOneShot, LocalStateOwnership};
use service::{ConnectorBackend, PrincipalContext};

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
async fn auth_one_shot_v3_refuses_persistent_control_before_configuration_or_state() {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("uncreated-state");
    let response = PersonalRuntime::one_shot_operation_v3(
        &root.path().join("absent-configuration.toml"),
        &state,
        context(),
        OperationRequest::SessionStatus(operation::SessionRequest {
            execution_ref: "execution:fixture".into(),
        }),
    )
    .await
    .unwrap();
    response.validate().unwrap();
    assert_eq!(response.protocol, operation::v3::CONTRACT);
    assert!(response.response.is_none());
    let error = response.error.unwrap();
    assert_eq!(error.code, operation::v3::OperationErrorCode::Unavailable);
    assert!(!error.retriable);
    assert!(error.authentication.is_none());
    assert!(!state.exists());
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
        panic!("an ephemeral operation must not start a WebSocket")
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
    LocalOneShot::new(
        ownership,
        Arc::new(BackendRegistry::new(vec![Arc::new(backend)])),
    )
    .unwrap()
    .operation(envelope(request))
    .await
    .unwrap()
}

#[tokio::test]
async fn ephemeral_catalog_uses_the_described_credential_and_rejects_read_only_writes_before_egress(
) {
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

#[derive(Default)]
struct UnknownLifetimeBackend {
    dispatched: std::sync::atomic::AtomicBool,
    shutdown: std::sync::atomic::AtomicBool,
}

#[async_trait::async_trait]
impl ConnectorBackend for UnknownLifetimeBackend {
    async fn ready(&self) -> Result<(), service::BackendReadinessError> {
        Ok(())
    }
    async fn handle(
        &self,
        _: &PrincipalContext,
        _: OperationRequest,
    ) -> Result<OperationResult, operation::OperationError> {
        self.dispatched
            .store(true, std::sync::atomic::Ordering::SeqCst);
        panic!("unknown lifetime must never dispatch")
    }
    async fn shutdown(&self) {
        self.shutdown
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

#[tokio::test]
async fn unknown_backend_lifetime_is_refused_and_shutdown_releases_ownership() {
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let socket = root.path().join("connectors.sock");
    let backend = Arc::new(UnknownLifetimeBackend::default());
    let owner = LocalStateOwnership::acquire(&socket).unwrap();
    assert!(LocalStateOwnership::acquire(&socket).is_err());
    let refused = LocalOneShot::new(owner, backend.clone())
        .unwrap()
        .operation(envelope(OperationRequest::Invoke(
            operation::InvokeRequest {
                operation_ref: "fixture.session".into(),
                connection_ref: "fixture.connection".into(),
                description_ref: "fixture.description".into(),
                input: serde_json::json!({}),
                approval_evidence_ref: None,
            },
        )))
        .await
        .unwrap();
    let error = refused.error.unwrap();
    assert!(error.message.contains("connectors serve local"));
    assert!(!backend.dispatched.load(std::sync::atomic::Ordering::SeqCst));
    assert!(backend.shutdown.load(std::sync::atomic::Ordering::SeqCst));
    assert!(LocalStateOwnership::acquire(socket).is_ok());
}

#[tokio::test]
async fn adversary_socket_publication_after_absence_probe_is_preserved_and_refused() {
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let socket = root.path().join("connectors.sock");
    assert!(connectors_runtime::local_socket_absent(root.path()).unwrap());
    let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600)).unwrap();
    let result = PersonalRuntime::one_shot_operation(
        &root.path().join("missing-config"),
        root.path(),
        context(),
        OperationRequest::Search(operation::SearchRequest {
            query: String::new(),
            limit: 1,
        }),
    )
    .await;
    assert!(result.is_err());
    assert!(
        socket.exists(),
        "the new socket must never be removed by a one-shot contender"
    );
    assert!(!root.path().join("event-reply-claims.sqlite").exists());
    drop(listener);
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
        let runtime = LocalOneShot::new(
            LocalStateOwnership::acquire(&socket).unwrap(),
            backend.clone(),
        )
        .unwrap();
        let task = tokio::spawn(runtime.operation(envelope(OperationRequest::Search(
            operation::SearchRequest {
                query: String::new(),
                limit: 1,
            },
        ))));
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
            "one-shot returned before backend teardown"
        );
        backend.release.notify_one();
        let response = task.await.unwrap().unwrap();
        assert_eq!(response.error.is_some(), refuse);
        assert!(LocalStateOwnership::acquire(&socket).is_ok());
        assert!(!socket.exists());
    }
}

struct ClaimedLifetimeBackend {
    claims: bool,
    shutdown: std::sync::atomic::AtomicBool,
}

#[async_trait::async_trait]
impl ConnectorBackend for ClaimedLifetimeBackend {
    async fn ready(&self) -> Result<(), service::BackendReadinessError> {
        Ok(())
    }
    fn owns_operation(&self, _: &OperationRequest) -> bool {
        self.claims
    }
    fn supports_ephemeral_invocation(&self, _: &operation::InvokeRequest) -> bool {
        true
    }
    async fn handle(
        &self,
        _: &PrincipalContext,
        _: OperationRequest,
    ) -> Result<OperationResult, operation::OperationError> {
        panic!("lifetime capability cannot replace a unique invocation owner")
    }
    async fn shutdown(&self) {
        self.shutdown
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

#[tokio::test]
async fn adversary_lifetime_capability_never_admits_unknown_or_ambiguous_owners() {
    for claims in [false, true] {
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let backends = (0..2)
            .map(|_| {
                Arc::new(ClaimedLifetimeBackend {
                    claims,
                    shutdown: std::sync::atomic::AtomicBool::new(false),
                })
            })
            .collect::<Vec<_>>();
        let registry = Arc::new(BackendRegistry::new(
            backends
                .iter()
                .cloned()
                .map(|backend| backend as Arc<dyn ConnectorBackend>)
                .collect(),
        ));
        let response = LocalOneShot::new(
            LocalStateOwnership::acquire(root.path().join("connectors.sock")).unwrap(),
            registry,
        )
        .unwrap()
        .operation(envelope(OperationRequest::Invoke(
            operation::InvokeRequest {
                operation_ref: "fixture-operation".into(),
                connection_ref: "fixture-connection".into(),
                description_ref: "fixture-description".into(),
                input: serde_json::json!({}),
                approval_evidence_ref: None,
            },
        )))
        .await
        .unwrap();
        assert!(response
            .error
            .unwrap()
            .message
            .contains("connectors serve local"));
        assert!(backends
            .iter()
            .all(|backend| backend.shutdown.load(std::sync::atomic::Ordering::SeqCst)));
    }
}

#[tokio::test]
async fn adversary_every_persistent_request_class_refuses_before_configuration_or_state() {
    use protocol::connection;
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("state");
    let missing = root.path().join("missing-config");
    for request in [
        serde_json::json!({"method":"session_status","params":{"execution_ref":"fixture"}}),
        serde_json::json!({"method":"session_reconcile","params":{"execution_ref":"fixture"}}),
        serde_json::json!({"method":"session_terminate","params":{"execution_ref":"fixture","reason":"cancelled"}}),
        serde_json::json!({"method":"session_signal","params":{"execution_ref":"fixture","signal":{"kind":"dtmf","digits":"1"}}}),
    ] {
        let request: OperationRequest = serde_json::from_value(request).unwrap();
        let response = PersonalRuntime::one_shot_operation(&missing, &state, context(), request)
            .await
            .unwrap();
        assert!(response
            .error
            .unwrap()
            .message
            .contains("connectors serve local"));
        assert!(!state.exists());
    }
    for request in [
        connection::ConnectionRequest::CandidateActivate(connection::CandidateActivateRequest {
            candidate_ref: "fixture".into(),
            label: "fixture".into(),
        }),
        connection::ConnectionRequest::Materialize(connection::MaterializeRequest {
            observation_ref: "fixture".into(),
        }),
        connection::ConnectionRequest::ConnectSessionCreate(
            connection::ConnectSessionCreateRequest {
                integration_ref: "fixture".into(),
                label: "fixture".into(),
                auth_profile: None,
            },
        ),
        connection::ConnectionRequest::ConnectSessionStatus(
            connection::ConnectSessionStatusRequest {
                connect_session_ref: "fixture".into(),
            },
        ),
    ] {
        let response = PersonalRuntime::one_shot_connection(&missing, &state, context(), request)
            .await
            .unwrap();
        assert!(response
            .error
            .unwrap()
            .message
            .contains("connectors serve local"));
        assert!(!state.exists());
    }
}

#[tokio::test]
async fn final_adversary_invalid_envelopes_refuse_before_reading_config_or_creating_state() {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("never-created");
    let missing_config = root.path().join("missing-config");
    let search = OperationRequest::Search(operation::SearchRequest {
        query: String::new(),
        limit: 1,
    });
    let mut malformed_owner = context();
    malformed_owner.authority_snapshot_sha256 = "invalid".into();
    for (owner, request) in [
        (malformed_owner, search),
        (
            context(),
            OperationRequest::Search(operation::SearchRequest {
                query: "x".repeat(513),
                limit: 1,
            }),
        ),
        (
            context(),
            OperationRequest::Invoke(operation::InvokeRequest {
                operation_ref: "fixture".into(),
                connection_ref: "fixture".into(),
                description_ref: "fixture".into(),
                input: serde_json::json!({"value":"x".repeat(65_536)}),
                approval_evidence_ref: None,
            }),
        ),
    ] {
        let response = PersonalRuntime::one_shot_operation(&missing_config, &state, owner, request)
            .await
            .unwrap();
        assert!(response.error.is_some());
        assert!(response.response.is_none());
        assert!(!state.exists());
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
    let runtime = LocalOneShot::new(
        LocalStateOwnership::acquire(&socket).unwrap(),
        backend.clone(),
    )
    .unwrap();
    let response = runtime
        .operation(envelope(OperationRequest::Search(
            operation::SearchRequest {
                query: String::new(),
                limit: 1,
            },
        )))
        .await
        .unwrap();
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

#[tokio::test]
async fn auth_one_shot_origin_precomposition_is_typed_and_preserves_legacy_envelopes() {
    use connectors_runtime::OneShotOperationV3Outcome;
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("never-created");
    let missing = root.path().join("missing-config");
    for (value, valid) in [
        (
            serde_json::json!({"method":"session_status","params":{"execution_ref":"fixture"}}),
            true,
        ),
        (
            serde_json::json!({"method":"session_reconcile","params":{"execution_ref":"fixture"}}),
            true,
        ),
        (
            serde_json::json!({"method":"session_terminate","params":{"execution_ref":"fixture","reason":"cancelled"}}),
            true,
        ),
        (
            serde_json::json!({"method":"session_signal","params":{"execution_ref":"fixture","signal":{"kind":"dtmf","digits":"1"}}}),
            true,
        ),
        (
            serde_json::json!({"method":"session_status","params":{"execution_ref":""}}),
            false,
        ),
    ] {
        let request: OperationRequest = serde_json::from_value(value).unwrap();
        let outcome = PersonalRuntime::one_shot_operation_v3_outcome(
            &missing,
            &state,
            context(),
            request.clone(),
        )
        .await
        .unwrap();
        assert_eq!(
            matches!(&outcome, OneShotOperationV3Outcome::RequiresDaemon(_)),
            valid
        );
        let response = match outcome {
            OneShotOperationV3Outcome::Reply(response)
            | OneShotOperationV3Outcome::RequiresDaemon(response) => response,
        };
        response.validate().unwrap();
        let legacy = PersonalRuntime::one_shot_operation_v3(&missing, &state, context(), request)
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(response).unwrap(),
            serde_json::to_value(legacy).unwrap()
        );
        assert!(!state.exists());
    }
}

struct OriginBackend {
    ephemeral: bool,
    dispatched: std::sync::atomic::AtomicUsize,
    shutdown_started: tokio::sync::Notify,
    shutdown_release: tokio::sync::Notify,
}
#[async_trait::async_trait]
impl ConnectorBackend for OriginBackend {
    async fn ready(&self) -> Result<(), service::BackendReadinessError> {
        Ok(())
    }
    fn supports_ephemeral_invocation(&self, _: &operation::InvokeRequest) -> bool {
        self.ephemeral
    }
    async fn handle(
        &self,
        _: &PrincipalContext,
        _: OperationRequest,
    ) -> Result<OperationResult, operation::OperationError> {
        self.dispatched
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Err(operation::OperationError::new(
            operation::OperationErrorCode::Unavailable,
            "private backend says run connectors serve local with --config private-value",
            false,
        ))
    }
    async fn shutdown(&self) {
        self.shutdown_started.notify_one();
        self.shutdown_release.notified().await;
    }
}

#[tokio::test]
async fn auth_one_shot_origin_comes_only_from_local_decisions_and_joins_shutdown() {
    use connectors_runtime::OneShotOperationV3Outcome;
    // Persistent control, unknown invoke lifetime, real backend outage, invalid persistent input.
    for (persistent, ephemeral, valid, needs_daemon, calls) in [
        (true, true, true, true, 0),
        (false, false, true, true, 0),
        (false, true, true, false, 1),
        (true, true, false, false, 0),
    ] {
        let mut previous = None;
        for typed in [true, false] {
            let root = tempfile::tempdir().unwrap();
            std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
            let socket = root.path().join("connectors.sock");
            let backend = Arc::new(OriginBackend {
                ephemeral,
                dispatched: 0.into(),
                shutdown_started: tokio::sync::Notify::new(),
                shutdown_release: tokio::sync::Notify::new(),
            });
            let request = operation::v3::RequestEnvelope {
                protocol: operation::v3::CONTRACT.into(),
                request_id: "origin-fixture".into(),
                context: context(),
                request: if persistent {
                    OperationRequest::SessionStatus(operation::SessionRequest {
                        execution_ref: if valid { "fixture" } else { "" }.into(),
                    })
                } else {
                    OperationRequest::Invoke(operation::InvokeRequest {
                        operation_ref: "fixture".into(),
                        connection_ref: "fixture".into(),
                        description_ref: "fixture".into(),
                        input: serde_json::json!({}),
                        approval_evidence_ref: None,
                    })
                },
            };
            let runtime = LocalOneShot::new(
                LocalStateOwnership::acquire(&socket).unwrap(),
                backend.clone(),
            )
            .unwrap();
            let task = tokio::spawn(async move {
                if typed {
                    runtime.operation_v3_outcome(request).await
                } else {
                    runtime
                        .operation_v3(request)
                        .await
                        .map(OneShotOperationV3Outcome::Reply)
                }
            });
            tokio::time::timeout(
                std::time::Duration::from_secs(3),
                backend.shutdown_started.notified(),
            )
            .await
            .unwrap();
            assert!(LocalStateOwnership::acquire(&socket).is_err());
            assert!(
                !task.is_finished(),
                "outcome returned before joined shutdown"
            );
            assert_eq!(
                backend.dispatched.load(std::sync::atomic::Ordering::SeqCst),
                calls
            );
            backend.shutdown_release.notify_one();
            let outcome = task.await.unwrap().unwrap();
            assert_eq!(
                matches!(&outcome, OneShotOperationV3Outcome::RequiresDaemon(_)),
                typed && needs_daemon
            );
            let response = match outcome {
                OneShotOperationV3Outcome::Reply(response)
                | OneShotOperationV3Outcome::RequiresDaemon(response) => response,
            };
            response.validate().unwrap();
            assert_eq!(response.request_id, "origin-fixture");
            assert!(!response.error.as_ref().unwrap().retriable);
            if calls == 1 {
                assert!(response
                    .error
                    .as_ref()
                    .unwrap()
                    .message
                    .contains("private-value"));
            }
            let value = serde_json::to_value(response).unwrap();
            if let Some(previous) = previous.take() {
                assert_eq!(value, previous);
            } else {
                previous = Some(value);
            }
            assert!(LocalStateOwnership::acquire(&socket).is_ok());
            assert!(!socket.exists());
        }
    }
}

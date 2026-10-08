use async_trait::async_trait;
use connectors_core::{Descriptor, ErrorCode, Invocation, Operation, Result, WIRE_VERSION};
use connectors_host::{
    credentials::{BoundCredential, CredentialRef, DirectorySecrets, MemorySecrets},
    federation::{DownstreamConfig, Federation, FederationConfig},
    server::ServiceConfig,
    server::router,
};
use connectors_sdk::{Adapter, Credential, Secret};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    os::unix::fs::PermissionsExt,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

struct FixedSecret;
#[async_trait]
impl Credential for FixedSecret {
    async fn resolve(&self) -> Result<Secret> {
        Ok(Secret(b"service-token".to_vec()))
    }
}
struct Echo(Arc<AtomicUsize>);
#[async_trait]
impl Adapter for Echo {
    fn descriptor(&self) -> Descriptor {
        Descriptor {
            version: WIRE_VERSION.into(),
            instance: "leaf".into(),
            adapter: "fixture".into(),
            revision: "rev-1".into(),
            configuration_schema: json!({"type":"object"}),
            operations: vec![Operation {
                id: "echo".into(),
                description: "Read a typed fixture".into(),
                contract: "operations/v1alpha1".into(),
                profile: "read".into(),
                input_schema: json!({"type":"object","properties":{"value":{"type":"integer"}},"required":["value"],"additionalProperties":false}),
                output_schema: json!({"type":"object"}),
            }],
        }
    }
    async fn invoke(&self, _operation: &str, input: Value) -> Result<Value> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(input)
    }
}
async fn start(adapter: Arc<dyn Adapter>) -> (String, tokio::task::JoinHandle<()>) {
    start_with_credential(adapter, Arc::new(FixedSecret)).await
}
async fn start_with_credential(
    adapter: Arc<dyn Adapter>,
    credential: Arc<dyn Credential>,
) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        axum::serve(listener, router(adapter, credential))
            .await
            .unwrap();
    });
    (format!("http://{address}/"), task)
}

#[tokio::test]
async fn wire_admission_and_freshness_refuse_before_dispatch() {
    let calls = Arc::new(AtomicUsize::new(0));
    let (endpoint, task) = start(Arc::new(Echo(calls.clone()))).await;
    let denied = connectors_client::Client::new(&endpoint, "wrong".into(), true).unwrap();
    assert_eq!(
        denied.describe().await.unwrap_err().code,
        ErrorCode::Unauthorized
    );
    let client = connectors_client::Client::new(&endpoint, "service-token".into(), true).unwrap();
    let descriptor = client.describe().await.unwrap();
    let mut invocation = Invocation {
        version: WIRE_VERSION.into(),
        request_id: "r1".into(),
        operation: "echo".into(),
        revision: "stale".into(),
        input: json!({"value":1}),
    };
    assert_eq!(
        client.send(&invocation).await.unwrap_err().code,
        ErrorCode::StaleDescription
    );
    invocation.revision = descriptor.revision.clone();
    invocation.input = json!({"value":"wrong type"});
    assert_eq!(
        client.send(&invocation).await.unwrap_err().code,
        ErrorCode::InvalidInput
    );
    invocation.input = json!({"value":1});
    invocation.version = "v99".into();
    assert_eq!(
        client.send(&invocation).await.unwrap_err().code,
        ErrorCode::Unsupported
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        client
            .invoke(&descriptor, "echo", json!({"value":42}))
            .await
            .unwrap(),
        json!({"value":42})
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    task.abort();
}

#[tokio::test]
async fn federation_preserves_result_and_reports_downstream_failure() {
    let calls = Arc::new(AtomicUsize::new(0));
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("token");
    std::fs::write(&path, b"service-token").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let credential = CredentialRef::File { path: path.clone() };
    let (endpoint, leaf) =
        start_with_credential(Arc::new(Echo(calls.clone())), Arc::new(credential.clone())).await;
    let config = FederationConfig {
        service: ServiceConfig {
            instance: "gateway".into(),
            listen: "127.0.0.1:0".parse().unwrap(),
            service_credential: credential.clone(),
            state: None,
        },
        downstreams: vec![DownstreamConfig {
            name: "source".into(),
            endpoint,
            credential,
            allow_plaintext: true,
        }],
    };
    let federation = Federation::connect(&config).await.unwrap();
    let schema = federation.descriptor().configuration_schema;
    let valid = serde_json::to_value(&config).unwrap();
    connectors_sdk::validate(&schema, &valid).unwrap();
    for invalid in [
        json!({}),
        json!({"service":valid["service"],"downstreams":[]}),
        json!({"service":valid["service"],"downstreams":vec![valid["downstreams"][0].clone();33]}),
        json!({"service":valid["service"],"downstreams":[{"name":"source","endpoint":5,"credential":valid["downstreams"][0]["credential"]}]}),
        json!({"service":valid["service"],"downstreams":valid["downstreams"],"unknown":true}),
    ] {
        assert!(connectors_sdk::validate(&schema, &invalid).is_err());
    }
    let mut duplicate = config.clone();
    duplicate.downstreams.push(config.downstreams[0].clone());
    assert_eq!(
        Federation::connect(&duplicate).await.err().unwrap().code,
        ErrorCode::InvalidInput
    );
    let mut reserved_name = config.clone();
    reserved_name.downstreams[0].name = "source__nested".into();
    assert_eq!(
        Federation::connect(&reserved_name)
            .await
            .err()
            .unwrap()
            .code,
        ErrorCode::InvalidInput
    );
    let mut cycle = config.clone();
    cycle.service.instance = "leaf".into();
    assert_eq!(
        Federation::connect(&cycle).await.err().unwrap().code,
        ErrorCode::Unsupported
    );
    let (endpoint, gateway) = start(Arc::new(federation)).await;
    let mut nested = config.clone();
    nested.service.instance = "outer".into();
    nested.downstreams[0].endpoint = endpoint.clone();
    assert_eq!(
        Federation::connect(&nested).await.err().unwrap().code,
        ErrorCode::Unsupported
    );
    let client = connectors_client::Client::new(&endpoint, "service-token".into(), true).unwrap();
    let descriptor = client.describe().await.unwrap();
    assert_eq!(
        client
            .invoke(&descriptor, "source__echo", json!({"value":7}))
            .await
            .unwrap(),
        json!({"value":7})
    );
    // Rotate the downstream's binding without rebuilding the gateway description.
    let replacement = temp.path().join("replacement");
    std::fs::write(&replacement, b"rotated-service-token").unwrap();
    std::fs::set_permissions(&replacement, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::rename(replacement, path).unwrap();
    assert_eq!(
        client
            .invoke(&descriptor, "source__echo", json!({"value":9}))
            .await
            .unwrap(),
        json!({"value":9})
    );
    leaf.abort();
    let _ = leaf.await;
    assert!(
        client
            .invoke(&descriptor, "source__echo", json!({"value":8}))
            .await
            .is_err()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    gateway.abort();
}

#[tokio::test]
async fn credential_stores_are_substitutable_and_file_permissions_are_enforced() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("credential");
    std::fs::write(&path, b"same-value").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let memory = BoundCredential {
        store: Arc::new(MemorySecrets(BTreeMap::from([(
            "credential".into(),
            b"same-value".to_vec(),
        )]))),
        reference: "credential".into(),
    };
    let file = BoundCredential {
        store: Arc::new(DirectorySecrets(temp.path().into())),
        reference: "credential".into(),
    };
    assert_eq!(
        memory.resolve().await.unwrap().0,
        file.resolve().await.unwrap().0
    );
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(file.resolve().await.is_err());
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let link = temp.path().join("symlink");
    std::os::unix::fs::symlink(path, &link).unwrap();
    assert!(CredentialRef::File { path: link }.resolve().await.is_err());
}

struct ChangingLeaf {
    calls: Arc<AtomicUsize>,
    revision: Arc<AtomicUsize>,
}
#[async_trait]
impl Adapter for ChangingLeaf {
    fn descriptor(&self) -> Descriptor {
        let mut d = Echo(self.calls.clone()).descriptor();
        let rev = self.revision.load(Ordering::SeqCst);
        d.revision = format!("rev-{rev}");
        d.operations[0].id = format!("echo{rev}");
        d
    }
    async fn invoke(&self, _: &str, input: Value) -> Result<Value> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(input)
    }
}
#[tokio::test]
async fn federation_refreshes_atomically_without_replaying_and_rotates_credentials() {
    let calls = Arc::new(AtomicUsize::new(0));
    let revision = Arc::new(AtomicUsize::new(1));
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("token");
    std::fs::write(&path, "service-token").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let credential = CredentialRef::File { path: path.clone() };
    let (endpoint, leaf) = start_with_credential(
        Arc::new(ChangingLeaf {
            calls: calls.clone(),
            revision: revision.clone(),
        }),
        Arc::new(credential.clone()),
    )
    .await;
    let federation = Arc::new(
        Federation::connect(&FederationConfig {
            service: ServiceConfig {
                instance: "gateway".into(),
                listen: "127.0.0.1:0".parse().unwrap(),
                service_credential: credential.clone(),
                state: None,
            },
            downstreams: vec![DownstreamConfig {
                name: "source".into(),
                endpoint,
                credential,
                allow_plaintext: true,
            }],
        })
        .await
        .unwrap(),
    );
    let (endpoint, gateway) = start(federation.clone()).await;
    let client = connectors_client::Client::new(&endpoint, "service-token".into(), true).unwrap();
    let old = client.describe().await.unwrap();
    revision.store(2, Ordering::SeqCst);
    std::fs::write(&path, "rotated-token").unwrap();
    assert_eq!(
        client
            .invoke(&old, "source__echo1", json!({"value":42}))
            .await
            .unwrap_err()
            .code,
        ErrorCode::StaleDescription
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let fresh = client.describe().await.unwrap();
    assert_ne!(old.revision, fresh.revision);
    assert!(fresh.operation("source__echo1").is_err());
    assert!(fresh.operation("source__echo2").is_ok());
    assert_eq!(
        federation
            .invoke_at(&old.revision, "source__echo2", json!({"value":1}))
            .await
            .unwrap_err()
            .code,
        ErrorCode::StaleDescription
    );
    assert_eq!(
        client
            .invoke(&fresh, "source__echo2", json!({"value":42}))
            .await
            .unwrap(),
        json!({"value":42})
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    leaf.abort();
    gateway.abort();
}

#[tokio::test]
async fn invalid_identifiers_cannot_inject_log_lines_or_terminal_controls() {
    use std::{io::Write, sync::Mutex};
    #[derive(Clone)]
    struct Capture(Arc<Mutex<Vec<u8>>>);
    impl Write for Capture {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let writer = Capture(buffer.clone());
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_writer(move || writer.clone())
        .finish();
    let _subscriber = tracing::subscriber::set_default(subscriber);
    // While one dispatcher is registered, tracing-core computes a newly
    // registered callsite's interest from the registering thread's default
    // alone, so a thread without this subscriber would cache the callsite as
    // never enabled. A second live dispatcher makes every registration
    // consult all of them, this thread-local one included.
    let _consult_every_dispatcher =
        tracing::Dispatch::new(tracing::subscriber::NoSubscriber::default());
    // Another test's server, on a thread without this subscriber, may be the
    // first to reach the completion log line; make that happen here.
    std::thread::spawn(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let (url, task) = start(Arc::new(Echo(Arc::new(AtomicUsize::new(0))))).await;
                let client =
                    connectors_client::Client::new(&url, "service-token".into(), true).unwrap();
                let description = client.describe().await.unwrap();
                client
                    .invoke(&description, "echo", json!({"value":1}))
                    .await
                    .unwrap();
                task.abort();
            });
    })
    .join()
    .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let app = router(Arc::new(Echo(calls.clone())), Arc::new(FixedSecret));
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let client =
        connectors_client::Client::new(&format!("http://{address}/"), "service-token".into(), true)
            .unwrap();
    let invocation = Invocation {
        version: WIRE_VERSION.into(),
        request_id: "injected\n\u{1b}[31m".into(),
        operation: "echo\r\nFORGED".into(),
        revision: "rev-1".into(),
        input: json!({"value":1}),
    };
    assert_eq!(
        client.send(&invocation).await.unwrap_err().code,
        ErrorCode::InvalidInput
    );
    let log = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
    assert!(log.contains("operation completed"));
    assert_eq!(log.lines().count(), 1);
    assert!(!log.contains('\u{1b}'));
    assert!(!log.contains('\r'));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    task.abort();
}

#[test]
fn configuration_yaml_and_json_remain_strict() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("config");
    for text in [
        "instance: test\nlisten: '127.0.0.1:0'\nservice_credential:\n  kind: file\n  path: token\n",
        r#"{"instance":"test","listen":"127.0.0.1:0","service_credential":{"kind":"file","path":"token"}}"#,
    ] {
        std::fs::write(&path, text).unwrap();
        assert_eq!(
            connectors_host::read_config::<ServiceConfig>(&path)
                .unwrap()
                .instance,
            "test"
        );
    }
    for text in [
        "instance: first\ninstance: second\n",
        r#"{"instance":"first","instance":"second"}"#,
        r#"{"instance":"test","listen":"127.0.0.1:0","service_credential":{"kind":"file","path":"token"},"unknown":1}"#,
    ] {
        std::fs::write(&path, text).unwrap();
        assert!(connectors_host::read_config::<ServiceConfig>(&path).is_err());
    }
    std::fs::write(&path, vec![b' '; 1024 * 1024 + 1]).unwrap();
    assert_eq!(
        connectors_host::read_config::<ServiceConfig>(&path)
            .unwrap_err()
            .message,
        "configuration must be a bounded file"
    );
}

#[test]
fn environment_credentials_apply_the_same_value_bounds_as_files() {
    const CHILD: &str = "CONNECTORS_CREDENTIAL_BOUND_TEST";
    const VALUE: &str = "CONNECTORS_CREDENTIAL_FIXTURE_VALUE";
    if let Ok(case) = std::env::var(CHILD) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let result = runtime.block_on(CredentialRef::Environment { name: VALUE.into() }.resolve());
        if case == "valid" {
            assert_eq!(result.unwrap().0, b"fixture-value");
        } else {
            assert_eq!(result.err().unwrap().code, ErrorCode::Unauthorized);
        }
        return;
    }
    for (case, value) in [
        ("valid", Some("fixture-value\r\n".into())),
        ("empty", Some(String::new())),
        ("oversized", Some("x".repeat(8193))),
        ("missing", None),
    ] {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "environment_credentials_apply_the_same_value_bounds_as_files",
            ])
            .env(CHILD, case)
            .env_remove(VALUE);
        if let Some(value) = value {
            command.env(VALUE, value);
        }
        let output = command.output().unwrap();
        assert!(output.status.success(), "environment fixture {case} failed");
    }
}

/// `POST /v1alpha2/invoke` and the HTTP host's mutation attempt ledger
/// (contracts/service/compatibility.md § 2.1 `mutation`,
/// ess/domains/mutations.yaml "The HTTP host's attempt ledger").
mod v1alpha2_ledger {
    use super::*;
    use connectors_core::v1alpha2::{
        AuditStatus, BaseErrorCode, CauseStage, EffectKnowledge, ErrorCode as WireCode,
        InvokeResponse, ResponseStatus,
    };
    use connectors_host::{
        local::{
            audit::{self, Reference},
            mutations,
        },
        server::{ServiceState, router_with_state},
    };
    use std::{
        path::{Path, PathBuf},
        sync::atomic::AtomicBool,
        time::Duration,
    };

    /// `write` applies and echoes; `lost-write` is dispatched and its answer is
    /// lost (a generic adapter error); `read` echoes; `failing-read` errors.
    /// `break_store` takes the state directory away during the next dispatch.
    struct Ledger {
        calls: Arc<AtomicUsize>,
        state: PathBuf,
        break_store: Arc<AtomicBool>,
    }
    fn operation(id: &str, profile: &str) -> Operation {
        Operation {
            id: id.into(),
            description: format!("{id} fixture"),
            contract: "operations/v1alpha1".into(),
            profile: profile.into(),
            input_schema: json!({"type":"object"}),
            output_schema: json!({"type":"object"}),
        }
    }
    #[async_trait]
    impl Adapter for Ledger {
        fn descriptor(&self) -> Descriptor {
            Descriptor {
                version: WIRE_VERSION.into(),
                instance: "leaf".into(),
                adapter: "fixture".into(),
                revision: "rev-1".into(),
                configuration_schema: json!({"type":"object"}),
                operations: vec![
                    operation("write", "mutation"),
                    operation("lost-write", "mutation"),
                    operation("read", "read"),
                    operation("failing-read", "read"),
                ],
            }
        }
        async fn invoke(&self, operation: &str, input: Value) -> Result<Value> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.break_store.swap(false, Ordering::SeqCst) {
                private(&self.state, false);
            }
            match operation {
                "lost-write" => Err(connectors_core::Error::new(
                    ErrorCode::Unavailable,
                    "connection reset before the answer",
                )),
                "failing-read" => Err(connectors_core::Error::new(
                    ErrorCode::NotFound,
                    "no such item",
                )),
                _ => Ok(input),
            }
        }
    }
    /// A group-readable state directory makes every metadata write fail.
    fn private(state: &Path, usable: bool) {
        let mode = if usable { 0o700 } else { 0o750 };
        std::fs::set_permissions(state, std::fs::Permissions::from_mode(mode)).unwrap();
    }

    struct Host {
        endpoint: String,
        task: tokio::task::JoinHandle<()>,
        state: Arc<ServiceState>,
        path: PathBuf,
        calls: Arc<AtomicUsize>,
        break_store: Arc<AtomicBool>,
        _root: tempfile::TempDir,
    }
    async fn host() -> Host {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("state");
        std::fs::create_dir(&path).unwrap();
        private(&path, true);
        let calls = Arc::new(AtomicUsize::new(0));
        let break_store = Arc::new(AtomicBool::new(false));
        let adapter = Arc::new(Ledger {
            calls: calls.clone(),
            state: path.clone(),
            break_store: break_store.clone(),
        });
        let state = Arc::new(ServiceState::open(&path, "leaf", "fixture", "rev-1").unwrap());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = router_with_state(adapter, Arc::new(FixedSecret), Some(state.clone()));
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Host {
            endpoint: format!("http://{address}/v1alpha2/invoke"),
            task,
            state,
            path,
            calls,
            break_store,
            _root: root,
        }
    }
    async fn invoke(host: &Host, body: Value) -> (u16, InvokeResponse) {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let response = reqwest::Client::new()
            .post(&host.endpoint)
            .bearer_auth("service-token")
            .body(serde_json::to_vec(&body).unwrap())
            .send()
            .await
            .unwrap();
        let status = response.status().as_u16();
        let bytes = response.bytes().await.unwrap();
        (status, InvokeResponse::decode(&bytes).unwrap())
    }
    fn request(operation: &str, request_id: &str) -> Value {
        json!({"version":"v1alpha2","request_id":request_id,"operation":operation,"revision":"rev-1","input":{"value":1}})
    }
    fn audit_record(path: &Path, audit_ref: &str) -> audit::Record {
        audit::Store::new(path, 100_000)
            .unwrap()
            .observe(&Reference {
                instance: "leaf".into(),
                audit_ref: audit_ref.into(),
            })
            .unwrap()
            .expect("the anchored record")
    }

    #[tokio::test]
    async fn a_write_returns_its_attempt_and_the_record_reads_back() {
        let host = host().await;
        let (status, response) = invoke(&host, request("write", "write-1")).await;
        assert_eq!(status, 200);
        assert_eq!(response.status, ResponseStatus::Success);
        assert_eq!(response.result, Some(json!({"value":1})));
        assert_eq!(response.audit_status, AuditStatus::Complete);
        let mutation = response.mutation.expect("a write carries its mutation");
        assert_eq!(mutation.classification, EffectKnowledge::Applied);
        assert_eq!(mutation.original_request_id.as_deref(), Some("write-1"));
        assert!(!mutation.replayed);
        assert_eq!(mutation.cause, None);
        let attempt = mutation.attempt.expect("the recorded attempt");
        assert_eq!(attempt.instance, "leaf");

        let record = host.state.attempt(attempt.id.as_str()).unwrap();
        assert_eq!(record.reference.attempt_id.to_string(), attempt.id.as_str());
        assert_eq!(record.request_id, "write-1");
        assert_eq!(record.state, mutations::State::Completed);
        assert_eq!(record.result, Some(json!({"classification":"applied"})));
        assert!(record.settled_at_ms.is_some());
        // The audit anchor names the same attempt (LinkAnchorAttempt).
        let anchored = audit_record(&host.path, &response.audit_ref.unwrap());
        assert_eq!(
            anchored
                .anchor
                .attempt_id
                .map(|id| id.to_string())
                .as_deref(),
            Some(attempt.id.as_str())
        );
        assert_eq!(host.state.attempt_count().unwrap(), 1);
        assert_eq!(host.calls.load(Ordering::SeqCst), 1);
        host.task.abort();
    }

    #[tokio::test]
    async fn a_refusal_before_dispatch_returns_no_attempt() {
        let host = host().await;
        let mut stale = request("write", "stale");
        stale["revision"] = json!("rev-0");
        let mut invalid = request("write", "invalid");
        invalid["input"] = json!("not an object");
        for (body, code) in [
            (stale, WireCode::StaleDescription),
            (request("missing-write", "unknown"), WireCode::NotFound),
            (invalid, WireCode::InvalidInput),
        ] {
            let (_, response) = invoke(&host, body).await;
            assert_eq!(response.status, ResponseStatus::Error);
            assert_eq!(response.error.unwrap().code, code);
            assert!(response.mutation.is_none(), "{code:?}");
            assert_eq!(response.audit_ref, None);
        }
        assert_eq!(host.state.attempt_count().unwrap(), 0);
        assert_eq!(host.calls.load(Ordering::SeqCst), 0);
        host.task.abort();
    }

    #[tokio::test]
    async fn a_lost_answer_is_outcome_unknown_with_the_recorded_attempt_and_no_second_dispatch() {
        let host = host().await;
        let (status, response) = invoke(&host, request("lost-write", "lost-1")).await;
        assert_eq!(status, 502);
        assert_eq!(response.status, ResponseStatus::Error);
        assert_eq!(response.error.unwrap().code, WireCode::OutcomeUnknown);
        let mutation = response.mutation.expect("the recorded attempt");
        assert_eq!(mutation.classification, EffectKnowledge::Unknown);
        assert_eq!(mutation.original_request_id.as_deref(), Some("lost-1"));
        let cause = mutation.cause.expect("the secondary cause");
        assert_eq!(cause.code, BaseErrorCode::Unavailable);
        assert_eq!(cause.stage, CauseStage::Dispatch);
        let attempt = mutation.attempt.expect("the attempt id");
        let record = host.state.attempt(attempt.id.as_str()).unwrap();
        assert_eq!(record.state, mutations::State::Indeterminate);
        assert_eq!(record.settled_at_ms, None);
        let observed = audit_record(&host.path, &response.audit_ref.unwrap())
            .final_observation
            .expect("the final observation");
        assert_eq!(observed.outcome, audit::Outcome::Unknown);
        // Settled once; the host did not dispatch again.
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(host.calls.load(Ordering::SeqCst), 1);
        assert_eq!(host.state.attempt_count().unwrap(), 1);
        host.task.abort();
    }

    #[tokio::test]
    async fn a_read_has_no_mutation_and_records_no_attempt() {
        let host = host().await;
        let (status, response) = invoke(&host, request("read", "read-1")).await;
        assert_eq!(status, 200);
        assert_eq!(response.status, ResponseStatus::Success);
        assert!(response.mutation.is_none());
        let (status, response) = invoke(&host, request("failing-read", "read-2")).await;
        assert_eq!(status, 404);
        assert_eq!(response.error.unwrap().code, WireCode::NotFound);
        assert!(response.mutation.is_none());
        assert_eq!(host.state.attempt_count().unwrap(), 0);
        host.task.abort();
    }

    /// audit.md § 3: the running host retries a lost final observation by
    /// itself; nothing here calls the recovery entry point.
    #[tokio::test]
    async fn the_running_host_recovers_a_lost_final_observation_by_itself() {
        let host = host().await;
        host.break_store.store(true, Ordering::SeqCst);
        let (status, response) = invoke(&host, request("read", "unobserved")).await;
        assert_eq!(status, 200);
        assert_eq!(response.audit_status, AuditStatus::Incomplete);
        let audit_ref = response.audit_ref.unwrap();
        private(&host.path, true);
        let mut observed = None;
        for _ in 0..50 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            observed = audit_record(&host.path, &audit_ref).final_observation;
            if observed.is_some() {
                break;
            }
        }
        assert_eq!(
            observed.expect("recovered without a caller").outcome,
            audit::Outcome::Success
        );
        assert_eq!(host.state.pending_observations(), 0);
        assert_eq!(host.calls.load(Ordering::SeqCst), 1);
        host.task.abort();
    }
}

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
        },
        downstreams: vec![DownstreamConfig {
            name: "source".into(),
            endpoint,
            credential,
            allow_plaintext: true,
        }],
    };
    let federation = Federation::connect(&config).await.unwrap();
    let (endpoint, gateway) = start(Arc::new(federation)).await;
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
}

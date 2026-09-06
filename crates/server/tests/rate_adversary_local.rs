use async_trait::async_trait;
use protocol::operation::*;
use serde_json::{json, Value};
use service::{ConnectorBackend, PrincipalContext};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};
struct Refuses(AtomicUsize);
#[async_trait]
impl ConnectorBackend for Refuses {
    async fn ready(&self) -> Result<(), service::BackendReadinessError> {
        Ok(())
    }
    async fn handle(
        &self,
        _: &PrincipalContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        let OperationRequest::Invoke(value) = request else {
            panic!()
        };
        let mut error = OperationError::rate_limited(
            "definite refusal",
            value.input.get("delay").and_then(Value::as_u64),
        );
        if value.input.get("bad") == Some(&Value::Bool(true)) {
            error.code = OperationErrorCode::Unavailable;
            error.retry_after_seconds = Some(30);
        }
        Err(error)
    }
}
#[tokio::test]
async fn rate_adversary_local_versions_validate_before_single_dispatch() {
    let root = std::env::temp_dir().join(format!("rate-local-{}", std::process::id()));
    let socket = root.join("connector.sock");
    let backend = Arc::new(Refuses(AtomicUsize::new(0)));
    let daemon = server::local::LocalOperationDaemon::bind(&socket, backend.clone())
        .await
        .unwrap();
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let serving = tokio::spawn(async move {
        daemon
            .serve_until(async {
                let _ = stopped.await;
            })
            .await
            .unwrap()
    });
    let mut expected = 0;
    for version in [CONTRACT, legacy::CONTRACT, "b10x.connector-operation.v9"] {
        for (delay, bad_context, bad_response) in [
            (None, false, false),
            (Some(0), false, false),
            (Some(u64::MAX), false, false),
            (Some(30), true, false),
            (Some(30), false, true),
        ] {
            let frame = json!({"protocol":version,"request_id":"local-adversary","context":{"tenant_id":"local","agent_id":"agent","agent_revision":if bad_context {0}else{1},"authority_snapshot_id":"snapshot","authority_snapshot_sha256":"a".repeat(64)},"request":{"method":"invoke","params":{"operation_ref":"fixture.read","connection_ref":"connection:fixture","description_ref":"description:fixture","input":{"delay":delay,"bad":bad_response}}}});
            let mut stream = tokio::net::UnixStream::connect(&socket).await.unwrap();
            stream
                .write_all(format!("{frame}\n").as_bytes())
                .await
                .unwrap();
            let mut line = String::new();
            tokio::time::timeout(
                std::time::Duration::from_secs(3),
                BufReader::new(stream).read_line(&mut line),
            )
            .await
            .unwrap()
            .unwrap();
            if bad_context || version.ends_with(".v9") {
                assert!(line.is_empty());
                continue;
            }
            expected += 1;
            let value: Value = serde_json::from_str(&line).unwrap();
            assert_eq!(value["protocol"], version);
            assert_eq!(value["request_id"], "local-adversary");
            let code = if bad_response {
                "protocol"
            } else if version == CONTRACT {
                "rate_limited"
            } else {
                "unavailable"
            };
            assert_eq!(value["error"]["code"], code);
            let wanted = if bad_response || version == legacy::CONTRACT {
                None
            } else {
                delay
            };
            assert_eq!(
                value["error"]
                    .get("retry_after_seconds")
                    .and_then(Value::as_u64),
                wanted
            );
        }
    }
    assert_eq!(backend.0.load(Ordering::SeqCst), expected);
    stop.send(()).unwrap();
    serving.await.unwrap();
    assert!(!socket.exists());
    std::fs::remove_file(root.join(".connectors.lock")).unwrap();
    std::fs::remove_dir(root).unwrap();
}

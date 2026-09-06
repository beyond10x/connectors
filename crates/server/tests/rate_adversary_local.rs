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

struct FinalDescribeBackend(std::sync::atomic::AtomicUsize);
#[async_trait]
impl ConnectorBackend for FinalDescribeBackend {
    async fn ready(&self) -> Result<(), service::BackendReadinessError> {
        Ok(())
    }
    async fn handle(
        &self,
        _: &PrincipalContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let OperationRequest::Describe(request) = request else {
            panic!("Describe only")
        };
        let source = if request.operation_ref.ends_with("valid")
            && !request.operation_ref.ends_with("invalid")
        {
            "https://%41.example.test:00065535/a%2fb?x=%23%40"
        } else {
            "https://docs.example.test:65536/private-SENTINEL"
        };
        Ok(OperationResult::Describe(
            protocol::operation::OperationDescription {
                operation_ref: request.operation_ref,
                title: "Fixture read".into(),
                description: "Fixture metadata".into(),
                input_schema: serde_json::json!({"type":"object","additionalProperties":false}),
                output_schema: serde_json::json!({"type":"object","properties":{"vendor":{"const":"retained"}}}),
                effect: EffectClass::ReadOnly,
                approval: ApprovalPosture::NotRequired,
                connections: vec![],
                description_ref: "fixture-description".into(),
                rate_advice: Some(protocol::operation::OperationRateAdvice {
                    fixed: None,
                    alternatives: vec![protocol::operation::ConditionalRateAdvice::new(
                        protocol::operation::ConditionalRateLimit {
                            applies_when: "Fixture category".into(),
                            rate: None,
                            source_url: source.into(),
                        },
                    )],
                }),
            },
        ))
    }
}

#[tokio::test]
async fn rate_final_local_describe_validates_before_version_loss() {
    let root = std::env::temp_dir().join(format!("rate-final-local-{}", std::process::id()));
    let socket = root.join("connector.sock");
    let backend = Arc::new(FinalDescribeBackend(AtomicUsize::new(0)));
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
    for version in [CONTRACT, legacy::CONTRACT] {
        for valid in [true, false] {
            let operation = if valid {
                "fixture.valid"
            } else {
                "fixture.invalid"
            };
            let frame = json!({"protocol":version,"request_id":"final-describe","context":{"tenant_id":"local","agent_id":"agent","agent_revision":1,"authority_snapshot_id":"snapshot","authority_snapshot_sha256":"a".repeat(64)},"request":{"method":"describe","params":{"operation_ref":operation}}});
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
            let reply: Value = serde_json::from_str(&line).unwrap();
            assert_eq!(reply["protocol"], version);
            assert_eq!(reply["request_id"], "final-describe");
            if valid {
                assert_eq!(reply["status"], "ok", "{reply}");
                let value = &reply["response"]["value"];
                assert_eq!(value["operation_ref"], operation);
                assert_eq!(
                    value["output_schema"]["properties"]["vendor"]["const"],
                    "retained"
                );
                if version == CONTRACT {
                    assert_eq!(
                        value["rate_advice"]["alternatives"][0]["declaration"]["source_url"],
                        "https://%41.example.test:00065535/a%2fb?x=%23%40"
                    );
                } else {
                    assert!(value.get("rate_advice").is_none());
                }
            } else {
                assert_eq!(reply["status"], "error");
                assert_eq!(reply["error"]["code"], "protocol");
                assert!(!line.contains("SENTINEL"));
            }
        }
    }
    assert_eq!(backend.0.load(Ordering::SeqCst), 4);
    stop.send(()).unwrap();
    serving.await.unwrap();
    std::fs::remove_file(root.join(".connectors.lock")).unwrap();
    std::fs::remove_dir(root).unwrap();
}

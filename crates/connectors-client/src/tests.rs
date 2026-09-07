use axum::body::Bytes;
use axum::extract::{Path as AxumPath, State};
use axum::http::HeaderMap;
use axum::response::IntoResponse as _;
use axum::routing::{get, post};
use axum::{Json, Router};
use protocol::operation::{
    OperationRequest, OperationResult, OwnerContext, ResponseEnvelope, ResponseStatus,
    SearchRequest,
};
use tempfile::tempdir;
use tokio::io::BufReader;
use tokio::net::{TcpListener, UnixListener};

use super::*;

fn context() -> OwnerContext {
    OwnerContext {
        tenant_id: "tenant-1".to_owned(),
        agent_id: "agent-1".to_owned(),
        agent_revision: 1,
        authority_snapshot_id: "snapshot-1".to_owned(),
        authority_snapshot_sha256: "a".repeat(64),
    }
}

#[tokio::test]
async fn local_client_frames_and_correlates_an_operation() {
    let root = tempdir().unwrap();
    let socket = root.path().join("connectors.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let serving = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut line = String::new();
        BufReader::new(&mut stream)
            .read_line(&mut line)
            .await
            .unwrap();
        let request: operation::RequestEnvelope = serde_json::from_str(&line).unwrap();
        request.validate().unwrap();
        assert_eq!(request.protocol, operation::CONTRACT);
        let response = ResponseEnvelope::success(
            request.request_id,
            OperationResult::Search {
                operations: Vec::new(),
            },
        );
        let mut bytes = serde_json::to_vec(&response).unwrap();
        bytes.push(b'\n');
        stream.write_all(&bytes).await.unwrap();
    });
    let response = LocalClient::new(&socket)
        .operation_v2(
            &context(),
            OperationRequest::Search(SearchRequest {
                query: "status".to_owned(),
                limit: 1,
            }),
        )
        .await
        .unwrap();
    assert_eq!(response.status, ResponseStatus::Ok);
    serving.await.unwrap();
}

#[tokio::test]
async fn completion_endpoint_is_validated_before_secret_submission() {
    let root = tempdir().unwrap();
    let sessions = root.path().join("connect-sessions");
    fs::create_dir(&sessions).unwrap();
    fs::set_permissions(&sessions, fs::Permissions::from_mode(0o700)).unwrap();
    let socket = sessions.join("complete.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).unwrap();
    let endpoint = CompletionEndpoint::validate(root.path(), &socket).unwrap();
    let serving = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut credential = String::new();
        BufReader::new(&mut stream)
            .read_line(&mut credential)
            .await
            .unwrap();
        assert_eq!(credential, "secret-value\n");
        stream.write_all(b"{\"accepted\":true}\n").await.unwrap();
    });
    endpoint.submit(b"secret-value").await.unwrap();
    serving.await.unwrap();

    let outside = root.path().join("outside.sock");
    let _outside_listener = UnixListener::bind(&outside).unwrap();
    assert!(matches!(
        CompletionEndpoint::validate(root.path(), &outside),
        Err(ClientError::UnsafeCompletionEndpoint)
    ));
}

#[test]
fn hosted_client_requires_https_except_on_loopback_or_internal_cluster_dns() {
    assert!(HostedClient::new("https://connectors.example/api/connectors/v1").is_ok());
    assert!(HostedClient::new("http://127.0.0.1:8091/api/connectors/v1").is_ok());
    assert!(HostedClient::new(
        "http://connectors.devcenter.svc.cluster.local:8091/api/connectors/v1"
    )
    .is_ok());
    assert!(matches!(
        HostedClient::new("http://connectors.example/api/connectors/v1"),
        Err(ClientError::InvalidHostedBase)
    ));
    assert!(matches!(
        HostedClient::new("https://user@connectors.example/api/connectors/v1"),
        Err(ClientError::InvalidHostedBase)
    ));
}

#[tokio::test]
async fn hosted_client_posts_the_same_typed_operation_frame() {
    async fn operation_handler(
        State(expected): State<OwnerContext>,
        headers: HeaderMap,
        body: Bytes,
    ) -> Bytes {
        assert_eq!(
            headers.get(reqwest::header::AUTHORIZATION).unwrap(),
            "Bearer session-1"
        );
        let request: operation::RequestEnvelope = serde_json::from_slice(&body).unwrap();
        request.validate().unwrap();
        assert_eq!(request.context, expected);
        Bytes::from(
            serde_json::to_vec(&ResponseEnvelope::success(
                request.request_id,
                OperationResult::Search {
                    operations: Vec::new(),
                },
            ))
            .unwrap(),
        )
    }

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = Router::new()
        .route("/api/connectors/v1/operations", post(operation_handler))
        .with_state(context());
    let serving = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let base = Url::parse(&format!("http://{address}/api/connectors/v1")).unwrap();
    let client = HostedClient::from_parts(base, reqwest::Client::new());
    let response = client
        .operation_v2(
            "session-1",
            &context(),
            OperationRequest::Search(SearchRequest {
                query: String::new(),
                limit: 1,
            }),
        )
        .await
        .unwrap();
    assert_eq!(response.status, ResponseStatus::Ok);
    serving.abort();
}

#[tokio::test]
async fn hosted_client_posts_and_validates_a_datasource_frame() {
    async fn datasource_handler(
        State(expected): State<OwnerContext>,
        headers: HeaderMap,
        body: Bytes,
    ) -> Bytes {
        assert_eq!(
            headers.get(reqwest::header::AUTHORIZATION).unwrap(),
            "Bearer session-1"
        );
        let request: datasource::RequestEnvelope = serde_json::from_slice(&body).unwrap();
        request.validate().unwrap();
        assert_eq!(request.context, expected);
        Bytes::from(
            serde_json::to_vec(&datasource::ResponseEnvelope::success(
                request.request_id,
                datasource::DatasourceResult::Search {
                    definitions: Vec::new(),
                },
            ))
            .unwrap(),
        )
    }

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = Router::new()
        .route("/api/connectors/v1/datasources", post(datasource_handler))
        .with_state(context());
    let serving = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let base = Url::parse(&format!("http://{address}/api/connectors/v1")).unwrap();
    let client = HostedClient::from_parts(base, reqwest::Client::new());
    let response = client
        .datasource(
            "session-1",
            &context(),
            datasource::DatasourceRequest::Search(datasource::SearchRequest {
                query: "gitlab".to_owned(),
                limit: 1,
            }),
        )
        .await
        .unwrap();
    assert_eq!(response.status, datasource::ResponseStatus::Ok);
    serving.abort();
}

#[tokio::test]
async fn hosted_subscription_client_redacts_and_redeems_one_attempt_capability() {
    async fn lease(headers: HeaderMap, body: Bytes) -> axum::response::Response {
        assert_eq!(
            headers[reqwest::header::AUTHORIZATION],
            "Bearer identity-access"
        );
        let request: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(request["attempt_id"], "attempt-one");
        (
            [
                (reqwest::header::CACHE_CONTROL, "no-store"),
                (reqwest::header::PRAGMA, "no-cache"),
            ],
            Json(serde_json::json!({
                "lease_id": "lease-one",
                "lease_token": "lease-capability-value",
                "expires_at": 4_000_000_000_u64
            })),
        )
            .into_response()
    }

    async fn redeem(
        AxumPath(lease_id): AxumPath<String>,
        headers: HeaderMap,
        body: Bytes,
    ) -> axum::response::Response {
        assert_eq!(lease_id, "lease-one");
        assert_eq!(
            headers[reqwest::header::AUTHORIZATION],
            "Bearer lease-capability-value"
        );
        let request: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(request["attempt_id"], "attempt-one");
        (
            [
                (reqwest::header::CACHE_CONTROL, "no-store"),
                (reqwest::header::PRAGMA, "no-cache"),
            ],
            Json(serde_json::json!({
                "credential": "synthetic-provider-credential",
                "kind": "oauth"
            })),
        )
            .into_response()
    }

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = Router::new()
        .route(
            "/api/connectors/v1/subscription-credentials/claude-code/leases",
            post(lease),
        )
        .route(
            "/api/connectors/v1/subscription-leases/{lease_id}/redeem",
            post(redeem),
        );
    let serving = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let base = Url::parse(&format!("http://{address}/api/connectors/v1")).unwrap();
    let client = HostedClient::from_parts(base, reqwest::Client::new());
    let lease = client
        .lease_claude_code_subscription(
            "identity-access",
            "attempt-one",
            Duration::from_secs(60),
            1,
        )
        .await
        .unwrap();
    assert!(!format!("{lease:?}").contains("capability-value"));
    let redeemed = client
        .redeem_claude_code_subscription(&lease, "attempt-one")
        .await
        .unwrap();
    assert_eq!(
        redeemed.expose_at_provider_boundary(),
        "synthetic-provider-credential"
    );
    assert!(!format!("{redeemed:?}").contains("synthetic-provider"));
    serving.abort();
}

#[tokio::test]
async fn hosted_subscription_client_starts_and_completes_pkce_without_retaining_the_code() {
    async fn start(headers: HeaderMap) -> axum::response::Response {
        assert_eq!(
            headers[reqwest::header::AUTHORIZATION],
            "Bearer identity-access"
        );
        (
            [
                (reqwest::header::CACHE_CONTROL, "no-store"),
                (reqwest::header::PRAGMA, "no-cache"),
            ],
            Json(serde_json::json!({
                "authorization_url":"https://provider.example/authorize?state=opaque",
                "flow_id":"opaque-flow-identifier",
                "expires_at":4_000_000_000_u64
            })),
        )
            .into_response()
    }

    async fn complete(headers: HeaderMap, body: Bytes) -> axum::response::Response {
        assert_eq!(
            headers[reqwest::header::AUTHORIZATION],
            "Bearer identity-access"
        );
        let request: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(request["flow_id"], "opaque-flow-identifier");
        if request["code"] == "refused-provider-code" {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error":"must-not-cross-client"})),
            )
                .into_response();
        }
        assert_eq!(request["code"], "one-use-provider-code");
        (
            [
                (reqwest::header::CACHE_CONTROL, "no-store"),
                (reqwest::header::PRAGMA, "no-cache"),
            ],
            Json(serde_json::json!({
                "provider":"claude-code",
                "connected":true
            })),
        )
            .into_response()
    }

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = Router::new()
        .route(
            "/api/connectors/v1/subscription-credentials/claude-code/oauth/start",
            post(start),
        )
        .route(
            "/api/connectors/v1/subscription-credentials/claude-code/oauth/complete",
            post(complete),
        );
    let serving = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = HostedClient::new(&format!("http://{address}/api/connectors/v1")).unwrap();
    let started = client
        .start_claude_code_subscription_oauth("identity-access")
        .await
        .unwrap();
    assert_eq!(started.flow_id, "opaque-flow-identifier");
    let completed = client
        .complete_claude_code_subscription_oauth(
            "identity-access",
            &started.flow_id,
            Zeroizing::new("one-use-provider-code".to_owned()),
        )
        .await
        .unwrap();
    assert!(completed.connected);
    assert!(matches!(
        client
            .complete_claude_code_subscription_oauth(
                "identity-access",
                &started.flow_id,
                Zeroizing::new("refused-provider-code".to_owned()),
            )
            .await,
        Err(ClientError::SubscriptionRefused(400))
    ));
    serving.abort();
}

#[tokio::test]
async fn hosted_subscription_client_refuses_a_cacheable_credential_boundary() {
    async fn status() -> Json<serde_json::Value> {
        Json(serde_json::json!({"provider":"claude-code","connected":false}))
    }
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = Router::new().route(
        "/api/connectors/v1/subscription-credentials/claude-code",
        get(status),
    );
    let serving = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let base = Url::parse(&format!("http://{address}/api/connectors/v1")).unwrap();
    let client = HostedClient::from_parts(base, reqwest::Client::new());
    assert!(matches!(
        client
            .claude_code_subscription_status("identity-access")
            .await,
        Err(ClientError::CacheableCredentialResponse)
    ));
    serving.abort();
}

fn auth_stage2_invoke() -> operation::OperationRequest {
    operation::OperationRequest::Invoke(operation::InvokeRequest {
        operation_ref: "fixture.write".into(),
        connection_ref: "connection:fixture".into(),
        description_ref: "description:fixture".into(),
        input: serde_json::json!({"synthetic": true}),
        approval_evidence_ref: None,
    })
}

fn auth_stage2_reply(request_id: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "protocol": operation::v3::CONTRACT,
        "request_id": request_id,
        "status": "error",
        "error": {
            "code": "authentication_required",
            "message": "Authentication is required.",
            "retriable": false,
            "authentication": {
                "operation_ref": "fixture.write",
                "connection_ref": "connection:fixture",
                "integration_ref": "fixture",
                "auth_profile": "fixture.user",
                "need": "reauthorize_existing",
                "attempt": "not_attempted",
                "next_action": "start_trusted_remediation"
            }
        }
    })
}

#[tokio::test]
async fn auth_stage2_hosted_409_is_typed_without_resend() {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let app = Router::new().route(
        "/operations",
        post(move |body: Bytes| {
            let observed = observed.clone();
            async move {
                observed.fetch_add(1, Ordering::SeqCst);
                let request: serde_json::Value = serde_json::from_slice(&body).unwrap();
                let reply = auth_stage2_reply(&request["request_id"]);
                let (version, decoded) =
                    operation::versions::decode_response(&serde_json::to_vec(&reply).unwrap())
                        .unwrap();
                assert_eq!(version, operation::versions::Version::V0Alpha3);
                assert_eq!(
                    decoded.error.unwrap().code,
                    operation::v3::OperationErrorCode::AuthenticationRequired
                );
                (axum::http::StatusCode::CONFLICT, Json(reply))
            }
        }),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = Url::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
    let serving = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let reply = HostedClient::from_parts(base, reqwest::Client::new())
        .operation("synthetic-identity", &context(), auth_stage2_invoke())
        .await;
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    serving.abort();
    let reply =
        reply.expect("one bounded admitted HTTP 409 retains its typed authentication result");
    let value = serde_json::to_value(reply).unwrap();
    assert_eq!(value["protocol"], operation::v3::CONTRACT);
    assert_eq!(value["error"]["code"], "authentication_required");
    assert_eq!(value["error"]["authentication"]["attempt"], "not_attempted");
    assert_eq!(
        value["error"]["authentication"]["connection_ref"],
        "connection:fixture"
    );
}

#[tokio::test]
async fn auth_stage2_local_versions_are_strict_without_resend() {
    let mut failures = Vec::new();
    for case in [
        "auth",
        "rate",
        "outcome-unknown",
        "v2",
        "unknown-version",
        "wrong-correlation",
        "wrong-target",
        "duplicate",
        "private-error-code",
    ] {
        let root = tempdir().unwrap();
        let socket = root.path().join("auth.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let serving = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut line = String::new();
            BufReader::new(&mut stream)
                .read_line(&mut line)
                .await
                .unwrap();
            let request: serde_json::Value = serde_json::from_str(&line).unwrap();
            let mut response = auth_stage2_reply(&request["request_id"]);
            match case {
                "auth" | "duplicate" => {}
                "rate" | "outcome-unknown" | "v2" => {
                    response["error"]
                        .as_object_mut()
                        .unwrap()
                        .remove("authentication");
                    response["error"]["code"] = serde_json::json!(match case {
                        "rate" => "rate_limited",
                        "outcome-unknown" => "outcome_unknown",
                        _ => "unavailable",
                    });
                    if case == "rate" {
                        response["error"]["retriable"] = serde_json::json!(true);
                        response["error"]["retry_after_seconds"] = serde_json::json!(30);
                    }
                    if case == "v2" {
                        response["protocol"] = serde_json::json!(operation::CONTRACT);
                    }
                }
                "unknown-version" => {
                    response["protocol"] = serde_json::json!("b10x.connector-operation.v99")
                }
                "wrong-correlation" => response["request_id"] = serde_json::json!("unrelated"),
                "wrong-target" => {
                    response["error"]["authentication"]["connection_ref"] =
                        serde_json::json!("connection:other")
                }
                "private-error-code" => {
                    response["error"]["code"] = serde_json::json!("SYNTHETIC_PRIVATE_INSTRUCTION")
                }
                _ => unreachable!(),
            }
            let mut encoded = response.to_string();
            if case == "duplicate" {
                encoded = format!(
                    "{{\"protocol\":\"{}\",{}",
                    operation::v3::CONTRACT,
                    &encoded[1..]
                );
            }
            stream
                .write_all(format!("{encoded}\n").as_bytes())
                .await
                .unwrap();
            drop(stream);
            let second = tokio::time::timeout(Duration::from_millis(50), listener.accept())
                .await
                .is_ok();
            (request["protocol"].clone(), second)
        });
        let reply = LocalClient::new(&socket)
            .operation(&context(), auth_stage2_invoke())
            .await;
        let (requested, second) = serving.await.unwrap();
        if requested != operation::v3::CONTRACT {
            failures.push(format!("{case}: requested predecessor"));
        }
        if second {
            failures.push(format!("{case}: resent"));
        }
        let wanted = matches!(case, "auth" | "rate" | "outcome-unknown");
        if reply.is_ok() != wanted {
            failures.push(format!("{case}: wrong acceptance"));
        }
        match reply {
            Ok(reply) if wanted => {
                let reply = serde_json::to_value(reply).unwrap();
                let code = match case {
                    "auth" => "authentication_required",
                    "rate" => "rate_limited",
                    _ => "outcome_unknown",
                };
                if reply["error"]["code"] != code {
                    failures.push(format!("{case}: wrong code"));
                }
                if case == "rate" && reply["error"]["retry_after_seconds"] != 30 {
                    failures.push("rate: lost delay".into());
                }
            }
            Err(error) if error.to_string().contains("SYNTHETIC_PRIVATE_INSTRUCTION") => {
                failures.push("arbitrary decoder text escaped".into())
            }
            _ => {}
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

fn auth_stage2_bound_status(deadline: u64, ready: bool) -> serde_json::Value {
    serde_json::json!({
        "connect_session_ref":"session:fixture", "operation_ref":"fixture.write", "connection_ref":"connection:fixture",
        "integration_ref":"fixture", "auth_profile":"fixture.user", "need":"reauthorize_existing",
        "session_state":if ready { "completed" } else { "pending" }, "expires_at_unix_ms":deadline,
        "resume_state":if ready { "ready" } else { "pending" },
        "session":{"connect_session_ref":"session:fixture", "integration_ref":"fixture",
          "state":if ready { "completed" } else { "pending" }, "expires_at_unix_ms":deadline,
          "browser_completion_url":if ready { serde_json::Value::Null } else { serde_json::json!(format!("http://127.0.0.1:18324/#token={}", "a".repeat(43))) },
          "connection_ref":if ready { serde_json::json!("connection:fixture") } else { serde_json::Value::Null }}
    })
}

#[tokio::test]
async fn auth_stage2_bound_completion_rechecks_target_schema_and_never_invokes() {
    use protocol::connection_v2 as v2;
    use std::sync::{Arc, Mutex};
    for case in [
        "ready",
        "wrong-start-target",
        "wrong-status-session",
        "wrong-status-target",
        "wrong-integration",
        "changed-profile",
        "changed-need",
        "changed-deadline",
        "expired-start",
        "consumed",
        "wrong-ack",
        "degraded",
        "wrong-description",
        "wrong-operation",
        "missing-binding",
        "changed-schema",
        "external-schema",
        "false-schema",
        "private-error",
    ] {
        let retrievals = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let external = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        external.set_nonblocking(true).unwrap();
        let external_ref = format!(
            "http://{}/SYNTHETIC_PRIVATE_INSTRUCTION",
            external.local_addr().unwrap()
        );
        let observed_retrievals = retrievals.clone();
        let stopped = stop.clone();
        let external_server = std::thread::spawn(move || {
            while !stopped.load(std::sync::atomic::Ordering::SeqCst) {
                match external.accept() {
                    Ok((mut stream, _)) => {
                        observed_retrievals.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        let body = "true";
                        let _ = std::io::Write::write_all(&mut stream, format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes());
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(1))
                    }
                    Err(error) => panic!("fixture listener failed: {error}"),
                }
            }
        });
        let temporary = tempdir().unwrap();
        let socket = temporary.path().join("bound.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let methods = Arc::new(Mutex::new(Vec::new()));
        let observed = methods.clone();
        let serving = tokio::spawn(async move {
            let deadline = unix_time_ms() + 60_000;
            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut line = String::new();
                BufReader::new(&mut stream)
                    .read_line(&mut line)
                    .await
                    .unwrap();
                let request: serde_json::Value = serde_json::from_str(&line).unwrap();
                let method = request["request"]["method"].as_str().unwrap();
                observed.lock().unwrap().push(method.to_owned());
                assert_ne!(
                    method, "invoke",
                    "remediation never dispatches the intended operation"
                );
                let mut response = serde_json::json!({"protocol":v2::CONTRACT,"request_id":request["request_id"],"status":"ok"});
                let result = match method {
                    "remediation_start" => {
                        assert_eq!(
                            request["request"]["params"],
                            serde_json::json!({"operation_ref":"fixture.write","connection_ref":"connection:fixture","input":{"synthetic":true}})
                        );
                        let mut status = auth_stage2_bound_status(
                            if case == "expired-start" { 1 } else { deadline },
                            false,
                        );
                        if case == "wrong-start-target" {
                            status["connection_ref"] = serde_json::json!("connection:other");
                        }
                        serde_json::json!({"result":method,"value":status})
                    }
                    "remediation_status" => {
                        assert_eq!(
                            request["request"]["params"]["connect_session_ref"],
                            "session:fixture"
                        );
                        let mut status = auth_stage2_bound_status(deadline, true);
                        match case {
                            "wrong-status-session" => {
                                status["connect_session_ref"] = serde_json::json!("session:other");
                                status["session"]["connect_session_ref"] =
                                    serde_json::json!("session:other");
                            }
                            "wrong-status-target" => {
                                status["connection_ref"] = serde_json::json!("connection:other");
                                status["session"]["connection_ref"] =
                                    serde_json::json!("connection:other");
                            }
                            "wrong-integration" => {
                                status["integration_ref"] = serde_json::json!("other");
                                status["session"]["integration_ref"] = serde_json::json!("other");
                            }
                            "changed-profile" => {
                                status["auth_profile"] = serde_json::json!("other.user")
                            }
                            "changed-need" => {
                                status["need"] = serde_json::json!("authorize_configured")
                            }
                            "changed-deadline" => {
                                status["expires_at_unix_ms"] = serde_json::json!(deadline + 1);
                                status["session"]["expires_at_unix_ms"] =
                                    serde_json::json!(deadline + 1);
                            }
                            "consumed" => status["resume_state"] = serde_json::json!("consumed"),
                            _ => {}
                        }
                        serde_json::json!({"result":method,"value":status})
                    }
                    "remediation_acknowledge" => {
                        assert_eq!(
                            request["request"]["params"],
                            serde_json::json!({"operation_ref":"fixture.write","connection_ref":"connection:fixture","connect_session_ref":"session:fixture"})
                        );
                        serde_json::json!({"result":method,"value":{"connect_session_ref":"session:fixture","operation_ref":"fixture.write","connection_ref":if case == "wrong-ack" { "connection:other" } else { "connection:fixture" },"next_action":"fresh_description_then_explicit_invoke"}})
                    }
                    "describe" if request["protocol"] == v2::CONTRACT => {
                        assert_eq!(
                            request["request"]["params"]["connection_ref"],
                            "connection:fixture"
                        );
                        serde_json::json!({"result":"describe","value":{"connection_ref":if case == "wrong-description" { "connection:other" } else { "connection:fixture" },"integration_ref":"fixture","label":"SYNTHETIC_PRIVATE_INSTRUCTION","state":if case == "degraded" { "degraded" } else { "callable" },"initiation":["b10x"],"route":{"kind":"direct"},"auth_profile":"fixture.user","channels":[]}})
                    }
                    "describe" => {
                        assert_eq!(request["protocol"], operation::v3::CONTRACT);
                        assert_eq!(
                            request["request"]["params"]["operation_ref"],
                            "fixture.write"
                        );
                        response["protocol"] = serde_json::json!(operation::v3::CONTRACT);
                        let schema = match case {
                            "changed-schema" => {
                                serde_json::json!({"type":"object","required":["new_field"]})
                            }
                            "external-schema" => {
                                serde_json::json!({"$ref":external_ref})
                            }
                            "false-schema" => serde_json::json!(false),
                            _ => {
                                serde_json::json!({"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object","required":["synthetic"],"properties":{"synthetic":{"const":true}},"additionalProperties":false})
                            }
                        };
                        serde_json::json!({"result":"describe","value":{"operation_ref":if case == "wrong-operation" { "other.write" } else { "fixture.write" },"title":"SYNTHETIC_PRIVATE_INSTRUCTION","description":"fixture","input_schema":schema,"output_schema":{},"effect":"read_only","approval":"not_required","connections":[{"connection_ref":if case == "missing-binding" { "connection:other" } else { "connection:fixture" },"label":"SYNTHETIC_PRIVATE_INSTRUCTION","provider":"fixture","audiences":[]}],"description_ref":"description:fresh"}})
                    }
                    _ => panic!("unexpected private workflow method"),
                };
                if case == "private-error" && method == "remediation_status" {
                    response["status"] = serde_json::json!("error");
                    response["error"] = serde_json::json!({"code":"unavailable","message":"SYNTHETIC_PRIVATE_INSTRUCTION","retriable":false});
                } else {
                    response["response"] = result;
                }
                // Every hostile response except the intentionally expired client clock case is
                // a valid DTO; correlation and trusted binding checks must still reject it.
                let bytes = response.to_string();
                if response["protocol"] == v2::CONTRACT {
                    v2::decode_response(bytes.as_bytes()).unwrap();
                } else {
                    operation::versions::decode_response(bytes.as_bytes()).unwrap();
                }
                stream
                    .write_all(format!("{bytes}\n").as_bytes())
                    .await
                    .unwrap();
            }
        });
        let client = LocalClient::new(&socket);
        let started = client
            .begin_remediation(
                &context(),
                v2::RemediationStartRequest {
                    operation_ref: "fixture.write".into(),
                    connection_ref: "connection:fixture".into(),
                    input: serde_json::json!({"synthetic":true}),
                },
                "fixture",
                "fixture.user",
            )
            .await;
        let result = match started {
            Ok(pending) => client.finish_remediation(&context(), pending).await,
            Err(error) => Err(error),
        };
        serving.abort();
        stop.store(true, std::sync::atomic::Ordering::SeqCst);
        external_server.join().unwrap();
        assert_eq!(
            retrievals.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "schema checking must not perform HTTP retrieval: {case}"
        );

        if case == "ready" {
            result.unwrap();
        } else {
            assert!(
                !result
                    .unwrap_err()
                    .to_string()
                    .contains("SYNTHETIC_PRIVATE_INSTRUCTION"),
                "{case}"
            );
        }
        let methods = methods.lock().unwrap();
        assert!(methods.iter().all(|method| method != "invoke"));
        assert!(
            methods
                .iter()
                .filter(|method| method.as_str() == "remediation_acknowledge")
                .count()
                <= 1
        );
        if case == "ready" {
            assert_eq!(
                *methods,
                [
                    "remediation_start",
                    "remediation_status",
                    "remediation_acknowledge",
                    "describe",
                    "describe"
                ]
            );
        }
        if matches!(case, "wrong-start-target" | "expired-start") {
            assert_eq!(methods.len(), 1);
        }
        if matches!(
            case,
            "wrong-status-session"
                | "wrong-status-target"
                | "wrong-integration"
                | "changed-profile"
                | "changed-need"
                | "changed-deadline"
                | "consumed"
        ) {
            assert_eq!(methods.len(), 2);
        }
    }
}

fn unix_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[tokio::test]
async fn auth_adversary_fresh_operation_binding_rejects_conflicting_purpose() {
    use protocol::connection_v2 as v2;
    let mut observations = Vec::new();
    for case in ["matching", "legacy-absent", "conflicting", "split-pair"] {
        let temporary = tempdir().unwrap();
        let socket = temporary.path().join("purpose.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let serving = tokio::spawn(async move {
            let deadline = unix_time_ms() + 60_000;
            let mut methods = Vec::new();
            for step in 0..5 {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut line = String::new();
                BufReader::new(&mut stream)
                    .read_line(&mut line)
                    .await
                    .unwrap();
                let request: serde_json::Value = serde_json::from_str(&line).unwrap();
                let method = request["request"]["method"].as_str().unwrap();
                methods.push(method.to_owned());
                let value = match step {
                    0 => auth_stage2_bound_status(deadline, false),
                    1 => auth_stage2_bound_status(deadline, true),
                    2 => {
                        serde_json::json!({"connect_session_ref":"session:fixture","operation_ref":"fixture.write","connection_ref":"connection:fixture","next_action":"fresh_description_then_explicit_invoke"})
                    }
                    3 => {
                        serde_json::json!({"connection_ref":"connection:fixture","integration_ref":"fixture","label":"fixture","state":"callable","initiation":["b10x"],"route":{"kind":"direct"},"auth_profile":"fixture.user","channels":[]})
                    }
                    4 => {
                        let mut binding = serde_json::json!({"connection_ref":"connection:fixture","label":"fixture","provider":"fixture","audiences":[]});
                        if case != "legacy-absent" {
                            binding["purpose"] = serde_json::json!(if case == "matching" {
                                "fixture.user"
                            } else {
                                "fixture.other"
                            });
                        }
                        let mut connections = vec![binding];
                        if case == "split-pair" {
                            connections.push(serde_json::json!({"connection_ref":"connection:other","label":"other","provider":"fixture","purpose":"fixture.user","audiences":[]}));
                        }
                        serde_json::json!({"operation_ref":"fixture.write","title":"fixture","description":"fixture","input_schema":{"type":"object","additionalProperties":false},"output_schema":{},"effect":"read_only","approval":"not_required","connections":connections,"description_ref":"description:fresh"})
                    }
                    _ => unreachable!(),
                };
                let response = serde_json::json!({"protocol":request["protocol"],"request_id":request["request_id"],"status":"ok","response":{"result":method,"value":value}});
                let bytes = serde_json::to_vec(&response).unwrap();
                if step == 4 {
                    operation::versions::decode_response(&bytes).unwrap();
                } else {
                    v2::decode_response(&bytes).unwrap();
                }
                stream.write_all(&bytes).await.unwrap();
                stream.write_all(b"\n").await.unwrap();
            }
            methods
        });
        let client = LocalClient::new(&socket);
        let result = async {
            let pending = client
                .begin_remediation(
                    &context(),
                    v2::RemediationStartRequest {
                        operation_ref: "fixture.write".into(),
                        connection_ref: "connection:fixture".into(),
                        input: serde_json::json!({}),
                    },
                    "fixture",
                    "fixture.user",
                )
                .await?;
            client.finish_remediation(&context(), pending).await
        }
        .await;
        let methods = tokio::time::timeout(Duration::from_secs(3), serving)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            methods,
            [
                "remediation_start",
                "remediation_status",
                "remediation_acknowledge",
                "describe",
                "describe"
            ]
        );
        observations.push((case, result.is_ok()));
    }
    eprintln!("valid DTO purpose observations after exactly five exchanges each: {observations:?}");
    assert_eq!(
        observations,
        [
            ("matching", true),
            ("legacy-absent", true),
            ("conflicting", false),
            ("split-pair", false)
        ],
        "an explicitly conflicting purpose must not be accepted as fresh matching binding"
    );
}

async fn auth_adversary2_completion_fixture(case: &'static str) -> (bool, Vec<String>) {
    use protocol::connection_v2 as v2;
    let temporary = tempdir().unwrap();
    let socket = temporary.path().join("fresh.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let methods = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let observed = methods.clone();
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let serving = tokio::spawn(async move {
        let deadline = unix_time_ms()
            + if case.starts_with("late-") {
                2_000
            } else {
                60_000
            };
        let mut stopped = Some(stopped);
        for step in 0..5 {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut line = String::new();
            BufReader::new(&mut stream)
                .read_line(&mut line)
                .await
                .unwrap();
            let request: serde_json::Value = serde_json::from_str(&line).unwrap();
            let method = request["request"]["method"].as_str().unwrap();
            observed.lock().unwrap().push(method.to_owned());
            assert_ne!(method, "invoke");
            if (case == "late-ack" && step == 2) || (case == "late-description" && step == 4) {
                stopped.take().unwrap().await.unwrap();
                return;
            }
            let value = match step {
                0 => auth_stage2_bound_status(deadline, false),
                1 => auth_stage2_bound_status(deadline, true),
                2 => {
                    serde_json::json!({"connect_session_ref":"session:fixture","operation_ref":"fixture.write","connection_ref":"connection:fixture","next_action":"fresh_description_then_explicit_invoke"})
                }
                3 => {
                    serde_json::json!({"connection_ref":"connection:fixture","integration_ref":"fixture","label":"SYNTHETIC_PRIVATE_INSTRUCTION","state":"callable","initiation":["b10x"],"route":{"kind":"direct"},"auth_profile":"fixture.user","channels":[]})
                }
                4 => {
                    let schema = serde_json::json!({"$schema":"https://json-schema.org/draft/2020-12/schema", "$id":"https://private.invalid/SYNTHETIC_PRIVATE_INSTRUCTION", "$defs":{"input":{"type":"object","required":["synthetic"],"properties":{"synthetic":{"type":"boolean"}},"additionalProperties":false}}, "$ref":"#/$defs/input", "if":{"properties":{"synthetic":{"const":true}}}, "then":if case == "changed-composition" { serde_json::json!(false) } else { serde_json::json!({}) }});
                    serde_json::json!({"operation_ref":"fixture.write","title":"SYNTHETIC_PRIVATE_INSTRUCTION","description":"fixture","input_schema":schema,"output_schema":{},"effect":"read_only","approval":"not_required","connections":[{"connection_ref":"connection:fixture","label":"fixture","provider":"fixture","purpose":"fixture.user","audiences":[]}],"description_ref":"description:fresh"})
                }
                _ => unreachable!(),
            };
            let response = serde_json::json!({"protocol":request["protocol"],"request_id":request["request_id"],"status":"ok","response":{"result":method,"value":value}});
            let bytes = serde_json::to_vec(&response).unwrap();
            if step == 4 {
                operation::versions::decode_response(&bytes).unwrap();
            } else {
                v2::decode_response(&bytes).unwrap();
            }
            stream.write_all(&bytes).await.unwrap();
            stream.write_all(b"\n").await.unwrap();
        }
    });
    let client = LocalClient::new(&socket);
    let result = tokio::time::timeout(Duration::from_secs(5), async {
        let pending = client
            .begin_remediation(
                &context(),
                v2::RemediationStartRequest {
                    operation_ref: "fixture.write".into(),
                    connection_ref: "connection:fixture".into(),
                    input: serde_json::json!({"synthetic":true}),
                },
                "fixture",
                "fixture.user",
            )
            .await?;
        client.finish_remediation(&context(), pending).await
    })
    .await;
    let _ = stop.send(());
    tokio::time::timeout(Duration::from_secs(1), serving)
        .await
        .unwrap()
        .unwrap();
    let result = result.expect("the captured session deadline must bound completion");
    if let Err(error) = &result {
        assert!(!error.to_string().contains("SYNTHETIC_PRIVATE_INSTRUCTION"));
    }
    let observed = methods.lock().unwrap().clone();
    (result.is_ok(), observed)
}

#[tokio::test]
async fn auth_adversary2_fresh_internal_schema_composition_validates_captured_input() {
    for (case, expected) in [("internal-reference", true), ("changed-composition", false)] {
        let (accepted, methods) = auth_adversary2_completion_fixture(case).await;
        eprintln!("fresh schema {case}: accepted={accepted}, methods={methods:?}");
        assert_eq!(
            methods,
            [
                "remediation_start",
                "remediation_status",
                "remediation_acknowledge",
                "describe",
                "describe"
            ]
        );
        assert_eq!(accepted, expected);
    }
}

#[tokio::test]
async fn auth_adversary2_deadline_after_ready_bounds_ack_and_fresh_description_without_resend() {
    for (case, expected_methods) in [("late-ack", 3), ("late-description", 5)] {
        let (accepted, methods) = auth_adversary2_completion_fixture(case).await;
        eprintln!("captured deadline {case}: accepted={accepted}, methods={methods:?}");
        assert!(!accepted);
        assert_eq!(methods.len(), expected_methods);
        assert_eq!(
            methods
                .iter()
                .filter(|method| method.as_str() == "remediation_acknowledge")
                .count(),
            1
        );
        assert!(methods.iter().all(|method| method != "invoke"));
    }
}

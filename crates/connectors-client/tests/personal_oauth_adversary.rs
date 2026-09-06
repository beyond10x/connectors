//! Public Unix/loopback witnesses; all private values are synthetic and all listeners are local.

use connectors_client::{ClientError, LocalClient};
use protocol::{connection as c, operation};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader};

const PRIVATE: &str = "OAUTH-PASS1-PRIVATE";
const TARGET: &str = "connection:configured";
const PROFILE: &str = "gitlab.oauth_token";

fn owner() -> operation::OwnerContext {
    operation::OwnerContext {
        tenant_id: "fixture".into(),
        agent_id: "fixture".into(),
        agent_revision: 1,
        authority_snapshot_id: "fixture".into(),
        authority_snapshot_sha256: "a".repeat(64),
    }
}

async fn daemon(
    listener: tokio::net::UnixListener,
    authority: String,
    mismatch: &'static str,
    calls: Arc<Mutex<Vec<&'static str>>>,
    mut stop: tokio::sync::oneshot::Receiver<()>,
) {
    let deadline = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
        + 30_000;
    let mut polls = 0;
    loop {
        let (mut stream, _) = tokio::select! {
            value = listener.accept() => value.unwrap(),
            _ = &mut stop => break,
        };
        let mut line = String::new();
        BufReader::new(&mut stream)
            .read_line(&mut line)
            .await
            .unwrap();
        let request: c::RequestEnvelope = serde_json::from_str(&line).unwrap();
        request.validate().unwrap();
        let mut status = c::ConnectSessionStatus {
            connect_session_ref: "session:configured".into(),
            integration_ref: "gitlab".into(),
            state: c::ConnectSessionState::Pending,
            expires_at_unix_ms: deadline,
            completion_endpoint: None,
            browser_completion_url: Some(format!("http://{authority}/#token={}", "z".repeat(43))),
            connection_ref: None,
        };
        let response = match request.request {
            c::ConnectionRequest::ConnectSessionCreate(create) => {
                calls.lock().unwrap().push("create");
                assert_eq!(create.auth_profile.as_deref(), Some(PROFILE));
                assert_eq!(create.label, "Trusted label");
                c::ResponseEnvelope::success(
                    request.request_id,
                    c::ConnectionResult::ConnectSessionCreate(status),
                )
            }
            c::ConnectionRequest::ConnectSessionStatus(poll) => {
                calls.lock().unwrap().push("status");
                assert_eq!(poll.connect_session_ref, "session:configured");
                polls += 1;
                if polls == 1 {
                    c::ResponseEnvelope::failure(
                        request.request_id,
                        c::ConnectionError::new(c::ConnectionErrorCode::Unavailable, PRIVATE, true),
                    )
                } else {
                    if polls >= 3 {
                        status.state = c::ConnectSessionState::Completed;
                        status.browser_completion_url = None;
                        status.connection_ref = Some(
                            if mismatch == "status_target" {
                                "connection:OAUTH-PASS1-PRIVATE"
                            } else {
                                TARGET
                            }
                            .into(),
                        );
                    }
                    c::ResponseEnvelope::success(
                        request.request_id,
                        c::ConnectionResult::ConnectSessionStatus(status),
                    )
                }
            }
            c::ConnectionRequest::Describe(describe) => {
                calls.lock().unwrap().push("describe");
                assert_eq!(describe.connection_ref, TARGET);
                c::ResponseEnvelope::success(
                    request.request_id,
                    c::ConnectionResult::Describe(c::ConnectionDescription {
                        summary: c::ConnectionSummary {
                            connection_ref: TARGET.into(),
                            integration_ref: if mismatch == "integration" {
                                "other"
                            } else {
                                "gitlab"
                            }
                            .into(),
                            label: PRIVATE.into(),
                            state: if mismatch == "authorized" {
                                c::ConnectionState::Authorized
                            } else {
                                c::ConnectionState::Callable
                            },
                            initiation: vec![c::ConnectionInitiator::Platform],
                            route: c::ConnectionRoute::Direct,
                            scope: None,
                            actor: None,
                            auth_profile: Some(
                                if mismatch == "profile" {
                                    "gitlab.token"
                                } else {
                                    PROFILE
                                }
                                .into(),
                            ),
                        },
                        channels: Vec::new(),
                    }),
                )
            }
            _ => panic!("no raw completion, operation, or alternate acquisition is permitted"),
        };
        response.validate().unwrap();
        let mut bytes = serde_json::to_vec(&response).unwrap();
        bytes.push(b'\n');
        stream.write_all(&bytes).await.unwrap();
    }
}

async fn instruction_request(listener: tokio::net::TcpListener, response: String) -> String {
    let (mut stream, _) = listener.accept().await.unwrap();
    let mut bytes = Vec::new();
    loop {
        bytes.push(stream.read_u8().await.unwrap());
        if bytes.ends_with(b"\r\n\r\n") {
            break;
        }
        assert!(bytes.len() < 8192);
    }
    let request = String::from_utf8(bytes).unwrap();
    assert!(request.starts_with("GET /instructions HTTP/1.1\r\n"));
    assert!(request
        .to_ascii_lowercase()
        .contains(&format!("x-connect-session: {}\r\n", "z".repeat(43))));
    assert!(!request.lines().next().unwrap().contains(&"z".repeat(43)));
    assert!(!request.to_ascii_lowercase().contains("referer:"));
    let _ = stream.write_all(response.as_bytes()).await;
    request
}

#[tokio::test]
async fn oauth_pass1_public_handoff_waits_for_bound_callable_success_without_replay() {
    for mismatch in [
        "none",
        "status_target",
        "integration",
        "profile",
        "authorized",
    ] {
        let root = tempfile::tempdir().unwrap();
        let socket = root.path().join("client.sock");
        let unix = tokio::net::UnixListener::bind(&socket).unwrap();
        let http = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let authority = http.local_addr().unwrap().to_string();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let serving = tokio::spawn(daemon(unix, authority, mismatch, calls.clone(), stopped));
        let body = serde_json::json!({
            "kind":"device_authorization", "verification_uri":"https://gitlab.example/device",
            "verification_uri_complete": null, "user_code": PRIVATE,
        })
        .to_string();
        let response = format!("HTTP/1.1 200 OK\r\nCache-Control: private, no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        let instruction = tokio::spawn(instruction_request(http, response));
        let client = LocalClient::new(socket);
        let pending = client
            .begin_personal_oauth(
                &owner(),
                "gitlab".into(),
                "Trusted label".into(),
                PROFILE.into(),
                TARGET.into(),
            )
            .await
            .unwrap();
        let private = client
            .personal_oauth_instructions(&pending, "https://gitlab.example")
            .await
            .unwrap();
        let mut human = Vec::new();
        private.write_human(&mut human).unwrap();
        let human = String::from_utf8(human).unwrap();
        assert!(human.contains(PRIVATE));
        assert!(
            !human.contains("user_code="),
            "an omitted complete URI remains omitted"
        );
        instruction.await.unwrap();
        let result = client.finish_personal_oauth(&owner(), &pending).await;
        stop.send(()).unwrap();
        serving.await.unwrap();
        let calls = calls.lock().unwrap();
        assert_eq!(calls.iter().filter(|call| **call == "create").count(), 1);
        assert_eq!(calls.iter().filter(|call| **call == "status").count(), 3);
        if mismatch == "none" {
            let result = result.unwrap();
            assert_eq!(result.summary.connection_ref, TARGET);
            assert_eq!(
                result.summary.label, PRIVATE,
                "typed client retains received description; console owns trusted presentation"
            );
        } else {
            let error = result.unwrap_err();
            assert!(matches!(error, ClientError::PersonalOAuthRefused));
            assert!(!format!("{error:?} / {error}").contains(PRIVATE));
        }
        assert_eq!(
            calls.iter().filter(|call| **call == "describe").count(),
            usize::from(mismatch != "status_target")
        );
    }
}

#[tokio::test]
async fn oauth_pass1_public_instruction_refusals_do_not_redirect_poll_or_echo_private_bytes() {
    for mode in [
        "redirect",
        "chunked_oversize",
        "cached",
        "wrong_origin",
        "control_code",
    ] {
        let root = tempfile::tempdir().unwrap();
        let socket = root.path().join("client.sock");
        let unix = tokio::net::UnixListener::bind(&socket).unwrap();
        let http = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let authority = http.local_addr().unwrap().to_string();
        let decoy = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let serving = tokio::spawn(daemon(unix, authority, "none", calls.clone(), stopped));
        let body = serde_json::json!({
            "kind":"device_authorization",
            "verification_uri": if mode == "wrong_origin" { "https://other.example/device" } else { "https://gitlab.example/device" },
            "user_code": if mode == "control_code" { "OAUTH-PASS1-PRIVATE\nescape" } else { PRIVATE },
        }).to_string();
        let response = if mode == "redirect" {
            format!("HTTP/1.1 302 Found\r\nLocation: http://{}/{PRIVATE}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n", decoy.local_addr().unwrap())
        } else if mode == "chunked_oversize" {
            let body = format!("{}{}", " ".repeat(65_536), body);
            format!("HTTP/1.1 200 OK\r\nCache-Control: no-store\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n{:x}\r\n{body}\r\n0\r\n\r\n", body.len())
        } else {
            format!("HTTP/1.1 200 OK\r\nCache-Control: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", if mode == "cached" { "public" } else { "no-store" }, body.len())
        };
        let instruction = tokio::spawn(instruction_request(http, response));
        let client = LocalClient::new(socket);
        let pending = client
            .begin_personal_oauth(
                &owner(),
                "gitlab".into(),
                "Trusted label".into(),
                PROFILE.into(),
                TARGET.into(),
            )
            .await
            .unwrap();
        let error = client
            .personal_oauth_instructions(&pending, "https://gitlab.example")
            .await
            .err()
            .unwrap();
        assert!(!format!("{error:?} / {error}").contains(PRIVATE), "{mode}");
        instruction.await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(20), decoy.accept())
                .await
                .is_err()
        );
        stop.send(()).unwrap();
        serving.await.unwrap();
        assert_eq!(*calls.lock().unwrap(), ["create"], "{mode}");
    }
}

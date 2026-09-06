use connectors_console::{output, Format};
use protocol::operation::{self, v3, versions};
use std::process::Command;

const PRIVATE: &str = "SYNTHETIC_PRIVATE_INSTRUCTION";
const HOSTILE: &str = r#"{
  "protocol": "b10x.connector-operation.v0alpha3",
  "request_id": "request-1",
  "status": "error",
  "error": {
    "code": "authentication_required",
    "message": "https://private.example.test/SYNTHETIC_PRIVATE_INSTRUCTION",
    "retriable": false,
    "authentication": {
      "operation_ref": "slack-conversations-history",
      "connection_ref": "connection:test",
      "integration_ref": "https://private.example.test/SYNTHETIC_PRIVATE_INSTRUCTION",
      "auth_profile": "slack.bot",
      "need": "authorize_configured",
      "attempt": "not_attempted",
      "next_action": "start_trusted_remediation"
    }
  }
}"#;

#[test]
fn auth_stage2_hostile_output_child() {
    let Ok(mode) = std::env::var("AUTH_STAGE2_OUTPUT_CHILD") else {
        return;
    };
    let format = match mode.as_str() {
        "text" => Format::Text,
        "compact" => Format::Compact,
        "json" => Format::Json,
        "yaml" => Format::Yaml,
        _ => panic!("closed fixture mode"),
    };
    let (version, reply) = versions::decode_response(HOSTILE.as_bytes())
        .expect("real strict decoder accepts this hostile but valid envelope");
    assert_eq!(version, versions::Version::V0Alpha3);
    assert_eq!(reply.protocol, v3::CONTRACT);
    assert_eq!(reply.request_id, "request-1");
    assert_eq!(reply.status, operation::ResponseStatus::Error);
    assert!(reply.response.is_none());
    let error = reply.error.as_ref().unwrap();
    assert_eq!(error.code, v3::OperationErrorCode::AuthenticationRequired);
    assert!(!error.retriable);
    let auth = error.authentication.as_ref().unwrap();
    assert_eq!(auth.operation_ref, "slack-conversations-history");
    assert_eq!(auth.connection_ref, "connection:test");
    assert_eq!(auth.attempt, v3::AuthenticationAttemptState::NotAttempted);
    assert!(auth.integration_ref.contains(PRIVATE));
    let projected =
        connectors_console::reduce_envelope!(reply, operation).expect_err("auth stays a refusal");
    output::emit_refusal_with_target(format, &projected, Some("local"));
}

#[test]
fn auth_stage2_valid_hostile_daemon_output_is_private_in_every_real_format() {
    let mut failures = Vec::new();
    for format in ["text", "compact", "json", "yaml"] {
        let output = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "auth_stage2_hostile_output_child", "--nocapture"])
            .env("AUTH_STAGE2_OUTPUT_CHILD", format)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "decoder/output child must execute successfully: {format}"
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        let combined = format!("{stdout}\n{stderr}");
        assert!(
            combined.contains("authentication_required"),
            "the auth refusal cannot disappear: {format}"
        );
        if combined.contains(PRIVATE) || combined.contains("https://private.example.test") {
            failures.push(format!("{format}: private daemon value escaped"));
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

fn bound_config() -> connectors_config::PersonalConfig {
    serde_json::from_value(serde_json::json!({
        "owner":{"tenant_id":"fixture","agent_id":"fixture","agent_revision":1,"authority_snapshot_id":"fixture","authority_snapshot_sha256":"a".repeat(64)},
        "catalog":[{"provider":"gitlab","instance":"personal","grant_ref":"grant:personal","initiation":"platform","operator_approved":true,
          "credential":"gitlab.oauth_token","endpoints":{"origin":"https://gitlab.example"},
          "oauth":{"auth_profile":"gitlab.oauth_token","flow":"authorization_code_pkce","client_authentication":"public","client_id":"fixture-client",
            "redirect_uri":"http://127.0.0.1:47193/oauth/callback","browser_placement":"same_machine","registration_use":"development_only","custody":"development_file","allowed_scopes":["read_api"]}}]
    })).unwrap()
}
fn bound_request() -> protocol::connection_v2::RemediationStartRequest {
    let config = bound_config();
    protocol::connection_v2::RemediationStartRequest {
        operation_ref: "gitlab-fixture-read".into(),
        connection_ref: integration_catalog::personal_oauth_admitted_connection_ref(
            &config.principal_context().unwrap(),
            &config.catalog[0],
        )
        .unwrap(),
        input: serde_json::json!({"synthetic":true}),
    }
}
fn private_root() -> tempfile::TempDir {
    use std::os::unix::fs::PermissionsExt as _;
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    root
}

#[tokio::test]
async fn auth_stage2_bound_presenter_refuses_before_session_or_output() {
    use std::os::unix::fs::PermissionsExt as _;
    for case in [
        "no-daemon",
        "wrong-binding",
        "existing-output",
        "public-directory",
    ] {
        let root = private_root();
        let output = root.path().join("private-instructions");
        let listener = if case == "no-daemon" {
            None
        } else {
            let path = root.path().join("connectors.sock");
            let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
            listener.set_nonblocking(true).unwrap();
            Some(listener)
        };
        if case == "existing-output" {
            std::fs::write(&output, "retain-existing").unwrap();
        }
        if case == "public-directory" {
            std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let mut request = bound_request();
        if case == "wrong-binding" {
            request.connection_ref = "connection:other".into();
        }
        let result = connectors_console::remediation::run(
            &bound_config(),
            root.path(),
            request,
            Some(&output),
        )
        .await;
        let error = result.unwrap_err();
        if case == "no-daemon" {
            assert_eq!(error.code(), "daemon-required");
        }
        assert!(!error.to_string().contains("fixture-client"));
        if case == "existing-output" {
            assert_eq!(std::fs::read_to_string(output).unwrap(), "retain-existing");
        } else {
            assert!(!output.exists());
        }
        if let Some(listener) = listener {
            assert_eq!(
                listener.accept().unwrap_err().kind(),
                std::io::ErrorKind::WouldBlock
            );
        }
    }
}

#[test]
fn auth_stage2_bound_input_is_bounded_and_errors_do_not_echo_values() {
    use connectors_console::remediation::read_input;
    assert_eq!(
        read_input(Some("{\"a\":1}".into()), None, None).unwrap(),
        serde_json::json!({"a":1})
    );
    for text in [
        "SYNTHETIC_PRIVATE_INSTRUCTION".to_owned(),
        format!("{{\"x\":\"{}\"}}", "a".repeat(65536)),
    ] {
        assert!(!read_input(Some(text), None, None)
            .unwrap_err()
            .to_string()
            .contains(PRIVATE));
    }
    let root = private_root();
    let path = root.path().join("input");
    std::fs::write(&path, "a".repeat(65537)).unwrap();
    assert!(read_input(None, Some(path), None).is_err());
    assert!(read_input(None, None, None).is_err());
    assert!(read_input(Some("{}".into()), None, Some("-".into())).is_err());
}

#[tokio::test]
async fn auth_stage2_bound_presenter_clears_written_inode_on_success_expiry_and_drop() {
    use protocol::connection_v2 as v2;
    use std::os::unix::fs::PermissionsExt as _;
    use std::sync::Arc;
    use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader};
    for case in ["success", "expiry", "drop"] {
        let root = private_root();
        let path = root.path().join("instructions");
        let socket = root.path().join("connectors.sock");
        let listener = tokio::net::UnixListener::bind(&socket).unwrap();
        std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600)).unwrap();
        let instruction_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = instruction_listener.local_addr().unwrap();
        let human = tokio::spawn(async move {
            let (mut stream, _) = instruction_listener.accept().await.unwrap();
            let mut headers = Vec::new();
            loop {
                let b = stream.read_u8().await.unwrap();
                headers.push(b);
                if headers.ends_with(b"\r\n\r\n") {
                    break;
                }
                assert!(headers.len() < 8192);
            }
            let headers = String::from_utf8(headers).unwrap().to_ascii_lowercase();
            assert!(headers.starts_with("get /instructions http/1.1\r\n"));
            assert!(headers.contains(&format!("x-connect-session: {}\r\n", "p".repeat(43))));
            let body = r#"{"kind":"device_authorization","verification_uri":"https://gitlab.example/oauth/device","verification_uri_complete":null,"user_code":"SYNTHETIC_PRIVATE_INSTRUCTION"}"#;
            stream.write_all(format!("HTTP/1.1 200 OK\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
        });
        let (release, released) = tokio::sync::oneshot::channel::<()>();
        let mut released = Some(released);
        let entered = Arc::new(tokio::sync::Notify::new());
        let observed = entered.clone();
        let private_path = path.clone();
        let expected = bound_request().connection_ref;
        let daemon = tokio::spawn(async move {
            let deadline = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
                + if case == "expiry" { 1000 } else { 60000 };
            for turn in 0..5 {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut line = String::new();
                BufReader::new(&mut stream)
                    .read_line(&mut line)
                    .await
                    .unwrap();
                let request: serde_json::Value = serde_json::from_str(&line).unwrap();
                let method = request["request"]["method"].as_str().unwrap();
                assert_ne!(method, "invoke");
                let result = match turn {
                    0 | 1 => {
                        assert_eq!(
                            method,
                            if turn == 0 {
                                "remediation_start"
                            } else {
                                "remediation_status"
                            }
                        );
                        if turn == 0 {
                            assert!(private_path.is_file());
                            assert!(std::fs::read(&private_path).unwrap().is_empty());
                            assert_eq!(
                                private_path.metadata().unwrap().permissions().mode() & 0o777,
                                0o600
                            );
                        } else {
                            let text = std::fs::read_to_string(&private_path).unwrap();
                            assert!(text.contains(PRIVATE));
                            assert!(text.contains("https://gitlab.example/oauth/device"));
                            observed.notify_one();
                            if case != "success" {
                                std::future::pending::<()>().await;
                            } else {
                                released.take().unwrap().await.unwrap();
                            }
                        }
                        serde_json::json!({"result":method,"value":{"connect_session_ref":"session:bound","operation_ref":"gitlab-fixture-read","connection_ref":expected,"integration_ref":"gitlab","auth_profile":"gitlab.oauth_token","need":"reauthorize_existing","session_state":if turn==0{"pending"}else{"completed"},"resume_state":if turn==0{"pending"}else{"ready"},"expires_at_unix_ms":deadline,"session":{"connect_session_ref":"session:bound","integration_ref":"gitlab","state":if turn==0{"pending"}else{"completed"},"expires_at_unix_ms":deadline,"browser_completion_url":if turn==0{serde_json::json!(format!("http://{address}/#token={}","p".repeat(43)))}else{serde_json::Value::Null},"connection_ref":if turn==0{serde_json::Value::Null}else{serde_json::json!(expected)}}}})
                    }
                    2 => {
                        assert_eq!(method, "remediation_acknowledge");
                        serde_json::json!({"result":method,"value":{"connect_session_ref":"session:bound","operation_ref":"gitlab-fixture-read","connection_ref":expected,"next_action":"fresh_description_then_explicit_invoke"}})
                    }
                    3 => {
                        assert_eq!(method, "describe");
                        serde_json::json!({"result":method,"value":{"connection_ref":expected,"integration_ref":"gitlab","label":PRIVATE,"state":"callable","initiation":["b10x"],"route":{"kind":"direct"},"auth_profile":"gitlab.oauth_token","channels":[]}})
                    }
                    4 => {
                        assert_eq!(method, "describe");
                        serde_json::json!({"result":method,"value":{"operation_ref":"gitlab-fixture-read","title":PRIVATE,"description":PRIVATE,"input_schema":{"type":"object","required":["synthetic"]},"output_schema":{},"effect":"read_only","approval":"not_required","connections":[{"connection_ref":expected,"label":PRIVATE,"provider":"gitlab","audiences":[]}],"description_ref":PRIVATE}})
                    }
                    _ => unreachable!(),
                };
                let response = serde_json::json!({"protocol":if turn==4{operation::v3::CONTRACT}else{v2::CONTRACT},"request_id":request["request_id"],"status":"ok","response":result});
                let bytes = response.to_string();
                if turn == 4 {
                    versions::decode_response(bytes.as_bytes()).unwrap();
                } else {
                    v2::decode_response(bytes.as_bytes()).unwrap();
                }
                stream
                    .write_all(format!("{bytes}\n").as_bytes())
                    .await
                    .unwrap();
            }
        });
        let root_path = root.path().to_owned();
        let output_path = path.clone();
        let acquisition = tokio::spawn(async move {
            connectors_console::remediation::run(
                &bound_config(),
                &root_path,
                bound_request(),
                Some(&output_path),
            )
            .await
        });
        tokio::time::timeout(std::time::Duration::from_secs(5), entered.notified())
            .await
            .unwrap();
        let inode = std::fs::File::open(&path).unwrap();
        if case == "success" {
            release.send(()).unwrap();
        }
        if case == "drop" {
            acquisition.abort();
            assert!(acquisition.await.unwrap_err().is_cancelled());
        } else {
            let result = tokio::time::timeout(std::time::Duration::from_secs(3), acquisition)
                .await
                .unwrap()
                .unwrap();
            if case == "success" {
                let public = result.unwrap();
                assert_eq!(
                    public,
                    serde_json::json!({"ready":true,"attempt":"not_attempted","next_action":"explicit_invoke"})
                );
                assert!(!public.to_string().contains(PRIVATE));
            } else {
                assert!(result.is_err());
            }
        }
        assert!(!path.exists());
        assert_eq!(inode.metadata().unwrap().len(), 0);
        human.await.unwrap();
        daemon.abort();
    }
}

#[tokio::test]
async fn auth_adversary_presenter_error_clears_original_inode_after_path_replacement() {
    use protocol::connection_v2 as v2;
    use std::os::unix::fs::PermissionsExt as _;
    use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader};
    for replacement in [false, true] {
        let root = private_root();
        let path = root.path().join("instructions");
        let retained = root.path().join("original-inode");
        let socket = root.path().join("connectors.sock");
        let listener = tokio::net::UnixListener::bind(&socket).unwrap();
        std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600)).unwrap();
        let instruction_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = instruction_listener.local_addr().unwrap();
        let human = tokio::spawn(async move {
            let (mut stream, _) = instruction_listener.accept().await.unwrap();
            let mut headers = Vec::new();
            while !headers.ends_with(b"\r\n\r\n") {
                headers.push(stream.read_u8().await.unwrap());
                assert!(headers.len() < 8192);
            }
            assert!(String::from_utf8(headers)
                .unwrap()
                .to_ascii_lowercase()
                .contains(&format!("x-connect-session: {}", "p".repeat(43))));
            let body = r#"{"kind":"device_authorization","verification_uri":"https://gitlab.example/oauth/device","verification_uri_complete":null,"user_code":"SYNTHETIC_PRIVATE_INSTRUCTION"}"#;
            stream.write_all(format!("HTTP/1.1 200 OK\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
        });
        let private_path = path.clone();
        let old_path = retained.clone();
        let expected = bound_request().connection_ref;
        let daemon = tokio::spawn(async move {
            let deadline = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
                + 60_000;
            let mut methods = Vec::new();
            for turn in 0..2 {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut line = String::new();
                BufReader::new(&mut stream)
                    .read_line(&mut line)
                    .await
                    .unwrap();
                let request: serde_json::Value = serde_json::from_str(&line).unwrap();
                methods.push(request["request"]["method"].as_str().unwrap().to_owned());
                let response = if turn == 0 {
                    assert!(
                        std::fs::read(&private_path).unwrap().is_empty(),
                        "destination reserved before Start"
                    );
                    serde_json::json!({"protocol":v2::CONTRACT,"request_id":request["request_id"],"status":"ok","response":{"result":"remediation_start","value":{"connect_session_ref":"session:bound","operation_ref":"gitlab-fixture-read","connection_ref":expected,"integration_ref":"gitlab","auth_profile":"gitlab.oauth_token","need":"reauthorize_existing","session_state":"pending","resume_state":"pending","expires_at_unix_ms":deadline,"session":{"connect_session_ref":"session:bound","integration_ref":"gitlab","state":"pending","expires_at_unix_ms":deadline,"browser_completion_url":format!("http://{address}/#token={}","p".repeat(43))}}}})
                } else {
                    assert!(std::fs::read_to_string(&private_path)
                        .unwrap()
                        .contains(PRIVATE));
                    std::fs::rename(&private_path, &old_path).unwrap();
                    if replacement {
                        std::fs::write(&private_path, "retain-replacement").unwrap();
                    }
                    serde_json::json!({"protocol":v2::CONTRACT,"request_id":request["request_id"],"status":"error","error":{"code":"unavailable","message":PRIVATE,"retriable":false}})
                };
                let bytes = serde_json::to_vec(&response).unwrap();
                v2::decode_response(&bytes).unwrap();
                stream.write_all(&bytes).await.unwrap();
                stream.write_all(b"\n").await.unwrap();
            }
            methods
        });
        let result = connectors_console::remediation::run(
            &bound_config(),
            root.path(),
            bound_request(),
            Some(&path),
        )
        .await;
        let methods = daemon.await.unwrap();
        human.await.unwrap();
        assert_eq!(methods, ["remediation_start", "remediation_status"]);
        assert!(!result.unwrap_err().to_string().contains(PRIVATE));
        assert_eq!(
            std::fs::metadata(&retained).unwrap().len(),
            0,
            "the original private inode is cleared even when its pathname changed"
        );
        if replacement {
            assert_eq!(
                std::fs::read_to_string(&path).unwrap(),
                "retain-replacement"
            );
        } else {
            assert!(!path.exists());
        }
    }
}

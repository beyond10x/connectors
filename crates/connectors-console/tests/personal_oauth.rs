//! The trusted setup surface reserves private output before it asks the daemon for a session.

use std::os::unix::fs::PermissionsExt as _;

use connectors_console::connect::{dispatch_with_personal_oauth, PersonalOAuthOptions};

fn config() -> connectors_config::PersonalConfig {
    serde_json::from_value(serde_json::json!({
        "owner": {"tenant_id": "fixture", "agent_id": "fixture", "agent_revision": 1,
            "authority_snapshot_id": "fixture", "authority_snapshot_sha256": "a".repeat(64)},
        "catalog": [{"provider": "gitlab", "instance": "personal", "grant_ref": "grant:personal", "initiation": "platform",
            "operator_approved": true, "credential": "gitlab.oauth_token", "endpoints": {"origin": "https://gitlab.example"},
            "oauth": {"auth_profile": "gitlab.oauth_token", "flow": "authorization_code_pkce", "client_authentication": "public",
                "client_id": "fixture-client", "redirect_uri": "http://127.0.0.1:47193/oauth/callback", "browser_placement": "same_machine",
                "registration_use": "development_only", "custody": "development_file", "allowed_scopes": ["read_api"]}}]
    })).unwrap()
}

#[tokio::test]
async fn unsafe_private_destination_refuses_before_any_daemon_connection() {
    let root = private_tempdir();
    let socket = root.path().join("connectors.sock");
    let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    listener.set_nonblocking(true).unwrap();
    let output = root.path().join("instructions");
    std::fs::write(&output, "retain-existing-content").unwrap();
    std::fs::set_permissions(&output, std::fs::Permissions::from_mode(0o600)).unwrap();
    let result = dispatch_with_personal_oauth(
        "gitlab",
        &config(),
        &root.path().join("config.toml"),
        root.path(),
        None,
        None,
        PersonalOAuthOptions {
            instruction_file: Some(output.clone()),
            ..Default::default()
        },
    )
    .await;
    assert!(result.is_err());
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert_eq!(
        std::fs::read_to_string(output).unwrap(),
        "retain-existing-content"
    );
    let rendered = result.unwrap_err().to_string();
    assert!(!rendered.contains("fixture-client"));
    assert!(!rendered.contains("47193"));
}

#[tokio::test]
async fn ambiguous_profile_refuses_before_output_file_or_daemon_connection() {
    let root = private_tempdir();
    let socket = root.path().join("connectors.sock");
    let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    listener.set_nonblocking(true).unwrap();
    let mut config = config();
    let mut second = config.catalog[0].clone();
    second.instance = Some("second".into());
    config.catalog.push(second);
    let output = root.path().join("instructions");
    assert!(dispatch_with_personal_oauth(
        "gitlab",
        &config,
        &root.path().join("config.toml"),
        root.path(),
        None,
        None,
        PersonalOAuthOptions {
            instruction_file: Some(output.clone()),
            ..Default::default()
        }
    )
    .await
    .is_err());
    assert!(!output.exists());
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn headless_oauth_requires_private_file_before_session_creation() {
    const CHILD: &str = "B10X_TEST_PERSONAL_OAUTH_HEADLESS";
    if std::env::var_os(CHILD).is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "headless_oauth_requires_private_file_before_session_creation",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    rustix::process::setsid().unwrap();
    assert!(std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .is_err());
    let root = private_tempdir();
    let listener =
        std::os::unix::net::UnixListener::bind(root.path().join("connectors.sock")).unwrap();
    listener.set_nonblocking(true).unwrap();
    let result = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(dispatch_with_personal_oauth(
            "gitlab",
            &config(),
            &root.path().join("config.toml"),
            root.path(),
            None,
            None,
            PersonalOAuthOptions::default(),
        ));
    assert!(result.is_err());
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    let error = result.unwrap_err().to_string();
    assert!(!error.contains("fixture-client"));
    assert!(!error.contains("47193"));
}

#[tokio::test]
async fn explicit_private_file_is_reserved_before_create_and_erased_before_public_success() {
    use protocol::connection::*;
    use std::io::{BufRead as _, Read as _, Write as _};
    let root = private_tempdir();
    let expected_reference = configured_reference();
    let daemon_reference = expected_reference.clone();
    let destination = root.path().join("instructions");
    let local =
        std::os::unix::net::UnixListener::bind(root.path().join("connectors.sock")).unwrap();
    let instructions = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let authority = instructions.local_addr().unwrap();
    let serving_instructions = std::thread::spawn(move || {
        let (mut stream, _) = instructions.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut request = Vec::new();
        loop {
            let mut byte = [0];
            stream.read_exact(&mut byte).unwrap();
            request.push(byte[0]);
            if request.ends_with(b"\r\n\r\n") {
                break;
            }
            assert!(request.len() < 8192);
        }
        let request = String::from_utf8(request).unwrap().to_ascii_lowercase();
        assert!(request.contains(&format!("x-connect-session: {}\r\n", "p".repeat(43))));
        let body = r#"{"kind":"browser_authorization","authorization_url":"https://gitlab.example/oauth/authorize?state=PRIVATE-AUTHORIZATION"}"#;
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
    });
    let private_path = destination.clone();
    let serving = std::thread::spawn(move || {
        let deadline = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
            + 30_000;
        for turn in 0..3 {
            let (mut stream, _) = local.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut line = String::new();
            std::io::BufReader::new(&mut stream)
                .read_line(&mut line)
                .unwrap();
            let request: RequestEnvelope = serde_json::from_str(&line).unwrap();
            request.validate().unwrap();
            let result = match (turn, request.request) {
                (0, ConnectionRequest::ConnectSessionCreate(create)) => {
                    assert_eq!(create.auth_profile.as_deref(), Some("gitlab.oauth_token"));
                    assert!(
                        private_path.is_file(),
                        "private output reservation precedes Create"
                    );
                    assert_eq!(
                        private_path.metadata().unwrap().permissions().mode() & 0o777,
                        0o600
                    );
                    assert!(std::fs::read(&private_path).unwrap().is_empty());
                    ConnectionResult::ConnectSessionCreate(ConnectSessionStatus {
                        connect_session_ref: "session:fixture".into(),
                        integration_ref: "gitlab".into(),
                        state: ConnectSessionState::Pending,
                        expires_at_unix_ms: deadline,
                        completion_endpoint: None,
                        browser_completion_url: Some(format!(
                            "http://{authority}/#token={}",
                            "p".repeat(43)
                        )),
                        connection_ref: None,
                    })
                }
                (1, ConnectionRequest::ConnectSessionStatus(_)) => {
                    assert!(std::fs::read_to_string(&private_path)
                        .unwrap()
                        .contains("PRIVATE-AUTHORIZATION"));
                    ConnectionResult::ConnectSessionStatus(ConnectSessionStatus {
                        connect_session_ref: "session:fixture".into(),
                        integration_ref: "gitlab".into(),
                        state: ConnectSessionState::Completed,
                        expires_at_unix_ms: deadline,
                        completion_endpoint: None,
                        browser_completion_url: None,
                        connection_ref: Some(daemon_reference.clone()),
                    })
                }
                (2, ConnectionRequest::Describe(describe)) => {
                    assert_eq!(describe.connection_ref, daemon_reference);
                    ConnectionResult::Describe(ConnectionDescription {
                        summary: ConnectionSummary {
                            connection_ref: describe.connection_ref,
                            integration_ref: "gitlab".into(),
                            label: "Display only".into(),
                            state: ConnectionState::Callable,
                            initiation: vec![ConnectionInitiator::Platform],
                            route: ConnectionRoute::Direct,
                            scope: None,
                            actor: None,
                            auth_profile: Some("gitlab.oauth_token".into()),
                        },
                        channels: Vec::new(),
                    })
                }
                _ => panic!(
                    "the trusted flow creates once, polls, then describes without operation replay"
                ),
            };
            let response = ResponseEnvelope::success(request.request_id, result);
            response.validate().unwrap();
            writeln!(stream, "{}", serde_json::to_string(&response).unwrap()).unwrap();
        }
    });
    let outcome = dispatch_with_personal_oauth(
        "gitlab",
        &config(),
        &root.path().join("config.toml"),
        root.path(),
        None,
        None,
        PersonalOAuthOptions {
            instruction_file: Some(destination.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    serving.join().unwrap();
    serving_instructions.join().unwrap();
    assert!(!destination.exists());
    let public = serde_json::to_string(&outcome).unwrap();
    assert!(!public.contains("PRIVATE-AUTHORIZATION"));
    assert!(!public.contains(&"p".repeat(43)));
    assert!(!public.contains("oauth/authorize"));
    assert!(!public.contains("instruction"));
    assert_eq!(outcome["connection_ref"], expected_reference);
    assert_eq!(outcome["connection"], config().catalog[0].label());
}

#[test]
fn private_daemon_refusal_is_closed_on_stdout_and_stderr_in_all_formats() {
    const CHILD: &str = "B10X_TEST_PERSONAL_OAUTH_PRIVATE_ERROR";
    let Some(format) = std::env::var_os(CHILD) else {
        for format in ["text", "compact", "json", "yaml"] {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "private_daemon_refusal_is_closed_on_stdout_and_stderr_in_all_formats",
                    "--nocapture",
                ])
                .env(CHILD, format)
                .output()
                .unwrap();
            assert!(output.status.success(), "fixture process must finish");
            let stdout = String::from_utf8(output.stdout).unwrap();
            let stderr = String::from_utf8(output.stderr).unwrap();
            assert!(!stdout.contains("PRIVATE-AUTHORIZATION"));
            assert!(!stderr.contains("PRIVATE-AUTHORIZATION"));
            let message = "personal OAuth authorization expired or was refused";
            if matches!(format, "json" | "yaml") {
                assert!(stdout.contains(message));
                assert!(!stderr.contains(message));
            } else {
                assert!(stderr.contains(message));
                assert!(!stdout.contains(message));
            }
        }
        return;
    };
    use protocol::connection::*;
    use std::io::{BufRead as _, Write as _};
    let root = private_tempdir();
    let destination = root.path().join("instructions");
    let local =
        std::os::unix::net::UnixListener::bind(root.path().join("connectors.sock")).unwrap();
    let (acknowledge, acknowledged) = std::sync::mpsc::channel();
    let serving = std::thread::spawn(move || {
        let (mut stream, _) = local.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut line = String::new();
        std::io::BufReader::new(&mut stream)
            .read_line(&mut line)
            .unwrap();
        let request: RequestEnvelope = serde_json::from_str(&line).unwrap();
        assert!(matches!(
            request.request,
            ConnectionRequest::ConnectSessionCreate(_)
        ));
        let response = ResponseEnvelope::failure(
            request.request_id,
            ConnectionError::new(
                ConnectionErrorCode::InvalidInput,
                "PRIVATE-AUTHORIZATION-DAEMON-FIXTURE",
                false,
            ),
        );
        response.validate().unwrap();
        writeln!(stream, "{}", serde_json::to_string(&response).unwrap()).unwrap();
        acknowledge.send(()).unwrap();
    });
    let error = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(dispatch_with_personal_oauth(
            "gitlab",
            &config(),
            &root.path().join("config.toml"),
            root.path(),
            None,
            None,
            PersonalOAuthOptions {
                instruction_file: Some(destination.clone()),
                ..Default::default()
            },
        ))
        .unwrap_err();
    acknowledged
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("the actual daemon refusal was delivered");
    serving.join().unwrap();
    assert!(
        !destination.exists(),
        "the failure erases the reserved private file"
    );
    let format = match format.to_str().unwrap() {
        "text" => connectors_console::output::Format::Text,
        "compact" => connectors_console::output::Format::Compact,
        "json" => connectors_console::output::Format::Json,
        "yaml" => connectors_console::output::Format::Yaml,
        _ => panic!("closed fixture formats"),
    };
    connectors_console::output::emit_error(format, "personal-oauth-refused", &error.to_string());
}

#[test]
fn doctor_reports_exact_redirect_and_unsealed_custody_without_client_material() {
    let root = private_tempdir();
    let path = root.path().join("config.toml");
    std::fs::write(&path, toml::to_string(&config()).unwrap()).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let report = connectors_console::doctor::run(&path, root.path()).to_value();
    let public = serde_json::to_string(&report).unwrap();
    assert!(public.contains("personal-oauth-custody"));
    assert!(public.contains("unsealed at rest"));
    assert!(public.contains("http://127.0.0.1:47193/oauth/callback"));
    assert!(!public.contains("fixture-client"));
    assert!(!public.contains("oauth/authorize"));
    assert!(
        !public.contains("credential-store"),
        "OAuth-only custody must not claim an unrelated keyring"
    );
}

#[test]
fn doctor_retains_ordinary_credential_store_diagnostic_for_an_owner_only_config() {
    let root = private_tempdir();
    let path = root.path().join("config.toml");
    let mut config = config();
    config.catalog.clear();
    std::fs::write(&path, toml::to_string(&config).unwrap()).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let public =
        serde_json::to_string(&connectors_console::doctor::run(&path, root.path()).to_value())
            .unwrap();
    assert!(public.contains("credential-store"));
    assert!(!public.contains("personal-oauth-custody"));
}

fn private_tempdir() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    root
}

#[tokio::test]
async fn successful_private_daemon_label_cannot_reach_the_public_summary() {
    use protocol::connection::*;
    use std::io::{BufRead as _, Read as _, Write as _};
    let root = private_tempdir();
    let expected_reference = configured_reference();
    let daemon_reference = expected_reference.clone();
    let destination = root.path().join("instructions");
    let local =
        std::os::unix::net::UnixListener::bind(root.path().join("connectors.sock")).unwrap();
    let instructions = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let authority = instructions.local_addr().unwrap();
    let serving_instructions = std::thread::spawn(move || {
        let (mut stream, _) = instructions.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut request = Vec::new();
        loop {
            let mut byte = [0];
            stream.read_exact(&mut byte).unwrap();
            request.push(byte[0]);
            if request.ends_with(b"\r\n\r\n") {
                break;
            }
            assert!(request.len() < 8192);
        }
        let request = String::from_utf8(request).unwrap().to_ascii_lowercase();
        assert!(request.contains(&format!("x-connect-session: {}\r\n", "p".repeat(43))));
        let body = r#"{"kind":"browser_authorization","authorization_url":"https://gitlab.example/oauth/authorize?state=PRIVATE-AUTHORIZATION"}"#;
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
    });
    let private_path = destination.clone();
    let serving = std::thread::spawn(move || {
        let deadline = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
            + 30_000;
        for turn in 0..3 {
            let (mut stream, _) = local.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut line = String::new();
            std::io::BufReader::new(&mut stream)
                .read_line(&mut line)
                .unwrap();
            let request: RequestEnvelope = serde_json::from_str(&line).unwrap();
            request.validate().unwrap();
            let result = match (turn, request.request) {
                (0, ConnectionRequest::ConnectSessionCreate(create)) => {
                    assert_eq!(create.auth_profile.as_deref(), Some("gitlab.oauth_token"));
                    assert!(
                        private_path.is_file(),
                        "private output reservation precedes Create"
                    );
                    assert_eq!(
                        private_path.metadata().unwrap().permissions().mode() & 0o777,
                        0o600
                    );
                    assert!(std::fs::read(&private_path).unwrap().is_empty());
                    ConnectionResult::ConnectSessionCreate(ConnectSessionStatus {
                        connect_session_ref: "session:fixture".into(),
                        integration_ref: "gitlab".into(),
                        state: ConnectSessionState::Pending,
                        expires_at_unix_ms: deadline,
                        completion_endpoint: None,
                        browser_completion_url: Some(format!(
                            "http://{authority}/#token={}",
                            "p".repeat(43)
                        )),
                        connection_ref: None,
                    })
                }
                (1, ConnectionRequest::ConnectSessionStatus(_)) => {
                    assert!(std::fs::read_to_string(&private_path)
                        .unwrap()
                        .contains("PRIVATE-AUTHORIZATION"));
                    ConnectionResult::ConnectSessionStatus(ConnectSessionStatus {
                        connect_session_ref: "session:fixture".into(),
                        integration_ref: "gitlab".into(),
                        state: ConnectSessionState::Completed,
                        expires_at_unix_ms: deadline,
                        completion_endpoint: None,
                        browser_completion_url: None,
                        connection_ref: Some(daemon_reference.clone()),
                    })
                }
                (2, ConnectionRequest::Describe(describe)) => {
                    assert_eq!(describe.connection_ref, daemon_reference);
                    ConnectionResult::Describe(ConnectionDescription {
                        summary: ConnectionSummary {
                            connection_ref: describe.connection_ref,
                            integration_ref: "gitlab".into(),
                            label: "PRIVATE-AUTHORIZATION-DAEMON-LABEL".into(),
                            state: ConnectionState::Callable,
                            initiation: vec![ConnectionInitiator::Platform],
                            route: ConnectionRoute::Direct,
                            scope: None,
                            actor: None,
                            auth_profile: Some("gitlab.oauth_token".into()),
                        },
                        channels: Vec::new(),
                    })
                }
                _ => panic!(
                    "the trusted flow creates once, polls, then describes without operation replay"
                ),
            };
            let response = ResponseEnvelope::success(request.request_id, result);
            response.validate().unwrap();
            writeln!(stream, "{}", serde_json::to_string(&response).unwrap()).unwrap();
        }
    });
    let outcome = dispatch_with_personal_oauth(
        "gitlab",
        &config(),
        &root.path().join("config.toml"),
        root.path(),
        None,
        None,
        PersonalOAuthOptions {
            instruction_file: Some(destination.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    serving.join().unwrap();
    serving_instructions.join().unwrap();
    assert!(!destination.exists());
    let public = serde_json::to_string(&outcome).unwrap();
    assert!(!public.contains("PRIVATE-AUTHORIZATION"));
    assert!(!public.contains(&"p".repeat(43)));
    assert!(!public.contains("oauth/authorize"));
    assert!(!public.contains("instruction"));
    assert_eq!(outcome["connection_ref"], expected_reference);
    assert_eq!(outcome["connection"], config().catalog[0].label());
}

fn configured_reference() -> String {
    let config = config();
    integration_catalog::personal_oauth_admitted_connection_ref(
        &config.principal_context().unwrap(),
        &config.catalog[0],
    )
    .unwrap()
}

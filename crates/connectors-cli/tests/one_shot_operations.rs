//! Separate real processes share durable authority without publishing a control socket.

use std::fs;
use std::io::{BufRead as _, Write as _};
use std::os::unix::fs::{symlink, PermissionsExt as _};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

struct Fixture {
    root: PathBuf,
    config: PathBuf,
    state: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "shot-{:x}-{:x}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let config = root.join("connectors.toml");
        let state = root.join("s");
        let this = Self {
            root,
            config,
            state,
        };
        this.configure(
            "[[catalog]]\nprovider = 'slack'\ngrant_ref = 'fixture'\ninitiation = 'platform'\n",
        );
        this
    }

    fn configure(&self, integrations: &str) {
        fs::write(
            &self.config,
            format!(
                "[owner]\ntenant_id = 'fixture'\nagent_id = 'fixture'\nagent_revision = 1\n\
             authority_snapshot_id = 'fixture'\nauthority_snapshot_sha256 = '{}'\n{integrations}",
                "a".repeat(64)
            ),
        )
        .unwrap();
        fs::set_permissions(&self.config, fs::Permissions::from_mode(0o600)).unwrap();
    }

    fn command(&self, arguments: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_connectors"));
        command
            .args(["--output", "json"])
            .args(arguments)
            .env("HOME", &self.root)
            .env("XDG_CONFIG_HOME", self.root.join("config"))
            .env("XDG_STATE_HOME", self.root.join("state"))
            .env("PATH", self.root.join("no-programs"))
            .env_remove("DBUS_SESSION_BUS_ADDRESS")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }

    fn run(&self, arguments: &[&str]) -> Output {
        self.command(arguments)
            .arg("--config")
            .arg(&self.config)
            .arg("--state-root")
            .arg(&self.state)
            .output()
            .unwrap()
    }

    fn platform(&self) -> UnixListener {
        let socket = self.root.join("work.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).unwrap();
        self.configure(&format!(
            "[platform.connection]\nconnection_ref = 'connection-fixture'\nlabel = 'fixture'\n\
             grant_ref = 'grant-fixture'\ninitiation = 'platform'\n\
             [platform.module_sockets]\nwork = '{}'\n",
            socket.display()
        ));
        listener
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

fn value(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|_| panic!("{output:?}"))
}

fn success(output: &Output) -> Value {
    assert!(output.status.success(), "{output:?}");
    value(output)
}

fn serve_http(listener: UnixListener) -> thread::JoinHandle<String> {
    listener.set_nonblocking(true).unwrap();
    thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(15);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        && Instant::now() < deadline =>
                {
                    thread::sleep(Duration::from_millis(10))
                }
                Err(error) => panic!("fixture was not called: {error}"),
            }
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut reader = std::io::BufReader::new(&mut stream);
        let mut request = String::new();
        loop {
            let mut line = String::new();
            assert!(reader.read_line(&mut line).unwrap() > 0);
            request.push_str(&line);
            if line == "\r\n" {
                break;
            }
        }
        let body = r#"{"items":[],"next_cursor":null}"#;
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        thread::sleep(Duration::from_millis(30));
        assert!(
            matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock),
            "no second dispatch"
        );
        request
    })
}

#[test]
fn separate_describe_and_invoke_processes_reuse_the_same_authority_without_a_daemon() {
    let fixture = Fixture::new();
    let listener = fixture.platform();
    let description =
        success(&fixture.run(&["operation", "describe", "--operation", "work.requests.list"]));
    assert!(!fixture.state.join("connectors.sock").exists());
    let lease = description["description_ref"].as_str().unwrap();
    let serving = serve_http(listener);
    let invoked = success(&fixture.run(&[
        "operation",
        "invoke",
        "--operation",
        "work.requests.list",
        "--connection",
        "connection-fixture",
        "--description-ref",
        lease,
        "--input-json",
        r#"{"cursor":"","limit":1}"#,
    ]));
    assert_eq!(invoked["output"], json!({"items":[], "next_cursor":null}));
    assert!(serving
        .join()
        .unwrap()
        .starts_with("GET /api/work/v2/requests?"));
    assert!(!fixture.state.join("connectors.sock").exists());
}

#[test]
fn ordinary_search_and_connection_list_use_default_paths_without_a_daemon() {
    let fixture = Fixture::new();
    let config_dir = fixture.root.join("config/b10x");
    fs::create_dir_all(&config_dir).unwrap();
    fs::copy(&fixture.config, config_dir.join("connectors.toml")).unwrap();
    for arguments in [["operation", "search"], ["connection", "list"]] {
        success(&fixture.command(&arguments).output().unwrap());
    }
    assert!(!fixture
        .root
        .join("state/b10x/connectors/connectors.sock")
        .exists());
}

#[test]
fn events_and_session_signals_name_the_persistent_daemon_requirement() {
    let fixture = Fixture::new();
    for arguments in [
        vec!["event", "receive", "--channel", "fixture"],
        vec!["event", "search"],
        vec![
            "operation",
            "signal",
            "--execution-ref",
            "fixture",
            "--dtmf",
            "1",
        ],
    ] {
        let output = fixture.run(&arguments);
        assert!(!output.status.success());
        assert!(
            value(&output)["error"]["message"]
                .as_str()
                .unwrap()
                .contains("connectors serve local"),
            "{output:?}"
        );
        assert!(
            !fixture.state.join("event-reply-claims.sqlite").exists(),
            "refusal precedes runtime construction"
        );
    }
}

#[test]
fn doctor_enumerates_bounded_and_persistent_verbs() {
    let fixture = Fixture::new();
    let report = success(&fixture.run(&["inspect", "doctor"]));
    let daemon = report["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["check"] == "daemon")
        .unwrap();
    let detail = daemon["detail"].as_str().unwrap();
    for word in [
        "operation",
        "connection",
        "session",
        "event",
        "connectors serve local",
    ] {
        assert!(detail.contains(word), "missing {word}: {detail}");
    }
}

#[test]
fn a_running_daemon_is_used_without_constructing_a_local_runtime() {
    let fixture = Fixture::new();
    fs::create_dir(&fixture.state).unwrap();
    fs::set_permissions(&fixture.state, fs::Permissions::from_mode(0o700)).unwrap();
    let listener = UnixListener::bind(fixture.state.join("connectors.sock")).unwrap();
    fs::set_permissions(
        fixture.state.join("connectors.sock"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut line = String::new();
        std::io::BufReader::new(&mut stream)
            .read_line(&mut line)
            .unwrap();
        let request: Value = serde_json::from_str(&line).unwrap();
        let response = json!({"protocol":request["protocol"],"request_id":request["request_id"],
            "status":"ok","response":{"result":"search","value":{"operations":[]}}});
        writeln!(stream, "{response}").unwrap();
    });
    success(&fixture.run(&["operation", "search"]));
    server.join().unwrap();
    assert!(!fixture.state.join("event-reply-claims.sqlite").exists());
}

#[test]
fn existing_or_unsafe_socket_objects_never_trigger_ephemeral_fallback() {
    for kind in ["file", "symlink", "stale-socket"] {
        let fixture = Fixture::new();
        fs::create_dir(&fixture.state).unwrap();
        fs::set_permissions(&fixture.state, fs::Permissions::from_mode(0o700)).unwrap();
        let socket = fixture.state.join("connectors.sock");
        match kind {
            "file" => fs::write(&socket, "preserve").unwrap(),
            "symlink" => symlink(fixture.root.join("missing"), &socket).unwrap(),
            _ => drop(UnixListener::bind(&socket).unwrap()),
        }
        assert!(!fixture.run(&["operation", "search"]).status.success());
        assert!(fs::symlink_metadata(socket).is_ok());
        assert!(!fixture.state.join("event-reply-claims.sqlite").exists());
    }
}

#[test]
fn a_transport_that_drops_the_request_is_never_retried_locally() {
    let fixture = Fixture::new();
    fs::create_dir(&fixture.state).unwrap();
    fs::set_permissions(&fixture.state, fs::Permissions::from_mode(0o700)).unwrap();
    let socket = fixture.state.join("connectors.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).unwrap();
    let serving = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = String::new();
        std::io::BufReader::new(&mut stream)
            .read_line(&mut request)
            .unwrap();
        fs::remove_file(socket).unwrap();
        drop(stream);
    });
    assert!(!fixture.run(&["operation", "search"]).status.success());
    serving.join().unwrap();
    assert!(!fixture.state.join("event-reply-claims.sqlite").exists());
}

#[test]
fn concurrent_commands_and_daemon_start_cannot_take_the_in_flight_invocation_state() {
    let fixture = Fixture::new();
    let listener = fixture.platform();
    let description =
        success(&fixture.run(&["operation", "describe", "--operation", "work.requests.list"]));
    let lease = description["description_ref"].as_str().unwrap();
    let (arrived, arrival) = std::sync::mpsc::channel();
    let (release, released) = std::sync::mpsc::channel();
    let serving = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut reader = std::io::BufReader::new(&mut stream);
        loop {
            let mut line = String::new();
            assert!(reader.read_line(&mut line).unwrap() > 0);
            if line == "\r\n" {
                break;
            }
        }
        arrived.send(()).unwrap();
        released.recv_timeout(Duration::from_secs(10)).unwrap();
        let body = r#"{"items":[],"next_cursor":null}"#;
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
    });
    let mut first = fixture.command(&[
        "operation",
        "invoke",
        "--operation",
        "work.requests.list",
        "--connection",
        "connection-fixture",
        "--description-ref",
        lease,
        "--input-json",
        r#"{"cursor":"","limit":1}"#,
    ]);
    first
        .arg("--config")
        .arg(&fixture.config)
        .arg("--state-root")
        .arg(&fixture.state);
    let child = first.spawn().unwrap();
    arrival.recv_timeout(Duration::from_secs(10)).unwrap();
    for arguments in [["operation", "search"], ["serve", "local"]] {
        let refused = fixture.run(&arguments);
        assert!(!refused.status.success(), "{refused:?}");
        assert!(
            value(&refused)["error"]["message"]
                .as_str()
                .unwrap()
                .contains("owns this state root"),
            "{refused:?}"
        );
        assert!(!fixture.state.join("connectors.sock").exists());
    }
    release.send(()).unwrap();
    success(&child.wait_with_output().unwrap());
    serving.join().unwrap();
    success(&fixture.run(&["operation", "search"]));
}

#[test]
fn invalid_bounds_and_unsafe_state_refuse_before_runtime_state_is_opened() {
    let fixture = Fixture::new();
    for arguments in [
        vec!["operation", "search", "--limit", "0"],
        vec!["operation", "search", "--limit", "26"],
        vec!["connection", "list", "--limit", "0"],
        vec!["connection", "list", "--limit", "65"],
    ] {
        assert!(!fixture.run(&arguments).status.success());
        assert!(
            !fixture.state.exists(),
            "invalid input must not create state"
        );
    }
    fs::create_dir(&fixture.state).unwrap();
    fs::set_permissions(&fixture.state, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(!fixture.run(&["operation", "search"]).status.success());
    assert!(!fixture.state.join("event-reply-claims.sqlite").exists());
    fs::set_permissions(&fixture.state, fs::Permissions::from_mode(0o700)).unwrap();
    symlink(
        fixture.root.join("missing-lock"),
        fixture.state.join(".connectors.lock"),
    )
    .unwrap();
    assert!(!fixture.run(&["operation", "search"]).status.success());
    assert!(!fixture.state.join("event-reply-claims.sqlite").exists());
    assert!(!fixture.root.join("missing-lock").exists());
}

#[test]
fn connection_mutations_require_daemon_before_creating_continuation_state() {
    let fixture = Fixture::new();
    for arguments in [
        vec![
            "connection",
            "activate",
            "--candidate",
            "candidate-fixture",
            "--label",
            "fixture",
        ],
        vec![
            "connection",
            "materialize",
            "--observation",
            "observation-fixture",
        ],
    ] {
        let refused = fixture.run(&arguments);
        assert!(!refused.status.success());
        assert!(value(&refused)["error"]["message"]
            .as_str()
            .unwrap()
            .contains("connectors serve local"));
        assert!(!fixture.state.exists());
    }
}

#[test]
fn a_changed_authority_or_selected_connection_never_reaches_fixture_egress() {
    let fixture = Fixture::new();
    let listener = fixture.platform();
    listener.set_nonblocking(true).unwrap();
    let description =
        success(&fixture.run(&["operation", "describe", "--operation", "work.requests.list"]));
    let lease = description["description_ref"].as_str().unwrap();
    for (connection, description_ref) in
        [("connection-other", lease), ("connection-fixture", "stale")]
    {
        let refused = fixture.run(&[
            "operation",
            "invoke",
            "--operation",
            "work.requests.list",
            "--connection",
            connection,
            "--description-ref",
            description_ref,
            "--input-json",
            r#"{"cursor":"","limit":1}"#,
        ]);
        assert!(!refused.status.success());
        assert!(
            matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
        );
    }
    let changed = fs::read_to_string(&fixture.config)
        .unwrap()
        .replace("agent_revision = 1", "agent_revision = 2");
    fs::write(&fixture.config, changed).unwrap();
    let refused = fixture.run(&[
        "operation",
        "invoke",
        "--operation",
        "work.requests.list",
        "--connection",
        "connection-fixture",
        "--description-ref",
        lease,
        "--input-json",
        r#"{"cursor":"","limit":1}"#,
    ]);
    assert!(!refused.status.success());
    assert_eq!(value(&refused)["error"]["code"], "stale_authority");
    assert!(
        matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
    );
}

#[test]
fn browser_session_operations_are_refused_under_canonical_and_published_aliases() {
    use protocol::browser::*;
    let fixture = Fixture::new();
    fixture.configure(&format!(
        "[platform.connection]\nconnection_ref = 'connection-fixture'\nlabel = 'fixture'\ngrant_ref = 'grant-fixture'\ninitiation = 'platform'\n\
         [platform.browser]\nuser_data_dir = '{}'\nartifacts_dir = '{}'\nmaximum_nodes = 20\nmaximum_navigation_seconds = 1\n",
        fixture.root.join("browser").display(), fixture.root.join("artifacts").display()
    ));
    for operation in [
        BROWSER_OPEN_OPERATION,
        BROWSER_OPEN_TOOL_REF,
        BROWSER_GOTO_OPERATION,
        BROWSER_GOTO_TOOL_REF,
        BROWSER_SNAPSHOT_OPERATION,
        BROWSER_SNAPSHOT_TOOL_REF,
        BROWSER_SCREENSHOT_OPERATION,
        BROWSER_SCREENSHOT_TOOL_REF,
        BROWSER_CLOSE_OPERATION,
        BROWSER_CLOSE_TOOL_REF,
    ] {
        let description =
            success(&fixture.run(&["operation", "describe", "--operation", operation]));
        let refused = fixture.run(&[
            "operation",
            "invoke",
            "--operation",
            operation,
            "--connection",
            "connection-fixture",
            "--description-ref",
            description["description_ref"].as_str().unwrap(),
            "--input-json",
            "{}",
        ]);
        assert!(!refused.status.success(), "{operation}: {refused:?}");
        assert!(value(&refused)["error"]["message"]
            .as_str()
            .unwrap()
            .contains("connectors serve local"));
        assert!(!fixture.root.join("browser").exists());
        assert!(!fixture.root.join("artifacts").exists());
    }
}

#[test]
fn adversary_uncertain_invoke_never_resends_after_the_control_socket_disappears() {
    for reply in ["eof", "malformed", "wrong-correlation"] {
        let fixture = Fixture::new();
        let egress = fixture.platform();
        egress.set_nonblocking(true).unwrap();
        let description =
            success(&fixture.run(&["operation", "describe", "--operation", "work.requests.list"]));
        let socket = fixture.state.join("connectors.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut line = String::new();
            std::io::BufReader::new(&mut stream)
                .read_line(&mut line)
                .unwrap();
            let request: Value = serde_json::from_str(&line).unwrap();
            assert_eq!(request["request"]["method"], "invoke");
            fs::remove_file(socket).unwrap();
            match reply {
                "eof" => {}
                "malformed" => writeln!(stream, "not-json").unwrap(),
                _ => writeln!(
                    stream,
                    "{}",
                    json!({
                        "protocol": request["protocol"], "request_id": "another-request",
                        "status": "ok", "response": {"result": "invoke", "value": {
                            "operation_ref": "work.requests.list", "output": {},
                            "connector_audit_ref": "fixture-audit"
                        }}
                    })
                )
                .unwrap(),
            }
            request
        });
        let output = fixture.run(&[
            "operation",
            "invoke",
            "--operation",
            "work.requests.list",
            "--connection",
            "connection-fixture",
            "--description-ref",
            description["description_ref"].as_str().unwrap(),
            "--input-json",
            r#"{"cursor":"","limit":1}"#,
        ]);
        assert!(!output.status.success(), "{reply}: {output:?}");
        let request = server.join().unwrap();
        assert_eq!(request["request"]["params"]["input"]["limit"], 1);
        assert!(
            matches!(egress.accept(), Err(error)
            if error.kind() == std::io::ErrorKind::WouldBlock),
            "{reply}: uncertain invocation was dispatched again through ephemeral egress"
        );
        assert!(!fixture.state.join("connectors.sock").exists());
    }
}

#[test]
fn adversary_json_source_and_size_refusals_precede_one_shot_state_creation() {
    let fixture = Fixture::new();
    let input_path = fixture.root.join("oversized-input.json");
    fs::write(
        &input_path,
        json!({"value": "x".repeat(65_536)}).to_string(),
    )
    .unwrap();
    let base = [
        "operation",
        "invoke",
        "--operation",
        "slack-conversations-list",
        "--connection",
        "fixture-connection",
        "--description-ref",
        "fixture-description",
    ];
    for source in [
        vec![],
        vec!["--input-json", "{"],
        vec!["--input-file", input_path.to_str().unwrap()],
        vec![
            "--input-json",
            "{}",
            "--input-file",
            input_path.to_str().unwrap(),
        ],
    ] {
        let arguments = base.iter().copied().chain(source).collect::<Vec<_>>();
        let output = fixture.run(&arguments);
        assert!(!output.status.success(), "{output:?}");
        assert!(
            !fixture.state.exists(),
            "input refusal initialized local runtime state"
        );
    }
}

#[test]
fn adversary_hosted_refusal_and_target_conflict_never_construct_the_local_runtime() {
    let fixture = Fixture::new();
    for arguments in [
        vec!["operation", "search", "--target", "hosted"],
        vec!["connection", "list", "--target", "hosted"],
    ] {
        let output = fixture.command(&arguments).output().unwrap();
        assert!(!output.status.success(), "{output:?}");
        assert_eq!(value(&output)["target"], "hosted");
        assert!(!fixture.root.join("state/b10x/connectors").exists());
    }
    let output = fixture.run(&[
        "operation",
        "invoke",
        "--target",
        "hosted",
        "--operation",
        "fixture",
        "--connection",
        "fixture",
        "--description-ref",
        "fixture",
        "--input-file",
        "/this-fixture-file-does-not-exist",
    ]);
    assert!(!output.status.success());
    assert_eq!(value(&output)["error"]["code"], "target-conflict");
    assert!(!fixture.state.exists());
}

#[test]
fn adversary_caller_input_cannot_rebind_routes_or_revoked_grants() {
    let fixture = Fixture::new();
    let egress = fixture.platform();
    egress.set_nonblocking(true).unwrap();
    let description =
        success(&fixture.run(&["operation", "describe", "--operation", "work.requests.list"]));
    for input in [
        r#"{"cursor":"","limit":1,"base_url":"http://attacker.invalid"}"#,
        r#"{"cursor":"","limit":1,"Authorization":"Bearer attacker"}"#,
    ] {
        let serving = serve_http(egress.try_clone().unwrap());
        let invoked = fixture.run(&[
            "operation",
            "invoke",
            "--operation",
            "work.requests.list",
            "--connection",
            "connection-fixture",
            "--description-ref",
            description["description_ref"].as_str().unwrap(),
            "--input-json",
            input,
        ]);
        success(&invoked);
        let actual = serving.join().unwrap();
        assert!(actual.starts_with("GET /api/work/v2/requests?"), "{actual}");
        assert!(
            !actual.contains("attacker"),
            "caller input changed the request: {actual}"
        );
        assert!(
            !actual.to_ascii_lowercase().contains("authorization:"),
            "{actual}"
        );
    }
    let changed = fs::read_to_string(&fixture.config)
        .unwrap()
        .replace("grant-fixture", "replacement-grant");
    fs::write(&fixture.config, changed).unwrap();
    let refused = fixture.run(&[
        "operation",
        "invoke",
        "--operation",
        "work.requests.list",
        "--connection",
        "connection-fixture",
        "--description-ref",
        description["description_ref"].as_str().unwrap(),
        "--input-json",
        r#"{"cursor":"","limit":1}"#,
    ]);
    assert!(!refused.status.success(), "{refused:?}");
    assert_eq!(value(&refused)["error"]["code"], "stale_authority");
    assert!(matches!(egress.accept(), Err(error)
        if error.kind() == std::io::ErrorKind::WouldBlock));
}

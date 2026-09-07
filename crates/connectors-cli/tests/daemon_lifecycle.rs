//! Real process acceptance for explicit daemon ownership, independent of cluster access.

use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Fixture {
    root: PathBuf,
    config: PathBuf,
    state: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "dl-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let config = root.join("c.toml");
        fs::write(&config, format!("[owner]\ntenant_id='fixture'\nagent_id='fixture'\nagent_revision=1\nauthority_snapshot_id='fixture'\nauthority_snapshot_sha256='{}'\n", "a".repeat(64))).unwrap();
        fs::set_permissions(&config, fs::Permissions::from_mode(0o600)).unwrap();
        let state = root.join("s");
        Self {
            root,
            config,
            state,
        }
    }
    fn run(&self, arguments: &[&str]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_connectors"));
        command
            .args(["-o", "json"])
            .args(arguments)
            .arg("--state-root")
            .arg(&self.state)
            .env("HOME", &self.root)
            .env("XDG_CONFIG_HOME", self.root.join("c"))
            .env("XDG_STATE_HOME", self.root.join("state"))
            .env("PATH", self.root.join("no-programs"))
            .env_remove("DBUS_SESSION_BUS_ADDRESS")
            .stdin(Stdio::null());
        if arguments == ["daemon", "start"] {
            command.arg("--config").arg(&self.config);
        }
        command.output().unwrap()
    }
    fn success(&self, arguments: &[&str]) -> Value {
        let output = self.run(arguments);
        assert!(output.status.success(), "{output:?}");
        serde_json::from_slice(&output.stdout).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if self.state.join("connectors.sock").exists() {
            let _ = self.run(&["daemon", "stop"]);
        }
        fs::remove_dir_all(&self.root).unwrap();
    }
}

#[test]
fn explicit_start_is_idempotent_and_stop_releases_the_daemon() {
    let fixture = Fixture::new();
    let before = fixture.success(&["daemon", "status"]);
    assert_eq!(before["running"], false);
    assert!(!fixture.state.exists());
    let first = fixture.success(&["daemon", "start"]);
    assert_eq!(first["started"], true);
    let second = fixture.success(&["daemon", "start"]);
    assert_eq!(second["started"], false);
    assert_eq!(
        first["daemon"]["process_id"],
        second["daemon"]["process_id"]
    );
    assert_eq!(fixture.success(&["daemon", "status"])["running"], true);
    assert_eq!(fixture.success(&["daemon", "stop"])["stopped"], true);
    assert!(!fixture.state.join("connectors.sock").exists());
    assert_eq!(fixture.success(&["daemon", "status"])["running"], false);
}

#[test]
fn endpoint_invocation_requires_exactly_one_target_and_the_current_protocol() {
    let parser = connectors_cli::command();
    let common = [
        "connectors",
        "operation",
        "invoke",
        "--operation",
        "loki-query-range",
        "--description-ref",
        "description:fixture",
        "--input-json",
        "{}",
    ];
    assert!(parser.clone().try_get_matches_from(common).is_err());
    let mut endpoint = common.to_vec();
    endpoint.extend(["--endpoint-ref", "endpoint:fixture"]);
    assert!(parser.clone().try_get_matches_from(&endpoint).is_ok());
    endpoint.extend(["--connection", "connection:fixture"]);
    assert!(parser.try_get_matches_from(&endpoint).is_err());
}

#[test]
fn binding_accepts_named_secret_keys_and_refuses_values_without_a_secret_reference() {
    let mut arguments = vec![
        "connectors",
        "endpoint",
        "bind",
        "--endpoint-ref",
        "endpoint:fixture",
        "--provider",
        "asterisk",
        "--credential-key",
        "asterisk.password=password",
    ];
    assert!(connectors_cli::command()
        .try_get_matches_from(&arguments)
        .is_err());
    arguments.extend(["--credential-secret", "voice/ari-auth", "--scheme", "https"]);
    assert!(connectors_cli::command()
        .try_get_matches_from(arguments)
        .is_ok());
}

#[test]
fn subscription_commands_use_v2_and_keep_parameters_separate_from_endpoint_identity() {
    use protocol::event::v2;
    use std::io::{BufRead as _, Write as _};
    let fixture = Fixture::new();
    fs::create_dir(&fixture.state).unwrap();
    fs::set_permissions(&fixture.state, fs::Permissions::from_mode(0o700)).unwrap();
    let socket = fixture.state.join("connectors.sock");
    let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let server = std::thread::spawn(move || {
        let mut requests = Vec::new();
        for _ in 0..2 {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            let (mut stream, _) = loop {
                match listener.accept() {
                    Ok(pair) => break pair,
                    Err(error)
                        if error.kind() == std::io::ErrorKind::WouldBlock
                            && std::time::Instant::now() < deadline =>
                    {
                        std::thread::sleep(std::time::Duration::from_millis(10))
                    }
                    Err(error) => panic!("subscription command did not send one request: {error}"),
                }
            };
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .unwrap();
            let mut line = String::new();
            std::io::BufReader::new(&mut stream)
                .read_line(&mut line)
                .unwrap();
            let request: v2::RequestEnvelope = serde_json::from_str(&line).unwrap();
            request.validate().unwrap();
            let result = match &request.request {
                v2::EventRequest::Subscribe(value) => {
                    assert_eq!(value.endpoint_ref, "endpoint:fixture");
                    assert_eq!(value.channel_binding, "asterisk.ari-events");
                    assert_eq!(value.parameters["app"], "developer");
                    v2::EventResult::Subscribe {
                        subscription_ref: "subscription:fixture".into(),
                        channel: v2::ChannelSummary {
                            channel_ref: "channel:fixture".into(),
                            connection_ref: "connection:fixture".into(),
                            integration_ref: "asterisk".into(),
                            binding_ref: "asterisk.ari-events".into(),
                            events: vec!["ari.event".into()],
                        },
                    }
                }
                v2::EventRequest::Unsubscribe(value) => {
                    assert_eq!(value.subscription_ref, "subscription:fixture");
                    v2::EventResult::Unsubscribe {
                        subscription_ref: value.subscription_ref.clone(),
                    }
                }
                _ => panic!("unexpected subscription method"),
            };
            writeln!(
                stream,
                "{}",
                serde_json::to_string(&v2::ResponseEnvelope::success(&request.request_id, result))
                    .unwrap()
            )
            .unwrap();
            requests.push(request.request);
        }
        requests
    });
    let config = fixture.config.to_str().unwrap();
    let subscribed = fixture.success(&[
        "event",
        "subscribe",
        "--config",
        config,
        "--endpoint-ref",
        "endpoint:fixture",
        "--channel-binding",
        "asterisk.ari-events",
        "--parameters-json",
        r#"{"app":"developer"}"#,
    ]);
    assert_eq!(subscribed["subscription_ref"], "subscription:fixture");
    assert_eq!(subscribed["target"], "local");
    let stopped = fixture.success(&[
        "event",
        "unsubscribe",
        "--config",
        config,
        "--subscription-ref",
        "subscription:fixture",
    ]);
    assert_eq!(stopped["subscription_ref"], "subscription:fixture");
    assert_eq!(server.join().unwrap().len(), 2);
    fs::remove_file(socket).unwrap();
}

#[test]
fn subscription_target_conflicts_and_bad_parameters_refuse_before_state() {
    let fixture = Fixture::new();
    for parameters in ["[]".to_owned(), "{".to_owned(), " ".repeat(16 * 1024 + 1)] {
        let output = fixture.run(&[
            "event",
            "subscribe",
            "--endpoint-ref",
            "endpoint:fixture",
            "--channel-binding",
            "asterisk.ari-events",
            "--parameters-json",
            &parameters,
        ]);
        assert!(!output.status.success());
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["error"]["code"], "invalid-argument");
        assert!(!fixture.state.exists());
    }
    let output = fixture.run(&[
        "event",
        "--target",
        "hosted",
        "subscribe",
        "--endpoint-ref",
        "endpoint:fixture",
        "--channel-binding",
        "asterisk.ari-events",
        "--parameters-json",
        "{",
    ]);
    assert!(!output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"]["code"], "target-conflict");
    assert!(!fixture.state.exists());
}

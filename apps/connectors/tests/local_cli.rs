use serde_json::Value;
#[path = "../../../crates/connectors-host/tests/fixtures/clock/server.rs"]
mod clock_fixture;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    process::{Command, Output},
};

fn command(root: &tempfile::TempDir, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_connectors"))
        .args(["--output", "json", "--config"])
        .arg(root.path().join("config/config.toml"))
        .arg("--state-dir")
        .arg(root.path().join("state"))
        .args(args)
        .output()
        .unwrap()
}

fn success(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ok"], true);
    value["result"].clone()
}

#[test]
fn clock_check_uses_current_configured_key_without_metadata_or_service_start() {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use std::{net::UdpSocket, time::Duration};
    let root = tempfile::tempdir().unwrap();
    success(&command(&root, &["setup", "init"]));
    let config_path = root.path().join("config/config.toml");
    let original = fs::read_to_string(&config_path).unwrap();
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let address = socket.local_addr().unwrap();
    let config = |key: String| {
        format!(
            "{original}\n[approval_clock]\nformat='roughtime-clock/1'\naddress='{address}'\npublic_key='{key}'\nmax_rate_error_ppm=10000\n[adapters.forge]\ninstance_id='forge-local'\nadapter_id='gitlab'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\n[adapters.forge.executable]\npath='/not-installed/connectors-gitlab'\nsha256='{}'\nargs=[]\n",
            "a".repeat(64)
        )
    };
    fs::write(
        &config_path,
        config(STANDARD.encode(clock_fixture::root_key())),
    )
    .unwrap();
    success(&command(&root, &["adapters", "list"]));
    // The command needs configuration only, and cannot accidentally open SQLite.
    let state = root.path().join("state");
    let retired_state = root.path().join("unused-state");
    fs::rename(&state, &retired_state).unwrap();
    socket.set_nonblocking(true).unwrap();
    let mut packet = [0; 1024];
    assert_eq!(
        socket.recv_from(&mut packet).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    socket.set_nonblocking(false).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let fixture = std::thread::spawn(move || {
        for _ in 0..2 {
            let mut b = [0; 1024];
            let (n, caller) = socket.recv_from(&mut b).unwrap();
            assert_eq!(n, 1024);
            socket
                .send_to(&clock_fixture::Fixture::default().reply(&b[..n]), caller)
                .unwrap();
        }
    });
    let view = success(&command(
        &root,
        &["approvals", "clock-check", "--adapter", "forge"],
    ));
    assert_eq!(view["adapter"], "forge");
    let observation = &view["observation"];
    assert_eq!(
        observation["configuration_sha256"].as_str().unwrap().len(),
        64
    );
    assert!(
        observation["upper_unix_ms"].as_i64().unwrap()
            - observation["lower_unix_ms"].as_i64().unwrap()
            <= 4000
    );
    assert!(!state.exists());
    fs::write(&config_path, config(STANDARD.encode([17; 32]))).unwrap();
    let refused = command(&root, &["approvals", "clock-check", "--adapter", "forge"]);
    assert_eq!(refused.status.code(), Some(1));
    assert!(refused.stdout.is_empty());
    let error = String::from_utf8(refused.stderr).unwrap();
    assert!(error.contains("unavailable"));
    assert!(!error.contains("lower_unix_ms"));
    assert!(!state.exists());
    fixture.join().unwrap();
}

#[test]
fn production_parser_initializes_and_inspects_across_processes() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
        success(&command(&root, &["setup", "init"]))["disposition"],
        "created"
    );
    assert_eq!(
        success(&command(&root, &["adapters", "list"]))["adapters"],
        serde_json::json!([])
    );
    let before = fs::read(root.path().join("config/config.toml")).unwrap();
    let exists = command(&root, &["setup", "init"]);
    assert_eq!(exists.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&exists.stderr).contains("configuration_exists"));
    assert_eq!(
        fs::read(root.path().join("config/config.toml")).unwrap(),
        before
    );
    // No configured adapter or credential is created by setup.
    let check = success(&command(&root, &["setup", "check"]));
    assert!(
        check["prerequisites"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["name"] == "persistent_custody_qualification"
                && matches!(p["state"].as_str(), Some("ready" | "failed")))
    );
}

#[test]
fn inventory_and_unavailable_status_never_launch_configured_executable() {
    let root = tempfile::tempdir().unwrap();
    success(&command(&root, &["setup", "init"]));
    let config_path = root.path().join("config/config.toml");
    let config = fs::read_to_string(&config_path).unwrap()
        + &format!(
            "\n[adapters.forge]\ninstance_id='forge-local'\nadapter_id='gitlab'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\n[adapters.forge.executable]\npath='/not-installed/connectors-gitlab'\nsha256='{}'\nargs=[]\n",
            "a".repeat(64)
        );
    fs::write(&config_path, config).unwrap();
    let list = success(&command(&root, &["adapters", "list"]));
    assert_eq!(list["adapters"][0]["adapter"], "forge");
    let status = success(&command(
        &root,
        &["adapters", "status", "--adapter", "forge"],
    ));
    assert_eq!(status["observation"]["state"], "owner_unavailable");
    let describe = success(&command(
        &root,
        &["adapters", "describe", "--adapter", "forge"],
    ));
    assert_eq!(describe["source"], "configuration");
    assert!(describe.get("descriptor").is_none());
    let connections = success(&command(
        &root,
        &["connections", "list", "--adapter", "forge"],
    ));
    assert_eq!(connections["connections"], serde_json::json!([]));
    assert_eq!(connections["source"], "authority");
    assert_eq!(connections["stale"], false);
    let keys = success(&command(
        &root,
        &["approvals", "key-status", "--adapter", "forge"],
    ));
    assert!(keys.get("issuer").is_none());
    for invalid in [
        "private-sentinel",
        "00000000-0000-0000-0000-000000000000",
        "70FD9DE3-9AED-4673-8330-12F20CD0B962",
    ] {
        let output = command(
            &root,
            &[
                "approvals",
                "key-rotate",
                "--adapter",
                "forge",
                "--expected-revision",
                invalid,
                "--expected-key",
                invalid,
            ],
        );
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"]["data"]["code"], "invalid_input");
        assert!(!String::from_utf8_lossy(&output.stderr).contains(invalid));
    }
    assert!(!root.path().join("state/approval-issuer.lock").exists());
    assert!(
        connections["valid_until_ms"].as_u64().unwrap()
            > connections["observed_at_ms"].as_u64().unwrap()
    );
    for args in [
        vec![
            "connections",
            "describe",
            "--adapter",
            "forge",
            "--connection",
            "missing",
        ],
        vec![
            "connections",
            "status",
            "--adapter",
            "forge",
            "--acquisition",
            "missing",
        ],
        vec![
            "connections",
            "revoke",
            "--adapter",
            "forge",
            "--connection",
            "missing",
            "--expected-revision",
            "private-sentinel",
        ],
    ] {
        let output = command(&root, &args);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"]["data"]["code"], "not_found");
        assert!(!String::from_utf8_lossy(&output.stderr).contains("sentinel"));
    }
    let stale = command(
        &root,
        &[
            "connections",
            "list",
            "--adapter",
            "forge",
            "--cursor",
            "unknown",
        ],
    );
    let error: Value = serde_json::from_slice(&stale.stderr).unwrap();
    assert_eq!(error["error"]["data"]["code"], "stale_cursor");
    // Authority loss must not become a successful empty list or recreate state.
    fs::remove_file(root.path().join("state/metadata.sqlite3")).unwrap();
    let unavailable = command(&root, &["connections", "list", "--adapter", "forge"]);
    let error: Value = serde_json::from_slice(&unavailable.stderr).unwrap();
    assert_eq!(error["error"]["data"]["code"], "metadata_unavailable");
    assert!(!root.path().join("state/metadata.sqlite3").exists());
    fs::set_permissions(&config_path, fs::Permissions::from_mode(0o644)).unwrap();
    let output = command(&root, &["adapters", "list"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("config.toml"));
}

#[test]
fn protected_sources_are_not_consumed_before_configuration_admission() {
    let root = tempfile::tempdir().unwrap();
    let output = command(
        &root,
        &[
            "connections",
            "connect",
            "--adapter",
            "forge",
            "--profile",
            "token",
            "--credential-file",
            "/private/sentinel-secret-path",
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!text.contains("sentinel"));
    let refusal: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(refusal["error"]["code"], "failure");
    assert_eq!(refusal["error"]["data"]["code"], "invalid_configuration");
}

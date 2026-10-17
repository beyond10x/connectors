use serde_json::Value;
#[path = "../../../crates/connectors-host/tests/fixtures/clock/server.rs"]
mod clock_fixture;
#[cfg(unix)]
#[path = "support/owner.rs"]
mod owner_start;
#[cfg(unix)]
#[path = "support/socket.rs"]
mod sockets;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    fs,
    process::{Command, Output},
};

/// A directory holding no executable. An entry's executable is an absolute local
/// path (`contracts/cli/v1alpha1/semantics.md` §3), and on Windows an absolute path
/// names its drive: `/not-installed` there is refused as `invalid_configuration`.
const NOT_INSTALLED: &str = if cfg!(windows) {
    "C:/not-installed"
} else {
    "/not-installed"
};

fn command(root: &tempfile::TempDir, args: &[&str]) -> Output {
    command_from(
        std::path::Path::new(env!("CARGO_BIN_EXE_connectors")),
        root,
        args,
    )
}

fn command_from(binary: &std::path::Path, root: &tempfile::TempDir, args: &[&str]) -> Output {
    let run = || {
        Command::new(binary)
            .args(["--output", "json", "--config"])
            .arg(root.path().join("config/config.toml"))
            .arg("--state-dir")
            .arg(root.path().join("state"))
            .args(args)
            .output()
    };
    // A freshly written executable can be briefly busy (ETXTBSY) while a
    // concurrent test thread forks; that is not the behaviour under test.
    for _ in 0..50 {
        match run() {
            Err(error) if error.raw_os_error() == Some(26) => {
                std::thread::sleep(std::time::Duration::from_millis(100))
            }
            output => return output.unwrap(),
        }
    }
    run().unwrap()
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

/// The fixtures' signed midpoint, in Unix seconds.
const CLOCK_T: u64 = 1_789_076_475;

/// A configuration whose `[approval_clock]` is `table`, with one adapter `forge`.
fn clock_configuration(original: &str, table: &str) -> String {
    format!(
        "{original}\n{table}[adapters.forge]\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/1'\n[adapters.forge.executable]\npath='{NOT_INSTALLED}/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n",
        "a".repeat(64)
    )
}

/// A set-up root whose `[approval_clock]` is `table`.
fn clock_root(table: &str) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    success(&command(&root, &["setup", "init"]));
    let config_path = root.path().join("config/config.toml");
    let original = fs::read_to_string(&config_path).unwrap();
    fs::write(&config_path, clock_configuration(&original, table)).unwrap();
    root
}

/// A synthetic source signing `midpoint` and `radius` (seconds) with key `seed`.
fn clock_source(seed: u8, midpoint: u64, radius: u32) -> clock_fixture::Server {
    clock_fixture::Server::fixture(clock_fixture::Fixture {
        seed,
        midpoint,
        radius,
        ..Default::default()
    })
}

/// `approvals clock-check` refused with `unavailable` at `stage = observation`;
/// its `Failure`, after checking that nothing of the exchange leaks.
fn clock_unavailable(root: &tempfile::TempDir) -> Value {
    let refused = command(root, &["approvals", "clock-check", "--adapter", "forge"]);
    assert_eq!(refused.status.code(), Some(1));
    assert!(refused.stdout.is_empty());
    let error = String::from_utf8(refused.stderr).unwrap();
    // Never the observation's bounds, nor packet contents.
    for leaked in ["lower_unix_ms", "upper_unix_ms", "ROUGHTIM", "midpoint"] {
        assert!(!error.contains(leaked), "{leaked}: {error}");
    }
    let envelope: Value = serde_json::from_str(&error).unwrap();
    let failure = envelope["error"]["data"].clone();
    assert_eq!(failure["kind"], "operational", "{error}");
    assert_eq!(failure["code"], "unavailable", "{error}");
    assert_eq!(failure["stage"], "observation", "{error}");
    assert!(failure.get("clock_radius_ms").is_none(), "{error}");
    failure
}

/// Each line's `result`, in order.
fn clock_results(lines: &Value) -> Vec<&str> {
    lines
        .as_array()
        .unwrap()
        .iter()
        .map(|line| line["result"].as_str().unwrap())
        .collect()
}

#[test]
fn clock_check_uses_current_configured_key_without_metadata_or_service_start() {
    let (a, b) = (clock_source(7, CLOCK_T, 1), clock_source(8, CLOCK_T, 1));
    let table = |second: u8| {
        clock_fixture::quorum_toml(2, 10_000, &[(&a.address, 7), (&b.address, second)])
    };
    let root = clock_root(&table(8));
    let config_path = root.path().join("config/config.toml");
    success(&command(&root, &["adapters", "list"]));
    // The command needs configuration only, and cannot accidentally open SQLite.
    let state = root.path().join("state");
    let retired_state = root.path().join("unused-state");
    fs::rename(&state, &retired_state).unwrap();
    for source in [&a, &b] {
        assert_eq!(source.requests.load(std::sync::atomic::Ordering::SeqCst), 0);
    }
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
            <= 8000
    );
    assert!(!state.exists());
    let original = fs::read_to_string(&config_path).unwrap();
    fs::write(&config_path, original.replace(&table(8), &table(17))).unwrap();
    let failure = clock_unavailable(&root);
    assert_eq!(failure["clock_refusal"], "below_quorum", "{failure}");
    assert!(!state.exists());
}

#[test]
fn clock_check_reports_each_source() {
    let (a, liar, b) = (
        clock_source(7, CLOCK_T, 1),
        clock_source(8, CLOCK_T + 20, 1),
        clock_source(9, CLOCK_T, 2),
    );
    let root = clock_root(&clock_fixture::quorum_toml(
        2,
        10_000,
        &[(&a.address, 7), (&liar.address, 8), (&b.address, 9)],
    ));
    let output = command(&root, &["approvals", "clock-check", "--adapter", "forge"]);
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    for leaked in ["ROUGHTIM", "midpoint", "nonce"] {
        assert!(!text.contains(leaked), "{leaked}: {text}");
    }
    let view = success(&output);
    let observation = &view["observation"];
    let truth = i64::try_from(CLOCK_T * 1000).unwrap();
    assert!(observation["lower_unix_ms"].as_i64().unwrap() <= truth);
    assert!(observation["upper_unix_ms"].as_i64().unwrap() >= truth);
    assert_eq!(
        observation["sources"],
        serde_json::json!([
            {"index": 0, "address": a.address, "result": "agreed", "radius_ms": 1000},
            {"index": 1, "address": liar.address, "result": "outlier", "radius_ms": 1000},
            {"index": 2, "address": b.address, "result": "agreed", "radius_ms": 2000},
        ])
    );
}

#[test]
fn clock_check_split_refusal() {
    let (a, b) = (
        clock_source(7, CLOCK_T, 1),
        clock_source(8, CLOCK_T + 20, 1),
    );
    let root = clock_root(&clock_fixture::quorum_toml(
        2,
        10_000,
        &[(&a.address, 7), (&b.address, 8)],
    ));
    let failure = clock_unavailable(&root);
    assert_eq!(failure["clock_refusal"], "split", "{failure}");
    assert_eq!(failure["next_action"], "choose_clock_source", "{failure}");
    assert_eq!(
        failure["clock_sources"],
        serde_json::json!([
            {"index": 0, "address": a.address, "result": "verified", "radius_ms": 1000},
            {"index": 1, "address": b.address, "result": "verified", "radius_ms": 1000},
        ])
    );
}

#[test]
fn clock_check_below_quorum_refusal() {
    let good = clock_source(7, CLOCK_T, 1);
    let silent = clock_fixture::Server::silent();
    // Signed with seed 8, configured with seed 12's key.
    let wrong_key = clock_source(8, CLOCK_T, 1);
    // A 5 s signed radius is a 10 s interval, wider than the 8,000 ms cap: it
    // still counts, since only the combined answer is capped.
    let wide = clock_source(9, CLOCK_T, 5);
    let (g, s, w, i) = (
        (good.address.as_str(), 7),
        (silent.address.as_str(), 10),
        (wrong_key.address.as_str(), 12),
        (wide.address.as_str(), 9),
    );
    // Every verified source counts, whatever its radius, so a row's quorum is
    // set above its verified count; the wide line is verified with its radius.
    for (sources, quorum, results, action) in [
        (
            vec![g, s, w, i],
            3,
            vec!["verified", "unreachable", "unverified", "verified"],
            "check_configuration",
        ),
        (
            vec![g, s, i],
            3,
            vec!["verified", "unreachable", "verified"],
            "retry_explicitly",
        ),
        (
            vec![g, s],
            2,
            vec!["verified", "unreachable"],
            "retry_explicitly",
        ),
    ] {
        let root = clock_root(&clock_fixture::quorum_toml(quorum, 10_000, &sources));
        let failure = clock_unavailable(&root);
        assert_eq!(failure["clock_refusal"], "below_quorum", "{failure}");
        assert_eq!(failure["next_action"], action, "{failure}");
        let lines = &failure["clock_sources"];
        assert_eq!(clock_results(lines), results, "{failure}");
        for (index, line) in lines.as_array().unwrap().iter().enumerate() {
            assert_eq!(line["index"], index, "{failure}");
            assert_eq!(line["address"], sources[index].0, "{failure}");
            let radius = match line["result"].as_str().unwrap() {
                "verified" if sources[index] == i => Some(5000),
                "verified" => Some(1000),
                _ => None,
            };
            assert_eq!(line["radius_ms"].as_i64(), radius, "{failure}");
        }
    }
}

#[test]
fn clock_check_legacy_format_refusal() {
    use base64::{Engine, engine::general_purpose::STANDARD};
    let source = clock_source(7, CLOCK_T, 1);
    let root = clock_root(&format!(
        "[approval_clock]\nformat='roughtime-clock/1'\naddress='{}'\npublic_key='{}'\nmax_rate_error_ppm=10000\n",
        source.address,
        STANDARD.encode(clock_fixture::root_key())
    ));
    // The retired table no longer fails the whole configuration.
    let listed = success(&command(&root, &["adapters", "list"]));
    assert_eq!(listed["adapters"][0]["adapter"], "forge", "{listed}");
    let refused = command(&root, &["approvals", "clock-check", "--adapter", "forge"]);
    assert_eq!(refused.status.code(), Some(2));
    let error = String::from_utf8(refused.stderr).unwrap();
    let envelope: Value = serde_json::from_str(&error).unwrap();
    let failure = &envelope["error"]["data"];
    assert_eq!(failure["kind"], "usage", "{error}");
    assert_eq!(failure["code"], "invalid_configuration", "{error}");
    assert_eq!(failure["stage"], "configuration", "{error}");
    assert_eq!(failure["next_action"], "check_configuration", "{error}");
    assert_eq!(
        failure["configuration_rule"], "approval_clock_format_retired",
        "{error}"
    );
    assert!(failure.get("clock_refusal").is_none(), "{error}");
    // Never read as a one-source clock: the source is never asked.
    assert_eq!(source.requests.load(std::sync::atomic::Ordering::SeqCst), 0);
    // Any other format string still refuses the whole configuration.
    let config_path = root.path().join("config/config.toml");
    let text = fs::read_to_string(&config_path).unwrap();
    fs::write(
        &config_path,
        text.replace("roughtime-clock/1", "roughtime-clock/2"),
    )
    .unwrap();
    let listed = command(&root, &["adapters", "list"]);
    assert_eq!(listed.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&listed.stderr).contains("invalid_configuration"));
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

/// `setup init` and `setup check` report the operating system the binary was
/// built for, and `setup check` the custody binding fixed with it
/// (`connectors.cli.CustodyBackend`, custody semantics §5.1).
#[test]
fn setup_reports_the_build_platform_and_its_custody_backend() {
    let (os, backend) = if cfg!(target_os = "linux") {
        ("linux", "secret_service")
    } else if cfg!(target_os = "macos") {
        ("macos", "macos_keychain")
    } else {
        ("windows", "windows_credential_manager")
    };
    let root = tempfile::tempdir().unwrap();
    assert_eq!(success(&command(&root, &["setup", "init"]))["os"], os);
    let check = success(&command(&root, &["setup", "check"]));
    assert_eq!(check["os"], os);
    assert_eq!(check["custody_backend"], backend);
}

#[test]
fn setup_init_admits_an_adapter_entry_that_selects_its_private_protocol() {
    let root = tempfile::tempdir().unwrap();
    success(&command(&root, &["setup", "init"]));
    let config_path = root.path().join("config/config.toml");
    let config = fs::read_to_string(&config_path).unwrap()
        + &format!(
            "\n[adapters.forge]\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[adapters.forge.executable]\npath='{NOT_INSTALLED}/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n",
            "a".repeat(64)
        );
    fs::write(&config_path, config).unwrap();
    let check = success(&command(&root, &["setup", "check"]));
    let prerequisites = check["prerequisites"].as_array().unwrap();
    assert!(
        prerequisites
            .iter()
            .any(|p| p["name"] == "configuration" && p["state"] == "ready")
    );
    assert!(prerequisites.iter().any(|p| p["name"] == "artifact:forge"));
}

fn configured_entry(root: &tempfile::TempDir, format: &str, private_protocol: &str) {
    success(&command(root, &["setup", "init"]));
    let config_path = root.path().join("config/config.toml");
    let config = fs::read_to_string(&config_path)
        .unwrap()
        .replace("connectors-local/2", format)
        + &format!(
            "\n[adapters.forge]\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\n{private_protocol}[adapters.forge.executable]\npath='{NOT_INSTALLED}/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n",
            "a".repeat(64)
        );
    fs::write(&config_path, config).unwrap();
}

/// Only the read-only `mounted_secret` binding takes `connections connect`
/// without a protected source; a platform binding refuses it by name, before
/// any owner starts, metadata opens or a credential is read.
#[test]
fn a_platform_binding_refuses_a_connect_with_no_protected_source() {
    let root = tempfile::tempdir().unwrap();
    configured_entry(
        &root,
        "connectors-local/2",
        "private_protocol='connectors-private/2'\n",
    );
    let output = command(
        &root,
        &[
            "connections",
            "connect",
            "--adapter",
            "forge",
            "--profile",
            "token",
        ],
    );
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(
        error["error"]["data"]["code"], "protected_entry_unavailable",
        "{error}"
    );
    assert_eq!(
        error["error"]["data"]["next_action"],
        "select_protected_source"
    );
}

#[test]
fn a_private_protocol_that_contradicts_the_format_is_refused_naming_format_and_entry() {
    for (format, private_protocol) in [
        (
            "connectors-local/1",
            "private_protocol='connectors-private/2'\n",
        ),
        ("connectors-local/2", ""),
    ] {
        let root = tempfile::tempdir().unwrap();
        configured_entry(&root, format, private_protocol);
        let output = command(&root, &["adapters", "list"]);
        assert_eq!(output.status.code(), Some(2), "{format}");
        assert!(output.stdout.is_empty());
        let refusal: Value = serde_json::from_slice(&output.stderr).unwrap();
        // Exactly the declared operator-written coordinates: no parser text.
        assert_eq!(
            refusal,
            serde_json::json!({"ok":false,"error":{"code":"failure","data":{
                "kind":"usage","code":"invalid_configuration","stage":"configuration",
                "next_action":"check_configuration",
                "configuration_format":format,"instance_id":"forge-local"}}}),
            "{format}"
        );
    }
    // Any other invalid entry keeps the uncoordinated refusal.
    let root = tempfile::tempdir().unwrap();
    configured_entry(
        &root,
        "connectors-local/2",
        "private_protocol='connectors-private/2'\nunreviewed='sentinel'\n",
    );
    let output = command(&root, &["adapters", "list"]);
    assert_eq!(output.status.code(), Some(2));
    let refusal: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(
        refusal["error"]["data"],
        serde_json::json!({"kind":"usage","code":"invalid_configuration",
            "stage":"configuration","next_action":"check_configuration"})
    );
}

#[test]
fn inventory_and_unavailable_status_never_launch_configured_executable() {
    let root = tempfile::tempdir().unwrap();
    success(&command(&root, &["setup", "init"]));
    let config_path = root.path().join("config/config.toml");
    let config = fs::read_to_string(&config_path).unwrap()
        + &format!(
            "\n[adapters.forge]\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/1'\n[adapters.forge.executable]\npath='{NOT_INSTALLED}/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n",
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
    // Unix only: a configuration readable by group and other is refused.
    #[cfg(unix)]
    {
        fs::set_permissions(&config_path, fs::Permissions::from_mode(0o644)).unwrap();
        let output = command(&root, &["adapters", "list"]);
        assert_eq!(output.status.code(), Some(2));
        assert!(!String::from_utf8_lossy(&output.stderr).contains("config.toml"));
    }
}

/// The test's own owner child; a failing assertion must not leave it serving.
#[cfg(unix)]
struct OwnerProcess(std::process::Child);
#[cfg(unix)]
impl Drop for OwnerProcess {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

/// Starts `__connectors-owner` from `binary` the way the CLI does, with the
/// startup channel on descriptor 3 and the lifetime lock on descriptor 4.
#[cfg(unix)]
fn start_owner(binary: &std::path::Path, root: &tempfile::TempDir) -> OwnerProcess {
    start_owner_with(binary, root, None)
}

/// As [`start_owner`], taking the lifetime lock the way the CLI does: the lock
/// file may already exist, and the start refuses unless no owner holds it.
/// `idle_ms` shortens the owner's idle exit bound (honoured by debug builds only).
#[cfg(unix)]
fn start_owner_with(
    binary: &std::path::Path,
    root: &tempfile::TempDir,
    idle_ms: Option<u64>,
) -> OwnerProcess {
    start_owner_in(binary, root, root, idle_ms)
}

/// As [`start_owner_with`], reading the configuration of `config_root` and
/// serving the state directory of `root`.
#[cfg(unix)]
fn start_owner_in(
    binary: &std::path::Path,
    config_root: &tempfile::TempDir,
    root: &tempfile::TempDir,
    idle_ms: Option<u64>,
) -> OwnerProcess {
    use std::os::{
        fd::AsRawFd,
        unix::{fs::OpenOptionsExt, net::UnixStream},
    };
    let state = root.path().join("state");
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .mode(0o600)
        .open(state.join("owner.lock"))
        .unwrap();
    // flock(2); this test crate has no libc dependency.
    unsafe extern "C" {
        fn flock(fd: i32, operation: i32) -> i32;
    }
    const LOCK_EX: i32 = 2;
    const LOCK_NB: i32 = 4;
    // SAFETY: a live descriptor owned by `lock`.
    let held = unsafe { flock(lock.as_raw_fd(), LOCK_EX | LOCK_NB) };
    assert_eq!(held, 0, "another owner still holds the lifetime lock");
    let (startup, owner_end) = UnixStream::pair().unwrap();
    let mut owner = Command::new("/bin/sh");
    if let Some(idle_ms) = idle_ms {
        owner.env("CONNECTORS_TEST_OWNER_IDLE_MS", idle_ms.to_string());
    }
    let mut owner = owner
        .arg("-c")
        .arg(r#"exec "$0" __connectors-owner "$1" "$2" 3<&0 4<&1 </dev/null >/dev/null"#)
        .arg(binary)
        .arg(config_root.path().join("config/config.toml"))
        .arg(&state)
        .stdin(std::process::Stdio::from(std::os::fd::OwnedFd::from(
            owner_end,
        )))
        .stdout(std::process::Stdio::from(lock))
        .stderr(owner_start::stderr(root.path()))
        .spawn()
        .unwrap();
    // Nobody greets on the startup channel; the owner keeps serving its socket.
    drop(startup);
    owner_start::wait_listening(&mut owner, root.path());
    OwnerProcess(owner)
}

/// A second build of the same CLI: identical behaviour, different executable digest.
#[cfg(unix)]
fn other_build(root: &tempfile::TempDir) -> std::path::PathBuf {
    let directory = root.path().join("other-build");
    fs::create_dir(&directory).unwrap();
    let binary = directory.join("connectors");
    let mut bytes = fs::read(env!("CARGO_BIN_EXE_connectors")).unwrap();
    bytes.extend_from_slice(b"another build");
    fs::write(&binary, bytes).unwrap();
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).unwrap();
    binary
}

#[cfg(unix)]
#[test]
fn a_cli_refuses_an_owner_running_a_different_build_and_leaves_it_running() {
    let root = tempfile::tempdir().unwrap();
    success(&command(&root, &["setup", "init"]));
    let config_path = root.path().join("config/config.toml");
    let config = fs::read_to_string(&config_path).unwrap()
        + &format!(
            "\n[adapters.forge]\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/1'\n[adapters.forge.executable]\npath='{NOT_INSTALLED}/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n",
            "a".repeat(64)
        );
    fs::write(&config_path, config).unwrap();
    let status = ["adapters", "status", "--adapter", "forge"];
    let started = std::path::Path::new(env!("CARGO_BIN_EXE_connectors"));
    let mut owner = start_owner(started, &root);

    let same = success(&command(&root, &status));
    assert_ne!(same["observation"]["state"], "owner_unavailable", "{same}");

    let other = other_build(&root);
    let refused = command_from(&other, &root, &status);
    assert_eq!(
        refused.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&refused.stdout)
    );
    assert!(refused.stdout.is_empty());
    let error: Value = serde_json::from_slice(&refused.stderr).unwrap();
    assert_eq!(error["error"]["data"]["kind"], "operational", "{error}");
    assert_eq!(error["error"]["data"]["code"], "owner_build_mismatch");
    assert_eq!(error["error"]["data"]["stage"], "readiness");
    assert_eq!(error["error"]["data"]["next_action"], "stop_owner");

    // The CLI never stops or replaces the owner itself.
    assert!(owner.0.try_wait().unwrap().is_none());
    success(&command(&root, &status));

    let paths = connectors_host::local::config::Paths::resolve(
        Some(&config_path),
        Some(&root.path().join("state")),
    )
    .unwrap();
    let client = connectors_host::local::owner::Client::connect(&paths, false).unwrap();
    let host = client.host_incarnation.clone();
    client.shutdown(&host).unwrap();
    assert!(owner.0.wait().unwrap().success());
}

/// story:owner-stop-after-upgrade: an owner another build started is refused
/// with `stop_owner`; `setup owner-stop` from this build stops it, the owner
/// exits cleanly, and a fresh owner of this build then serves the next command.
#[cfg(unix)]
#[test]
fn setup_owner_stop_stops_an_owner_another_build_started_and_a_fresh_owner_serves() {
    let root = adversary_owner_root();
    let state = root.path().join("state");
    let status = ["adapters", "status", "--adapter", "forge"];
    let older = other_build(&root);
    let mut owner = start_owner(&older, &root);
    let refused = command(&root, &status);
    let error: Value = serde_json::from_slice(&refused.stderr).unwrap();
    assert_eq!(
        error["error"]["data"]["code"], "owner_build_mismatch",
        "{error}"
    );
    assert_eq!(error["error"]["data"]["next_action"], "stop_owner");
    let old_host =
        success(&command_from(&older, &root, &status))["observation"]["host_incarnation"].clone();
    assert!(old_host.is_string(), "{old_host}");

    let stopped = success(&command(&root, &["setup", "owner-stop"]));
    assert_eq!(stopped["disposition"], "stopped", "{stopped}");
    assert_eq!(stopped["host_incarnation"], old_host, "{stopped}");
    assert_eq!(
        stopped["state_path"].as_str().map(std::path::PathBuf::from),
        Some(state.clone()),
        "{stopped}"
    );
    let exit = owner_exit(&mut owner, std::time::Duration::from_secs(10))
        .expect("the stopped owner is still running");
    assert!(
        exit.success(),
        "the owner drained and exited cleanly: {exit}"
    );
    assert!(!state.join("owner.sock").exists(), "socket left behind");

    // The lifetime lock is free: this build's owner starts and serves.
    let _fresh = start_owner(
        std::path::Path::new(env!("CARGO_BIN_EXE_connectors")),
        &root,
    );
    let served = success(&command(&root, &status));
    assert_ne!(
        served["observation"]["state"], "owner_unavailable",
        "{served}"
    );
    assert!(served["observation"]["host_incarnation"].is_string());
    assert_ne!(served["observation"]["host_incarnation"], old_host);
    let again = success(&command(&root, &["setup", "owner-stop"]));
    assert_eq!(again["disposition"], "stopped", "{again}");
    assert_eq!(
        again["host_incarnation"],
        served["observation"]["host_incarnation"]
    );
}

/// No owner runs: a state directory where none ever started, one where an owner
/// came and went, and one that does not exist each answer `no_owner`, exit 0,
/// and nothing is started or created.
#[cfg(unix)]
#[test]
fn setup_owner_stop_without_an_owner_answers_no_owner_and_starts_nothing() {
    let root = adversary_owner_root();
    let state = root.path().join("state");
    let stop = |root: &tempfile::TempDir| {
        let output = command(root, &["setup", "owner-stop"]);
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        assert!(output.stderr.is_empty(), "{output:?}");
        success(&output)
    };
    let never = stop(&root);
    assert_eq!(never["disposition"], "no_owner", "{never}");
    assert!(never.get("host_incarnation").is_none(), "{never}");
    assert!(
        !state.join("owner.lock").exists(),
        "a lock file was created"
    );
    assert!(!state.join("owner.sock").exists(), "an owner was started");

    let mut owner = start_owner(
        std::path::Path::new(env!("CARGO_BIN_EXE_connectors")),
        &root,
    );
    shutdown_owner(&root);
    owner_exit(&mut owner, std::time::Duration::from_secs(10)).expect("owner still running");
    assert!(state.join("owner.lock").exists());
    let gone = stop(&root);
    assert_eq!(gone["disposition"], "no_owner", "{gone}");
    assert!(!state.join("owner.sock").exists(), "an owner was started");

    let missing = tempfile::tempdir().unwrap();
    success(&command(&missing, &["setup", "init"]));
    fs::remove_dir_all(missing.path().join("state")).unwrap();
    let absent = stop(&missing);
    assert_eq!(absent["disposition"], "no_owner", "{absent}");
    assert!(
        !missing.path().join("state").exists(),
        "the state directory was created"
    );
}

/// A process that holds the lifetime lock but never answers, with no socket or
/// with a socket nobody serves, is refused by name at the bound and is never
/// signalled.
#[cfg(unix)]
#[test]
fn setup_owner_stop_refuses_an_owner_that_does_not_answer_and_never_signals_it() {
    use std::os::{fd::AsRawFd, unix::fs::OpenOptionsExt};
    let root = adversary_owner_root();
    let state = root.path().join("state");
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .mode(0o600)
        .open(state.join("owner.lock"))
        .unwrap();
    unsafe extern "C" {
        fn flock(fd: i32, operation: i32) -> i32;
    }
    // SAFETY: a live descriptor owned by `lock`; LOCK_EX | LOCK_NB.
    assert_eq!(unsafe { flock(lock.as_raw_fd(), 2 | 4) }, 0);
    // The holder is a process that is not an owner; it keeps the lock through
    // its inherited descriptor.
    let mut holder = OwnerProcess(
        Command::new("/bin/sleep")
            .arg("120")
            .stdout(std::process::Stdio::from(lock))
            .spawn()
            .unwrap(),
    );
    let stop = || {
        Command::new(env!("CARGO_BIN_EXE_connectors"))
            .env("CONNECTORS_TEST_OWNER_STOP_MS", "1500")
            .args(["--output", "json", "--config"])
            .arg(root.path().join("config/config.toml"))
            .arg("--state-dir")
            .arg(&state)
            .args(["setup", "owner-stop"])
            .output()
            .unwrap()
    };
    let refused = |output: Output, case: &str| {
        assert_eq!(output.status.code(), Some(1), "{case}: {output:?}");
        assert!(output.stdout.is_empty(), "{case}: {output:?}");
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        let data = &error["error"]["data"];
        assert_eq!(
            (&data["code"], &data["stage"], &data["next_action"]),
            (
                &Value::from("unavailable"),
                &Value::from("stop"),
                &Value::from("retry_explicitly")
            ),
            "{case}: {error}"
        );
    };
    refused(stop(), "no socket");
    let _silent = sockets::bind(&state, "owner.sock");
    fs::set_permissions(state.join("owner.sock"), fs::Permissions::from_mode(0o600)).unwrap();
    refused(stop(), "a socket nobody serves");
    assert!(
        holder.0.try_wait().unwrap().is_none(),
        "the lock holder was signalled"
    );
}

/// Config with one adapter whose executable is never launched by `adapters status`.
#[cfg(unix)]
fn adversary_owner_root() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    success(&command(&root, &["setup", "init"]));
    let config_path = root.path().join("config/config.toml");
    let config = fs::read_to_string(&config_path).unwrap()
        + &format!(
            "\n[adapters.forge]\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/1'\n[adapters.forge.executable]\npath='{NOT_INSTALLED}/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n",
            "a".repeat(64)
        );
    fs::write(&config_path, config).unwrap();
    root
}

/// A second root whose state directory is a copy of `root`'s, taken before any
/// owner runs: an owner started there with `root`'s configuration answers a CLI
/// of `root` (same authority, same configuration path) while `root`'s socket
/// path stays free for a relay.
#[cfg(unix)]
fn adversary2_owner_root(root: &tempfile::TempDir) -> tempfile::TempDir {
    use std::os::unix::fs::DirBuilderExt;
    fn copy(from: &std::path::Path, to: &std::path::Path) {
        fs::DirBuilder::new().mode(0o700).create(to).unwrap();
        for entry in fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let target = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy(&entry.path(), &target);
            } else {
                fs::copy(entry.path(), target).unwrap();
            }
        }
    }
    let owner_root = tempfile::tempdir().unwrap();
    copy(&root.path().join("state"), &owner_root.path().join("state"));
    owner_root
}

/// Owner-channel frame: three big-endian u32 section sizes, then the sections.
#[cfg(unix)]
fn adversary_frame(control: &Value) -> Vec<u8> {
    let bytes = serde_json::to_vec(control).unwrap();
    let mut frame = Vec::new();
    frame.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    frame.extend_from_slice(&0u32.to_be_bytes());
    frame.extend_from_slice(&0u32.to_be_bytes());
    frame.extend_from_slice(&bytes);
    frame
}

/// Fills the owner's 32 client slots at `socket` and returns the held clients
/// once the owner itself shows it holds all of them: a probe connection
/// answered with its `capacity` refusal (transport.rs `handle`). Connecting
/// only queues a client; the owner counts it when its accept loop takes it,
/// which under load can be long after the connect returned. A probe the owner
/// takes into a free slot instead gets no reply; it is held too and the next
/// probe asks again. The owner takes clients in order, so a refused probe
/// means every earlier one was taken. The short read timeout only paces the
/// probes, and the deadline only ends a test whose owner never fills; neither
/// decides the result. Each held client keeps its slot until the owner's
/// 10-second greeting deadline from when it was taken.
#[cfg(unix)]
fn fill_owner(directory: &sockets::Directory, socket: &str) -> Vec<std::os::unix::net::UnixStream> {
    use std::io::Read;
    let mut held: Vec<_> = (0..32).map(|_| directory.connect(socket)).collect();
    let fail_safe = std::time::Instant::now() + std::time::Duration::from_secs(120);
    loop {
        assert!(
            std::time::Instant::now() < fail_safe,
            "the owner never refused a client at capacity ({} held)",
            held.len()
        );
        let mut probe = directory.connect(socket);
        // macOS refuses a socket option with EINVAL once the owner has answered
        // and closed the probe; its reply is then already readable.
        tolerate_closed(probe.set_read_timeout(Some(std::time::Duration::from_millis(100))));
        let mut first = [0u8; 1];
        match probe.read(&mut first) {
            Ok(1) => {
                tolerate_closed(probe.set_read_timeout(None));
                let mut rest = [0u8; 11];
                probe.read_exact(&mut rest).unwrap();
                let mut sizes = [0u8; 12];
                sizes[0] = first[0];
                sizes[1..].copy_from_slice(&rest);
                let size =
                    |at: usize| u32::from_be_bytes(sizes[at..at + 4].try_into().unwrap()) as usize;
                let mut control = vec![0; size(0)];
                probe.read_exact(&mut control).unwrap();
                let reply: Value = serde_json::from_slice(&control).unwrap();
                assert_eq!(
                    (reply["kind"].as_str(), reply["error"]["code"].as_str()),
                    (Some("failed"), Some("capacity")),
                    "{reply}"
                );
                return held;
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                held.push(probe);
            }
            other => panic!("the owner closed a probe without its capacity reply: {other:?}"),
        }
    }
}

/// A socket option set on a probe the owner already closed: EINVAL on macOS.
#[cfg(unix)]
fn tolerate_closed(result: std::io::Result<()>) {
    if let Err(error) = result {
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput, "{error}");
    }
}

#[cfg(unix)]
fn adversary_read_control(stream: &mut std::os::unix::net::UnixStream) -> Value {
    use std::io::Read;
    let mut sizes = [0u8; 12];
    stream.read_exact(&mut sizes).unwrap();
    let size = |at: usize| u32::from_be_bytes(sizes[at..at + 4].try_into().unwrap()) as usize;
    let mut control = vec![0; size(0)];
    stream.read_exact(&mut control).unwrap();
    let mut rest = vec![0; size(4) + size(8)];
    stream.read_exact(&mut rest).unwrap();
    serde_json::from_slice(&control).unwrap()
}

/// Adversary: an owner of the SAME build that refuses one greeting transiently
/// (its `handle` drops a stream without a reply once 32 clients are in flight,
/// transport.rs `handle`) must not be reported as another build. The proxy below
/// drops the first connection exactly as that capacity refusal does and relays
/// every later one to the real owner.
#[cfg(unix)]
#[test]
fn adversary_a_transient_greeting_refusal_from_the_same_build_is_not_a_build_mismatch() {
    use std::os::unix::net::UnixStream;
    let root = adversary_owner_root();
    let owner_root = adversary2_owner_root(&root);
    let started = std::path::Path::new(env!("CARGO_BIN_EXE_connectors"));
    let _owner = start_owner_in(started, &root, &owner_root, None);
    let state = root.path().join("state");
    let directory = sockets::Directory::open(&state);
    let owner_directory = sockets::Directory::open(&owner_root.path().join("state"));
    let listener = directory.bind("owner.sock");
    fs::set_permissions(state.join("owner.sock"), fs::Permissions::from_mode(0o600)).unwrap();
    let real = owner_directory.path("owner.sock");
    std::thread::spawn(move || {
        // `real` names the socket through this descriptor.
        let _directory = (&directory, &owner_directory);
        let mut first = true;
        for stream in listener.incoming() {
            let Ok(client) = stream else { return };
            if first {
                first = false;
                drop(client);
                continue;
            }
            let Ok(upstream) = UnixStream::connect(&real) else {
                return;
            };
            let (mut a, mut b) = (client.try_clone().unwrap(), upstream.try_clone().unwrap());
            std::thread::spawn(move || {
                let _ = std::io::copy(&mut a, &mut b);
                let _ = b.shutdown(std::net::Shutdown::Write);
            });
            let (mut a, mut b) = (upstream, client);
            std::thread::spawn(move || {
                let _ = std::io::copy(&mut a, &mut b);
                let _ = b.shutdown(std::net::Shutdown::Write);
            });
        }
    });

    let output = command(&root, &["adapters", "status", "--adapter", "forge"]);
    let code = serde_json::from_slice::<Value>(&output.stderr)
        .ok()
        .map(|error| error["error"]["data"]["code"].clone());
    assert_ne!(
        code,
        Some(Value::from("owner_build_mismatch")),
        "a same-build owner was reported as another build after one refused greeting: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Adversary: owner.md promises "A caller that sends no `build` gets none back,
/// so earlier callers still read the reply" and "the owner's reply returns its
/// own" digest. Drives the real owner with raw greetings from both kinds of caller.
#[cfg(unix)]
#[test]
fn adversary_owner_answers_build_only_when_asked_and_answers_its_own_digest() {
    use std::io::Write;
    let root = adversary_owner_root();
    let binary = std::path::Path::new(env!("CARGO_BIN_EXE_connectors"));
    let _owner = start_owner(binary, &root);
    let state = root.path().join("state");
    // The authority is not public API; read it with the system sqlite3 (read-only).
    let authority = Command::new("sqlite3")
        .arg("-readonly")
        .arg(state.join("metadata.sqlite3"))
        .arg("SELECT authority_id FROM local_authority WHERE singleton=1")
        .output()
        .unwrap();
    let authority = String::from_utf8(authority.stdout)
        .unwrap()
        .trim()
        .to_owned();
    assert_eq!(authority.len(), 36, "authority {authority:?}");
    let expected = ring::digest::digest(&ring::digest::SHA256, &fs::read(binary).unwrap())
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let greet = |build: Option<&str>| {
        let mut stream = sockets::connect(&state, "owner.sock");
        let mut hello = serde_json::json!({
            "kind": "hello", "version": "connectors-owner/1",
            "challenge": "00000000-0000-4000-8000-000000000002",
            "configuration": root.path().join("config/config.toml"), "authority": authority
        });
        if let Some(build) = build {
            hello["build"] = Value::from(build);
        }
        stream.write_all(&adversary_frame(&hello)).unwrap();
        adversary_read_control(&mut stream)
    };
    let earlier = greet(None);
    assert_eq!(earlier["kind"], "hello", "{earlier}");
    assert!(earlier.get("build").is_none(), "{earlier}");
    let current = greet(Some(&"0".repeat(64)));
    assert_eq!(current["build"], Value::from(expected), "{current}");
}

/// The owner waits for a greeting that arrives after the connection does. Its
/// listener is non-blocking, and a macOS `accept` hands that on to the stream
/// it returns (Linux's does not); an owner reading such a stream found nothing
/// yet, ended the read as a timeout and closed the stream without a reply. A
/// same-build CLI then saw a silent close and named the owner another build
/// or unavailable.
#[cfg(unix)]
#[test]
fn an_owner_answers_a_greeting_sent_after_the_connection_settled() {
    use std::io::Write;
    let root = adversary_owner_root();
    let binary = std::path::Path::new(env!("CARGO_BIN_EXE_connectors"));
    let _owner = start_owner(binary, &root);
    let state = root.path().join("state");
    let authority = Command::new("sqlite3")
        .arg("-readonly")
        .arg(state.join("metadata.sqlite3"))
        .arg("SELECT authority_id FROM local_authority WHERE singleton=1")
        .output()
        .unwrap();
    let authority = String::from_utf8(authority.stdout)
        .unwrap()
        .trim()
        .to_owned();
    assert_eq!(authority.len(), 36, "authority {authority:?}");
    let mut stream = sockets::connect(&state, "owner.sock");
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(10)))
        .unwrap();
    // Long past the owner's accept, short of its 10 s greeting deadline.
    std::thread::sleep(std::time::Duration::from_millis(300));
    let hello = serde_json::json!({
        "kind": "hello", "version": "connectors-owner/1",
        "challenge": "00000000-0000-4000-8000-000000000003",
        "configuration": root.path().join("config/config.toml"), "authority": authority,
        "build": "0".repeat(64)
    });
    stream
        .write_all(&adversary_frame(&hello))
        .unwrap_or_else(|error| panic!("the owner closed the stream before the greeting: {error}"));
    let reply = adversary_read_control(&mut stream);
    assert_eq!(reply["kind"], "hello", "{reply}");
}

#[cfg(unix)]
#[test]
fn a_same_build_owner_at_its_client_limit_answers_capacity_not_another_build() {
    let root = tempfile::tempdir().unwrap();
    success(&command(&root, &["setup", "init"]));
    let config_path = root.path().join("config/config.toml");
    let config = fs::read_to_string(&config_path).unwrap()
        + &format!(
            "\n[adapters.forge]\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/1'\n[adapters.forge.executable]\npath='{NOT_INSTALLED}/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n",
            "a".repeat(64)
        );
    fs::write(&config_path, config).unwrap();
    let started = std::path::Path::new(env!("CARGO_BIN_EXE_connectors"));
    let _owner = start_owner(started, &root);
    let directory = sockets::Directory::open(&root.path().join("state"));
    // 32 silent clients occupy every slot until the owner's greeting timeout.
    let held = fill_owner(&directory, "owner.sock");
    let refused = command(&root, &["adapters", "status", "--adapter", "forge"]);
    assert_eq!(refused.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&refused.stderr).unwrap();
    assert_eq!(error["error"]["data"]["code"], "capacity", "{error}");
    drop(held);
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

/// Adversary pass 2: a relay in front of the real owner's socket. The owner runs
/// in `owner_root`'s state directory and the relay listens at `root`'s, so no
/// owner's socket is replaced (an owner exits once its socket path names another
/// socket). `drop_first` decides, from the first frame's control JSON, whether this connection is
/// closed without a reply; every other connection is relayed byte for byte.
/// Returns the number of connections relayed to the real owner.
#[cfg(unix)]
fn adversary2_relay(
    root: &tempfile::TempDir,
    owner_root: &tempfile::TempDir,
    drop_first: impl Fn(usize, &Value) -> bool + Send + 'static,
) -> std::sync::Arc<std::sync::atomic::AtomicUsize> {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;
    let state = root.path().join("state");
    let directory = sockets::Directory::open(&state);
    let owner_directory = sockets::Directory::open(&owner_root.path().join("state"));
    let listener = directory.bind("owner.sock");
    fs::set_permissions(state.join("owner.sock"), fs::Permissions::from_mode(0o600)).unwrap();
    let real = owner_directory.path("owner.sock");
    let relayed = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = relayed.clone();
    std::thread::spawn(move || {
        // `real` names the socket through this descriptor.
        let _directory = (&directory, &owner_directory);
        for (index, stream) in listener.incoming().enumerate() {
            let Ok(mut client) = stream else { return };
            let mut sizes = [0u8; 12];
            if client.read_exact(&mut sizes).is_err() {
                continue;
            }
            let size =
                |at: usize| u32::from_be_bytes(sizes[at..at + 4].try_into().unwrap()) as usize;
            let mut rest = vec![0; size(0) + size(4) + size(8)];
            if client.read_exact(&mut rest).is_err() {
                continue;
            }
            let control: Value = serde_json::from_slice(&rest[..size(0)]).unwrap_or(Value::Null);
            if drop_first(index, &control) {
                drop(client);
                continue;
            }
            let Ok(mut upstream) = UnixStream::connect(&real) else {
                return;
            };
            counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if upstream.write_all(&sizes).is_err() || upstream.write_all(&rest).is_err() {
                continue;
            }
            let (mut a, mut b) = (client.try_clone().unwrap(), upstream.try_clone().unwrap());
            std::thread::spawn(move || {
                let _ = std::io::copy(&mut a, &mut b);
                let _ = b.shutdown(std::net::Shutdown::Write);
            });
            let (mut a, mut b) = (upstream, client);
            std::thread::spawn(move || {
                let _ = std::io::copy(&mut a, &mut b);
                let _ = b.shutdown(std::net::Shutdown::Write);
            });
        }
    });
    relayed
}

#[cfg(unix)]
fn adversary2_code(output: &Output) -> Value {
    serde_json::from_slice::<Value>(&output.stderr)
        .map(|error| error["error"]["data"]["code"].clone())
        .unwrap_or_else(|_| {
            let result: Value = serde_json::from_slice(&output.stdout).unwrap_or(Value::Null);
            Value::from(format!(
                "ok:{}",
                result["result"]["observation"]["state"]
                    .as_str()
                    .unwrap_or("?")
            ))
        })
}

/// Adversary pass 2, the pass-1 measurement against the corrected binary: 32
/// idle clients fill the real owner, the CLI of the SAME build asks for status,
/// and one held client is released at a seeded random point. No run may report
/// the owner as another build.
#[cfg(unix)]
#[test]
fn adversary2_a_same_build_owner_at_capacity_is_never_another_build_over_100_runs() {
    let root = adversary_owner_root();
    let binary = std::path::Path::new(env!("CARGO_BIN_EXE_connectors"));
    let mut owner = start_owner(binary, &root);
    let directory = sockets::Directory::open(&root.path().join("state"));
    let mut seed: u64 = 0x5eed_2026_0929;
    let mut tally = std::collections::BTreeMap::<String, usize>::new();
    for _ in 0..100 {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let delay = (seed >> 33) % 300;
        let mut held: Vec<_> = (0..32).map(|_| directory.connect("owner.sock")).collect();
        let child = Command::new(binary)
            .args(["--output", "json", "--config"])
            .arg(root.path().join("config/config.toml"))
            .arg("--state-dir")
            .arg(root.path().join("state"))
            .args(["adapters", "status", "--adapter", "forge"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(delay));
        held.pop();
        let output = child.wait_with_output().unwrap();
        *tally
            .entry(adversary2_code(&output).to_string())
            .or_default() += 1;
        drop(held);
        std::thread::sleep(std::time::Duration::from_millis(150));
    }
    eprintln!("adversary2 tally over 100 runs: {tally:?}");
    assert!(owner.0.try_wait().unwrap().is_none(), "owner died");
    assert_eq!(
        tally.get("\"owner_build_mismatch\""),
        None,
        "a same-build owner at capacity was reported as another build: {tally:?}"
    );
}

/// Adversary pass 2: the new `build` probe sent to a same-build owner that is at
/// its client limit. The first (build) greeting is closed silently in front of
/// the owner; the probe greeting then reaches the real owner holding 32 clients.
/// The CLI must say `capacity`, not another build and not `unavailable`.
#[cfg(unix)]
#[test]
fn adversary2_the_build_probe_against_an_owner_at_capacity_reports_capacity() {
    let root = adversary_owner_root();
    let owner_root = adversary2_owner_root(&root);
    let binary = std::path::Path::new(env!("CARGO_BIN_EXE_connectors"));
    let _owner = start_owner_in(binary, &root, &owner_root, None);
    let _relayed = adversary2_relay(&root, &owner_root, |index, _| index == 0);
    let directory = sockets::Directory::open(&owner_root.path().join("state"));
    let held = fill_owner(&directory, "owner.sock");
    let filled = std::time::Instant::now();
    let output = command(&root, &["adapters", "status", "--adapter", "forge"]);
    // A slot frees only when a held client reaches the owner's 10-second
    // greeting deadline; then the probe and the greeting are served and the
    // status answers `ok:not_running`. The elapsed time says which it was.
    assert_eq!(
        adversary2_code(&output),
        Value::from("capacity"),
        "{:?} after the owner was full: {}",
        filled.elapsed(),
        String::from_utf8_lossy(&output.stderr)
    );
    drop(held);
}

/// Adversary pass 2: deadline handling in the identification loop. An owner of
/// the same build that answers the build-less greeting and the `build` probe but
/// never answers a greeting carrying `build` sends the CLI round the loop. The
/// CLI must stop at its 10-second connect deadline, must not report another
/// build, and must not open connections without bound while it waits.
#[cfg(unix)]
#[test]
fn adversary2_the_identification_retry_loop_ends_at_its_deadline_without_flooding_the_owner() {
    let root = adversary_owner_root();
    let owner_root = adversary2_owner_root(&root);
    let binary = std::path::Path::new(env!("CARGO_BIN_EXE_connectors"));
    let mut owner = start_owner_in(binary, &root, &owner_root, None);
    let relayed = adversary2_relay(&root, &owner_root, |_, control| {
        control.get("build").is_some()
    });
    let started = std::time::Instant::now();
    let output = command(&root, &["adapters", "status", "--adapter", "forge"]);
    let elapsed = started.elapsed();
    let relayed = relayed.load(std::sync::atomic::Ordering::SeqCst);
    eprintln!(
        "adversary2 loop: {elapsed:?}, {relayed} connections relayed to the owner, code {}",
        adversary2_code(&output)
    );
    assert_ne!(
        adversary2_code(&output),
        Value::from("owner_build_mismatch")
    );
    assert!(elapsed < std::time::Duration::from_secs(12), "{elapsed:?}");
    assert!(owner.0.try_wait().unwrap().is_none(), "owner died");
    assert!(
        relayed <= 100,
        "the identification loop opened {relayed} owner connections in {elapsed:?} with no pause between rounds"
    );
}

/// Adversary pass 2 (ignored; needs a released pre-handshake binary, e.g.
/// `CONNECTORS_ADVERSARY_PRE_HANDSHAKE=$HOME/.cargo/bin/connectors` at 0.15.1):
/// a REAL earlier owner, which answers neither `build` nor the `build` request,
/// is refused by name, left running, and can still be shut down. Then the
/// reverse: that earlier CLI against an owner of this build.
#[cfg(unix)]
#[test]
#[ignore]
fn adversary2_a_real_pre_handshake_owner_is_refused_by_name_and_the_reverse_is_observed() {
    let Some(old) = std::env::var_os("CONNECTORS_ADVERSARY_PRE_HANDSHAKE") else {
        panic!("set CONNECTORS_ADVERSARY_PRE_HANDSHAKE to a pre-handshake connectors binary");
    };
    let old = std::path::PathBuf::from(old);
    let status = ["adapters", "status", "--adapter", "forge"];
    let root = adversary_owner_root();
    let mut owner = start_owner(&old, &root);
    let refused = command(&root, &status);
    assert_eq!(
        adversary2_code(&refused),
        Value::from("owner_build_mismatch"),
        "{}",
        String::from_utf8_lossy(&refused.stderr)
    );
    assert!(owner.0.try_wait().unwrap().is_none());
    let paths = connectors_host::local::config::Paths::resolve(
        Some(&root.path().join("config/config.toml")),
        Some(&root.path().join("state")),
    )
    .unwrap();
    let client = connectors_host::local::owner::Client::connect(&paths, false).unwrap();
    let host = client.host_incarnation.clone();
    client.shutdown(&host).unwrap();
    assert!(owner.0.wait().unwrap().success());

    let root = adversary_owner_root();
    let binary = std::path::Path::new(env!("CARGO_BIN_EXE_connectors"));
    let _owner = start_owner(binary, &root);
    let reverse = command_from(&old, &root, &status);
    eprintln!(
        "adversary2 reverse (earlier CLI, current owner): exit {:?} stdout {} stderr {}",
        reverse.status.code(),
        String::from_utf8_lossy(&reverse.stdout),
        String::from_utf8_lossy(&reverse.stderr)
    );
}

/// Adversary pass 2, the reverse direction of the acceptance statement ("a CLI
/// whose executable digest differs from the running owner's refuses"): a CLI
/// built before the handshake (e.g. after a rollback) greets without `build`
/// and then asks for work. Only the owner can refuse it, and this owner serves
/// it. Measured with a real 0.15.1 CLI too (the ignored case above prints it).
#[cfg(unix)]
#[test]
fn an_owner_still_serves_a_caller_that_never_named_its_build() {
    use std::io::Write;
    let root = adversary_owner_root();
    let binary = std::path::Path::new(env!("CARGO_BIN_EXE_connectors"));
    let _owner = start_owner(binary, &root);
    let state = root.path().join("state");
    let authority = Command::new("sqlite3")
        .arg("-readonly")
        .arg(state.join("metadata.sqlite3"))
        .arg("SELECT authority_id FROM local_authority WHERE singleton=1")
        .output()
        .unwrap();
    let authority = String::from_utf8(authority.stdout)
        .unwrap()
        .trim()
        .to_owned();
    let mut stream = sockets::connect(&state, "owner.sock");
    let hello = serde_json::json!({
        "kind": "hello", "version": "connectors-owner/1",
        "challenge": "00000000-0000-4000-8000-000000000003",
        "configuration": root.path().join("config/config.toml"), "authority": authority
    });
    stream.write_all(&adversary_frame(&hello)).unwrap();
    let greeted = adversary_read_control(&mut stream);
    assert_eq!(greeted["kind"], "hello", "{greeted}");
    stream
        .write_all(&adversary_frame(
            &serde_json::json!({"kind": "status", "adapter": "forge"}),
        ))
        .unwrap();
    let served = adversary_read_control(&mut stream);
    assert_eq!(
        served["kind"], "success",
        "owner serves build-less callers by design until story:owner-refuses-buildless-callers decides otherwise; flip this assertion when it does: {served}"
    );
}

/// Waits for `owner` to exit on its own within `limit`; `None` if it is still running.
#[cfg(unix)]
fn owner_exit(
    owner: &mut OwnerProcess,
    limit: std::time::Duration,
) -> Option<std::process::ExitStatus> {
    let until = std::time::Instant::now() + limit;
    loop {
        if let Some(status) = owner.0.try_wait().unwrap() {
            return Some(status);
        }
        if std::time::Instant::now() >= until {
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

#[cfg(unix)]
fn shutdown_owner(root: &tempfile::TempDir) {
    let paths = connectors_host::local::config::Paths::resolve(
        Some(&root.path().join("config/config.toml")),
        Some(&root.path().join("state")),
    )
    .unwrap();
    let client = connectors_host::local::owner::Client::connect(&paths, false).unwrap();
    let host = client.host_incarnation.clone();
    client.shutdown(&host).unwrap();
}

/// story:owner-idle-exit: an owner with no client and no child work for its
/// idle bound exits cleanly, and the next start gets a new owner.
#[cfg(unix)]
#[test]
#[cfg_attr(
    not(debug_assertions),
    ignore = "the idle bound can be shortened only in a debug build"
)]
fn an_idle_owner_exits_and_the_next_start_gets_a_new_owner() {
    let root = adversary_owner_root();
    let state = root.path().join("state");
    let binary = std::path::Path::new(env!("CARGO_BIN_EXE_connectors"));
    let status = ["adapters", "status", "--adapter", "forge"];
    let mut owner = start_owner_with(binary, &root, Some(1500));
    let first = success(&command(&root, &status));
    let first_host = first["observation"]["host_incarnation"].clone();
    assert!(first_host.is_string(), "{first}");

    let exit = owner_exit(&mut owner, std::time::Duration::from_secs(20))
        .expect("an idle owner is still running 20 s after a 1.5 s idle bound");
    assert!(exit.success(), "idle exit is clean: {exit}");
    assert!(!state.join("owner.sock").exists(), "socket left behind");
    let none = success(&command(&root, &status));
    assert_eq!(none["observation"]["state"], "owner_unavailable", "{none}");

    // The CLI's own start: the lifetime lock is free and a new owner serves.
    let _next = start_owner_with(binary, &root, None);
    let second = success(&command(&root, &status));
    assert_ne!(second["observation"]["state"], "owner_unavailable");
    assert!(second["observation"]["host_incarnation"].is_string());
    assert_ne!(second["observation"]["host_incarnation"], first_host);
    shutdown_owner(&root);
}

/// A connected client keeps an owner alive across its idle bound, and the idle
/// interval restarts only once that client is gone.
#[cfg(unix)]
#[test]
#[cfg_attr(
    not(debug_assertions),
    ignore = "the idle bound can be shortened only in a debug build"
)]
fn a_connected_client_keeps_an_owner_past_its_idle_bound() {
    let root = adversary_owner_root();
    let state = root.path().join("state");
    let binary = std::path::Path::new(env!("CARGO_BIN_EXE_connectors"));
    let mut owner = start_owner_with(binary, &root, Some(1500));
    // Held without a greeting; the owner waits up to 10 s for one.
    let held = sockets::connect(&state, "owner.sock");
    assert!(
        owner_exit(&mut owner, std::time::Duration::from_secs(4)).is_none(),
        "the owner exited with a client connected"
    );
    assert!(state.join("owner.sock").exists());
    success(&command(
        &root,
        &["adapters", "status", "--adapter", "forge"],
    ));
    drop(held);
    let exit = owner_exit(&mut owner, std::time::Duration::from_secs(20))
        .expect("the owner never went idle after its client left");
    assert!(exit.success(), "idle exit is clean: {exit}");
    assert!(!state.join("owner.sock").exists());
}

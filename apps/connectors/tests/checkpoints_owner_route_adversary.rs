//! Adversary cases for story:metadata-invoke-cost-flat-in-store-size (wave
//! 20261007c, U2): CLI metadata reads routed through a running owner of the
//! same build (`Request::Cached`, owner.md "A command run while an owner of its
//! own build runs ... refuses with the codes a direct read gives"), and
//! `setup checkpoints-enable` against a real running owner and a wrong
//! confirmation value (semantics.md §3 steps 1-2).
//!
//! The oracle for every routed read is the same command run with no owner:
//! the direct read this unit kept. No adapter executable exists and none is
//! ever launched.
use connectors_host::local::{
    config::{Config, Paths},
    registry, runtime,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    process::{Command, Output},
    time::{Duration, Instant},
};

const READ: &str = "item.read";

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

/// Exit status, stdout and stderr as JSON, for comparing two runs.
fn answer(output: &Output) -> (Option<i32>, Value, Value) {
    let parse = |bytes: &[u8]| {
        if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(bytes)
                .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(bytes).into_owned()))
        }
    };
    (
        output.status.code(),
        parse(&output.stdout),
        parse(&output.stderr),
    )
}

fn bootstrap() -> runtime::Bootstrap {
    let descriptor = connectors_core::Descriptor {
        version: "v1alpha1".into(),
        instance: "forge-local".into(),
        adapter: "catalog".into(),
        revision: "desc-1".into(),
        operations: vec![connectors_core::Operation {
            id: READ.into(),
            description: "fixture operation".into(),
            contract: "operations/v1alpha1".into(),
            profile: "resource".into(),
            input_schema: json!({"type":"object"}),
            output_schema: json!({"type":"object"}),
        }],
        configuration_schema: json!({"type":"object"}),
    };
    runtime::Bootstrap {
        instance: "forge-local".into(),
        adapter: "catalog".into(),
        protocol: "v1alpha1".into(),
        configuration_revision: "cfg-1".into(),
        provider_authority: "https://fixture.invalid".into(),
        descriptor: serde_json::to_string(&descriptor).unwrap(),
        profiles: vec![runtime::Profile {
            id: "token".into(),
            revision: "profile-1".into(),
            purpose: registry::Purpose::DelegatedUser,
            subject: registry::Subject::User,
            scheme: "http_bearer".into(),
            capability: "http-bearer".into(),
            minimum_scopes: BTreeSet::new(),
            evidence_lifetime_ms: 60_000,
            fields: vec![runtime::EntryField {
                name: "token".into(),
                label: "Token".into(),
                max_bytes: 1024,
            }],
            acquisition: None,
        }],
        requirements: vec![runtime::Requirement {
            operation: READ.into(),
            profile: "token".into(),
            scopes: BTreeSet::new(),
            effect: runtime::Effect::Read,
        }],
    }
}

/// `forge` has a cached description of one granted read; `bare` has none.
fn configured() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let init = command(&root, &["setup", "init"]);
    assert!(init.status.success(), "{init:?}");
    let config_path = root.path().join("config/config.toml");
    let adapter = |alias: &str, instance: &str| {
        format!(
            "\n[adapters.{alias}]\ninstance_id='{instance}'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[adapters.{alias}.executable]\npath='/not-installed/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n[adapters.{alias}.permissions]\nprofiles=['token']\noperations=['{READ}']\n",
            "a".repeat(64)
        )
    };
    let config = fs::read_to_string(&config_path).unwrap()
        + &adapter("forge", "forge-local")
        + &adapter("bare", "bare-local");
    fs::write(&config_path, config).unwrap();
    let paths = Paths {
        config: config_path.clone(),
        state: root.path().join("state"),
    };
    let forge = Config::load(&paths.config).unwrap().adapters["forge"].clone();
    runtime::state::State::new(&paths.state)
        .remember(&forge.selection(), &bootstrap())
        .unwrap();
    root
}

/// The test's own owner child; a failing assertion must not leave it serving.
struct OwnerProcess(std::process::Child);
impl Drop for OwnerProcess {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

/// Starts `__connectors-owner` of this build the way the CLI does (startup
/// channel on fd 3, lifetime lock on fd 4), as owner_idle_exit_adversary.rs.
fn start_owner(root: &tempfile::TempDir) -> OwnerProcess {
    use std::os::unix::{fs::OpenOptionsExt, net::UnixStream};
    let state = root.path().join("state");
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .mode(0o600)
        .open(state.join("owner.lock"))
        .unwrap();
    let (startup, owner_end) = UnixStream::pair().unwrap();
    let owner = Command::new("/bin/sh")
        .env("CONNECTORS_TEST_OWNER_IDLE_MS", "120000")
        .arg("-c")
        .arg(r#"exec "$0" __connectors-owner "$1" "$2" 3<&0 4<&1 </dev/null >/dev/null"#)
        .arg(env!("CARGO_BIN_EXE_connectors"))
        .arg(root.path().join("config/config.toml"))
        .arg(&state)
        .stdin(std::process::Stdio::from(std::os::fd::OwnedFd::from(
            owner_end,
        )))
        .stdout(std::process::Stdio::from(lock))
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    drop(startup);
    let until = Instant::now() + Duration::from_secs(20);
    while !state.join("owner.sock").exists() {
        assert!(Instant::now() < until, "owner never listened");
        std::thread::sleep(Duration::from_millis(20));
    }
    OwnerProcess(owner)
}

/// Each read answers the same with an owner of this build running as with
/// none: the routed read keeps the direct read's answer and refusal codes.
#[test]
fn routed_metadata_reads_answer_as_the_direct_read_does() {
    let root = configured();
    let reads: [&[&str]; 5] = [
        &["adapters", "describe", "--adapter", "forge"],
        &["adapters", "describe", "--adapter", "bare"],
        &["operations", "list", "--adapter", "forge"],
        &["operations", "list", "--adapter", "bare"],
        &[
            "operations",
            "describe",
            "--adapter",
            "forge",
            "--operation",
            READ,
        ],
    ];
    let direct = reads
        .iter()
        .map(|args| answer(&command(&root, args)))
        .collect::<Vec<_>>();
    assert!(!root.path().join("state/owner.sock").exists());
    let mut owner = start_owner(&root);
    let routed = reads
        .iter()
        .map(|args| answer(&command(&root, args)))
        .collect::<Vec<_>>();
    assert!(
        owner.0.try_wait().unwrap().is_none(),
        "the owner exited during the routed reads"
    );
    let differing = reads
        .iter()
        .zip(direct.iter().zip(&routed))
        .filter(|(_, (direct, routed))| direct != routed)
        .map(|(args, (direct, routed))| {
            format!("{args:?}\n direct: {direct:?}\n routed: {routed:?}")
        })
        .collect::<Vec<_>>();
    assert!(differing.is_empty(), "{}", differing.join("\n"));
    drop(owner);
}

/// A real running owner, not a test-held flock, makes the enable answer
/// `lifecycle_conflict` and changes nothing.
#[test]
fn a_real_running_owner_refuses_the_enable() {
    let root = configured();
    let _owner = start_owner(&root);
    let output = command(
        &root,
        &["setup", "checkpoints-enable", "--confirm", "one-way"],
    );
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let envelope: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(
        envelope["error"]["data"]["code"], "lifecycle_conflict",
        "{envelope}"
    );
    assert_eq!(
        envelope["error"]["data"]["next_action"], "stop_owner",
        "{envelope}"
    );
}

/// semantics.md §3 step 1: "`--confirm` admits only the value `one-way`; any
/// other value is the presentation's `cli_input`" (exit 2, empty data).
#[test]
fn a_confirmation_other_than_one_way_is_cli_input() {
    let root = configured();
    let output = command(&root, &["setup", "checkpoints-enable", "--confirm", "yes"]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let envelope: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(
        envelope,
        json!({"ok":false,"error":{"code":"cli_input","data":{}}})
    );
}

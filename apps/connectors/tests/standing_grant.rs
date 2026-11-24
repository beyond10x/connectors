//! story:local-invoke-admits-a-keyed-write-under-a-standing-grant: the three
//! commands of `contracts/cli/v1alpha1/semantics.md`, "Standing grants",
//! `approvals grant-standing`, `approvals revoke-standing` and
//! `approvals list-standing`, run without a keyring. Each refusal here is
//! decided in the CLI process before any approval key is read.
// The fixture configures a Unix absolute executable path.
#![cfg(unix)]
use connectors_host::local::{
    config::{Config, Paths},
    registry, runtime,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    path::Path,
    process::{Command, Output},
};

const CONNECTOR: &str = "fixture";
const INSTANCE: &str = "main";
const ALIAS: &str = "forge";
const GRANT: &str = "7b0c4a52-55a4-4a5c-9a39-0d6b1c0e7a11";

fn command(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_connectors"))
        .args(["--output", "json", "--config"])
        .arg(root.join("config/config.toml"))
        .arg("--state-dir")
        .arg(root.join("state"))
        .args(args)
        .output()
        .unwrap()
}

/// The refusal's failure data, from whichever stream carries the envelope.
#[track_caller]
fn failure(output: &Output) -> Value {
    assert!(!output.status.success(), "{output:?}");
    let stream = if output.stdout.is_empty() {
        &output.stderr
    } else {
        &output.stdout
    };
    let envelope: Value = serde_json::from_slice(stream).unwrap();
    envelope["error"]["data"].clone()
}

/// An `[approval_clock]` table of two loopback sources nothing listens on: a
/// command that reads the clock gets no reading from it.
fn clock() -> String {
    let key = |seed: u8| {
        use base64::Engine;
        use ring::signature::KeyPair;
        let pair = ring::signature::Ed25519KeyPair::from_seed_unchecked(&[seed; 32]).unwrap();
        base64::engine::general_purpose::STANDARD.encode(pair.public_key().as_ref())
    };
    format!(
        "[approval_clock]\nformat='roughtime-quorum/1'\nquorum=2\nmax_rate_error_ppm=10000\n[[approval_clock.sources]]\naddress='127.0.0.1:9'\npublic_key='{}'\nversion='roughtime-0x8000000c'\n[[approval_clock.sources]]\naddress='127.0.0.1:10'\npublic_key='{}'\nversion='roughtime-0x8000000c'\n",
        key(7),
        key(8)
    )
}

/// One `connectors-local/5` instance, `fixture/main`, alias `forge`, with an
/// approval clock; `member` is written among the instance's scalars.
fn configured(member: &str) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let init = command(root.path(), &["setup", "init"]);
    assert!(init.status.success(), "{init:?}");
    let config_path = root.path().join("config/config.toml");
    let text = fs::read_to_string(&config_path).unwrap();
    let owner = text
        .lines()
        .find(|line| line.starts_with("owner_uid"))
        .unwrap();
    let table = format!("connectors.{CONNECTOR}.instances.{INSTANCE}");
    fs::write(
        &config_path,
        format!(
            "format = \"connectors-local/5\"\n{owner}\n{}[connectors.{CONNECTOR}]\nenabled=true\n[{table}]\nalias='{ALIAS}'\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n{member}[{table}.executable]\npath='/not-installed/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n[{table}.permissions]\nprofiles=['token']\noperations=['item.write']\n",
            clock(),
            "a".repeat(64)
        ),
    )
    .unwrap();
    root
}

/// The grant request of `approvals grant-standing --input-file`.
fn request(root: &Path) -> String {
    let path = root.join("request.json");
    let document = json!({
        "operation": "item.write",
        "target": [{"pointer": "/project", "value": "widgets"}],
        "not_after_unix_ms": 4_102_444_800_000_i64,
        "max_per_hour": 10
    });
    fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
    path.to_str().unwrap().to_owned()
}

/// The three commands with `selection` as their instance selection.
fn commands<'a>(selection: &[&'a str], request: &'a str) -> [(&'static str, Vec<&'a str>); 3] {
    let with = |head: &[&'a str], tail: &[&'a str]| [head, selection, tail].concat();
    [
        (
            "grant-standing",
            with(&["approvals", "grant-standing"], &["--input-file", request]),
        ),
        (
            "revoke-standing",
            with(&["approvals", "revoke-standing", GRANT], &[]),
        ),
        ("list-standing", with(&["approvals", "list-standing"], &[])),
    ]
}

/// The names in the state directory.
fn state(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(root.join("state"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

/// Acceptance 16. An instance whose entry has no `standing_grants` takes no
/// grant: each command is `invalid_configuration`, exit 2, and neither reads
/// the clock (the one configured here answers nothing) nor locks or writes.
#[test]
fn standing_commands_refuse_without_the_configuration_member() {
    let root = configured("");
    let request = request(root.path());
    let before = state(root.path());
    for selection in [
        &["--connector", CONNECTOR, "--instance", INSTANCE][..],
        &["--adapter", ALIAS][..],
    ] {
        for (name, args) in commands(selection, &request) {
            let output = command(root.path(), &args);
            let data = failure(&output);
            assert_eq!(output.status.code(), Some(2), "{name}: {output:?}");
            assert_eq!(data["code"], "invalid_configuration", "{name}: {data}");
        }
    }
    // No owner was started, and nothing was locked, counted or written.
    assert_eq!(state(root.path()), before);
    for name in ["owner.lock", "owner.sock", "standing-grants.lock"] {
        assert!(!before.iter().any(|held| held == name), "{before:?}");
    }
}

/// With the member the commands go on to the clock, as `approvals issue`
/// does: the configured clock forms no reading, which is `unavailable` with
/// exit 1. Nothing is written at the grant file's path, and `grant-standing`
/// decides its `--input-file` first, naming `input_file`.
#[test]
fn standing_commands_with_the_member_read_the_clock() {
    let grants = tempfile::tempdir().unwrap();
    let path = grants.path().join("forge.grants");
    let root = configured(&format!("standing_grants='{}'\n", path.display()));
    let request = request(root.path());
    let selection = ["--connector", CONNECTOR, "--instance", INSTANCE];
    for (name, args) in commands(&selection, &request) {
        let output = command(root.path(), &args);
        let data = failure(&output);
        assert_eq!(output.status.code(), Some(1), "{name}: {output:?}");
        assert_eq!(data["code"], "unavailable", "{name}: {data}");
    }
    assert!(!path.exists());
    let refused = root.path().join("refused.json");
    fs::write(&refused, br#"{"operation":"item.write","id":"g1"}"#).unwrap();
    for file in [refused.to_str().unwrap(), "/absent/request.json"] {
        let output = command(
            root.path(),
            &[
                &["approvals", "grant-standing"][..],
                &selection,
                &["--input-file", file],
            ]
            .concat(),
        );
        let data = failure(&output);
        assert_eq!(output.status.code(), Some(2), "{file}: {output:?}");
        assert_eq!(data["code"], "invalid_input", "{file}: {data}");
        assert_eq!(data["field"], "input_file", "{file}: {data}");
    }
    assert!(!path.exists());
}

/// The instance selection of the three commands is every approval command's:
/// a refused one is `invalid_input` naming `adapter`.
#[test]
fn standing_commands_refusing_their_selection_name_adapter() {
    let root = configured("");
    let request = request(root.path());
    for selection in [
        &["--connector", CONNECTOR][..],
        &["--adapter", ALIAS, "--connector", CONNECTOR][..],
        &[][..],
    ] {
        for (name, args) in commands(selection, &request) {
            let output = command(root.path(), &args);
            let data = failure(&output);
            assert_eq!(output.status.code(), Some(2), "{name}: {output:?}");
            assert_eq!(
                data["code"], "invalid_input",
                "{name} {selection:?}: {data}"
            );
            assert_eq!(data["field"], "adapter", "{name} {selection:?}: {data}");
        }
    }
}

/// The cached description of the fixture instance: one write operation.
fn bootstrap() -> runtime::Bootstrap {
    let descriptor = connectors_core::Descriptor {
        version: "v1alpha1".into(),
        instance: "forge-local".into(),
        adapter: "catalog".into(),
        revision: "desc-1".into(),
        operations: vec![connectors_core::Operation {
            id: "item.write".into(),
            description: "fixture operation".into(),
            contract: "operations/v1alpha1".into(),
            profile: "resource".into(),
            input_schema: json!({"type":"object"}),
            output_schema: json!({"type":"object"}),
        }],
        configuration_schema: json!({"type":"object"}),
    };
    runtime::Bootstrap {
        bindings: None,
        credential_revision: None,
        executable: None,
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
            token_recipient: None,
        }],
        requirements: vec![runtime::Requirement {
            operation: "item.write".into(),
            custody: false,
            profile: "token".into(),
            scopes: BTreeSet::new(),
            effect: runtime::Effect::Write,
        }],
    }
}

/// [`configured`] with the instance's description cached, as a launch for a
/// description leaves it: `invoke` reaches its handler's decisions.
fn described(member: &str) -> tempfile::TempDir {
    let root = configured(member);
    let paths = Paths {
        config: root.path().join("config/config.toml"),
        state: root.path().join("state"),
    };
    let adapter = Config::load(&paths.config).unwrap().adapters[ALIAS].clone();
    runtime::state::State::new(&paths.state)
        .remember(&adapter.selection(), &bootstrap())
        .unwrap();
    root
}

/// What a write without an approval answers, as
/// `invoke_short.rs::a_write_without_an_approval_file_is_approval_required_naming_the_steps`
/// pins it.
fn no_approval() -> Value {
    json!({"kind":"operational","code":"approval_required","stage":"admission","next_action":"none",
        "approval_steps":["approvals prepare","approvals clock-check","approvals issue"]})
}

/// A write of the fixture instance with `extra` arguments, standard input
/// closed.
fn write(root: &Path, extra: &[&str]) -> Output {
    let target = format!("{CONNECTOR}/{INSTANCE}");
    command(
        root,
        &[
            &[
                "invoke",
                &target,
                "item.write",
                "--connection",
                "c1",
                "--input-json",
                "{}",
            ][..],
            extra,
        ]
        .concat(),
    )
}

/// Acceptance 17. Without `--approval-file`, a write goes on to the owner's
/// write path only with an idempotency key on an instance that names its
/// grant file. With the member and no key, and with a key and no member, the
/// answer is the one a write without an approval had before grants existed,
/// byte for byte, and no use store is made. The control, with both: the write
/// is not refused at the gate (here the keyed lookup refuses it, the fixture
/// holding no connection). The three cases run side by side: the one without
/// a key starts an owner, as every `invoke` that names neither does.
///
/// An `--approval-file` that holds no byte (acceptance 18) is not shown here:
/// this fixture has no registered connection, and the keyed lookup refuses
/// the call before the approval file is read. The journey
/// `adapters/catalog/tests/local_runtime/slack_standing.rs` shows it.
#[test]
fn a_write_without_a_key_or_without_the_member_changes_no_byte() {
    let grants = tempfile::tempdir().unwrap();
    let member = format!(
        "standing_grants='{}'\n",
        grants.path().join("forge.grants").display()
    );
    let keyed = &["--idempotency-key", "k"][..];
    let case = |member: &str, extra: &[&str]| {
        let root = described(member);
        let output = write(root.path(), extra);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(output.stdout.is_empty(), "{output:?}");
        let held = state(root.path());
        assert!(
            !held.iter().any(|held| held == "standing-grants.sqlite3"),
            "{held:?}"
        );
        (failure(&output), held)
    };
    std::thread::scope(|scope| {
        let unkeyed = scope.spawn(|| case(&member, &[]));
        let without_member = scope.spawn(|| case("", keyed));
        let both = scope.spawn(|| case(&member, keyed));
        let (data, _) = unkeyed.join().unwrap();
        assert_eq!(data, no_approval(), "the member and no key");
        let (data, held) = without_member.join().unwrap();
        assert_eq!(data, no_approval(), "a key and no member");
        // The gate refused it: no owner was started for it.
        for name in ["owner.lock", "owner.sock"] {
            assert!(!held.iter().any(|held| held == name), "{held:?}");
        }
        let (data, _) = both.join().unwrap();
        assert_ne!(data["code"], "approval_required", "{data}");
        assert!(data.get("approval_steps").is_none(), "{data}");
    });
}

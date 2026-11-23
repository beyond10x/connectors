//! story:local-invoke-admits-a-keyed-write-under-a-standing-grant: the three
//! commands of `contracts/cli/v1alpha1/semantics.md`, "Standing grants",
//! `approvals grant-standing`, `approvals revoke-standing` and
//! `approvals list-standing`, run without a keyring. Each refusal here is
//! decided in the CLI process before any approval key is read.
// The fixture configures a Unix absolute executable path.
#![cfg(unix)]
use serde_json::{Value, json};
use std::{
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

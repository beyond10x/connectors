//! `setup init` with an existing configuration and no metadata store
//! (story:setup-initialises-missing-state). A valid configuration whose state
//! is missing gets its state initialised and answers `state_initialized`;
//! configuration and state both present still answer `configuration_exists`;
//! an invalid existing configuration is refused and never rewritten.
use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};

const ENTRY: &str = r#"
[adapters.forge]
instance_id = "forge-local"
adapter_id = "catalog"
configuration_revision = "cfg-1"
protocol = "v1alpha1"
private_protocol = "connectors-private/2"
startup = "on-demand"
restart = "never"

[adapters.forge.executable]
path = "/not-installed/connectors-catalog-provider"
sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
args = []
"#;

fn command(root: &tempfile::TempDir, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_connectors"))
        .args(["--output", "json", "--config"])
        .arg(config_path(root))
        .arg("--state-dir")
        .arg(state_path(root))
        .args(args)
        .output()
        .unwrap()
}

fn config_path(root: &tempfile::TempDir) -> PathBuf {
    root.path().join("config/config.toml")
}

fn state_path(root: &tempfile::TempDir) -> PathBuf {
    root.path().join("state")
}

fn success(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ok"], true, "{value}");
    value["result"].clone()
}

fn refusal(output: &Output, exit: i32) -> Value {
    assert_eq!(
        output.status.code(),
        Some(exit),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty(), "{output:?}");
    let value: Value = serde_json::from_slice(&output.stderr).unwrap();
    value["error"]["data"].clone()
}

/// A configuration with one adapter entry, created by `setup init` and then
/// extended, as an operator does.
fn configured() -> (tempfile::TempDir, Vec<u8>) {
    let root = tempfile::tempdir().unwrap();
    let created = success(&command(&root, &["setup", "init"]));
    assert_eq!(created["disposition"], "created", "{created}");
    let text = fs::read_to_string(config_path(&root)).unwrap() + ENTRY;
    fs::write(config_path(&root), &text).unwrap();
    success(&command(
        &root,
        &["connections", "list", "--adapter", "forge"],
    ));
    (root, text.into_bytes())
}

fn initialises_state_for(root: &tempfile::TempDir, before: &[u8]) {
    let read = command(root, &["connections", "list", "--adapter", "forge"]);
    assert_eq!(
        refusal(&read, 1)["code"],
        "metadata_unavailable",
        "precondition: reads find no metadata store"
    );
    let init = success(&command(root, &["setup", "init"]));
    assert_eq!(init["disposition"], "state_initialized", "{init}");
    assert_eq!(
        init["config_path"].as_str(),
        config_path(root).to_str(),
        "{init}"
    );
    assert_eq!(
        init["state_path"].as_str(),
        state_path(root).to_str(),
        "{init}"
    );
    assert_eq!(fs::read(config_path(root)).unwrap(), before);
    let adapters = success(&command(root, &["adapters", "list"]));
    let listed: Vec<_> = adapters["adapters"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["adapter"].clone())
        .collect();
    assert_eq!(listed, vec![Value::from("forge")], "{adapters}");
    let connections = success(&command(
        root,
        &["connections", "list", "--adapter", "forge"],
    ));
    assert_eq!(
        connections["connections"].as_array().map(Vec::len),
        Some(0),
        "{connections}"
    );
    let again = command(root, &["setup", "init"]);
    assert_eq!(refusal(&again, 1)["code"], "configuration_exists");
    assert_eq!(fs::read(config_path(root)).unwrap(), before);
}

#[test]
fn missing_state_directory_is_initialised_for_an_existing_configuration() {
    let (root, before) = configured();
    fs::remove_dir_all(state_path(&root)).unwrap();
    initialises_state_for(&root, &before);
}

#[test]
fn state_directory_without_a_metadata_store_is_initialised() {
    let (root, before) = configured();
    fs::remove_dir_all(state_path(&root)).unwrap();
    fs::create_dir(state_path(&root)).unwrap();
    fs::set_permissions(
        state_path(&root),
        std::os::unix::fs::PermissionsExt::from_mode(0o700),
    )
    .unwrap();
    initialises_state_for(&root, &before);
}

#[test]
fn configuration_and_state_both_present_still_answer_configuration_exists() {
    let (root, before) = configured();
    let store = state_path(&root).join("metadata.sqlite3");
    let store_before = fs::read(&store).unwrap();
    let again = command(&root, &["setup", "init"]);
    let data = refusal(&again, 1);
    assert_eq!(data["code"], "configuration_exists", "{data}");
    assert_eq!(data["stage"], "configuration", "{data}");
    assert_eq!(data["next_action"], "check_configuration", "{data}");
    assert_eq!(fs::read(config_path(&root)).unwrap(), before);
    assert_eq!(fs::read(&store).unwrap(), store_before);
}

#[test]
fn invalid_existing_configuration_is_refused_and_unchanged() {
    let (root, _) = configured();
    fs::remove_dir_all(state_path(&root)).unwrap();
    let invalid = b"format = \"connectors-local/9\"\nthis is not toml [\n".to_vec();
    fs::write(config_path(&root), &invalid).unwrap();
    let init = command(&root, &["setup", "init"]);
    let data = refusal(&init, 1);
    assert_eq!(data["code"], "configuration_exists", "{data}");
    assert_eq!(fs::read(config_path(&root)).unwrap(), invalid);
    assert!(
        !state_path(&root).exists(),
        "an invalid configuration's state must not be initialised"
    );
}

//! Adversary cases for story:store-version-skew-refusal
//! (contracts/cli/v1alpha1/semantics.md, "Store writer marker and binaries of
//! different versions"). The contract: a store a newer binary wrote is refused
//! by name, `metadata_schema_newer`, `next_action = upgrade`, and "never
//! `metadata_unavailable` or `retry_status`, because running the same binary
//! again cannot change the record".
use connectors_host::local::metadata::Metadata;
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
path = "NOT_INSTALLED/connectors-catalog-provider"
sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
args = []
"#;

const OTHER_MODEL: &str = "0000000000000000000000000000000000000000000000000000000000000000";

fn command(root: &tempfile::TempDir, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_connectors"))
        .args(["--output", "json", "--config"])
        .arg(root.path().join("config/config.toml"))
        .arg("--state-dir")
        .arg(state(root))
        .args(args)
        .output()
        .unwrap()
}

fn state(root: &tempfile::TempDir) -> PathBuf {
    root.path().join("state")
}

fn configured() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let init = command(&root, &["setup", "init"]);
    assert!(init.status.success(), "{init:?}");
    let entry = ENTRY.replace(
        "NOT_INSTALLED",
        if cfg!(windows) {
            "C:/not-installed"
        } else {
            "/not-installed"
        },
    );
    let path = root.path().join("config/config.toml");
    let text = fs::read_to_string(&path).unwrap() + &entry;
    fs::write(&path, text).unwrap();
    root
}

/// The failure a newer store must answer: not a retryable unavailability.
fn assert_not_retryable_newer(output: &Output) {
    let value: Value = serde_json::from_slice(&output.stderr)
        .unwrap_or_else(|_| serde_json::from_slice(&output.stdout).unwrap_or(Value::Null));
    let data = &value["error"]["data"];
    assert_eq!(
        (data["code"].as_str(), data["next_action"].as_str()),
        (Some("metadata_schema_newer"), Some("upgrade")),
        "exit {:?}: {value}",
        output.status.code()
    );
}

/// A marker whose `binary_version` this binary cannot parse. `StoreWriter`
/// declares `binary_version: String` with no format (ess/domains/cli.yaml), so
/// the Entity Runtime records it; a later release that tags `1.0.0-rc.1`, or an
/// edited store, leaves exactly this. `recorded_writer` (er.rs) then answers
/// `MetadataUnavailable`, which the CLI maps to `observation`/`retry_status` on
/// every command: the defect the story exists to remove.
#[test]
fn adversary_an_unparseable_marker_version_is_not_a_retryable_unavailability() {
    let root = configured();
    Metadata::record_writer_for_tests(&state(&root), "1.0.0-rc.1", OTHER_MODEL).unwrap();
    assert_not_retryable_newer(&command(
        &root,
        &["connections", "list", "--adapter", "forge"],
    ));
}

/// A future release that adds a SQL migration level (the store has had nine)
/// raises `user_version` past this binary's `er::LEVEL`. `validate_now`
/// (metadata.rs) refuses it as `MetadataUnavailable` before the marker is ever
/// read, so the older binary answers `retry_status` again, although the marker
/// names a newer writer with another model.
#[test]
fn adversary_a_store_a_newer_binary_raised_past_this_level_is_refused_by_name() {
    let root = configured();
    Metadata::record_writer_for_tests(&state(&root), "999.0.0", OTHER_MODEL).unwrap();
    let connection = rusqlite::Connection::open(state(&root).join("metadata.sqlite3")).unwrap();
    let level: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    connection
        .pragma_update(None, "user_version", level + 1)
        .unwrap();
    drop(connection);
    assert_not_retryable_newer(&command(
        &root,
        &["connections", "list", "--adapter", "forge"],
    ));
}

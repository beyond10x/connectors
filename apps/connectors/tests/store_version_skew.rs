//! An older binary on a store a newer one wrote says so
//! (story:store-version-skew-refusal, contracts/cli/v1alpha1/semantics.md §3).
//! The store's writer marker names the newest binary that wrote it and the
//! digest of the record model it persists. A newer release with another model
//! is refused by name, `metadata_schema_newer`, before any record is read: by a
//! read, by `setup check` and by a write. The same model under a newer release
//! is admitted, and a store without a marker gets one on its first write.
use connectors_host::local::{Release, metadata::Metadata};
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

/// A model digest no build persists.
const OTHER_MODEL: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const NEWER: &str = "999.0.0";

fn command(root: &tempfile::TempDir, args: &[&str]) -> Output {
    command_in(root, &state(root), args)
}

fn command_in(root: &tempfile::TempDir, state: &std::path::Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_connectors"))
        .args(["--output", "json", "--config"])
        .arg(root.path().join("config/config.toml"))
        .arg("--state-dir")
        .arg(state)
        .args(args)
        .output()
        .unwrap()
}

fn state(root: &tempfile::TempDir) -> PathBuf {
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

/// A configuration with one adapter entry and its store, created by
/// `setup init` and then extended, as an operator does.
fn configured() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    success(&command(&root, &["setup", "init"]));
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

/// The refusal `semantics.md` §3 names: exit 1, both versions, `upgrade`.
fn assert_schema_newer(output: &Output, store_version: &str) {
    assert_eq!(
        output.status.code(),
        Some(1),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty(), "{output:?}");
    let value: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(value["ok"], false, "{value}");
    assert_eq!(value["error"]["code"], "failure", "{value}");
    let data = &value["error"]["data"];
    assert_eq!(data["code"], "metadata_schema_newer", "{value}");
    assert_eq!(data["kind"], "operational", "{value}");
    assert_eq!(data["stage"], "admission", "{value}");
    assert_eq!(data["next_action"], "upgrade", "{value}");
    assert_eq!(data["store_version"], store_version, "{value}");
    assert_eq!(data["binary_version"], env!("CARGO_PKG_VERSION"), "{value}");
}

#[test]
fn a_store_a_newer_binary_with_another_model_wrote_is_refused_by_name() {
    let root = configured();
    Metadata::record_writer_for_tests(&state(&root), NEWER, OTHER_MODEL).unwrap();
    assert_schema_newer(
        &command(&root, &["connections", "list", "--adapter", "forge"]),
        NEWER,
    );
    assert_schema_newer(&command(&root, &["setup", "check"]), NEWER);
    assert_schema_newer(
        &command(
            &root,
            &[
                "connections",
                "revoke",
                "--adapter",
                "forge",
                "--connection",
                "absent",
                "--expected-revision",
                "1",
            ],
        ),
        NEWER,
    );
}

#[test]
fn the_same_model_under_a_newer_release_is_admitted_and_keeps_its_marker() {
    let root = configured();
    let model = Metadata::model_digest().unwrap();
    Metadata::record_writer_for_tests(&state(&root), NEWER, model).unwrap();
    let listed = success(&command(
        &root,
        &["connections", "list", "--adapter", "forge"],
    ));
    assert_eq!(listed["connections"], Value::Array(Vec::new()), "{listed}");
    let checked = success(&command(&root, &["setup", "check"]));
    assert!(
        checked["prerequisites"]
            .as_array()
            .unwrap()
            .iter()
            .any(|check| check["name"] == "metadata" && check["state"] == "ready"),
        "{checked}"
    );
    assert_eq!(
        Metadata::writer_for_tests(&state(&root)).unwrap(),
        Some((NEWER.to_owned(), model.to_owned())),
        "an older binary never lowers the marker"
    );
}

/// Every store a release before the marker wrote has none.
#[test]
fn a_store_without_a_marker_is_admitted_and_marked_by_its_first_write() {
    let root = configured();
    let unmarked = root.path().join("unmarked");
    Metadata::unmarked_store_for_tests(&unmarked).unwrap();
    assert_eq!(Metadata::writer_for_tests(&unmarked).unwrap(), None);
    success(&command_in(&root, &unmarked, &["setup", "check"]));
    assert_eq!(
        Metadata::writer_for_tests(&unmarked).unwrap(),
        None,
        "a check writes nothing"
    );
    // A list writes: it records the registry clock it observed at.
    success(&command_in(
        &root,
        &unmarked,
        &["connections", "list", "--adapter", "forge"],
    ));
    assert_eq!(
        Metadata::writer_for_tests(&unmarked).unwrap(),
        Some((
            Release::current().to_string(),
            Metadata::model_digest().unwrap().to_owned()
        ))
    );
}

/// A pre-release is a release this binary orders by SemVer precedence: the
/// marker names a newer one, so the store is refused naming it.
#[test]
fn a_newer_pre_release_marker_is_ordered_and_named() {
    let root = configured();
    let pre_release = "999.0.0-rc.1";
    Metadata::record_writer_for_tests(&state(&root), pre_release, OTHER_MODEL).unwrap();
    assert_schema_newer(
        &command(&root, &["connections", "list", "--adapter", "forge"]),
        pre_release,
    );
    assert_schema_newer(&command(&root, &["setup", "check"]), pre_release);
}

/// A marker version this binary cannot order (an edited store) under another
/// model is refused as a newer store it cannot name, never as unavailable.
#[test]
fn an_unorderable_marker_version_with_another_model_is_a_newer_store_unrecorded() {
    let root = configured();
    Metadata::record_writer_for_tests(&state(&root), "edited", OTHER_MODEL).unwrap();
    assert_schema_newer(
        &command(&root, &["connections", "list", "--adapter", "forge"]),
        "unrecorded",
    );
    assert_schema_newer(&command(&root, &["setup", "check"]), "unrecorded");
}

/// A store whose SQL level is above this binary's highest: a newer binary
/// migrated it. Its marker is not read through a schema this binary does not
/// know, so the refusal names no version.
#[test]
fn a_store_level_above_this_binary_is_a_newer_store_unrecorded() {
    let root = configured();
    Metadata::record_writer_for_tests(&state(&root), NEWER, OTHER_MODEL).unwrap();
    let connection = rusqlite::Connection::open(state(&root).join("metadata.sqlite3")).unwrap();
    let level: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    connection
        .pragma_update(None, "user_version", level + 1)
        .unwrap();
    drop(connection);
    assert_schema_newer(
        &command(&root, &["connections", "list", "--adapter", "forge"]),
        "unrecorded",
    );
    assert_schema_newer(&command(&root, &["setup", "check"]), "unrecorded");
    assert_schema_newer(
        &command(
            &root,
            &[
                "connections",
                "revoke",
                "--adapter",
                "forge",
                "--connection",
                "absent",
                "--expected-revision",
                "1",
            ],
        ),
        "unrecorded",
    );
}

//! `setup checkpoints-enable` (contracts/cli/v1alpha1/semantics.md §3): a
//! store `setup init` creates already has durable open checkpoints; an existing
//! store gets them only through this command, which refuses without its
//! confirmation and while an owner runs, and answers `already_enabled` when run
//! again. The existing store is the one Entity Runtime 0.26.0 wrote.
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::{fd::AsRawFd, unix::fs::MetadataExt, unix::fs::PermissionsExt},
    path::{Path, PathBuf},
    process::{Command, Output},
};

const NAME: &str = "metadata.sqlite3";

fn command(root: &Path, json: bool, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_connectors"));
    if json {
        command.args(["--output", "json"]);
    }
    command
        .arg("--config")
        .arg(root.join("config/config.toml"))
        .arg("--state-dir")
        .arg(root.join("state"))
        .args(args)
        .output()
        .unwrap()
}

fn initialized() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let init = command(root.path(), true, &["setup", "init"]);
    assert!(init.status.success(), "{init:?}");
    root
}

fn previous_pin_fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/connectors-host/tests/fixtures/metadata-store-er-0.26.0")
}

/// Replaces the initialized store with a copy of the store Entity Runtime
/// 0.26.0 wrote, handed to the uid running the test, as a restore would.
fn restore_previous_store(root: &Path) {
    let state = root.join("state");
    for suffix in ["", "-wal", "-shm"] {
        let _ = fs::remove_file(state.join(format!("{NAME}{suffix}")));
    }
    let target = state.join(NAME);
    Connection::open_with_flags(
        previous_pin_fixture().join(NAME),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap()
    .execute("VACUUM INTO ?1", [target.to_str().unwrap()])
    .unwrap();
    let restored = Connection::open(&target).unwrap();
    restored.pragma_update(None, "journal_mode", "WAL").unwrap();
    let owner = fs::metadata(&state).unwrap().uid();
    restored
        .execute(
            "UPDATE local_authority SET owner_uid = ?1 WHERE singleton = 1",
            [owner],
        )
        .unwrap();
    drop(restored);
    fs::copy(
        previous_pin_fixture().join("registry-clock.floor"),
        state.join("registry-clock.floor"),
    )
    .unwrap();
    for name in [NAME, "registry-clock.floor"] {
        fs::set_permissions(state.join(name), fs::Permissions::from_mode(0o600)).unwrap();
    }
}

fn continuity(root: &Path) -> bool {
    Connection::open_with_flags(
        root.join("state").join(NAME),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap()
    .query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE name='connectors_er_capture_continuity')",
        [],
        |row| row.get(0),
    )
    .unwrap()
}

/// Every file of the state directory but the owner lock, by name and digest.
fn state_files(root: &Path) -> Vec<(String, String)> {
    let mut files = fs::read_dir(root.join("state"))
        .unwrap()
        .map(|entry| entry.unwrap())
        .filter(|entry| entry.file_name() != "owner.lock")
        .map(|entry| {
            (
                entry.file_name().to_string_lossy().into_owned(),
                hex::encode(Sha256::digest(fs::read(entry.path()).unwrap())),
            )
        })
        .collect::<Vec<_>>();
    files.sort();
    files
}

fn result(output: &Output) -> Value {
    assert!(output.status.success(), "{output:?}");
    serde_json::from_slice::<Value>(&output.stdout).unwrap()["result"].clone()
}

fn refusal(output: &Output, exit: i32) -> Value {
    assert_eq!(output.status.code(), Some(exit), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let envelope: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(envelope["error"]["code"], "failure", "{envelope}");
    envelope["error"]["data"].clone()
}

#[test]
fn a_store_setup_init_creates_already_has_checkpoints() {
    let root = initialized();
    assert!(continuity(root.path()));
    let answer = result(&command(
        root.path(),
        true,
        &["setup", "checkpoints-enable", "--confirm", "one-way"],
    ));
    assert_eq!(answer["disposition"], "already_enabled", "{answer}");
}

#[test]
fn an_existing_store_is_enabled_only_when_confirmed_and_once() {
    let root = initialized();
    restore_previous_store(root.path());
    assert!(!continuity(root.path()));
    let before = state_files(root.path());

    let refused = refusal(
        &command(root.path(), true, &["setup", "checkpoints-enable"]),
        2,
    );
    assert_eq!(refused["kind"], "usage", "{refused}");
    assert_eq!(refused["code"], "confirmation_required", "{refused}");
    assert_eq!(refused["stage"], "arguments", "{refused}");
    assert_eq!(refused["next_action"], "retry_explicitly", "{refused}");
    assert_eq!(
        state_files(root.path()),
        before,
        "an unconfirmed run changed the store"
    );
    assert!(!continuity(root.path()));

    let enabled = result(&command(
        root.path(),
        true,
        &["setup", "checkpoints-enable", "--confirm", "one-way"],
    ));
    assert_eq!(enabled["disposition"], "enabled", "{enabled}");
    assert_eq!(enabled["change"], "one-way", "{enabled}");
    assert_eq!(
        enabled["newest_incompatible_release"], "0.32.0",
        "{enabled}"
    );
    assert_eq!(
        Path::new(enabled["state_path"].as_str().unwrap()),
        root.path().join("state")
    );
    assert!(continuity(root.path()));

    let again = command(
        root.path(),
        false,
        &["setup", "checkpoints-enable", "--confirm", "one-way"],
    );
    assert!(again.status.success(), "{again:?}");
    let human = String::from_utf8(again.stdout).unwrap();
    for stated in ["already_enabled", "one-way", "0.32.0"] {
        assert!(human.contains(stated), "{stated} missing from {human}");
    }
    // The enabled store keeps answering.
    let check = result(&command(root.path(), true, &["setup", "check"]));
    let metadata = check["prerequisites"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["name"] == "metadata")
        .unwrap()
        .clone();
    assert_eq!(metadata["state"], "ready", "{metadata}");
}

#[test]
fn a_running_owner_refuses_the_enable_with_the_store_unchanged() {
    let root = initialized();
    restore_previous_store(root.path());
    let before = state_files(root.path());
    let lock_path = root.path().join("state/owner.lock");
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)
        .unwrap();
    fs::set_permissions(&lock_path, fs::Permissions::from_mode(0o600)).unwrap();
    // SAFETY: `lock` owns the descriptor for the whole test; this process
    // holds the owner lifetime lock as a running owner does.
    assert_eq!(
        unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
        0
    );
    let refused = refusal(
        &command(
            root.path(),
            true,
            &["setup", "checkpoints-enable", "--confirm", "one-way"],
        ),
        1,
    );
    assert_eq!(refused["code"], "lifecycle_conflict", "{refused}");
    assert_eq!(refused["stage"], "admission", "{refused}");
    assert_eq!(refused["next_action"], "stop_owner", "{refused}");
    assert_eq!(state_files(root.path()), before);
    assert!(!continuity(root.path()));
    drop(lock);
}

#[test]
fn a_missing_store_is_not_created() {
    let root = initialized();
    for suffix in ["", "-wal", "-shm"] {
        let _ = fs::remove_file(root.path().join("state").join(format!("{NAME}{suffix}")));
    }
    let refused = refusal(
        &command(
            root.path(),
            true,
            &["setup", "checkpoints-enable", "--confirm", "one-way"],
        ),
        1,
    );
    assert_eq!(refused["code"], "metadata_unavailable", "{refused}");
    assert_eq!(refused["stage"], "observation", "{refused}");
    assert!(!root.path().join("state").join(NAME).exists());
}

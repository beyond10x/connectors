//! story:owner-errors-are-readable, security review. Conformance checks of the
//! owner log's own contract (contracts/cli/v1alpha1/owner.md, "Owner log"):
//! `owner_error` is the code of the failure the command saw, and the log never
//! writes through another name for a file.
//!
//! Unix only: the fixtures use mode bits and hard links.
#![cfg(unix)]
use connectors_host::local::{
    config::{Config, Paths},
    metadata::Metadata,
    owner::{Client, Code, diagnostics},
};
use std::{os::unix::fs::PermissionsExt, sync::Mutex};

/// `observed` reads one process-wide watch: the cases that use it take turns.
static WATCH: Mutex<()> = Mutex::new(());

fn root() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("owner-log-")
        .tempdir()
        .unwrap()
}

/// A private state directory this process connected to, with no owner running:
/// the connect fails after it noted the log's length.
fn watched(root: &tempfile::TempDir) -> Paths {
    let paths = Paths::resolve(
        Some(&root.path().join("config/config.toml")),
        Some(&root.path().join("state")),
    )
    .unwrap();
    Config::initialize(&paths).unwrap();
    drop(Metadata::initialize(&paths.state).unwrap());
    assert!(Client::connect(&paths, false).is_err(), "no owner runs");
    paths
}

/// Another connection of the same owner failed after this command connected;
/// this command's own failure left no code (its channel broke). The log line
/// is not this command's, so its data must not name it as `owner_error`.
#[test]
fn owner_error_is_never_a_code_another_connection_logged() {
    let _turn = WATCH.lock().unwrap_or_else(|p| p.into_inner());
    let root = root();
    let paths = watched(&root);
    diagnostics::record(&paths.state, "request failed", Code::NotFound);
    let log = diagnostics::observed(Code::OutcomeUnknown).expect("the log is named");
    assert_ne!(
        log.code.as_deref(),
        Some("not_found"),
        "owner_error names a code another connection's request logged"
    );
}

/// The owner panicked while serving this command: the newest line is the
/// panic, which carries no code. `owner_error` must not then name an older code
/// that is not the cause.
#[test]
fn owner_error_is_not_an_older_code_when_the_newest_line_is_a_panic() {
    let _turn = WATCH.lock().unwrap_or_else(|p| p.into_inner());
    let root = root();
    let paths = watched(&root);
    diagnostics::record(&paths.state, "request failed", Code::NotFound);
    diagnostics::record_panic(&paths.state, Some(std::panic::Location::caller()));
    let log = diagnostics::observed(Code::OutcomeUnknown).expect("the log is named");
    assert_eq!(
        log.code, None,
        "a panic is reported as the code of an earlier, unrelated line"
    );
}

/// A symbolic link at the log's name is never written through, and the CLI
/// names no log there (cleared check, expected green).
#[test]
fn a_symlink_at_the_log_is_neither_written_nor_named() {
    let _turn = WATCH.lock().unwrap_or_else(|p| p.into_inner());
    let root = root();
    let paths = watched(&root);
    let target = root.path().join("target");
    std::fs::write(&target, b"keep").unwrap();
    std::os::unix::fs::symlink(&target, diagnostics::path(&paths.state)).unwrap();
    diagnostics::record(&paths.state, "request failed", Code::NotFound);
    assert_eq!(std::fs::read(&target).unwrap(), b"keep");
    assert_eq!(diagnostics::observed(Code::OutcomeUnknown), None);
}

/// A second link to another file of the same user at the log's name: the
/// repository admits a private file only with one link
/// (`filesystem::check_private_file`); the log appends to, and may empty and
/// chmod, whatever inode the name reaches.
#[test]
fn the_log_never_writes_through_a_second_link() {
    let root = root();
    let state = root.path().join("state");
    std::fs::create_dir(&state).unwrap();
    std::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o700)).unwrap();
    let other = root.path().join("other");
    std::fs::write(&other, b"keep").unwrap();
    std::fs::hard_link(&other, diagnostics::path(&state)).unwrap();
    diagnostics::record(&state, "request failed", Code::NotFound);
    assert_eq!(
        std::fs::read(&other).unwrap(),
        b"keep",
        "the owner log wrote into another file through a hard link"
    );
}

/// A refusal is never logged into a state directory that is not private
/// (cleared check, expected green).
#[test]
fn a_refusal_is_not_logged_into_a_wider_state_directory() {
    let root = root();
    let state = root.path().join("state");
    std::fs::create_dir(&state).unwrap();
    std::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o755)).unwrap();
    diagnostics::record_refusal(&state, Code::Unavailable);
    assert!(!diagnostics::path(&state).exists());
}

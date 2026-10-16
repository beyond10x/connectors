//! story:journey-owners-are-stopped. An owner exits when its socket or its
//! state directory goes away, and a test build's owner exits when the process
//! it is tied to (`CONNECTORS_TEST_OWNER_PARENT`) has ended
//! (contracts/cli/v1alpha1/owner.md). Each case removes the thing or ends the
//! process and finds the owner gone within a bound far below the 600 s idle
//! retirement, which these owners keep.
//!
//! Unix only: the owner is started as the CLI starts it on Unix, with its
//! startup channel on descriptor 3 and its lifetime lock on descriptor 4.
#![cfg(unix)]
#[path = "support/owner.rs"]
mod owner_start;
use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Child, Command, ExitStatus, Output},
    time::{Duration, Instant},
};

/// The bound within which an owner leaves; the contract checks every 250 ms.
const BOUND: Duration = Duration::from_secs(5);

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

fn owner_root() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let output = command(root.path(), &["setup", "init"]);
    assert!(output.status.success(), "{output:?}");
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ok"], true);
    root
}

/// The test's own owner child; a failing assertion must not leave it serving.
struct OwnerProcess(Child);
impl Drop for OwnerProcess {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}
impl OwnerProcess {
    fn exit_within(&mut self, limit: Duration) -> Option<ExitStatus> {
        let until = Instant::now() + limit;
        loop {
            if let Some(status) = self.0.try_wait().unwrap() {
                return Some(status);
            }
            if Instant::now() >= until {
                return None;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

/// Starts `__connectors-owner` the way the CLI does (startup channel on fd 3,
/// lifetime lock on fd 4), tied to `parent` when one is named.
fn start_owner(root: &Path, parent: Option<u32>) -> OwnerProcess {
    use std::os::unix::{fs::OpenOptionsExt, net::UnixStream};
    let state = root.join("state");
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .mode(0o600)
        .open(state.join("owner.lock"))
        .unwrap();
    let (startup, owner_end) = UnixStream::pair().unwrap();
    let mut owner = Command::new("/bin/sh");
    owner.env_remove("CONNECTORS_TEST_OWNER_PARENT");
    if let Some(parent) = parent {
        owner.env("CONNECTORS_TEST_OWNER_PARENT", parent.to_string());
    }
    let mut owner = owner
        .arg("-c")
        .arg(r#"exec "$0" __connectors-owner "$1" "$2" 3<&0 4<&1 </dev/null >/dev/null"#)
        .arg(env!("CARGO_BIN_EXE_connectors"))
        .arg(root.join("config/config.toml"))
        .arg(&state)
        .stdin(std::process::Stdio::from(std::os::fd::OwnedFd::from(
            owner_end,
        )))
        .stdout(std::process::Stdio::from(lock))
        .stderr(owner_start::stderr(root))
        .spawn()
        .unwrap();
    drop(startup);
    owner_start::wait_listening(&mut owner, root);
    OwnerProcess(owner)
}

/// Acceptance: "an owner exits when its state directory goes away".
#[test]
fn an_owner_exits_within_the_bound_once_its_state_directory_is_removed() {
    let root = owner_root();
    let mut owner = start_owner(root.path(), None);
    assert!(owner.exit_within(Duration::from_millis(500)).is_none());
    fs::remove_dir_all(root.path().join("state")).unwrap();
    assert!(
        owner.exit_within(BOUND).is_some(),
        "owner still serves {BOUND:?} after its state directory was removed"
    );
}

/// Acceptance: "an owner exits when its socket directory goes away". The state
/// directory is replaced by another at the same path, as a harness re-creating
/// its fixture does; the owner leaves the new directory's contents alone.
#[test]
fn an_owner_exits_within_the_bound_once_its_state_directory_is_replaced() {
    let root = owner_root();
    let mut owner = start_owner(root.path(), None);
    let state = root.path().join("state");
    fs::rename(&state, root.path().join("moved")).unwrap();
    fs::create_dir(&state).unwrap();
    fs::write(state.join("marker"), b"another directory").unwrap();
    assert!(
        owner.exit_within(BOUND).is_some(),
        "owner still serves {BOUND:?} after its state directory was replaced"
    );
    assert_eq!(
        fs::read(state.join("marker")).unwrap(),
        b"another directory"
    );
}

/// Acceptance: an owner whose socket is removed exits: nothing can reach it.
#[test]
fn an_owner_exits_within_the_bound_once_its_socket_is_removed() {
    let root = owner_root();
    let mut owner = start_owner(root.path(), None);
    fs::remove_file(root.path().join("state/owner.sock")).unwrap();
    assert!(
        owner.exit_within(BOUND).is_some(),
        "owner still serves {BOUND:?} after its socket was removed"
    );
    assert!(root.path().join("state").is_dir());
}

/// Acceptance: "a test owner exits when the process that started it is gone".
#[test]
#[cfg_attr(not(debug_assertions), ignore = "the parent tie is debug-only")]
fn a_test_owner_exits_within_the_bound_once_its_named_process_ends() {
    let root = owner_root();
    let mut parent = Command::new("/bin/sleep").arg("600").spawn().unwrap();
    let mut owner = start_owner(root.path(), Some(parent.id()));
    assert!(
        owner.exit_within(Duration::from_millis(500)).is_none(),
        "an owner whose named process lives keeps serving"
    );
    parent.kill().unwrap();
    parent.wait().unwrap();
    assert!(
        owner.exit_within(BOUND).is_some(),
        "owner still serves {BOUND:?} after the process it is tied to ended"
    );
}

/// Without the variable an owner is tied to no process: the process that
/// started it ending leaves it serving (its lifetime is the idle bound).
#[test]
fn an_owner_tied_to_no_process_outlives_the_process_that_started_it() {
    let root = owner_root();
    let mut starter = Command::new("/bin/sleep").arg("600").spawn().unwrap();
    let mut owner = start_owner(root.path(), None);
    starter.kill().unwrap();
    starter.wait().unwrap();
    assert!(
        owner.exit_within(Duration::from_secs(1)).is_none(),
        "an owner with no named process exited"
    );
    assert!(root.path().join("state/owner.sock").exists());
}

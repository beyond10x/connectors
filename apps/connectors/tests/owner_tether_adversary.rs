//! Adversary cases for story:journey-owners-are-stopped
//! (contracts/cli/v1alpha1/owner.md: "the state directory's path no longer
//! names the directory the owner opened (it was removed, or replaced)" and
//! "lets the clients already connected finish for at most 5 seconds").
//!
//! Unix only, started as the CLI starts an owner (startup channel on fd 3,
//! lifetime lock on fd 4), like `owner_tether.rs`.
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

const BOUND: Duration = Duration::from_secs(5);

/// The owner socket as this test binds and connects it: through the open state directory on
/// Linux, as the host does, so a long checkout path does not push it past `SUN_LEN`.
fn socket_path(state: &fs::File, root: &Path) -> std::path::PathBuf {
    if cfg!(target_os = "linux") {
        use std::os::fd::AsRawFd;
        format!("/proc/self/fd/{}/owner.sock", state.as_raw_fd()).into()
    } else {
        root.join("state/owner.sock")
    }
}

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

fn init(root: &Path) {
    let output = command(root, &["setup", "init"]);
    assert!(output.status.success(), "{output:?}");
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ok"], true);
}

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

fn start_owner(root: &Path) -> OwnerProcess {
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

/// The state directory's parent is moved away and a regular file takes its
/// name. `<parent>/state` now names nothing the owner opened (stat fails with
/// ENOTDIR), and its socket still exists inside the moved directory, so only
/// the directory check can notice. The contract says the owner exits.
#[test]
fn an_owner_exits_once_an_ancestor_of_its_state_directory_is_replaced_by_a_file() {
    let top = tempfile::tempdir().unwrap();
    let root = top.path().join("a");
    fs::create_dir(&root).unwrap();
    init(&root);
    let mut owner = start_owner(&root);
    assert!(owner.exit_within(Duration::from_millis(500)).is_none());
    fs::rename(&root, top.path().join("a.moved")).unwrap();
    fs::write(&root, b"not a directory").unwrap();
    assert!(
        fs::metadata(root.join("state")).is_err(),
        "the state path names nothing"
    );
    assert!(
        owner.exit_within(BOUND).is_some(),
        "owner still serves {BOUND:?} after its state path stopped naming its directory (ENOTDIR)"
    );
}

/// The same, with the parent replaced by an unsearchable directory: stat of
/// the state path fails with EACCES, and the path names nothing the owner
/// opened.
#[test]
fn an_owner_exits_once_an_ancestor_of_its_state_directory_is_replaced_by_an_unsearchable_one() {
    use std::os::unix::fs::PermissionsExt;
    // SAFETY: getuid has no preconditions.
    if unsafe { libc::getuid() } == 0 {
        // root searches any directory; the case does not arise.
        return;
    }
    let top = tempfile::tempdir().unwrap();
    let root = top.path().join("a");
    fs::create_dir(&root).unwrap();
    init(&root);
    let mut owner = start_owner(&root);
    assert!(owner.exit_within(Duration::from_millis(500)).is_none());
    fs::rename(&root, top.path().join("a.moved")).unwrap();
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o000)).unwrap();
    let exited = owner.exit_within(BOUND);
    // Let the temporary directory be removed.
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(
        exited.is_some(),
        "owner still serves {BOUND:?} after its state path stopped naming its directory (EACCES)"
    );
}

/// "lets the clients already connected finish for at most 5 seconds": a
/// client that connected and never sends its greeting is counted for the
/// owner's 10 s greeting deadline. Once the socket is removed the owner waits
/// for it, but no longer than 5 s. Kills two mutants the existing cases miss:
/// no drain (exit within ~250 ms) and an unbounded drain (exit at ~10 s).
#[test]
fn a_lost_owner_waits_for_a_connected_client_and_no_longer_than_the_drain_bound() {
    use std::os::unix::net::UnixStream;
    let top = tempfile::tempdir().unwrap();
    init(top.path());
    let mut owner = start_owner(top.path());
    let state = fs::File::open(top.path().join("state")).unwrap();
    let socket = socket_path(&state, top.path());
    let client = UnixStream::connect(&socket).unwrap();
    // Give the owner time to accept and count the client.
    std::thread::sleep(Duration::from_millis(300));
    fs::remove_file(&socket).unwrap();
    let removed = Instant::now();
    assert!(
        owner.exit_within(Duration::from_secs(2)).is_none(),
        "the owner left at {:?} while a connected client had not finished",
        removed.elapsed()
    );
    assert!(
        owner
            .exit_within(BOUND + Duration::from_secs(2) - removed.elapsed())
            .is_some(),
        "the owner still serves {:?} after its socket was removed; the drain is bounded at 5 s",
        removed.elapsed()
    );
    drop(client);
}

/// "when the socket it listens on is no longer in its state directory": the
/// owner's socket is removed and another socket file is bound at the same
/// name within one check interval. The socket the owner listens on is gone
/// (a connect reaches the stand-in, never the owner), yet the check compares
/// only the file type at the path, not the socket's identity.
#[test]
fn an_owner_exits_once_its_socket_is_replaced_by_another_socket() {
    use std::os::unix::net::UnixListener;
    let top = tempfile::tempdir().unwrap();
    init(top.path());
    let mut owner = start_owner(top.path());
    let state = fs::File::open(top.path().join("state")).unwrap();
    let socket = socket_path(&state, top.path());
    fs::remove_file(&socket).unwrap();
    let _stand_in = UnixListener::bind(&socket).unwrap();
    assert!(
        owner.exit_within(BOUND).is_some(),
        "owner still serves {BOUND:?} after the socket it listens on was replaced by another"
    );
}

/// "The owner checks both at least every 250 ms": with no client connected
/// the drain is empty, so a removed socket stops the owner well inside one
/// second. The existing cases allow 5 s and so pass with a check interval up
/// to ~4.5 s; this one fails for any interval above ~1 s.
#[test]
fn an_owner_without_clients_exits_within_a_second_of_its_socket_removal() {
    let top = tempfile::tempdir().unwrap();
    init(top.path());
    let mut owner = start_owner(top.path());
    // Let the startup exchange finish, so no client is counted.
    std::thread::sleep(Duration::from_millis(300));
    fs::remove_file(top.path().join("state/owner.sock")).unwrap();
    let removed = Instant::now();
    assert!(
        owner.exit_within(Duration::from_millis(1500)).is_some(),
        "owner still serves {:?} after its socket was removed; the contract checks every 250 ms",
        removed.elapsed()
    );
}

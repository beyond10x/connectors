//! A directly started `__connectors-owner`: where its standard error goes, and
//! the wait for its socket, which fails with the owner's own refusal.
#[allow(clippy::duplicate_mod)]
#[path = "reap.rs"]
pub mod reap;
use std::{
    fs::File,
    path::{Path, PathBuf},
    process::{Child, Stdio},
    time::{Duration, Instant},
};

fn stderr_path(root: &Path) -> PathBuf {
    root.join("owner.stderr")
}

/// The owner's standard error: a fresh file in the test's root directory.
pub fn stderr(root: &Path) -> Stdio {
    Stdio::from(File::create(stderr_path(root)).unwrap())
}

/// Waits up to 20 s for `root/state/owner.sock`. An owner that exits first, or
/// is still not listening at the deadline, fails the test with its exit status
/// and its standard error, which names its refusal.
pub fn wait_listening(owner: &mut Child, root: &Path) {
    let socket = root.join("state/owner.sock");
    let until = Instant::now() + Duration::from_secs(20);
    while !socket.exists() {
        let exited = owner.try_wait().unwrap();
        if exited.is_some() || Instant::now() >= until {
            // An exit may have raced the socket's appearance.
            if socket.exists() {
                return;
            }
            let error = std::fs::read_to_string(stderr_path(root)).unwrap_or_default();
            panic!(
                "owner never listened ({}); its standard error: {error:?}",
                exited.map_or_else(|| "still running".to_owned(), |status| status.to_string())
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Starts `binary`'s `__connectors-owner` for `root` (`root/config/config.toml`,
/// `root/state`, which must exist) the way the CLI does: startup channel on
/// descriptor 3, lifetime lock on descriptor 4. Returns once it listens. The
/// caller owns the child and stops it.
#[cfg(unix)]
#[allow(dead_code)]
pub fn spawn(binary: &Path, root: &Path) -> Child {
    use std::os::unix::{fs::OpenOptionsExt, net::UnixStream};
    let state = root.join("state");
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .mode(0o600)
        .open(state.join("owner.lock"))
        .unwrap();
    let (startup, owner_end) = UnixStream::pair().unwrap();
    let mut owner = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(r#"exec "$0" __connectors-owner "$1" "$2" 3<&0 4<&1 </dev/null >/dev/null"#)
        .arg(binary)
        .arg(root.join("config/config.toml"))
        .arg(&state)
        .stdin(Stdio::from(std::os::fd::OwnedFd::from(owner_end)))
        .stdout(Stdio::from(lock))
        .stderr(stderr(root))
        .spawn()
        .unwrap();
    drop(startup);
    wait_listening(&mut owner, root);
    owner
}

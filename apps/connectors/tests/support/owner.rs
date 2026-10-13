//! A directly started `__connectors-owner`: where its standard error goes, and
//! the wait for its socket, which fails with the owner's own refusal.
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

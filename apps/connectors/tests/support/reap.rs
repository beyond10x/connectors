//! Stopping the `__connectors-owner` processes a test started
//! (story:tests-stop-the-owners-they-start). A fixture calls [`under`] or
//! [`child`] from its `Drop`, so a passing and a panicking test alike leave no
//! owner serving a temporary root that is about to be removed.
#![allow(dead_code)]
#![cfg(unix)]
use std::{
    fs,
    os::unix::ffi::OsStrExt,
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

/// How long a drop waits for owners to exit after the stop request, and again
/// after the kill.
const BOUND: Duration = Duration::from_secs(10);

/// The pids of every `__connectors-owner` of `root`: one whose root directory
/// is `root` (a jail, also after the directory was removed: the kernel then
/// names it with a ` (deleted)` suffix), or one given a path under `root` as an
/// argument (its configuration or state directory).
pub fn owners_under(root: &Path) -> Vec<i32> {
    let mut pids = Vec::new();
    for entry in fs::read_dir("/proc").into_iter().flatten().flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<i32>().ok())
        else {
            continue;
        };
        let Ok(command) = fs::read(entry.path().join("cmdline")) else {
            continue;
        };
        let words: Vec<&[u8]> = command.split(|byte| *byte == 0).collect();
        if !words.contains(&&b"__connectors-owner"[..]) {
            continue;
        }
        let jailed = fs::read_link(entry.path().join("root")).is_ok_and(|at| {
            let at = at.to_string_lossy();
            at.strip_suffix(" (deleted)").unwrap_or(&at) == root.to_string_lossy()
        });
        let given = words.iter().any(|word| {
            !word.is_empty()
                && word.first() == Some(&b'/')
                && Path::new(std::ffi::OsStr::from_bytes(word)).starts_with(root)
        });
        if jailed || given {
            pids.push(pid);
        }
    }
    pids
}

fn wait_gone(root: &Path, bound: Duration) -> Vec<i32> {
    let until = Instant::now() + bound;
    loop {
        let alive = owners_under(root);
        if alive.is_empty() || Instant::now() >= until {
            return alive;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

/// SIGKILL `pid` and, when it leads a group other than the test's own, the
/// group.
pub fn kill_group(pid: i32) {
    // SAFETY: getpgrp has no preconditions; getpgid and kill take plain integers.
    unsafe {
        let own = libc::getpgrp();
        let group = libc::getpgid(pid);
        if group > 1 && group != own {
            libc::kill(-group, libc::SIGKILL);
        }
        libc::kill(pid, libc::SIGKILL);
    }
}

/// `setup owner-stop` of the owner of `state`, for the stop route; the
/// stop's own bound is shortened to 5 s.
pub fn owner_stop(command: &mut Command) {
    let stop = command
        .args(["--output", "json"])
        .env("CONNECTORS_TEST_OWNER_STOP_MS", "5000")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    if let Ok(mut stop) = stop {
        let _ = stop.wait();
    }
}

/// Stop every owner of `root`: `ask` is the owner-stop route (it runs only when
/// an owner is there); a kill of each remaining owner's process group follows
/// when one is still running after the bound. Panics only when an owner
/// survives the kill and the test is not already panicking.
pub fn under(root: &Path, ask: impl FnOnce()) {
    if owners_under(root).is_empty() {
        return;
    }
    ask();
    let alive = wait_gone(root, BOUND);
    if alive.is_empty() {
        return;
    }
    for pid in &alive {
        kill_group(*pid);
    }
    let alive = wait_gone(root, BOUND);
    if !alive.is_empty() && !std::thread::panicking() {
        panic!("owners {alive:?} of {root:?} survived a kill");
    }
}

/// As [`under`], asking through the CLI `binary` for the owner of `root`
/// (`root/config/config.toml`, `root/state`).
pub fn under_root(root: &Path, binary: &Path) {
    under(root, || {
        let mut stop = Command::new(binary);
        stop.arg("--config")
            .arg(root.join("config/config.toml"))
            .arg("--state-dir")
            .arg(root.join("state"))
            .args(["setup", "owner-stop"]);
        owner_stop(&mut stop);
    });
}

/// A directly spawned owner a fixture holds: SIGKILL its group and itself, and
/// reap it. Safe on an owner that already exited.
pub fn child(owner: &mut Child) {
    let pid = owner.id() as i32;
    if owner.try_wait().ok().flatten().is_none() {
        kill_group(pid);
    }
    let _ = owner.wait();
}

/// A temporary root whose owners are stopped before it is removed.
pub struct Root(pub tempfile::TempDir);

impl Root {
    pub fn new() -> Self {
        Self(tempfile::tempdir().unwrap())
    }
}

impl std::ops::Deref for Root {
    type Target = tempfile::TempDir;
    fn deref(&self) -> &tempfile::TempDir {
        &self.0
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        under_root(self.0.path(), Path::new(env!("CARGO_BIN_EXE_connectors")));
    }
}

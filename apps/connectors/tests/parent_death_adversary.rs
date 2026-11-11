//! story:test-children-die-with-their-parent, acceptance: a test that starts an
//! owner through the usual fixture and is SIGKILLed finds, within 2 s, that no
//! process of that fixture remains.
//!
//! The case re-runs this binary as a harness that starts an owner the way the
//! suites do (`support/owner.rs`), publishes its pid and waits; the case
//! SIGKILLs the harness, which runs no `Drop`, and requires the owner gone
//! within 2 s. The harness is itself bound to the case and reaped by it.
#![cfg(target_os = "linux")]
#[path = "support/owner.rs"]
mod owner_start;
use std::{
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const ROOT: &str = "CONNECTORS_ADVERSARY_PARENT_DEATH_ROOT";

fn running(pid: i32) -> bool {
    match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(stat) => stat
            .rsplit_once(')')
            .and_then(|(_, rest)| rest.split_whitespace().next().map(|state| state != "Z"))
            .unwrap_or(false),
        Err(_) => false,
    }
}

fn kill(pid: i32) {
    // SAFETY: signals one pid this case started or read from its own fixture.
    unsafe { libc::kill(pid, libc::SIGKILL) };
}

/// Kills every pid it holds when dropped, so a red case leaves none.
struct Reaper(Vec<i32>);
impl Drop for Reaper {
    fn drop(&mut self) {
        for pid in &self.0 {
            kill(*pid);
        }
    }
}

/// Kills the harness when dropped; a no-op once it has been waited for.
struct Harness(std::process::Child);
impl Drop for Harness {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// The harness: this binary, bound to the case that starts it so that a case
/// killed in its turn leaves no harness.
fn harness(root: &Path) -> std::process::Child {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "owner_harness_fixture",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(ROOT, root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // SAFETY: the hook calls only async-signal-safe functions on a scalar.
    let parent = unsafe { libc::getpid() };
    unsafe {
        command.pre_exec(move || {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) != 0 || libc::getppid() != parent
            {
                return Err(std::io::Error::from_raw_os_error(libc::ECHILD));
            }
            Ok(())
        });
    }
    command.spawn().unwrap()
}

fn pid_in(file: &Path, harness: &mut std::process::Child) -> i32 {
    let until = Instant::now() + Duration::from_secs(60);
    loop {
        if let Ok(text) = std::fs::read_to_string(file) {
            return text.trim().parse().unwrap();
        }
        assert!(Instant::now() < until, "the harness never published a pid");
        assert!(harness.try_wait().unwrap().is_none(), "harness ended early");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Harness: `setup init` and an owner started by the usual fixture
/// (`support/owner.rs` `spawn`, as local_cli.rs and others do). An ordinary
/// run does nothing.
#[test]
fn owner_harness_fixture() {
    let Some(root) = std::env::var_os(ROOT).map(PathBuf::from) else {
        return;
    };
    let binary = Path::new(env!("CARGO_BIN_EXE_connectors"));
    let init = Command::new(binary)
        .args(["--output", "json", "--config"])
        .arg(root.join("config/config.toml"))
        .arg("--state-dir")
        .arg(root.join("state"))
        .args(["setup", "init"])
        .output()
        .unwrap();
    assert!(init.status.success(), "{init:?}");
    let mut owner = owner_start::spawn(binary, &root);
    let staging = root.join("owner.tmp");
    std::fs::write(&staging, format!("{}\n", owner.id())).unwrap();
    std::fs::rename(staging, root.join("owner.pid")).unwrap();
    let _ = owner.wait();
}

#[test]
fn a_killed_test_leaves_no_owner_from_the_usual_fixture() {
    let root = owner_start::reap::Root::new();
    let mut harness = Harness(harness(root.path()));
    let owner = pid_in(&root.path().join("owner.pid"), &mut harness.0);
    let _reaper = Reaper(vec![owner]);
    assert!(running(owner));
    kill(harness.0.id() as i32);
    harness.0.wait().unwrap();
    let killed = Instant::now();
    while running(owner) {
        assert!(
            killed.elapsed() < Duration::from_secs(2),
            "owner {owner} still serves 2 s after the test that started it was SIGKILLed"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

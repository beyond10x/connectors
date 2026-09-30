//! Adversary cases for story:owner-idle-exit (wave 20260929c).
//!
//! The owner's metadata recovery thread runs a sweep every 5 s whether or not
//! anything is pending (maintenance.rs `Background::start`), and the unit marks
//! the whole sweep as work (`sweeping`), which the accept loop polls every
//! 10 ms. The production idle bound is 600 s, so 120 empty sweeps fall inside
//! it. The unit's own tests use a 1.5 s bound, which no sweep ever crosses.
//! These cases use a bound longer than one sweep interval, as production does.
use serde_json::Value;
use std::{
    fs,
    process::{Command, Output},
    time::{Duration, Instant},
};

fn command(root: &tempfile::TempDir, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_connectors"))
        .args(["--output", "json", "--config"])
        .arg(root.path().join("config/config.toml"))
        .arg("--state-dir")
        .arg(root.path().join("state"))
        .args(args)
        .output()
        .unwrap()
}

fn success(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ok"], true);
    value["result"].clone()
}

fn owner_root() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    success(&command(&root, &["setup", "init"]));
    let config_path = root.path().join("config/config.toml");
    let config = fs::read_to_string(&config_path).unwrap()
        + &format!(
            "\n[adapters.forge]\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/1'\n[adapters.forge.executable]\npath='/not-installed/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n",
            "a".repeat(64)
        );
    fs::write(&config_path, config).unwrap();
    root
}

/// The test's own owner child; a failing assertion must not leave it serving.
struct OwnerProcess(std::process::Child);
impl Drop for OwnerProcess {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

/// Starts `__connectors-owner` the way the CLI does (startup channel on fd 3,
/// lifetime lock on fd 4), with the debug-build idle override.
fn start_owner(root: &tempfile::TempDir, idle_ms: u64) -> OwnerProcess {
    use std::os::unix::{fs::OpenOptionsExt, net::UnixStream};
    let state = root.path().join("state");
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .mode(0o600)
        .open(state.join("owner.lock"))
        .unwrap();
    let (startup, owner_end) = UnixStream::pair().unwrap();
    let owner = Command::new("/bin/sh")
        .env("CONNECTORS_TEST_OWNER_IDLE_MS", idle_ms.to_string())
        .arg("-c")
        .arg(r#"exec "$0" __connectors-owner "$1" "$2" 3<&0 4<&1 </dev/null >/dev/null"#)
        .arg(env!("CARGO_BIN_EXE_connectors"))
        .arg(root.path().join("config/config.toml"))
        .arg(&state)
        .stdin(std::process::Stdio::from(std::os::fd::OwnedFd::from(
            owner_end,
        )))
        .stdout(std::process::Stdio::from(lock))
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    drop(startup);
    let until = Instant::now() + Duration::from_secs(20);
    while !state.join("owner.sock").exists() {
        assert!(Instant::now() < until, "owner never listened");
        std::thread::sleep(Duration::from_millis(20));
    }
    OwnerProcess(owner)
}

#[allow(dead_code)]
fn owner_exit(owner: &mut OwnerProcess, limit: Duration) -> Option<std::process::ExitStatus> {
    let until = Instant::now() + limit;
    loop {
        if let Some(status) = owner.0.try_wait().unwrap() {
            return Some(status);
        }
        if Instant::now() >= until {
            return None;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Acceptance: "an owner with no client and no child work for a bounded idle
/// period exits". Nothing is pending, no client ever connects and no adapter is
/// started; the only activity is each owner's own empty recovery sweep. The
/// bound is 30 s: six sweep intervals, as 600 s is 120. Every owner must exit
/// within the bound plus 15 s. Four owners run at once, as one user's sandbox
/// runs leave several (the story's own 2026-09-29 observation).
#[test]
#[cfg_attr(not(debug_assertions), ignore = "the idle override is debug-only")]
fn idle_owners_exit_within_their_bound_when_it_spans_several_recovery_sweeps() {
    let roots: Vec<_> = (0..4).map(|_| owner_root()).collect();
    let started = Instant::now();
    let mut owners: Vec<_> = roots.iter().map(|root| start_owner(root, 30_000)).collect();
    let limit = Duration::from_secs(45);
    let mut exits = vec![None; owners.len()];
    while started.elapsed() < limit && exits.iter().any(Option::is_none) {
        for (exit, owner) in exits.iter_mut().zip(owners.iter_mut()) {
            if exit.is_none() && owner.0.try_wait().unwrap().is_some() {
                *exit = Some(started.elapsed());
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        exits.iter().all(Option::is_some),
        "owners with no client, no child and nothing pending, idle bound 30 s, exit times after {limit:?} (None = still running): {exits:?}; an empty 5 s recovery sweep observed by the accept loop restarts the idle clock"
    );
    for (root, owner) in roots.iter().zip(owners.iter_mut()) {
        assert!(owner.0.try_wait().unwrap().unwrap().success());
        assert!(!root.path().join("state/owner.sock").exists());
    }
}

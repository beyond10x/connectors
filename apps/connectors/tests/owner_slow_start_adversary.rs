//! Adversary cases for story:owner-start-outlasts-a-fixed-bound
//! (contracts/cli/v1alpha1/owner.md, the paragraph after "admitted client.").
//!
//! The production CLI, owner and an adapter child against the read-only
//! `mounted_secret` binding in a jail (`support/mounted.rs`), with the owner's
//! startup slowed by the test build's `CONNECTORS_TEST_OWNER_START_DELAY_MS`,
//! as `owner_slow_start.rs` does. Linux only: the binding is.
#![cfg(target_os = "linux")]
#[path = "support/mounted.rs"]
mod mounted;
use connectors_custody::mounted::fixture::{Jail, Map};
use mounted::*;
use std::{
    os::fd::AsRawFd,
    process::{Output, Stdio},
    time::{Duration, Instant},
};

const ADAPTER_FIXTURE: &str = "owner_slow_start_adversary_adapter_fixture";

/// The adapter child the owner starts: this binary, filtered to this test.
/// The ordinary run does nothing.
#[test]
fn owner_slow_start_adversary_adapter_fixture() {
    serve_when_child();
}

/// A container with one ready connection and no owner running: its reference
/// and revision.
fn connected() -> (Container, String, String) {
    let container = Container::new(ADAPTER_FIXTURE);
    container.deliver(PROFILE, SECRET);
    container.configure("connectors-local/4", &Container::mounted());
    let init = success(&container.run(&["setup", "init"]));
    assert_eq!(init["disposition"], "state_initialized", "{init}");
    let connected = success(&container.connect(PROFILE));
    let summary = &connected["connection"]["summary"];
    let (connection, revision) = (
        summary["connection"].as_str().unwrap().to_owned(),
        summary["revision"].as_str().unwrap().to_owned(),
    );
    success(&container.run(&["setup", "owner-stop"]));
    (container, connection, revision)
}

/// The CLI in the jail, as [`Container::run`] runs it, with any owner it starts
/// taking `delay` to answer its greeting: the stand-in for a large store.
fn slow(container: &Container, delay: Duration, args: &[&str]) -> Output {
    Jail::output(
        container
            .jail
            .command("/bin-test/connectors", Map::Own)
            .args(["--output", "json", "--config", CONFIG, "--state-dir", STATE])
            .args(args)
            .env_clear()
            .env(
                "CONNECTORS_TEST_OWNER_START_DELAY_MS",
                delay.as_millis().to_string(),
            )
            .stdin(Stdio::null()),
    )
}

fn read_args<'a>(connection: &'a str, schema: &'a str) -> Vec<&'a str> {
    vec![
        "operations",
        "invoke",
        "--adapter",
        ALIAS,
        "--operation",
        READ,
        "--connection",
        connection,
        "--schema",
        schema,
        "--revision",
        REVISION,
        "--input-json",
        "{}",
    ]
}

/// Blocks until the owner lifetime lock is held by someone other than this
/// test: a CLI has started an owner, which is now verifying the store.
fn wait_lock_held(container: &Container, within: Duration) {
    let path = container.path(STATE).join("owner.lock");
    let until = Instant::now() + within;
    loop {
        if let Ok(file) = std::fs::File::open(&path) {
            // SAFETY: `file` owns the live descriptor; closing it releases a
            // lock this probe took.
            let taken = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
            if taken != 0 {
                return;
            }
        }
        assert!(Instant::now() < until, "no owner start began");
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// story:owner-start-outlasts-a-fixed-bound, "Not in scope": "A store that
/// grows past the read's 30 s still times out the first read; a later read
/// reaches the owner that the first command left running for its 10-minute
/// idle bound." With the owner taking longer than the read's 30-second
/// deadline to start, the first read times out; the claim is that the next
/// read (same store, same startup cost) reaches the owner the first one left.
#[test]
fn a_read_after_one_that_outlasted_its_deadline_reaches_the_owner_it_left() {
    let (container, connection, _) = connected();
    let schema = container.schema();
    let args = read_args(&connection, &schema);
    let start = Duration::from_secs(34);
    let first = slow(&container, start, &args);
    let (_, data) = refusal(&first);
    assert_eq!(data["code"], "timeout", "{data}");
    let started = Instant::now();
    let second = slow(&container, start, &args);
    let read = success(&second);
    assert_eq!(read["result"]["credential"], digest(SECRET), "{read}");
    assert!(started.elapsed() < start, "{:?}", started.elapsed());
}

/// A command without a deadline of its own (`connections revalidate`) that
/// arrives while another command's owner is still in its slow start: owner.md
/// says such a command waits 60 s for an owner it started and 10 s for a
/// running one. The owner here is neither yet; the case asserts the command
/// still answers `ready` once that owner greets.
#[test]
fn a_revalidation_during_another_commands_slow_owner_start_reaches_that_owner() {
    let (container, connection, revision) = connected();
    let schema = container.schema();
    let args = read_args(&connection, &schema);
    let start = Duration::from_secs(14);
    std::thread::scope(|scope| {
        let reader = scope.spawn(|| slow(&container, start, &args));
        wait_lock_held(&container, Duration::from_secs(20));
        let revalidated = slow(
            &container,
            start,
            &[
                "connections",
                "revalidate",
                "--adapter",
                ALIAS,
                "--connection",
                &connection,
                "--expected-revision",
                &revision,
            ],
        );
        let revalidated = success(&revalidated);
        assert_eq!(
            revalidated["connection"]["summary"]["state"], "ready",
            "{revalidated}"
        );
        let read = success(&reader.join().unwrap());
        assert_eq!(read["result"]["credential"], digest(SECRET), "{read}");
    });
}

/// `setup owner-stop` while an owner another command started is still in its
/// slow start: the owner holds the lifetime lock and does not listen yet.
/// owner.md: owner-stop greets it once it listens and stops it within its
/// 30-second bound.
#[test]
fn owner_stop_during_a_slow_owner_start_stops_that_owner() {
    let (container, connection, _) = connected();
    let schema = container.schema();
    let args = read_args(&connection, &schema);
    let start = Duration::from_secs(14);
    std::thread::scope(|scope| {
        let reader = scope.spawn(|| slow(&container, start, &args));
        wait_lock_held(&container, Duration::from_secs(20));
        let stopped = success(&container.run(&["setup", "owner-stop"]));
        assert_eq!(stopped["disposition"], "stopped", "{stopped}");
        let _ = reader.join().unwrap();
    });
}

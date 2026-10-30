//! story:owner-start-outlasts-a-fixed-bound. An owner verifies the whole
//! metadata store before it answers its startup greeting, so the time it takes
//! to start grows with the store. The observed defect: on a store of a few
//! hundred megabytes that verification took longer than the CLI's fixed
//! 10-second wait for the greeting; the CLI then killed the owner it had
//! started and answered `timeout`, and so did every later read, because each
//! started an owner of its own and killed it again. A read carries its own
//! deadline through owner startup (contracts/cli/v1alpha1/owner.md), and a
//! command without one waits for the owner it started for the startup bound.
//!
//! The production CLI, owner and an adapter child against the read-only
//! `mounted_secret` binding in a jail (`support/mounted.rs`). A test build's
//! owner waits `CONNECTORS_TEST_OWNER_START_DELAY_MS` before it answers its
//! greeting, standing in for a large store. Linux only: the binding is.
#![cfg(target_os = "linux")]
#[path = "support/mounted.rs"]
mod mounted;
use connectors_custody::mounted::fixture::{Jail, Map};
use mounted::*;
use std::{
    process::{Output, Stdio},
    time::{Duration, Instant},
};

const ADAPTER_FIXTURE: &str = "owner_slow_start_adapter_fixture";
/// Longer than the CLI's fixed 10-second wait for an owner it reaches, and
/// shorter than a read's 30-second deadline.
const SLOW_START: Duration = Duration::from_secs(12);

/// The adapter child the owner starts: this binary, filtered to this test.
/// The ordinary run does nothing.
#[test]
fn owner_slow_start_adapter_fixture() {
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

/// The CLI in the jail, as [`Container::run`] runs it, with the owner it
/// starts taking [`SLOW_START`] to answer its greeting.
fn slow(container: &Container, args: &[&str]) -> Output {
    Jail::output(
        container
            .jail
            .command("/bin-test/connectors", Map::Own)
            .args(["--output", "json", "--config", CONFIG, "--state-dir", STATE])
            .args(args)
            .env_clear()
            .env(
                "CONNECTORS_TEST_OWNER_START_DELAY_MS",
                SLOW_START.as_millis().to_string(),
            )
            .stdin(Stdio::null()),
    )
}

/// The reported failure: with no owner running, a read whose owner takes
/// longer than 10 seconds to start succeeds within the read's own deadline.
#[test]
fn a_read_waits_for_a_slow_owner_start_within_its_deadline() {
    let (container, connection, _) = connected();
    let schema = container.schema();
    let target = format!("{CONNECTOR}/{ALIAS}");
    let started = Instant::now();
    let read = success(&slow(
        &container,
        &[
            "invoke",
            &target,
            READ,
            "--connection",
            &connection,
            "--schema",
            &schema,
            "--revision",
            REVISION,
            "--input-json",
            "{}",
        ],
    ));
    assert!(started.elapsed() >= SLOW_START, "{:?}", started.elapsed());
    assert_eq!(read["result"]["credential"], digest(SECRET), "{read}");
    // The owner it started stays: the next read reaches it at once.
    let started = Instant::now();
    let read = success(&container.invoke(&connection));
    assert!(started.elapsed() < SLOW_START, "{:?}", started.elapsed());
    assert_eq!(read["result"]["credential"], digest(SECRET), "{read}");
}

/// A command with no deadline of its own (`connections revalidate` sets its
/// 30 seconds once the owner answers) waits for the owner it started for the
/// startup bound, not for the 10 seconds it waits for a running owner.
#[test]
fn a_revalidation_waits_for_a_slow_owner_start() {
    let (container, connection, revision) = connected();
    let started = Instant::now();
    let revalidated = success(&slow(
        &container,
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
    ));
    assert!(started.elapsed() >= SLOW_START, "{:?}", started.elapsed());
    assert_eq!(
        revalidated["connection"]["summary"]["state"], "ready",
        "{revalidated}"
    );
}

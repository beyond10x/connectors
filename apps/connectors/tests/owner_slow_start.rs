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
    let connected = serving();
    success(&connected.0.run(&["setup", "owner-stop"]));
    connected
}

/// A container with one ready connection, the owner and the adapter child its
/// connect started still running: its reference and revision.
fn serving() -> (Container, String, String) {
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

/// story:cli-connect-admission-timeout-under-load. On a loaded host the CLI
/// spends longer than 10 seconds launching the owner (reading and sealing its
/// own executable) before the owner can answer its greeting. The launch is
/// part of the start a command without a deadline of its own waits for up to
/// the startup bound: it is not `timeout` at `admission`.
#[test]
fn a_revalidation_waits_for_a_slow_owner_launch() {
    let (container, connection, revision) = connected();
    let started = Instant::now();
    let output = Jail::output(
        container
            .jail
            .command("/bin-test/connectors", Map::Own)
            .args(["--output", "json", "--config", CONFIG, "--state-dir", STATE])
            .args([
                "connections",
                "revalidate",
                "--adapter",
                ALIAS,
                "--connection",
                &connection,
                "--expected-revision",
                &revision,
            ])
            .env_clear()
            .env(
                "CONNECTORS_TEST_OWNER_LAUNCH_DELAY_MS",
                SLOW_START.as_millis().to_string(),
            )
            .stdin(Stdio::null()),
    );
    let revalidated = success(&output);
    assert!(started.elapsed() >= SLOW_START, "{:?}", started.elapsed());
    assert_eq!(
        revalidated["connection"]["summary"]["state"], "ready",
        "{revalidated}"
    );
}

/// How one `invoke` ended: its result, or its refusal's code and stage.
#[derive(Debug, PartialEq, Eq)]
enum Ended {
    Answered,
    Refused(String, String),
}

fn ended(output: &Output) -> Ended {
    if output.status.success() {
        let read = success(output);
        assert_eq!(read["result"]["credential"], digest(SECRET), "{read}");
        Ended::Answered
    } else {
        let (_, data) = refusal(output);
        let text = |member: &str| data[member].as_str().unwrap_or_default().to_owned();
        Ended::Refused(text("code"), text("stage"))
    }
}

/// The fixture read through `connection` with `extra` arguments, not yet
/// started: the CLI in the jail, as [`Container::invoke`] runs it, with `env`
/// as its whole environment and its output captured.
fn reading(
    container: &Container,
    connection: &str,
    extra: &[&str],
    env: &[(&str, &str)],
) -> std::process::Command {
    let schema = container.schema();
    let target = format!("{CONNECTOR}/{ALIAS}");
    let mut command = container.jail.command("/bin-test/connectors", Map::Own);
    command
        .args(["--output", "json", "--config", CONFIG, "--state-dir", STATE])
        .args(["invoke", &target, READ, "--connection", connection])
        .args(["--schema", &schema, "--revision", REVISION])
        .args(["--input-json", "{}"])
        .args(extra)
        .env_clear()
        .envs(env.iter().copied())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

/// A container whose owner and adapter child run, the provider taking 1.5 s
/// for each read; the connection.
fn serving_slowly() -> (Container, String) {
    let (container, connection, _) = serving();
    assert!(container.seen("invocations").is_empty());
    container.set_knob("invoke_ms", 1_500);
    (container, connection)
}

/// Three reads of one instance started together, each with `--deadline-seconds
/// <seconds>`; how each ended.
fn three_together(container: &Container, connection: &str, seconds: &str) -> Vec<Ended> {
    let running: Vec<_> = (0..3)
        .map(|_| {
            reading(container, connection, &["--deadline-seconds", seconds], &[])
                .spawn()
                .unwrap()
        })
        .collect();
    running
        .into_iter()
        .map(|read| ended(&read.wait_with_output().unwrap()))
        .collect()
}

/// story:invoke-admission-queues-under-load. An instance's worker takes its
/// reads one at a time. Three reads of 1.5 s each, started together with
/// `--deadline-seconds 2`, cannot all have their turn: one still waiting in
/// the queue at its deadline is refused `timeout` at `admission`, and a read
/// refused there was never sent to the provider: the provider recorded every
/// read that answered and none but those and the reads refused at `dispatch`,
/// whose turn came before their deadline and whose answer did not.
#[test]
fn a_read_still_queued_at_its_named_deadline_is_refused_at_admission_and_never_sent() {
    let (container, connection) = serving_slowly();
    let ended = three_together(&container, &connection, "2");
    let count = |wanted: &Ended| ended.iter().filter(|read| *read == wanted).count();
    let answered = count(&Ended::Answered);
    let admission = count(&Ended::Refused("timeout".into(), "admission".into()));
    let dispatched = count(&Ended::Refused("timeout".into(), "dispatch".into()));
    assert_eq!(answered + admission + dispatched, 3, "{ended:?}");
    assert!(
        admission >= 1,
        "no read was refused timeout at admission: {ended:?}"
    );
    let sent = container.seen("invocations").len();
    assert!(
        (answered..=answered + dispatched).contains(&sent),
        "the provider recorded {sent} reads, {answered} answered and {admission} were refused at admission: {ended:?}"
    );
}

/// story:invoke-admission-queues-under-load. The same three reads with
/// `--deadline-seconds 20` wait for their turn: all three answer.
#[test]
fn reads_that_name_a_longer_deadline_wait_for_their_turn() {
    let (container, connection) = serving_slowly();
    let ended = three_together(&container, &connection, "20");
    assert_eq!(
        ended,
        [Ended::Answered, Ended::Answered, Ended::Answered],
        "a read was refused within 20 seconds"
    );
    assert_eq!(container.seen("invocations").len(), 3);
}

/// story:invoke-admission-queues-under-load. The owner start and the read
/// share one cutoff: an owner that takes 1.5 s to answer its greeting and a
/// read of 1.5 s do not fit `--deadline-seconds 2`. With a window each (2 s
/// for the start, then 2 s for the read) the call would answer. The stage is
/// `dispatch` when the cutoff passed after the read was sent to the provider,
/// and `admission` when it passed before, as on a host that takes longer
/// than the 0.5 s the start left.
#[test]
fn the_owner_start_and_the_read_share_one_deadline() {
    let (container, connection, _) = connected();
    container.set_knob("invoke_ms", 1_500);
    let output = Jail::output(&mut reading(
        &container,
        &connection,
        &["--deadline-seconds", "2"],
        &[("CONNECTORS_TEST_OWNER_START_DELAY_MS", "1500")],
    ));
    match ended(&output) {
        Ended::Refused(code, stage)
            if code == "timeout" && (stage == "dispatch" || stage == "admission") => {}
        other => panic!("the start and the read each had a window of their own: {other:?}"),
    }
}

/// story:invoke-admission-queues-under-load. A read whose owner never answers
/// its greeting ends at the deadline it names, with `timeout`, and nothing was
/// sent. (A read that waited on would be killed by the test runner at 60 s.)
#[test]
fn a_read_whose_owner_never_greets_ends_at_its_named_deadline() {
    let (container, connection, _) = connected();
    let output = Jail::output(&mut reading(
        &container,
        &connection,
        &["--deadline-seconds", "2"],
        &[("CONNECTORS_TEST_OWNER_START_DELAY_MS", "600000")],
    ));
    let read = ended(&output);
    // The owner it started still waits to greet: ended here, so the
    // container's drop does not wait for its stop to pass its bound.
    for owner in container.owners() {
        reap::kill_group(owner);
    }
    assert!(
        matches!(&read, Ended::Refused(code, _) if code == "timeout"),
        "{read:?}"
    );
    assert!(container.seen("invocations").is_empty());
}

//! story:owner-recovers-after-failed-acquisition. A consumer saw the owner that
//! a failed first `connect` started answer every later `invoke` with `timeout`
//! at `admission` after a retried `connect` succeeded, while `connections list`
//! said `ready`; only stopping that owner cleared it.
//!
//! The production CLI, owner and an adapter child against the read-only
//! `mounted_secret` binding in a jail (`support/mounted.rs`). A test build's
//! owner answers its first publication `outcome_unknown`, either without
//! committing (`CONNECTORS_TEST_PUBLICATION_FAILS_ONCE`) or after committing
//! (`CONNECTORS_TEST_PUBLICATION_REPLY_LOST_ONCE`), and keeps an acquisition in
//! flight for seconds, not four minutes (`CONNECTORS_TEST_ACQUISITION_MS`).
//! Linux only: the binding is.
#![cfg(target_os = "linux")]
#[path = "support/mounted.rs"]
mod mounted;
use connectors_custody::mounted::fixture::{Jail, Map};
use mounted::*;
use serde_json::{Value, json};
use std::{
    process::{Output, Stdio},
    time::{Duration, Instant},
};

const ADAPTER_FIXTURE: &str = "owner_recovers_after_failed_acquisition_adapter_fixture";
/// How long the failed attempt's acquisition stays in flight.
const WINDOW: Duration = Duration::from_secs(4);

/// The adapter child the owner starts: this binary, filtered to this test.
/// The ordinary run does nothing.
#[test]
fn owner_recovers_after_failed_acquisition_adapter_fixture() {
    serve_when_child();
}

/// The CLI in the jail as [`Container::run`] runs it, with the owner it
/// starts losing its first publication through `hook` and keeping an
/// acquisition in flight for [`WINDOW`].
fn failing(container: &Container, hook: &str, args: &[&str]) -> Output {
    Jail::output(
        container
            .jail
            .command("/bin-test/connectors", Map::Own)
            .args(["--output", "json", "--config", CONFIG, "--state-dir", STATE])
            .args(args)
            .env_clear()
            .env(hook, "/work/fail_once")
            .env(
                "CONNECTORS_TEST_ACQUISITION_MS",
                WINDOW.as_millis().to_string(),
            )
            .stdin(Stdio::null()),
    )
}

fn listed_states(container: &Container) -> Vec<Value> {
    let listed = success(&container.run(&["connections", "list", "--adapter", ALIAS]));
    listed["connections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|connection| connection["state"].clone())
        .collect()
}

/// A container whose first `connect`, run through `hook`, is `outcome_unknown`
/// at `publication`; the one owner that connect started is running.
fn failed_first_connect(hook: &str) -> (Container, Vec<i32>) {
    let container = Container::new(ADAPTER_FIXTURE);
    container.deliver(PROFILE, SECRET);
    container.configure("connectors-local/4", &Container::mounted());
    let init = success(&container.run(&["setup", "init"]));
    assert_eq!(init["disposition"], "state_initialized", "{init}");
    std::fs::write(container.path("/work/fail_once"), b"").unwrap();
    let (code, data) = refusal(&failing(
        &container,
        hook,
        &[
            "connections",
            "connect",
            "--adapter",
            ALIAS,
            "--profile",
            PROFILE,
        ],
    ));
    assert_eq!(
        (code, &data["code"], &data["stage"]),
        (Some(1), &json!("outcome_unknown"), &json!("publication")),
        "{data}"
    );
    let owners = container.owners();
    assert_eq!(owners.len(), 1, "the failed connect started one owner");
    (container, owners)
}

/// The invoke the same owner answers, and a list that agrees with it: the list
/// never says `ready` while the owner refuses the invoke.
fn admits_and_agrees(container: &Container, owners: &[i32], connection: &str) {
    let invoked = container.invoke(connection);
    let states = listed_states(container);
    if !invoked.status.success() {
        let (_, data) = refusal(&invoked);
        assert!(
            !states.contains(&json!("ready")),
            "list says ready while invoke is refused: {data} {states:?}"
        );
        panic!("the same owner refused the invoke: {data}; list {states:?}");
    }
    let read = success(&invoked);
    assert_eq!(read["result"]["credential"], digest(SECRET), "{read}");
    assert_eq!(states, [json!("ready")]);
    assert_eq!(container.owners(), owners, "no owner was restarted");
}

/// Steps 2 to 4 of the report, the publication not committed: the acquisition
/// stays in flight for the window, then fails; a retried `connect` is `ready`
/// on the same owner, which admits an `invoke`.
#[test]
fn a_connect_that_failed_publication_leaves_the_owner_admitting_work() {
    let (container, owners) = failed_first_connect("CONNECTORS_TEST_PUBLICATION_FAILS_ONCE");
    let until = Instant::now() + WINDOW + Duration::from_secs(20);
    let connected = loop {
        let attempt = container.connect(PROFILE);
        if attempt.status.success() {
            break success(&attempt);
        }
        assert!(
            Instant::now() < until,
            "no retried connect succeeded: {}",
            String::from_utf8_lossy(&attempt.stderr)
        );
        std::thread::sleep(Duration::from_millis(500));
    };
    let summary = &connected["connection"]["summary"];
    assert_eq!(summary["state"], "ready", "{connected}");
    assert_eq!(
        container.owners(),
        owners,
        "the retried connect reached the same owner"
    );
    admits_and_agrees(&container, &owners, summary["connection"].as_str().unwrap());
}

/// The publication committed and only its answer was lost: the list says
/// `ready` because the connection is, and the same owner admits the `invoke`
/// and a `revalidate` (the report's further observations).
#[test]
fn a_connect_whose_publication_answer_was_lost_leaves_the_owner_admitting_work() {
    let (container, owners) = failed_first_connect("CONNECTORS_TEST_PUBLICATION_REPLY_LOST_ONCE");
    let listed = success(&container.run(&["connections", "list", "--adapter", ALIAS]));
    let connection = listed["connections"][0]["connection"]
        .as_str()
        .unwrap()
        .to_owned();
    let revision = listed["connections"][0]["revision"]
        .as_str()
        .unwrap()
        .to_owned();
    admits_and_agrees(&container, &owners, &connection);
    let revalidated = success(&container.revalidate(&connection, &revision));
    assert_eq!(
        revalidated["connection"]["summary"]["state"], "ready",
        "{revalidated}"
    );
    admits_and_agrees(&container, &owners, &connection);
}

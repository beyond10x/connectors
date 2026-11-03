//! Security review of story:cold-owner-start-verifies-once: integrity
//! conformance for "a store that fails verification still refuses before any
//! provider call" on the paths the unit added. A read without `--connection`
//! chooses its connection from the owner's new `connections` listing, and a
//! read starts the owner first and falls back to the direct read when that
//! owner refuses. `cold_owner_start.rs` checks the refusal only for a cold read
//! that names its connection.
//!
//! The production CLI, owner and an adapter child against the read-only
//! `mounted_secret` binding in a jail (`support/mounted.rs`). Linux only: the
//! binding is.
#![cfg(target_os = "linux")]
#[path = "support/mounted.rs"]
mod mounted;
use connectors_custody::mounted::fixture::{Jail, Map};
use mounted::*;
use std::process::{Output, Stdio};

const ADAPTER_FIXTURE: &str = "cold_owner_start_security_adapter_fixture";

/// The adapter child the owner starts: this binary, filtered to this test.
/// The ordinary run does nothing.
#[test]
fn cold_owner_start_security_adapter_fixture() {
    serve_when_child();
}

/// A container with one ready connection, its description cached and no
/// owner running: its reference.
fn connected() -> (Container, String) {
    let container = Container::new(ADAPTER_FIXTURE);
    container.deliver(PROFILE, SECRET);
    container.configure("connectors-local/4", &Container::mounted());
    let init = success(&container.run(&["setup", "init"]));
    assert_eq!(init["disposition"], "state_initialized", "{init}");
    let connected = success(&container.connect(PROFILE));
    let connection = connected["connection"]["summary"]["connection"]
        .as_str()
        .unwrap()
        .to_owned();
    success(&container.run(&["setup", "owner-stop"]));
    (container, connection)
}

fn cli(container: &Container, args: &[&str]) -> Output {
    Jail::output(
        container
            .jail
            .command("/bin-test/connectors", Map::Own)
            .args(["--output", "json", "--config", CONFIG, "--state-dir", STATE])
            .args(args)
            .env_clear()
            .stdin(Stdio::null()),
    )
}

/// A target read; `connection` names its connection or leaves the choice to
/// the listing.
fn read(container: &Container, connection: Option<&str>) -> Output {
    let schema = container.schema();
    let target = format!("{CONNECTOR}/{ALIAS}");
    let mut args = vec!["invoke", target.as_str(), READ];
    if let Some(connection) = connection {
        args.extend(["--connection", connection]);
    }
    args.extend([
        "--schema",
        schema.as_str(),
        "--revision",
        REVISION,
        "--input-json",
        "{}",
    ]);
    cli(container, &args)
}

/// Zeroes every blob through SQLite, as the unit's own refusal case does: a
/// store that fails verification.
fn alter(container: &Container) {
    let store = rusqlite::Connection::open(container.path(STATE).join("metadata.sqlite3")).unwrap();
    let altered = store
        .execute(
            "UPDATE connectors_er_blobs SET bytes=zeroblob(length(bytes))",
            [],
        )
        .unwrap();
    assert!(altered > 0);
    store.close().unwrap();
}

/// `output` refuses `metadata_unavailable`, and the provider saw no call and
/// no validation beyond `invocations` and `validations`.
fn refused_before_any_provider_call(
    container: &Container,
    output: &Output,
    invocations: &[String],
    validations: &[String],
) {
    let (_, data) = refusal(output);
    assert_eq!(data["code"], "metadata_unavailable", "{data}");
    assert_eq!(container.seen("invocations"), invocations);
    assert_eq!(container.seen("validations"), validations);
}

/// A cold read without `--connection` on a store that fails verification:
/// the owner it starts refuses the store, the listing falls back to the
/// direct read, and that read refuses by its own code before any provider
/// call.
#[test]
fn a_cold_read_without_a_connection_on_an_altered_store_refuses_before_any_provider_call() {
    let (container, _) = connected();
    let validations = container.seen("validations");
    assert_eq!(container.seen("invocations"), Vec::<String>::new());
    alter(&container);
    let output = read(&container, None);
    refused_before_any_provider_call(&container, &output, &[], &validations);
}

/// A running owner, already past its verification, whose store is then
/// altered through SQLite: a read without `--connection` lists through the
/// owner's `connections` request, which the owner must refuse (a write
/// through SQLite makes its next read verify completely), and the read then
/// refuses by the direct read's code before any provider call.
#[test]
fn a_warm_read_without_a_connection_on_a_store_altered_after_the_owner_started_refuses() {
    let (container, connection) = connected();
    let first = success(&read(&container, None));
    assert_eq!(first["result"]["credential"], digest(SECRET), "{first}");
    let invocations = container.seen("invocations");
    let validations = container.seen("validations");
    assert_eq!(invocations.len(), 1, "{invocations:?}");
    assert!(
        container.path(STATE).join("owner.sock").exists(),
        "the read started no owner"
    );
    alter(&container);
    let output = read(&container, None);
    refused_before_any_provider_call(&container, &output, &invocations, &validations);
    // The same store, the read naming its connection.
    let output = read(&container, Some(&connection));
    refused_before_any_provider_call(&container, &output, &invocations, &validations);
}

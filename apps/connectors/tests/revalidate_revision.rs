//! story:connectors-do-short-invocation, decision D11: `connections revalidate
//! --expected-revision` names the revision it expects. A consumer gave the
//! instance's `configuration_revision` instead of the connection's revision and
//! was answered `lifecycle_conflict` / `retry_status`, which said neither which
//! revision was wanted nor where to read it.
//!
//! The production CLI, owner and an adapter child against the read-only
//! `mounted_secret` binding in a jail (`support/mounted.rs`), whose instance
//! `forge` is configured under `configuration_revision = 'cfg-1'`. Linux only:
//! the binding is.
#![cfg(target_os = "linux")]
#[path = "support/mounted.rs"]
mod mounted;
use mounted::*;
use serde_json::{Value, json};

const ADAPTER_FIXTURE: &str = "revalidate_revision_adapter_fixture";
/// The instance's configuration revision in `Container::configure`.
const CONFIGURATION_REVISION: &str = "cfg-1";

/// The adapter child the owner starts: this binary, filtered to this test.
#[test]
fn revalidate_revision_adapter_fixture() {
    serve_when_child();
}

/// A container with one connection: its reference and revision.
fn connected() -> (Container, String, String) {
    let container = Container::new(ADAPTER_FIXTURE);
    container.deliver(PROFILE, SECRET);
    container.configure("connectors-local/4", &Container::mounted());
    let init = success(&container.run(&["setup", "init"]));
    assert_eq!(init["disposition"], "state_initialized", "{init}");
    let connected = success(&container.connect(PROFILE));
    let summary = &connected["connection"]["summary"];
    (
        container,
        summary["connection"].as_str().unwrap().to_owned(),
        summary["revision"].as_str().unwrap().to_owned(),
    )
}

fn by_revision(container: &Container, verb: &str, connection: &str, revision: &str) -> Value {
    let (code, data) = refusal(&container.run(&[
        "connections",
        verb,
        "--adapter",
        ALIAS,
        "--connection",
        connection,
        "--expected-revision",
        revision,
    ]));
    json!({"exit": code, "data": data})
}

/// story:connectors-do-short-invocation: given the instance's
/// configuration_revision, revalidate is refused at the arguments, naming
/// `expected_revision`, the revision it expects (the connection's), the one it
/// was given (the configuration's) and the command that reads the expected
/// one. Nothing is validated.
#[test]
fn revalidate_given_the_configuration_revision_names_the_connection_revision() {
    let (container, connection, revision) = connected();
    assert_ne!(revision, CONFIGURATION_REVISION);
    let validations = container.seen("validations").len();
    let refused = by_revision(
        &container,
        "revalidate",
        &connection,
        CONFIGURATION_REVISION,
    );
    assert_eq!(refused["exit"], 2, "{refused}");
    let data = &refused["data"];
    assert_eq!(
        (
            &data["code"],
            &data["stage"],
            &data["field"],
            &data["revision_expected"],
            &data["revision_given"],
            &data["read_with"],
        ),
        (
            &json!("invalid_input"),
            &json!("arguments"),
            &json!("expected_revision"),
            &json!("connection"),
            &json!("configuration"),
            &json!("connections describe"),
        ),
        "{data}"
    );
    assert_eq!(container.seen("validations").len(), validations);
    // The connection's own revision still revalidates, naming nothing more.
    let revalidated = success(&container.revalidate(&connection, &revision));
    assert_eq!(
        revalidated["connection"]["summary"]["state"], "ready",
        "{revalidated}"
    );
}

/// story:connectors-do-short-invocation: any other revision that is not the
/// connection's current one keeps `lifecycle_conflict` / `retry_status` and
/// names the field, the revision expected and where to read it; it names no
/// given kind, since the value is no revision the instance holds.
#[test]
fn revalidate_given_a_stale_connection_revision_names_where_to_read_it() {
    let (container, connection, revision) = connected();
    let refused = by_revision(
        &container,
        "revalidate",
        &connection,
        "0000000000000000-stale",
    );
    assert_eq!(refused["exit"], 1, "{refused}");
    let data = &refused["data"];
    assert_eq!(
        (
            &data["code"],
            &data["next_action"],
            &data["field"],
            &data["revision_expected"],
            &data["read_with"],
        ),
        (
            &json!("lifecycle_conflict"),
            &json!("retry_status"),
            &json!("expected_revision"),
            &json!("connection"),
            &json!("connections describe"),
        ),
        "{data}"
    );
    assert!(data.get("revision_given").is_none(), "{data}");
    // `connections describe` reads the revision that is expected.
    let described = success(&container.run(&[
        "connections",
        "describe",
        "--adapter",
        ALIAS,
        "--connection",
        &connection,
    ]));
    assert_eq!(
        described["connection"]["summary"]["revision"], revision,
        "{described}"
    );
}

/// story:connectors-do-short-invocation: `connections revoke` takes its
/// `--expected-revision` in the same handler and names it the same way; the
/// connection stays unrevoked.
#[test]
fn revoke_given_another_revision_names_the_connection_revision() {
    let (container, connection, revision) = connected();
    let refused = by_revision(&container, "revoke", &connection, CONFIGURATION_REVISION);
    assert_eq!(refused["exit"], 2, "{refused}");
    let data = &refused["data"];
    assert_eq!(
        (
            &data["code"],
            &data["field"],
            &data["revision_expected"],
            &data["revision_given"],
            &data["read_with"],
        ),
        (
            &json!("invalid_input"),
            &json!("expected_revision"),
            &json!("connection"),
            &json!("configuration"),
            &json!("connections describe"),
        ),
        "{data}"
    );
    let refused = by_revision(&container, "revoke", &connection, "0000000000000000-stale");
    let data = &refused["data"];
    assert_eq!(
        (
            &data["code"],
            &data["field"],
            &data["revision_expected"],
            &data["read_with"]
        ),
        (
            &json!("lifecycle_conflict"),
            &json!("expected_revision"),
            &json!("connection"),
            &json!("connections describe"),
        ),
        "{data}"
    );
    assert!(data.get("revision_given").is_none(), "{data}");
    let revoked = success(&container.run(&[
        "connections",
        "revoke",
        "--adapter",
        ALIAS,
        "--connection",
        &connection,
        "--expected-revision",
        &revision,
    ]));
    assert_eq!(revoked["local_state"], "revoked", "{revoked}");
}

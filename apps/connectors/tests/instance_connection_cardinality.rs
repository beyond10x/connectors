//! story:connect-refused-while-instance-holds-connection. An instance holds at
//! most one published non-revoked connection, so `--connection` may be left
//! out where it holds exactly one: `connections describe`, `revalidate`,
//! `reconnect` and `revoke` then select it. With none the command is
//! `not_found` at admission naming a connect, written under `--output json` as
//! the one failure envelope on standard error. The production CLI, owner and
//! an adapter child against the read-only `mounted_secret` binding
//! (`support/mounted.rs`), which needs no Secret Service. Linux only: the
//! binding is.
#![cfg(target_os = "linux")]
#[path = "support/mounted.rs"]
mod mounted;
use mounted::*;
use serde_json::{Value, json};

const ADAPTER_FIXTURE: &str = "cardinality_adapter_fixture";

/// The adapter child the owner starts: this binary, filtered to this test.
/// The ordinary run does nothing.
#[test]
fn cardinality_adapter_fixture() {
    serve_when_child();
}

fn container() -> Container {
    let container = Container::new(ADAPTER_FIXTURE);
    container.deliver(PROFILE, SECRET);
    container.configure("connectors-local/4", &Container::mounted());
    let init = success(&container.run(&["setup", "init"]));
    assert_eq!(init["disposition"], "state_initialized", "{init}");
    container
}

fn summary(result: &Value) -> (String, String) {
    let summary = &result["connection"]["summary"];
    (
        summary["connection"].as_str().unwrap().to_owned(),
        summary["revision"].as_str().unwrap().to_owned(),
    )
}

#[test]
fn a_command_without_connection_selects_the_one_the_instance_holds() {
    let container = container();
    // None yet: the instance has nothing to select, and a connect makes it.
    let (code, data) = refusal(&container.run(&["connections", "describe", "--adapter", ALIAS]));
    assert_eq!(
        (code, &data["code"], &data["stage"], &data["next_action"]),
        (
            Some(1),
            &json!("not_found"),
            &json!("admission"),
            &json!("create_connection")
        ),
        "{data}"
    );

    let (connection, _) = summary(&success(&container.connect(PROFILE)));
    let described = success(&container.run(&["connections", "describe", "--adapter", ALIAS]));
    let (described_connection, revision) = summary(&described);
    assert_eq!(described_connection, connection, "{described}");

    let revalidated = success(&container.run(&[
        "connections",
        "revalidate",
        "--adapter",
        ALIAS,
        "--expected-revision",
        &revision,
    ]));
    let (revalidated_connection, revision) = summary(&revalidated);
    assert_eq!(revalidated_connection, connection, "{revalidated}");

    // The platform replaced the item: the reconnect publishes the new bytes.
    container.rotate(
        "..2026_10_16_00_00_00.000000002",
        b"fictional-mounted-token-0002",
    );
    let reconnected = success(&container.run(&[
        "connections",
        "reconnect",
        "--adapter",
        ALIAS,
        "--expected-revision",
        &revision,
    ]));
    let (reconnected_connection, revision) = summary(&reconnected);
    assert_eq!(reconnected_connection, connection, "{reconnected}");

    let revoked = success(&container.run(&[
        "connections",
        "revoke",
        "--adapter",
        ALIAS,
        "--expected-revision",
        &revision,
    ]));
    assert_eq!(revoked["connection"], connection.as_str(), "{revoked}");
    assert_eq!(revoked["local_state"], "revoked", "{revoked}");

    // A revoked connection is history: nothing is selected any longer, and
    // the next connect is admitted.
    let (code, data) = refusal(&container.run(&[
        "connections",
        "revoke",
        "--adapter",
        ALIAS,
        "--expected-revision",
        &revision,
    ]));
    assert_eq!(
        (code, &data["code"], &data["next_action"]),
        (Some(1), &json!("not_found"), &json!("create_connection")),
        "{data}"
    );
    let (fresh, _) = summary(&success(&container.connect(PROFILE)));
    assert_ne!(fresh, connection);
    let described = success(&container.run(&["connections", "describe", "--adapter", ALIAS]));
    assert_eq!(summary(&described).0, fresh, "{described}");
}

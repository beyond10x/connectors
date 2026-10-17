//! story:connect-refused-while-instance-holds-connection, adversary. The
//! contract section this story added (`contracts/cli/v1alpha1/semantics.md`,
//! "One connection per instance") states: `connections connect` is refused
//! while the selected instance holds a published Live or Stale connection, as
//! `lifecycle_conflict` at `admission`, and for one connection under the
//! configured binding its `next_action` is `repair_connection`, which
//! `connections reconnect` follows. Driven here through the production CLI,
//! owner and adapter child on the read-only `mounted_secret` binding
//! (`support/mounted.rs`), the one binding a headless run reaches. Linux only:
//! the binding is.
#![cfg(target_os = "linux")]
#[path = "support/mounted.rs"]
mod mounted;
use mounted::*;
use serde_json::{Value, json};

const ADAPTER_FIXTURE: &str = "cardinality_adversary_adapter_fixture";

/// The adapter child the owner starts: this binary, filtered to this test.
/// The ordinary run does nothing.
#[test]
fn cardinality_adversary_adapter_fixture() {
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

/// A second connect on an instance that holds its connection names the
/// reconnect that clears it.
#[test]
fn a_second_connect_names_the_reconnect() {
    let container = container();
    success(&container.connect(PROFILE));

    let (code, data) = refusal(&container.connect(PROFILE));
    assert_eq!(
        (code, &data["code"], &data["stage"], &data["next_action"]),
        (
            Some(1),
            &json!("lifecycle_conflict"),
            &json!("admission"),
            &json!("repair_connection")
        ),
        "a connect on an instance holding its connection: {data}"
    );
}

/// The step that refusal names, followed literally right after the connect:
/// `connections reconnect` of the selected connection succeeds and keeps its
/// reference.
#[test]
fn the_named_reconnect_succeeds_after_the_refused_connect() {
    let container = container();
    let (connection, _) = summary(&success(&container.connect(PROFILE)));
    refusal(&container.connect(PROFILE));

    // repair_connection: `connections reconnect`, the connection selected.
    let (_, revision) = summary(&success(&container.run(&[
        "connections",
        "describe",
        "--adapter",
        ALIAS,
    ])));
    let reconnected = success(&container.run(&[
        "connections",
        "reconnect",
        "--adapter",
        ALIAS,
        "--expected-revision",
        &revision,
    ]));
    assert_eq!(summary(&reconnected).0, connection, "{reconnected}");
}

//! Adversarial cases for `story:fixture-input-conformance`, written 2026-09-29 against the
//! uncommitted tree on base d215b3569.
//!
//! The unit's own cases call `kernel_refusal` and `fixture_values` directly. These drive the
//! same code through `connectors-build metadata-conformance run`, the function the new `gate`
//! step calls, with one-scenario suites planted for the purpose, and read the counts the run
//! reports and its exit status.

use serde_json::{Map, Value, json};
use std::path::Path;
use std::process::Command;

const COMMAND: &str = "connectors.execution_audit.AcknowledgeAnchor";

/// Every input of an admitted invocation anchor that satisfies `AuditRecord`'s invariants.
fn anchor_input() -> Map<String, Value> {
    let literal = |value: Value| json!({"kind": "literal", "value": value});
    let mut input = Map::new();
    for (name, value) in [
        ("anchor_kind", json!("admitted_execution")),
        ("activity", json!("invoke")),
        ("stage", json!("admission")),
        ("attempt_id", json!("00000000-0000-4000-8000-e1b424dd670e")),
        ("audit_ref", json!("audit_ref")),
        ("connection_ref", json!("connection_ref")),
        ("decision", json!("allow")),
        ("descriptor_revision", json!("descriptor_revision")),
        ("hop_role", json!("gateway")),
        ("instance_id", json!("instance_id")),
        ("operation_id", json!("operation_id")),
        ("principal_ref", json!("principal_ref")),
        ("recorded_at", json!("2020-01-01T00:00:00Z")),
        ("request_id", json!("request_id")),
    ] {
        input.insert(name.to_owned(), literal(value));
    }
    input
}

fn scenario(steps: Vec<Value>) -> Value {
    json!({
        "purpose": "planted by the adversary",
        "source": [{"kind": "command", "name": COMMAND}],
        "steps": steps,
    })
}

fn execute(input: Map<String, Value>) -> Value {
    json!({
        "step": "execute_command",
        "command": COMMAND,
        "actor": "connectors.execution_audit.LocalHost",
        "input": input,
    })
}

fn expect(outcome: &str) -> Value {
    json!({"step": "expect_outcome", "outcome": {"command": COMMAND, "outcome": outcome}})
}

/// Runs a one-scenario suite; returns whether the run exited 0 and the counts it printed.
fn run(name: &str, scenario: Value) -> (bool, Value) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = tempfile::tempdir().unwrap();
    let suite = dir.path().join("suite.json");
    let document = json!({
        "provenance": {
            "suite_version": "ess-conformance/18",
            "system": "connectors",
            "specification_version": "v1",
            "spec_digest": "0".repeat(64),
            "contract_digest": "0".repeat(64),
            "component": "local-metadata-authority",
        },
        "scenarios": {format!("{COMMAND}/outcome/{name}"): scenario},
    });
    std::fs::write(&suite, serde_json::to_vec(&document).unwrap()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_connectors-build"))
        .arg("--root")
        .arg(&root)
        .args(["metadata-conformance", "run", "--suite"])
        .arg(&suite)
        .arg("--report-out")
        .arg(dir.path().join("report.json"))
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let counts = stdout
        .lines()
        .find_map(|line| serde_json::from_str::<Value>(line).ok())
        .unwrap_or_else(|| {
            panic!(
                "no counts line; stdout {stdout} stderr {}",
                String::from_utf8_lossy(&output.stderr)
            )
        });
    (output.status.success(), counts)
}

/// The control: the planted scenarios below fail for what they plant, not for their setup.
#[test]
fn a_well_formed_anchor_passes() {
    let (ok, counts) = run(
        "control",
        scenario(vec![execute(anchor_input()), expect("anchored")]),
    );
    assert!(ok, "{counts}");
    assert_eq!(counts["passed"], 1, "{counts}");
}

/// The brief's plant: one scenario whose expectation the implementation does not meet must
/// fail the run the gate step makes.
#[test]
fn a_planted_wrong_outcome_fails_the_run() {
    let (ok, counts) = run(
        "wrong-outcome",
        scenario(vec![execute(anchor_input()), expect("capacity")]),
    );
    assert!(!ok, "{counts}");
    assert_eq!(counts["failed"], 1, "{counts}");
}

/// A fixture name the provider does not declare errors the scenario and fails the run.
#[test]
fn an_unknown_fixture_name_errors_the_run() {
    let mut input = anchor_input();
    input.insert(
        "stage".to_owned(),
        json!({"kind": "fixture", "fixture": "no-such-fixture"}),
    );
    let resolve = json!({
        "step": "resolve_fixtures",
        "fixtures": {
            "fields": [{"name": "no-such-fixture", "type": "connectors.execution_audit.AnchorStage"}],
            "declarations": {"connectors.execution_audit.AnchorStage": {
                "kind": "enum", "variants": ["authentication", "decoding", "admission"]}},
        },
    });
    let (ok, counts) = run(
        "unknown-fixture",
        scenario(vec![resolve, execute(input), expect("anchored")]),
    );
    assert!(!ok, "{counts}");
    assert_eq!(counts["error"], 1, "{counts}");
    assert_eq!(counts["unsupported"], 0, "{counts}");
}

/// An admitted anchor without a principal violates `AuditRecord`'s first invariant; the unit
/// scores the kernel's `InvariantViolation` as a failed scenario.
#[test]
fn an_invariant_violation_is_failed_through_the_runner() {
    let mut input = anchor_input();
    input.remove("principal_ref");
    let (ok, counts) = run(
        "violation",
        scenario(vec![execute(input), expect("anchored")]),
    );
    assert!(!ok, "{counts}");
    assert_eq!(counts["failed"], 1, "{counts}");
    assert_eq!(counts["unsupported"], 0, "{counts}");
}

/// An early-refusal anchor with an operation and no activity, which the host's own
/// `Anchor::validate` admits (`connectors-host/src/local/audit/types.rs:141-153`). The kernel
/// discards the state with `InvariantUnobservable` on `AuditRecord`'s third invariant
/// (`not activity == describe` over an absent `activity`) — "exactly as for a violation", in
/// `entity-core` 0.25.1's own words. The unit's rationale for scoring a violation as failed
/// ("the resulting state was discarded, no declared outcome was reached and nothing was
/// written") holds word for word, yet `kernel_refusal` matches only `InvariantViolation`, so
/// this scenario is reported unsupported instead of failed.
#[test]
fn an_unobservable_invariant_is_failed_as_a_violation_is() {
    let mut input = anchor_input();
    input.remove("activity");
    input.insert(
        "anchor_kind".to_owned(),
        json!({"kind": "literal", "value": "early_refusal"}),
    );
    let (ok, counts) = run(
        "unobservable",
        scenario(vec![execute(input), expect("anchored")]),
    );
    assert!(!ok, "{counts}");
    assert_eq!(counts["unsupported"], 0, "{counts}");
    assert_eq!(counts["failed"], 1, "{counts}");
}

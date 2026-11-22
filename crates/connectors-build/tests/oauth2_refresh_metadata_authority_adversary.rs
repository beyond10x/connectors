//! Adversarial cases for `story:oauth2-refresh-metadata-authority`, written 2026-10-25 against
//! 95b65bc0a (base ca5071ada).
//!
//! The unit lowers `connectors.refresh` and `connectors.oauth2` into the host's committed Entity
//! Runtime definitions and adds them to the `local-metadata-authority` component. These cases
//! drive what it committed through `connectors-build metadata-conformance run` and `emit`, the
//! functions its acceptance names, and through `entity-core` over the committed definitions.

use serde_json::{Map, Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;

const RESERVE: &str = "connectors.refresh.ReserveRefresh";
const SOURCE_GENERATION: &str = "00000000-0000-4000-8000-1fde3096cd25";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn literal(value: Value) -> Value {
    json!({"kind": "literal", "value": value})
}

/// The `reserved` scenario the gate synthesizes for `ReserveRefresh`, with `view_fields` as the
/// `RefreshStates` row it expects.
fn reserved_scenario(view_fields: Map<String, Value>) -> Value {
    let mut input = Map::new();
    for (name, value) in [
        ("decision", json!("allow")),
        (
            "expected_binding_revision",
            json!("expected_binding_revision"),
        ),
        ("owner", json!("owner")),
        (
            "ownership_token",
            json!("00000000-0000-4000-8000-a42bc572f1f0"),
        ),
        ("source_generation_id", json!(SOURCE_GENERATION)),
    ] {
        input.insert(name.to_owned(), literal(value));
    }
    json!({
        "purpose": "planted by the adversary",
        "source": [{"kind": "command", "name": RESERVE}],
        "steps": [
            {"step": "execute_command", "command": RESERVE, "input": input},
            {"step": "expect_outcome", "outcome": {"command": RESERVE, "outcome": "reserved"}},
            {"step": "expect_event", "event": "connectors.refresh.RefreshReserved",
             "shape": {"attempt_id": {"holds": "primitive", "kind": "uuid"}}},
            {"step": "query_view", "view": "connectors.refresh.RefreshStates"},
            {"step": "expect_view", "view": "connectors.refresh.RefreshStates",
             "expectation": {"expect": "contains", "fields": view_fields}},
        ],
    })
}

fn reserved_row() -> Map<String, Value> {
    let mut fields = Map::new();
    fields.insert(
        "attempt_id".to_owned(),
        json!({"kind": "observed", "event": "connectors.refresh.RefreshReserved", "field": "attempt_id"}),
    );
    fields.insert("state".to_owned(), literal(json!("Reserved")));
    fields
}

/// Runs a one-scenario suite through `metadata-conformance run`; returns whether it exited 0
/// and the counts it printed.
fn run(name: &str, scenario: Value) -> (bool, Value) {
    let dir = tempfile::tempdir().unwrap();
    let suite = dir.path().join("suite.json");
    let document = json!({
        "provenance": {
            "suite_version": "ess-conformance/34",
            "system": "connectors",
            "specification_version": "v1",
            "spec_digest": "0".repeat(64),
            "contract_digest": "0".repeat(64),
            "component": "local-metadata-authority",
            "scenario_initial_state": "empty",
        },
        "scenarios": {format!("{RESERVE}/outcome/{name}"): scenario},
    });
    std::fs::write(&suite, serde_json::to_vec(&document).unwrap()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_connectors-build"))
        .arg("--root")
        .arg(root())
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

/// The control: the synthesized `reserved` expectation passes through the planted suite, so the
/// case below fails for the field it adds, not for its setup.
#[test]
fn the_synthesized_reserved_row_passes_as_planted() {
    let (ok, counts) = run("control", reserved_scenario(reserved_row()));
    assert!(ok, "{counts}");
    assert_eq!(counts["passed"], 1, "{counts}");
}

/// `RefreshAttempt` is "one rotating refresh per credential generation" (`refresh.yaml`
/// summary), and `RefreshStates` shows each attempt's `source_generation_id`. The reservation
/// names that generation in its input, yet `reserved` has no `sets:` (`refresh.yaml:114-121`), so
/// the lowering makes `source_generation_id`, `initial_owner`, `initial_ownership_token` and
/// `expected_binding_revision` required `undetermined` slots (`entity-runtime-definitions.json`,
/// `ReserveRefresh` slots b00000002-b00000005): the authority records a value of its own
/// choosing, and the gate's suite never compares it with the input.
#[test]
fn a_reserved_attempt_records_the_source_generation_its_reservation_names() {
    let mut row = reserved_row();
    row.insert(
        "source_generation_id".to_owned(),
        literal(json!(SOURCE_GENERATION)),
    );
    let (ok, counts) = run("source-generation", reserved_scenario(row));
    assert!(
        ok,
        "the reserved attempt does not record the reservation's source generation: {counts}"
    );
    assert_eq!(counts["passed"], 1, "{counts}");
}

/// The emission of the component scoped to `domain` (its `local-metadata-authority` owns only
/// `domain` and accepts only its commands, every mutant class), as
/// `connectors-build metadata-conformance emissions` committed it. Emitting it takes ESS minutes,
/// so it is made once; the unit test `the_committed_metadata_emissions_are_current` holds it to
/// the current `ess/`, ESS crates and pin, and the `slow` workflow regenerates it byte for byte.
fn admitted_scoped_manifest(domain: &str) -> ess_conformance::mutate::Manifest {
    let path = root().join(format!(
        "crates/connectors-build/generated/metadata-emissions/{domain}/manifest.json"
    ));
    let text = std::fs::read_to_string(&path).unwrap();
    let manifest = ess_conformance::mutate::Manifest::from_json(&text)
        .unwrap_or_else(|error| panic!("the pinned ESS refuses {}: {error}", path.display()));
    assert_eq!(
        manifest.component.as_deref(),
        Some("local-metadata-authority")
    );
    assert!(
        !manifest.mutants.is_empty(),
        "{domain}: the emission holds no mutant"
    );
    manifest
}

/// The acceptance names `metadata_conformance::emit` as the emission `mutate --collect` scores
/// for `connectors.refresh` and `connectors.oauth2`. `emit` wrote `ess-mutation-manifest/2`, and
/// the pinned ESS 0.97.3 reader refuses a `/2` manifest holding a `sets-drop`, `precedence-swap`
/// or `emit-swap` mutant (`ess-conformance` `mutate.rs:185-193`). `IssueAccessToken`'s four
/// precedence swaps are such mutants.
#[test]
fn emit_writes_a_manifest_for_the_oauth2_domain_that_the_pinned_ess_admits() {
    admitted_scoped_manifest("connectors.oauth2");
}

/// The same refusal over `connectors.approval_issuers`, which the component owned before this
/// unit and whose source is unchanged since d2842557d: `emit` failed there too, so the defect
/// predated the unit.
#[test]
fn emit_writes_a_manifest_for_the_approval_issuers_domain_that_the_pinned_ess_admits() {
    admitted_scoped_manifest("connectors.approval_issuers");
}

#[derive(serde::Deserialize)]
struct Bundle {
    definitions: Vec<BundleEntry>,
}

#[derive(serde::Deserialize)]
struct BundleEntry {
    definition: entity_core::EntityDefinition,
}

fn committed_registry() -> entity_core::Registry {
    let bundle: Bundle = serde_json::from_slice(
        &std::fs::read(
            root()
                .join("crates/connectors-host/src/local/metadata/entity-runtime-definitions.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let mut registry = entity_core::Registry::new();
    for entry in bundle.definitions {
        registry.register(entry.definition).unwrap();
    }
    registry.validate_all().unwrap();
    registry
}

/// `issued-current` answers from the published generation without a refresh (`oauth2.yaml`
/// `IssueAccessToken` comment, and its event says `refreshed: false`), and sets no
/// `refresh_attempt_id`. The lowering makes that field an optional `undetermined` slot
/// (b00000001) instead of absent, so the committed definitions accept an issue that names a
/// refresh attempt it never ran. Built here by supplying the slot; no host code supplies it yet.
#[test]
fn an_issued_current_access_token_names_no_refresh_attempt() {
    let registry = committed_registry();
    let runtime = entity_core::Runtime::new(&registry);
    let arguments = json!({
        "input": {
            "decision": "current",
            "connection_ref": "connection",
            "generation_id": "00000000-0000-4000-8000-5a183923d085",
            "expires_at": "2020-01-01T00:00:00Z",
            "scopes": ["read"],
            "delivery": "metadata_only",
        },
        "bound": {
            "b00000000": "issue",
            "b00000001": "00000000-0000-4000-8000-08976e317bc2",
        },
    });
    let evaluation =
        match runtime.decide_create_derived("connectors.oauth2.AccessTokenIssue", 1, arguments) {
            Ok(evaluation) => evaluation,
            // The definitions declare no slot for it: the supplied attempt is refused before
            // any outcome is selected, so no issue can record it.
            Err(entity_core::CoreError::Validation(errors))
                if !errors.is_empty()
                    && errors
                        .iter()
                        .all(|error| error.path == "arguments.bound.b00000001") =>
            {
                return;
            }
            Err(error) => panic!("kernel error: {error:?}"),
        };
    let entity_core::Evaluation::Accepted(decision) = evaluation else {
        // Refusing the supplied attempt is a correct answer too.
        return;
    };
    assert_eq!(decision.record.outcome.as_deref(), Some("issued-current"));
    assert_eq!(
        decision.instance.fields.get("refresh_attempt_id"),
        None,
        "an issued-current issue records a refresh attempt"
    );
}

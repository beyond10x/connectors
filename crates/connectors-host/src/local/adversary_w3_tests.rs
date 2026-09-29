//! Adversary cases for story:resolve-spec-markers (wave 3 unit C).
//!
//! These drive the regenerated Entity Runtime definitions the host embeds
//! (`metadata/entity-runtime-definitions.json`) through the kernel's own
//! `decide`, against the two wrong-state refusals the unit declared in
//! `ess/domains/auth_bindings.yaml` and `ess/domains/cli.yaml`.
use entity_core::{EntityDefinition, EntityInstance, Evaluation, Registry, decide};
use serde_json::{Map, Value, json};

#[derive(serde::Deserialize)]
struct Bundle {
    definitions: Vec<Entry>,
}

#[derive(serde::Deserialize)]
struct Entry {
    definition: EntityDefinition,
}

fn registry() -> Registry {
    let bundle: Bundle =
        serde_json::from_slice(include_bytes!("metadata/entity-runtime-definitions.json"))
            .expect("bundled definitions parse");
    let mut registry = Registry::new();
    for entry in bundle.definitions {
        registry.register(entry.definition).expect("registers");
    }
    registry.validate_all().expect("validates");
    registry
}

fn instance(entity: &str, state: &str) -> EntityInstance {
    EntityInstance {
        entity: entity.into(),
        version: 1,
        id: "fixture".into(),
        lifecycle_state: state.into(),
        revision: 3,
        fields: Map::new(),
    }
}

fn refusal_error(evaluation: Evaluation) -> String {
    match evaluation {
        Evaluation::Refused(refusal) => refusal.error,
        Evaluation::Accepted(decision) => panic!("accepted, not refused: {decision:?}"),
    }
}

fn publish(decision: &str) -> Value {
    json!({
        "bound": {"b00000000": "fence-2"},
        "input": {
            "connection_ref": "fixture",
            "decision": decision,
            "publication": {
                "connection_ref": "fixture",
                "expected_publication_fence": "fence-1",
                "external_identity": {"kind": "account", "subject": "subject-1"},
                "profile_record_ref": "profile-1"
            }
        }
    })
}

const CONNECTION: &str = "connectors.auth_bindings.Connection";
const PUBLISH: &str = "connectors.auth_bindings.PublishBinding";
const CURSOR: &str = "connectors.cli.ConnectionListCursor";
const EXPIRE: &str = "connectors.cli.ExpireConnectionListCursor";

#[test]
fn publish_binding_allowed_on_a_revoked_connection_refuses_with_connection_state_conflict() {
    let registry = registry();
    let definition = registry.get(CONNECTION, 1).unwrap();
    // A recorded Connection always holds its fence; PublishBinding compares it first.
    let mut revoked = instance(CONNECTION, "Revoked");
    revoked
        .fields
        .insert("publication_fence".into(), json!("fence-1"));
    let evaluation =
        decide(definition, &revoked, PUBLISH, publish("allow")).expect("the kernel answers");
    assert_eq!(
        refusal_error(evaluation),
        "connectors.auth_bindings.ConnectionStateConflict"
    );
}

/// ess/domains/auth_bindings.yaml:266 says, without qualification, that
/// "PublishBinding on a Revoked Connection refuses with ConnectionStateConflict".
/// The kernel answers the input guard before the held state, so on a Revoked
/// connection only `decision: allow` reaches the wrong-state branch.
#[test]
fn expiring_an_expired_cursor_refuses_with_cursor_state_conflict() {
    let registry = registry();
    let definition = registry.get(CURSOR, 1).unwrap();
    let evaluation = decide(
        definition,
        &instance(CURSOR, "Expired"),
        EXPIRE,
        json!({"bound": {}, "input": {"cursor_id": "fixture"}}),
    )
    .expect("the kernel answers");
    assert_eq!(
        refusal_error(evaluation),
        "connectors.cli.CursorStateConflict"
    );
}

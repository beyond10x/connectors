//! Required host effects and the five beyond-HTTP axes are closed declaration facts.

use connector_spec::{provider, HostEffect, OperationDirection, SemanticEffect};

fn operation(facts: &str) -> String {
    format!(
        r#"id = "acme"
vendor = "Acme"
base_url = "https://api.acme.test"

[[operations]]
id = "acme-read"
method = "GET"
direction = "read"
path = "/v1/items"
risk = "low"
idempotency = "idempotent"
{facts}
"#
    )
}

const AXES: &str = r#"interaction_shape = "unary"
protocol_driver = "http_v1"
placement_requirement = "connectors_deployment"
implementation_form = "built_in"
required_capabilities = ["public_network"]"#;

#[test]
fn effects_are_required_and_never_derived_from_http() {
    let error = provider::load("providers/acme.toml", &operation(AXES)).unwrap_err();
    assert!(
        error.to_string().contains("missing field `effects`"),
        "{error}"
    );
}

#[test]
fn unknown_effect_driver_and_capability_values_are_refused_by_name() {
    for (field, unknown) in [
        ("effects = [\"telepathy\"]", "telepathy"),
        (
            "effects = [\"read\", \"network\"]\nprotocol_driver = \"raw_tcp\"",
            "raw_tcp",
        ),
        (
            "effects = [\"read\", \"network\"]\nrequired_capabilities = [\"ambient_root\"]",
            "ambient_root",
        ),
    ] {
        let facts = if field.contains("protocol_driver") {
            AXES.replace("protocol_driver = \"http_v1\"", field)
        } else if field.contains("required_capabilities") {
            AXES.replace("required_capabilities = [\"public_network\"]", field)
        } else {
            format!("{field}\n{AXES}")
        };
        let error = provider::load("providers/acme.toml", &operation(&facts)).unwrap_err();
        assert!(error.to_string().contains(unknown), "{error}");
    }
}

#[test]
fn host_and_semantic_effects_remain_independent_axes() {
    let facts =
        format!("effects = [\"read\", \"network\"]\nsemantic_effects = [\"money\"]\n{AXES}");
    let loaded = provider::load("providers/acme.toml", &operation(&facts)).unwrap_err();
    assert!(loaded.to_string().contains("money"));

    let facts = format!("effects = [\"read\", \"network\"]\nsemantic_effects = [\"read\"]\n{AXES}");
    let loaded = provider::load("providers/acme.toml", &operation(&facts)).unwrap();
    assert_eq!(
        loaded.connector.operations[0].effects,
        [HostEffect::Read, HostEffect::Network]
    );
    assert_eq!(
        loaded.connector.operations[0].semantic_effects,
        [SemanticEffect::Read]
    );
}

#[test]
fn a_seeded_write_remains_write_and_carries_write_effects() {
    let source = operation(&format!("effects = [\"write\", \"network\"]\n{AXES}"))
        .replace("id = \"acme-read\"", "id = \"acme-write\"")
        .replace("method = \"GET\"", "method = \"POST\"")
        .replace("direction = \"read\"", "direction = \"write\"")
        .replace("risk = \"low\"", "risk = \"high\"")
        .replace(
            "idempotency = \"idempotent\"",
            "idempotency = \"non_idempotent\"",
        );
    let loaded = provider::load("providers/acme.toml", &source).expect("seeded write loads");
    let operation = &loaded.connector.operations[0];
    assert_eq!(operation.direction, OperationDirection::Write);
    assert_eq!(operation.effects, [HostEffect::Write, HostEffect::Network]);
}

#[test]
fn predecessor_runtime_and_quirks_vocabularies_are_refused_by_name() {
    let runtime = operation(&format!("effects = [\"read\", \"network\"]\n{AXES}")).replacen(
        "vendor = \"Acme\"",
        "vendor = \"Acme\"\nruntime = \"http\"",
        1,
    );
    let error = provider::load("providers/acme.toml", &runtime)
        .unwrap_err()
        .to_string();
    assert!(error.contains("unknown field `runtime`"), "{error}");

    let quirks = operation(&format!(
        "effects = [\"read\", \"network\"]\nquirks = {{ rate_limit = {{ requests = 1, per_seconds = 1 }} }}\n{AXES}"
    ));
    let error = provider::load("providers/acme.toml", &quirks)
        .unwrap_err()
        .to_string();
    assert!(error.contains("unknown field `quirks`"), "{error}");
}

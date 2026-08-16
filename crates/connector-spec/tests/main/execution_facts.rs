//! Required host effects and the five beyond-HTTP axes are closed declaration facts.

use connector_spec::{
    provider, HostEffect, InteractionShape, OperationDirection, ProtocolDriver, SemanticEffect,
};

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
fn sip_v1_is_a_closed_session_establishment_driver() {
    let axes = AXES
        .replace(
            "interaction_shape = \"unary\"",
            "interaction_shape = \"session_establishment\"",
        )
        .replace(
            "protocol_driver = \"http_v1\"",
            "protocol_driver = \"sip_v1\"",
        );
    let source = operation(&format!("effects = [\"write\", \"network\"]\n{axes}"))
        .replace("id = \"acme-read\"", "id = \"acme-call-establish\"")
        .replace("method = \"GET\"\n", "")
        .replace("direction = \"read\"", "direction = \"write\"")
        .replace("path = \"/v1/items\"\n", "")
        .replace("risk = \"low\"", "risk = \"high\"")
        .replace(
            "idempotency = \"idempotent\"",
            "idempotency = \"non_idempotent\"",
        );
    let loaded = provider::load("providers/acme.toml", &source).expect("SIP operation loads");
    let operation = &loaded.connector.operations[0];
    assert_eq!(
        operation.interaction_shape,
        InteractionShape::SessionEstablishment
    );
    assert_eq!(operation.request.driver(), ProtocolDriver::SipV1);
    assert!(operation.request.http_method().is_none());
    assert!(operation.request.http_path().is_none());

    let schema = include_str!("../../schema/provider-toml.schema.json");
    assert!(
        schema.contains(r#""enum": ["http_v1", "sip_v1", "audio_v1", "cdp_v1"]"#),
        "the authored-provider schema must admit the same closed driver vocabulary"
    );
}

#[test]
fn audio_v1_is_a_closed_unary_device_driver() {
    let axes = AXES.replace(
        "protocol_driver = \"http_v1\"",
        "protocol_driver = \"audio_v1\"",
    );
    let source = operation(&format!("effects = [\"read\", \"local_system\"]\n{axes}"))
        .replace("id = \"acme-read\"", "id = \"acme-speech-status\"")
        .replace("method = \"GET\"\n", "")
        .replace("path = \"/v1/items\"\n", "")
        .replace(
            "required_capabilities = [\"public_network\"]",
            "required_capabilities = [\"device\"]",
        );
    let loaded = provider::load("providers/acme.toml", &source).expect("audio operation loads");
    let operation = &loaded.connector.operations[0];
    assert_eq!(operation.interaction_shape, InteractionShape::Unary);
    assert_eq!(operation.request.driver(), ProtocolDriver::AudioV1);
    assert!(operation.request.http_method().is_none());
    assert!(operation.request.http_path().is_none());

    // A device answers one bounded request and returns; no other lifecycle is admitted, and the
    // HTTP-only request fields remain structurally impossible.
    let session = source.replace(
        "interaction_shape = \"unary\"",
        "interaction_shape = \"session_establishment\"",
    );
    let error = provider::load("providers/acme.toml", &session)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("audio v1 is admitted only as `unary`"),
        "{error}"
    );

    let with_method = source.replace(
        "direction = \"read\"",
        "method = \"GET\"\ndirection = \"read\"",
    );
    let error = provider::load("providers/acme.toml", &with_method)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("audio_v1 operation refuses HTTP-only `method` and `path`"),
        "{error}"
    );
}

#[test]
fn cdp_v1_is_a_closed_leased_session_browser_driver() {
    let axes = AXES
        .replace(
            "interaction_shape = \"unary\"",
            "interaction_shape = \"leased_session\"",
        )
        .replace(
            "protocol_driver = \"http_v1\"",
            "protocol_driver = \"cdp_v1\"",
        )
        .replace(
            "required_capabilities = [\"public_network\"]",
            "required_capabilities = [\"public_network\", \"process\"]",
        );
    let source = operation(&format!("effects = [\"read\", \"browser\"]\n{axes}"))
        .replace("id = \"acme-read\"", "id = \"acme-browser-snapshot\"")
        .replace("method = \"GET\"\n", "")
        .replace("path = \"/v1/items\"\n", "");
    let loaded = provider::load("providers/acme.toml", &source).expect("browser operation loads");
    let operation = &loaded.connector.operations[0];
    assert_eq!(operation.interaction_shape, InteractionShape::LeasedSession);
    assert_eq!(operation.request.driver(), ProtocolDriver::CdpV1);
    assert!(operation.request.http_method().is_none());
    assert!(operation.request.http_path().is_none());

    // A browser is held across calls. `unary` would deny that the process, profile and page
    // survive between operations; `session_establishment` would promise a direct-byte plane that
    // does not exist. Neither is admitted, and the HTTP-only request fields stay structurally
    // impossible.
    for denied in ["unary", "session_establishment", "stream", "subscription"] {
        let wrong = source.replace(
            "interaction_shape = \"leased_session\"",
            &format!("interaction_shape = {denied:?}"),
        );
        let error = provider::load("providers/acme.toml", &wrong)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("CDP v1 is admitted only as `leased_session`"),
            "{denied} was admitted: {error}"
        );
    }

    let with_path = source.replace(
        "direction = \"read\"",
        "path = \"/v1/items\"\ndirection = \"read\"",
    );
    let error = provider::load("providers/acme.toml", &with_path)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("cdp_v1 operation refuses HTTP-only `method` and `path`"),
        "{error}"
    );
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

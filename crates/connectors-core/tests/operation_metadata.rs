use connectors_core::operation_metadata::{Error, Metadata};
use serde_json::{Value, json};

fn read() -> Value {
    json!({
        "effects":["network"], "semantic_effects":[], "risk":"low",
        "idempotency":{"kind":"none"}, "approval":"not_required",
        "limits":{"request_bytes":65536,"result_bytes":4194304,
            "execution_ms":20000,"provider_ms":15000,"connect_ms":5000}
    })
}
fn mutation() -> Value {
    let mut value = read();
    value["effects"] = json!(["external_write", "network"]);
    value["semantic_effects"] = json!(["human_visible"]);
    value["risk"] = json!("medium");
    value["approval"] = json!("required");
    value["idempotency"] =
        json!({"kind":"keyed","key":"caller_supplied","retention_seconds":86400});
    value
}
fn parse(value: &Value, profile: &str) -> Result<Metadata, Error> {
    Metadata::parse(&serde_json::to_vec(value).unwrap(), profile, 65536)
}

#[test]
fn operation_metadata_canonical_roundtrip_retains_all_declared_requirements() {
    for (mut value, profile) in [(read(), "read"), (mutation(), "mutation")] {
        value["requires_auth"] = json!([{"profile":"oauth","scopes":["read:objects"]}]);
        value["realization"] = json!("implemented");
        assert_eq!(parse(&value, profile).unwrap().to_value().unwrap(), value);
    }
}

#[test]
fn operation_metadata_required_fields_and_closed_shapes_cannot_be_guessed() {
    let original = read();
    for field in original.as_object().unwrap().keys() {
        let mut bad = original.clone();
        bad.as_object_mut().unwrap().remove(field);
        assert!(parse(&bad, "read").is_err(), "missing {field}");
    }
    for pointer in ["", "/idempotency", "/limits"] {
        let mut bad = read();
        bad.pointer_mut(pointer).unwrap()["secret_reference"] = json!("private");
        assert!(parse(&bad, "read").is_err(), "unknown member at {pointer}");
    }
    let bytes = serde_json::to_vec(&original).unwrap();
    assert!(Metadata::parse(&bytes, "read", bytes.len()).is_ok());
    assert_eq!(
        Metadata::parse(&bytes, "read", bytes.len() - 1).unwrap_err(),
        Error::Size
    );
    let duplicate = String::from_utf8(bytes)
        .unwrap()
        .replacen("{", "{\"risk\":\"high\",", 1);
    assert_eq!(
        Metadata::parse(duplicate.as_bytes(), "read", 65536).unwrap_err(),
        Error::Json
    );
}

#[test]
fn operation_metadata_effect_discriminator_cannot_be_replaced_by_a_hint() {
    assert!(parse(&read(), "read").is_ok());
    assert!(parse(&mutation(), "mutation").is_ok());
    for (effects, profile) in [
        (json!(["external_write"]), "read"),
        (json!(["network"]), "mutation"),
        (json!(["send_external"]), "read"),
        (json!(["session_establishment"]), "read"),
        (json!(["network", "network"]), "read"),
        (json!(["human_visible"]), "read"),
    ] {
        let mut bad = read();
        bad["effects"] = effects;
        assert!(parse(&bad, profile).is_err(), "{profile}: {bad}");
    }
    for effects in [json!(["network"]), json!(["billable", "billable"])] {
        let mut bad = read();
        bad["semantic_effects"] = effects;
        assert!(parse(&bad, "read").is_err());
    }
}

#[test]
fn operation_metadata_keyed_shape_is_exact_and_does_not_invent_retention() {
    let mut value = mutation();
    for bad in [
        json!({"kind":"keyed"}),
        json!({"kind":"keyed","key":"caller_supplied","retention_seconds":86399}),
        json!({"kind":"keyed","key":"provider","retention_seconds":86400}),
        json!({"kind":"none","key":"caller_supplied"}),
        json!({"kind":"natural","retention_seconds":86400}),
    ] {
        value["idempotency"] = bad;
        assert!(parse(&value, "mutation").is_err(), "{value}");
    }
    for kind in ["none", "natural"] {
        value["idempotency"] = json!({"kind":kind});
        assert!(parse(&value, "mutation").is_ok());
    }
    let mut bad = read();
    bad["idempotency"] = mutation()["idempotency"].clone();
    assert_eq!(parse(&bad, "read").unwrap_err(), Error::Idempotency);
}

#[test]
fn operation_metadata_lossless_numbers_reject_spoofs_and_noninteger_limits() {
    let mut value = read();
    value["limits"]["request_bytes"] = json!(9_007_199_254_740_993u64);
    assert_eq!(parse(&value, "read").unwrap().to_value().unwrap(), value);
    for bad in [
        json!(0),
        json!(-1),
        json!(1.5),
        json!("20000"),
        Value::Null,
        json!({"$serde_json::private::Number":"20000"}),
    ] {
        value["limits"]["execution_ms"] = bad;
        assert_eq!(parse(&value, "read").unwrap_err(), Error::Limits);
    }
}

#[test]
fn operation_metadata_optional_presence_and_auth_alternatives_are_explicit() {
    assert_eq!(parse(&read(), "read").unwrap().to_value().unwrap(), read());
    for key in ["requires_auth", "realization"] {
        let mut bad = read();
        bad[key] = Value::Null;
        assert!(parse(&bad, "read").is_err());
    }
    for alternative in [
        json!([]),
        json!([{"profile":"","scopes":[]}]),
        json!([{"profile":"oauth","scopes":["read", "read"]}]),
        json!([{"profile":"oauth","scopes":[""]}]),
        json!([{"profile":"oauth","scopes":["a","b"]},
            {"profile":"oauth","scopes":["b","a"]}]),
        json!([{"profile":"oauth","scopes":[],"credential":"secret"}]),
    ] {
        let mut bad = read();
        bad["requires_auth"] = alternative;
        assert!(parse(&bad, "read").is_err(), "{bad}");
    }
    let mut bad = read();
    bad["realization"] = json!("unresolved");
    assert_eq!(parse(&bad, "read").unwrap_err(), Error::Realization);
}

#[test]
fn operation_metadata_does_not_extend_the_closed_legacy_operation_codec() {
    let legacy = json!({"id":"list", "description":"List records",
        "contract":"operations/v1alpha1", "profile":"read",
        "input_schema":{"type":"object"}, "output_schema":{}});
    let decoded: connectors_core::Operation = serde_json::from_value(legacy.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), legacy);
    let mut extended = legacy.clone();
    extended
        .as_object_mut()
        .unwrap()
        .extend(read().as_object().unwrap().clone());
    assert!(serde_json::from_value::<connectors_core::Operation>(extended.clone()).is_err());
    // Metadata is a selected companion to an operation, not an implicit decoder
    // for either an old operation or a new whole public descriptor.
    assert!(parse(&legacy, "read").is_err());
    assert!(parse(&extended, "read").is_err());
}

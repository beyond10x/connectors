//! Adversary pass 2 against the catalog engine's `body_types` and
//! `body_required` selection fields: the declared input schema and `prepare`
//! must admit and refuse the same bodies, load-time refusals must hold, and a
//! selection without the new fields must declare what it declared before.
//! No network, no credential.
use connectors_catalog::{bundle::Bundle, ingest, inventory};
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::sync::Mutex;

struct NoReads {
    calls: Mutex<usize>,
}
#[async_trait::async_trait]
impl AuthenticatedHttp for NoReads {
    async fn get(&self, _path: &[&str], _query: &[(&str, String)]) -> Result<HttpResponse> {
        *self.calls.lock().unwrap() += 1;
        panic!("unexpected provider read")
    }
}

fn things_bundle() -> Bundle {
    let bytes = serde_json::to_vec(&json!({
        "openapi": "3.0.0",
        "info": {"title": "things", "version": "1"},
        "paths": {
            "/v1/things": {"post": {
                "operationId": "createThing",
                "requestBody": {"required": true, "content": {"application/json": {"schema": {"type": "object"}}}},
                "responses": {"200": {"description": "OK", "content": {"application/json": {}}}}
            }}
        }
    }))
    .unwrap();
    let source = ingest("things.json", &bytes).unwrap();
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    Bundle {
        provider: "things".into(),
        source,
        inventory: inventory::extract(&document),
        auth_profile: "fixture.token".into(),
    }
}

fn written(value: Value) -> Selection {
    serde_json::from_value(value).unwrap()
}

fn typed() -> Engine {
    let selection = written(json!({
        "id": "thing.create", "operation_id": "createThing", "effect": "write",
        "body_keys": ["count", "flag", "name", "free"],
        "body_types": {"count": "integer", "flag": "boolean", "name": "string"},
        "body_required": ["count", "free"]
    }));
    Engine::new(&things_bundle(), "/v1", &[selection]).unwrap()
}

/// Whether the declared input schema admits `input`, and whether `prepare`
/// does. The two must agree: the declaration is what a caller is told, and
/// `prepare` is what the engine does.
async fn verdicts(engine: &Engine, input: &Value) -> (bool, bool) {
    let schema = &engine.declarations(&[Effect::Write])[0].input_schema;
    let declared = connectors_sdk::validate(schema, input).is_ok();
    let http = NoReads {
        calls: Mutex::new(0),
    };
    let prepared = engine
        .prepare(&http, "fixture", "thing.create", input.clone())
        .await;
    if let Err(error) = &prepared {
        assert_eq!(error.code, ErrorCode::InvalidInput, "{input}");
    }
    (declared, prepared.is_ok())
}

/// The declaration writes `{"type": "integer"}`, which JSON Schema satisfies
/// with any number whose value is integral, so `3.0`, `1e2`, `-0.0` and
/// `18446744073709551616` (u64::MAX + 1) pass the declared schema. The body is
/// sent as given, so `prepare` admits only an integer literal in the i64 or
/// u64 range and refuses each of them before any request, as a typed
/// parameter refuses `2.0`; the documentation states that the engine is the
/// stricter of the two here.
#[tokio::test]
async fn adversary_integer_typed_key_admits_only_integer_literals_the_schema_cannot_tell_apart() {
    let engine = typed();
    let mut admitted = Vec::new();
    for count in ["3.0", "1e2", "-0.0", "18446744073709551616"] {
        let count: Value = serde_json::from_str(count).unwrap();
        let input = json!({"body": {"count": count, "free": 1}});
        let (declared, prepared) = verdicts(&engine, &input).await;
        assert!(
            declared,
            "{count}: the declared schema is expected to admit it"
        );
        if prepared {
            admitted.push(count.to_string());
        }
    }
    assert!(
        admitted.is_empty(),
        "prepare admitted a number that is not an integer literal: {admitted:?}"
    );
}

/// Could not break, kept as evidence: on every other boundary the two agree.
#[tokio::test]
async fn adversary_typed_and_required_keys_agree_between_schema_and_prepare_on_boundaries() {
    let engine = typed();
    for (body, admitted) in [
        (json!({"count": -5, "free": null}), true),
        (json!({"count": i64::MIN, "free": 0}), true),
        (json!({"count": u64::MAX, "free": 0}), true),
        (
            json!({"count": 0, "free": [], "flag": false, "name": ""}),
            true,
        ),
        (json!({"count": "3", "free": 0}), false),
        (json!({"count": null, "free": 0}), false),
        (json!({"count": 1.5, "free": 0}), false),
        (json!({"count": true, "free": 0}), false),
        (json!({"count": 1, "free": 0, "flag": "true"}), false),
        (json!({"count": 1, "free": 0, "flag": 0}), false),
        (json!({"count": 1, "free": 0, "flag": null}), false),
        (json!({"count": 1, "free": 0, "name": 7}), false),
        (json!({"count": 1, "free": 0, "name": null}), false),
        (json!({"count": 1}), false),
        (json!({"free": 1}), false),
        (json!({}), false),
    ] {
        let input = json!({"body": body});
        let (declared, prepared) = verdicts(&engine, &input).await;
        assert_eq!(declared, admitted, "declared, {input}");
        assert_eq!(prepared, admitted, "prepared, {input}");
    }
}

/// Could not break, kept as evidence: a type name outside the three is
/// refused when the selection is read; empty `body_types` and `body_required`
/// load without `body_keys` and declare exactly what a plain selection
/// declares; a key both typed and required loads and is declared once.
#[test]
fn adversary_load_time_edges_of_body_types_and_body_required() {
    for name in ["number", "null", "String", "array", "object", ""] {
        let parsed: std::result::Result<Selection, _> = serde_json::from_value(json!({
            "id": "thing.create", "operation_id": "createThing", "effect": "write",
            "body_keys": ["count"], "body_types": {"count": name}
        }));
        assert!(parsed.is_err(), "type name {name:?} was read");
    }
    let plain = Engine::new(
        &things_bundle(),
        "/v1",
        &[written(json!({
            "id": "thing.create", "operation_id": "createThing", "effect": "write"
        }))],
    )
    .unwrap();
    let empty = Engine::new(
        &things_bundle(),
        "/v1",
        &[written(json!({
            "id": "thing.create", "operation_id": "createThing", "effect": "write",
            "body_types": {}, "body_required": []
        }))],
    )
    .unwrap();
    assert_eq!(
        serde_json::to_vec(&plain.declarations(&[Effect::Write])[0].input_schema).unwrap(),
        serde_json::to_vec(&empty.declarations(&[Effect::Write])[0].input_schema).unwrap()
    );
    let both = Engine::new(
        &things_bundle(),
        "/v1",
        &[written(json!({
            "id": "thing.create", "operation_id": "createThing", "effect": "write",
            "body_keys": ["count"], "body_types": {"count": "integer"}, "body_required": ["count"]
        }))],
    )
    .unwrap();
    assert_eq!(
        both.declarations(&[Effect::Write])[0].input_schema["properties"]["body"],
        json!({"type": "object", "properties": {"count": {"type": "integer"}},
               "required": ["count"], "additionalProperties": false})
    );
}

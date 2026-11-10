//! Adversary pass 2 against `optional_values` on a guard preflight
//! (story:catalog-guard-optional-preflight-values): the correction dropped
//! optional references from `guarded_body_paths`, so a closed body no longer
//! checks them at load. Each case here was refused at load before it.
use connectors_catalog::{bundle::Bundle, ingest, inventory};
use connectors_catalog_provider::{Engine, Selection};
use connectors_core::Result;
use serde_json::{Value, json};

fn document() -> Vec<u8> {
    let id = json!({"name": "id", "in": "path", "required": true, "schema": {"type": "string"}});
    let mode =
        json!({"name": "mode", "in": "query", "required": false, "schema": {"type": "string"}});
    serde_json::to_vec(&json!({
        "openapi": "3.0.0",
        "info": {"title": "fixture", "version": "1"},
        "paths": {
            "/items/{id}": {
                "get": {
                    "operationId": "GetItem",
                    "parameters": [id, mode],
                    "responses": {"200": {"description": "OK", "content": {"application/json": {}}}}
                },
                "patch": {
                    "operationId": "UpdateItem",
                    "parameters": [id],
                    "requestBody": {"content": {"application/json": {"schema": {"type": "object"}}}},
                    "responses": {"200": {"description": "OK", "content": {"application/json": {}}}}
                }
            }
        }
    }))
    .unwrap()
}

fn bundle() -> Bundle {
    let bytes = document();
    let source = ingest("fixture.json", &bytes).unwrap();
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    Bundle {
        provider: "fixture".into(),
        source,
        inventory: inventory::extract(&document),
        auth_profile: "fixture.basic".into(),
    }
}

/// An item update whose preflight forwards `body.mode` as the read's `mode`
/// when the caller gives it, with the body closed or open.
fn update(body_keys: &[&str], preflight: Value) -> Value {
    let mut selection = json!({
        "id": "item.update", "operation_id": "UpdateItem", "effect": "write",
        "guard": {
            "preflight": preflight,
            "postflight": {"checks": [{"pointer": "/id", "expect": {"input": "id"}}]}
        }
    });
    if !body_keys.is_empty() {
        selection["body_keys"] = json!(body_keys);
    }
    selection
}

fn engine(selection: Value) -> Result<Engine> {
    let selection: Selection = serde_json::from_value(selection).unwrap();
    Engine::new(&bundle(), "/", &[selection])
}

fn optional(reference: &str) -> Value {
    json!({"operation_id": "GetItem", "values": {"id": "id"},
           "optional_values": {"mode": reference},
           "checks": [{"pointer": "/id", "expect": {"input": "id"}}]})
}

fn refusal(selection: Value) -> Option<String> {
    engine(selection).err().map(|e| e.message)
}

/// A closed body that does not admit the key an optional value reads: the
/// closed body refuses the key on input, so the read can never carry it. The
/// same reference in `values` is refused at load ("which its body_keys do not
/// admit"); before the correction this one was too.
#[test]
fn optional_value_reading_a_body_key_the_closed_body_does_not_admit_is_refused() {
    let refused = refusal(update(&["name"], optional("body.mode")));
    assert!(
        refused
            .as_deref()
            .is_some_and(|m| m.contains("body_keys do not admit")),
        "an optional value reading `body.mode`, which body_keys [name] do not admit, loaded: {refused:?}"
    );
}

/// A key `body_types` types as a `string_list`, read as an optional scalar:
/// every write that gives it is refused at prepare (`is not a scalar`). The
/// same reference in `values` is refused at load; before the correction this
/// one was too.
#[test]
fn optional_value_reading_a_string_list_body_key_is_refused() {
    let mut selection = update(&["mode", "name"], optional("body.mode"));
    selection["body_types"] = json!({"mode": "string_list"});
    let refused = refusal(selection);
    assert!(
        refused
            .as_deref()
            .is_some_and(|m| m.contains("as a scalar")),
        "an optional value reading the string_list key `body.mode` as a scalar loaded: {refused:?}"
    );
}

/// The controls: the same two selections with the reference in `values` are
/// refused at load today.
#[test]
fn required_value_controls_are_refused() {
    let required = json!({"operation_id": "GetItem", "values": {"id": "id", "mode": "body.mode"},
                          "checks": [{"pointer": "/id", "expect": {"input": "id"}}]});
    assert!(refusal(update(&["name"], required.clone())).is_some());
    let mut typed = update(&["mode", "name"], required);
    typed["body_types"] = json!({"mode": "string_list"});
    assert!(refusal(typed).is_some());
}

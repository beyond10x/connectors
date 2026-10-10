//! Adversary case for the `values` bound
//! (story:catalog-path-correction-and-value-bound).
//!
//! A string parameter takes a JSON integer and sends it as its decimal text
//! (`declared_type`, `scalar`), and `values_enum` documents that beside each
//! allowed string that is an integer's decimal text the declaration admits
//! that integer, so `7` passes against `["7"]`. The integer is only added when
//! the text parses as an `i64`, so an allowed value above `i64::MAX`, which a
//! JSON integer (a `u64`) still carries and the engine's own check admits, is
//! refused by the declared schema before the check runs.
use connectors_catalog::{bundle::Bundle, ingest, inventory};
use connectors_catalog_provider::{Engine, Selection};
use connectors_core::Result;
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::sync::Mutex;

struct Reads {
    calls: Mutex<Vec<Vec<(String, String)>>>,
}
#[async_trait::async_trait]
impl AuthenticatedHttp for Reads {
    async fn get(&self, _path: &[&str], query: &[(&str, String)]) -> Result<HttpResponse> {
        self.calls.lock().unwrap().push(
            query
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        );
        Ok(HttpResponse {
            status: 200,
            headers: Default::default(),
            body: b"[]".to_vec(),
        })
    }
}

fn fixture() -> Bundle {
    let bytes = serde_json::to_vec(&json!({
        "openapi": "3.0.0",
        "info": {"title": "fixture", "version": "1"},
        "paths": {
            "/api/things/{id}/search": {
                "get": {
                    "operationId": "searchThings",
                    "parameters": [
                        {"name": "id", "in": "path", "required": true, "schema": {"type": "string"}},
                        {"name": "scope", "in": "query", "required": false, "schema": {"type": "string"}}
                    ],
                    "responses": {"200": {"description": "OK", "content": {"application/json": {}}}}
                }
            }
        }
    }))
    .unwrap();
    let source = ingest("fixture.json", &bytes).unwrap();
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    Bundle {
        provider: "fixture".into(),
        source,
        inventory: inventory::extract(&document),
        auth_profile: "fixture.token".into(),
    }
}

#[tokio::test]
async fn adversary_an_allowed_integer_text_above_i64_is_admitted_as_its_json_integer() {
    let selection: Selection = serde_json::from_value(json!({
        "id": "things.search", "operation_id": "searchThings", "effect": "read",
        "bounds": {"scope": {"values": ["9223372036854775808"]}}
    }))
    .unwrap();
    let engine = Engine::new(&fixture(), "/api", &[selection]).unwrap();
    let http = Reads {
        calls: Mutex::new(Vec::new()),
    };
    // The string form is admitted, as `7` is against `["7"]`...
    engine
        .read(
            &http,
            "one",
            "things.search",
            json!({"id": "a", "scope": "9223372036854775808"}),
        )
        .await
        .unwrap();
    // ...and so must the JSON integer whose decimal text it is.
    let result = engine
        .read(
            &http,
            "one",
            "things.search",
            json!({"id": "a", "scope": 9_223_372_036_854_775_808u64}),
        )
        .await;
    assert!(
        result.is_ok(),
        "the JSON integer 9223372036854775808 was refused against [\"9223372036854775808\"]: {result:?}"
    );
}

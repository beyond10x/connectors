//! A selection's `values` bound (story:catalog-path-correction-and-value-bound):
//! a string query parameter held to an allowed set of values, checked on the
//! text the value is sent as and refused as `invalid_input` before any request;
//! what is refused when the selection loads; the declaration's `enum`; and the
//! serialised form, beside which an existing range bound keeps its bytes.
use connectors_catalog::{bundle::Bundle, ingest, inventory};
use connectors_catalog_provider::{Engine, Selection};
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{collections::VecDeque, sync::Mutex};

type Call = (Vec<String>, Vec<(String, String)>);

struct Reads {
    responses: Mutex<VecDeque<HttpResponse>>,
    calls: Mutex<Vec<Call>>,
}
#[async_trait::async_trait]
impl AuthenticatedHttp for Reads {
    async fn get(&self, path: &[&str], query: &[(&str, String)]) -> Result<HttpResponse> {
        self.calls.lock().unwrap().push((
            path.iter().map(|s| s.to_string()).collect(),
            query
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        ));
        Ok(self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected provider read"))
    }
}
fn reads() -> Reads {
    Reads {
        responses: Mutex::new(
            [HttpResponse {
                status: 200,
                headers: Default::default(),
                body: b"[]".to_vec(),
            }]
            .into(),
        ),
        calls: Mutex::new(Vec::new()),
    }
}

/// One read with a path `id`, a string `scope`, an integer `per_page`, a
/// boolean `flag`, a repeated string `kinds` and a string header.
fn fixture() -> Bundle {
    let query = |name: &str, schema: Value| json!({"name": name, "in": "query", "required": false, "schema": schema});
    let bytes = serde_json::to_vec(&json!({
        "openapi": "3.0.0",
        "info": {"title": "fixture", "version": "1"},
        "paths": {
            "/api/things/{id}/search": {
                "get": {
                    "operationId": "searchThings",
                    "parameters": [
                        {"name": "id", "in": "path", "required": true, "schema": {"type": "string"}},
                        query("scope", json!({"type": "string"})),
                        query("per_page", json!({"type": "integer"})),
                        query("flag", json!({"type": "boolean"})),
                        {"name": "kinds", "in": "query", "required": false, "style": "form",
                         "explode": true, "schema": {"type": "array", "items": {"type": "string"}}},
                        {"name": "X-Scope", "in": "header", "required": false, "schema": {"type": "string"}}
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
fn selection(bounds: Value) -> Selection {
    serde_json::from_value(json!({
        "id": "things.search", "operation_id": "searchThings", "effect": "read",
        "bounds": bounds
    }))
    .unwrap()
}
fn engine(bounds: Value) -> Engine {
    Engine::new(&fixture(), "/api", &[selection(bounds)]).unwrap()
}

/// A value in the set is sent as given; the declaration lists the set as the
/// parameter's `enum`.
#[tokio::test]
async fn a_value_in_the_set_is_sent_and_declared_as_an_enum() {
    let engine = engine(json!({"scope": {"values": ["blobs", "commits"]}}));
    let declaration = engine
        .declarations(&[connectors_catalog_provider::Effect::Read])
        .into_iter()
        .find(|o| o.id == "things.search")
        .unwrap();
    assert_eq!(
        declaration.input_schema["properties"]["scope"]["enum"],
        json!(["blobs", "commits"])
    );
    for scope in ["blobs", "commits"] {
        let http = reads();
        engine
            .read(
                &http,
                "one",
                "things.search",
                json!({"id": "a", "scope": scope}),
            )
            .await
            .unwrap();
        let calls = http.calls.lock().unwrap().clone();
        assert_eq!(calls[0].0, ["things", "a", "search"]);
        assert_eq!(calls[0].1, vec![("scope".to_owned(), scope.to_owned())]);
    }
    // Absent stays absent: the bound does not make the parameter required.
    let http = reads();
    engine
        .read(&http, "one", "things.search", json!({"id": "a"}))
        .await
        .unwrap();
    assert!(http.calls.lock().unwrap()[0].1.is_empty());
}

/// Anything else is refused before any request: another value, a case or
/// whitespace variant, a comma-joined list, an empty string, a JSON number
/// (compared as the text it would be sent as) and a non-scalar.
#[tokio::test]
async fn a_value_outside_the_set_is_refused_before_any_request() {
    let engine = engine(json!({"scope": {"values": ["blobs"]}}));
    for scope in [
        json!("issues"),
        json!("Blobs"),
        json!("blobs "),
        json!("blobs,issues"),
        json!(""),
        json!(1),
        json!(["blobs"]),
        json!({"scope": "blobs"}),
    ] {
        let http = reads();
        let error = engine
            .read(
                &http,
                "one",
                "things.search",
                json!({"id": "a", "scope": scope}),
            )
            .await
            .err()
            .unwrap_or_else(|| panic!("scope={scope} was read"));
        assert_eq!(error.code, ErrorCode::InvalidInput, "scope={scope}");
        assert!(http.calls.lock().unwrap().is_empty(), "scope={scope}");
    }
    // A number whose text is in the set passes as that text.
    let engine = self::engine(json!({"scope": {"values": ["7"]}}));
    let http = reads();
    engine
        .read(
            &http,
            "one",
            "things.search",
            json!({"id": "a", "scope": 7}),
        )
        .await
        .unwrap();
    assert_eq!(
        http.calls.lock().unwrap()[0].1,
        vec![("scope".to_owned(), "7".to_owned())]
    );
}

/// A repeated parameter's bound holds for each element.
#[tokio::test]
async fn a_values_bound_holds_for_each_element_of_a_repeated_parameter() {
    let engine = engine(json!({"kinds": {"values": ["a", "b"]}}));
    let http = reads();
    engine
        .read(
            &http,
            "one",
            "things.search",
            json!({"id": "x", "kinds": ["a", "b"]}),
        )
        .await
        .unwrap();
    let http = reads();
    let error = engine
        .read(
            &http,
            "one",
            "things.search",
            json!({"id": "x", "kinds": ["a", "c"]}),
        )
        .await
        .expect_err("an element outside the set was read");
    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert!(http.calls.lock().unwrap().is_empty());
}

/// Refused when the selection loads: `values` on a parameter the source does
/// not type as a string, on a path or header parameter or one the operation
/// lacks; an empty set, an empty or repeated value; and a bound carrying both
/// forms or a `minimum` beside `values`.
#[test]
fn an_unusable_values_bound_is_refused_at_load() {
    let bundle = fixture();
    for bounds in [
        json!({"per_page": {"values": ["1"]}}),
        json!({"flag": {"values": ["true"]}}),
        json!({"id": {"values": ["a"]}}),
        json!({"X-Scope": {"values": ["a"]}}),
        json!({"nothing": {"values": ["a"]}}),
        json!({"scope": {"values": []}}),
        json!({"scope": {"values": [""]}}),
        json!({"scope": {"values": ["blobs", "blobs"]}}),
    ] {
        let refused = Engine::new(&bundle, "/api", &[selection(bounds.clone())])
            .err()
            .unwrap_or_else(|| panic!("{bounds} loaded"));
        assert_eq!(refused.code, ErrorCode::InvalidInput, "{bounds}");
    }
    for bounds in [
        json!({"scope": {"values": ["blobs"], "maximum": 10}}),
        json!({"scope": {"values": ["blobs"], "minimum": 1}}),
        json!({"scope": {}}),
        json!({"scope": {"values": "blobs"}}),
        json!({"scope": {"values": [1]}}),
        json!({"scope": {"value": ["blobs"]}}),
    ] {
        let value = json!({
            "id": "things.search", "operation_id": "searchThings", "effect": "read",
            "bounds": bounds
        });
        let read = serde_json::from_value::<Selection>(value);
        if let Ok(selection) = read {
            let refused = Engine::new(&bundle, "/api", &[selection])
                .err()
                .unwrap_or_else(|| panic!("{bounds} loaded"));
            assert_eq!(refused.code, ErrorCode::InvalidInput, "{bounds}");
        }
    }
}

/// `values` round-trips; a range bound serialises exactly as before `values`
/// existed, so every shipped selection keeps its configuration bytes.
#[test]
fn a_values_bound_round_trips_and_a_range_keeps_its_bytes() {
    let bounded = selection(
        json!({"scope": {"values": ["blobs"]}, "per_page": {"minimum": 1, "maximum": 100}}),
    );
    let written = serde_json::to_value(&bounded).unwrap();
    assert_eq!(
        written["bounds"],
        json!({"scope": {"values": ["blobs"]}, "per_page": {"minimum": 1, "maximum": 100}})
    );
    assert_eq!(
        serde_json::from_value::<Selection>(written).unwrap(),
        bounded
    );
    let range = selection(json!({"per_page": {"maximum": 10}}));
    assert_eq!(
        serde_json::to_value(&range).unwrap()["bounds"],
        json!({"per_page": {"maximum": 10}})
    );
}

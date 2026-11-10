//! Adversary pass 1 against `optional_values` on a guard preflight
//! (story:catalog-guard-optional-preflight-values), driven from what
//! `docs/local-catalog-provider.md` says of it: "parameters of the read bound
//! from input references like `values`, except that an input value the caller
//! did not give is left out of the read instead of refusing the write; a given
//! one is forwarded". A fixture shaped like Drive's `files.update`: `GET` and
//! `PATCH` of `/items/{id}`, the read taking an optional `mode` query value.
use connectors_catalog::{bundle::Bundle, ingest, inventory};
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_core::Result;
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{collections::VecDeque, sync::Mutex};

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

fn mode_preflight() -> Value {
    json!({"operation_id": "GetItem", "values": {"id": "id"},
           "optional_values": {"mode": "body.mode"},
           "checks": [{"pointer": "/id", "expect": {"input": "id"}}]})
}

fn engine(selection: Value) -> Result<Engine> {
    let selection: Selection = serde_json::from_value(selection).unwrap();
    Engine::new(&bundle(), "/", &[selection])
}

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
fn item() -> Reads {
    Reads {
        responses: Mutex::new(VecDeque::from([HttpResponse {
            status: 200,
            headers: [("content-type".to_owned(), "application/json".to_owned())].into(),
            body: serde_json::to_vec(&json!({"id": "i-1"})).unwrap(),
        }])),
        calls: Mutex::new(Vec::new()),
    }
}

/// A closed body whose key an optional value reads: the key is still optional.
/// The guide says an absent optional value is left out of the read; the
/// declaration must not require it, and a body without it reaches the
/// preflight, sent without `mode`.
#[tokio::test]
async fn closed_body_key_read_only_as_an_optional_value_is_not_required() {
    let engine = engine(update(&["mode", "name"], mode_preflight())).expect("the selection loads");
    let declaration = engine
        .declarations(&[Effect::Write])
        .into_iter()
        .find(|operation| operation.id == "item.update")
        .unwrap();
    let body = &declaration.input_schema["properties"]["body"];
    assert!(
        !body["required"]
            .as_array()
            .is_some_and(|required| required.iter().any(|name| name == "mode")),
        "an optional preflight value made the closed body key `mode` required: {body}"
    );
    let http = item();
    let prepared = engine
        .prepare(
            &http,
            "fixture",
            "item.update",
            json!({"id": "i-1", "body": {"name": "renamed"}}),
        )
        .await;
    assert!(
        prepared.is_ok(),
        "a closed body without the optional key was refused: {:?}",
        prepared.err()
    );
    let calls = http.calls.lock().unwrap();
    assert_eq!(calls[0].1, Vec::<(String, String)>::new());
}

/// The same selection with an open body: an absent optional body value is left
/// out, a given one forwarded (the control for the case above).
#[tokio::test]
async fn open_body_optional_value_is_forwarded_when_given() {
    let engine = engine(update(&[], mode_preflight())).expect("the selection loads");
    let http = item();
    engine
        .prepare(
            &http,
            "fixture",
            "item.update",
            json!({"id": "i-1", "body": {"name": "renamed", "mode": "all"}}),
        )
        .await
        .expect("prepared");
    let calls = http.calls.lock().unwrap();
    assert_eq!(calls[0].1, vec![("mode".to_owned(), "all".to_owned())]);
}

/// A given optional value that is not a scalar is neither forwarded nor
/// refused: the write carries it, the read is made without it. A required
/// value in its place refuses the write (`guard value ... is absent`).
#[tokio::test]
async fn given_non_scalar_optional_value_is_not_silently_dropped() {
    let engine = engine(update(&[], mode_preflight())).expect("the selection loads");
    let http = item();
    let prepared = engine
        .prepare(
            &http,
            "fixture",
            "item.update",
            json!({"id": "i-1", "body": {"name": "renamed", "mode": {"scope": "all"}}}),
        )
        .await;
    let calls = http.calls.lock().unwrap();
    assert!(
        prepared.is_err(),
        "a given non-scalar optional value was dropped from the read and the write prepared; reads: {calls:?}"
    );
}

/// An optional value naming no parameter of the read loads, and is refused
/// only when a caller gives it: a test that omits it never sees the typo.
#[test]
fn optional_value_naming_no_parameter_of_the_read_is_refused_when_it_loads() {
    let preflight = json!({"operation_id": "GetItem", "values": {"id": "id"},
                           "optional_values": {"mdoe": "body.mode"},
                           "checks": [{"pointer": "/id", "expect": {"input": "id"}}]});
    assert!(
        engine(update(&[], preflight)).is_err(),
        "a preflight binding `mdoe`, which `GetItem` does not declare, loaded"
    );
}

/// The same check for `values`, for comparison with the case above.
#[test]
fn required_value_naming_no_parameter_of_the_read_is_refused_when_it_loads() {
    let preflight = json!({"operation_id": "GetItem", "values": {"id": "id", "mdoe": "body.mode"},
                           "checks": [{"pointer": "/id", "expect": {"input": "id"}}]});
    assert!(
        engine(update(&[], preflight)).is_err(),
        "a preflight binding `mdoe`, which `GetItem` does not declare, loaded"
    );
}

/// One read parameter bound both as a required and as an optional value, from
/// different inputs: which one the read carries is not said anywhere, and the
/// selection loads.
#[test]
fn parameter_bound_both_required_and_optional_is_refused_when_it_loads() {
    let preflight = json!({"operation_id": "GetItem", "values": {"id": "id", "mode": "body.kind"},
                           "optional_values": {"mode": "body.mode"},
                           "checks": [{"pointer": "/id", "expect": {"input": "id"}}]});
    assert!(
        engine(update(&[], preflight)).is_err(),
        "a preflight binding `mode` both in values and optional_values loaded"
    );
}

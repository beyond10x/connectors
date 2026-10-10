//! Adversarial cases for story:catalog-selection-parameter-bounds: value forms
//! that parse unexpectedly, every request path of a bounded selection, the
//! refusal of bounds on non-query parameters, and the serialisation of
//! selections that carry no bound.
use connectors_catalog::{bundle, bundle::Bundle, ingest, inventory};
use connectors_catalog_provider::{Bound, Engine, Selection};
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    path::Path,
    sync::Mutex,
};

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
fn reads(bodies: Vec<Value>) -> Reads {
    Reads {
        responses: Mutex::new(
            bodies
                .into_iter()
                .map(|body| HttpResponse {
                    status: 200,
                    headers: Default::default(),
                    body: serde_json::to_vec(&body).unwrap(),
                })
                .collect(),
        ),
        calls: Mutex::new(Vec::new()),
    }
}
fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped(provider: &str) -> Vec<Selection> {
    let file: Value = serde_json::from_slice(
        &std::fs::read(root().join(format!("providers/{provider}/operations.json"))).unwrap(),
    )
    .unwrap();
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn gitlab() -> Engine {
    let bundle = bundle::load(&root().join("generated/bundles"), "gitlab").unwrap();
    Engine::new(&bundle, "/api/v4", &shipped("gitlab")).unwrap()
}

/// A fixture with a header, a path and a query parameter on one read, and a
/// guarded write whose query carries a pageable parameter.
fn fixture() -> Bundle {
    let path = |name: &str| json!({"name": name, "in": "path", "required": true, "schema": {"type": "string"}});
    let query = |name: &str| json!({"name": name, "in": "query", "required": false, "schema": {"type": "integer"}});
    let bytes = serde_json::to_vec(&json!({
        "openapi": "3.0.0",
        "info": {"title": "fixture", "version": "1"},
        "paths": {
            "/api/v4/things/{id}": {
                "get": {
                    "operationId": "listThings",
                    "parameters": [
                        path("id"),
                        query("per_page"),
                        {"name": "X-Page-Size", "in": "header", "required": false, "schema": {"type": "integer"}}
                    ],
                    "responses": {"200": {"description": "OK", "content": {"application/json": {}}}}
                },
                "post": {
                    "operationId": "createThing",
                    "parameters": [path("id"), query("per_page")],
                    "requestBody": {"required": true, "content": {"application/json": {"schema": {"type": "object"}}}},
                    "responses": {"201": {"description": "Created", "content": {"application/json": {}}}}
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
fn selection(value: Value) -> Selection {
    serde_json::from_value(value).unwrap()
}

/// GitLab's stop rule is "a page shorter than `per_page` ends the walk". A
/// `per_page` of zero or below cannot satisfy it on any page, and it reaches
/// GitLab today because the bound carries only a maximum. The story decided
/// only the maximum; this case asks whether the minimum belongs in it.
#[tokio::test]
async fn non_positive_per_page_is_refused_before_any_request() {
    let engine = gitlab();
    let mut escaped = Vec::new();
    for per_page in [json!(0), json!(-1), json!("0"), json!("-0"), json!("-100")] {
        let http = reads(vec![json!([])]);
        let outcome = engine
            .read(
                &http,
                "one",
                "issues.list",
                json!({"id": "org/project", "per_page": per_page}),
            )
            .await;
        let calls = http.calls.lock().unwrap().clone();
        if !calls.is_empty()
            || outcome.as_ref().err().map(|e| &e.code) != Some(&ErrorCode::InvalidInput)
        {
            escaped.push(format!("per_page={per_page} sent {:?}", calls));
        }
    }
    assert!(escaped.is_empty(), "reached the provider: {escaped:#?}");
}

/// Forms a lenient integer parser accepts are refused; a form that passes is
/// sent exactly as it was checked.
#[tokio::test]
async fn unusual_integer_forms_are_refused_or_sent_as_checked() {
    let engine = gitlab();
    let long_small = format!("{}100", "0".repeat(400));
    let long_large = format!("{}101", "0".repeat(400));
    let huge = "9".repeat(400);
    for per_page in [
        json!("+100"),
        json!("+101"),
        json!(" 100"),
        json!("100 "),
        json!("100\n"),
        json!("1e2"),
        json!(1e2),
        json!(100.0),
        json!("0x64"),
        json!("0101"),
        json!("١٠٠"),
        json!("１００"),
        json!("--1"),
        json!("-"),
        json!(long_large),
        json!(huge),
        json!(u64::MAX),
        json!(i64::MIN),
    ] {
        let http = reads(vec![json!([])]);
        let outcome = engine
            .read(
                &http,
                "one",
                "issues.list",
                json!({"id": "org/project", "per_page": per_page.clone()}),
            )
            .await;
        let calls = http.calls.lock().unwrap().clone();
        match outcome {
            Err(error) => {
                assert_eq!(error.code, ErrorCode::InvalidInput, "per_page={per_page}");
                assert!(calls.is_empty(), "per_page={per_page} sent {calls:?}");
            }
            Ok(_) => {
                // Only a value that is a decimal integer at most 100 may pass,
                // and then verbatim.
                let text = per_page
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or(per_page.to_string());
                let value: i128 = text
                    .parse()
                    .unwrap_or_else(|_| panic!("per_page={per_page} passed and is not an integer"));
                assert!(value <= 100, "per_page={per_page} passed");
                assert_eq!(calls[0].1, vec![("per_page".to_owned(), text)]);
            }
        }
    }
    for (per_page, sent) in [("0100", "0100"), (long_small.as_str(), long_small.as_str())] {
        let http = reads(vec![json!([])]);
        engine
            .read(
                &http,
                "one",
                "issues.list",
                json!({"id": "org/project", "per_page": per_page}),
            )
            .await
            .unwrap();
        assert_eq!(
            http.calls.lock().unwrap()[0].1,
            vec![("per_page".to_owned(), sent.to_owned())]
        );
    }
}

/// A bound on a header or path parameter is refused when the selection
/// loads; a bound on the query parameter of the same operation loads.
#[test]
fn a_bound_on_a_non_query_parameter_is_refused_at_load() {
    let bundle = fixture();
    for parameter in ["X-Page-Size", "id", "Per_Page", "query:per_page"] {
        let refused = Engine::new(
            &bundle,
            "/api/v4",
            &[selection(json!({
                "id": "things.list", "operation_id": "listThings", "effect": "read",
                "bounds": {parameter: {"maximum": 10}}
            }))],
        )
        .err()
        .unwrap_or_else(|| panic!("a bound on `{parameter}` loaded"));
        assert_eq!(refused.code, ErrorCode::InvalidInput, "`{parameter}`");
    }
    Engine::new(
        &bundle,
        "/api/v4",
        &[selection(json!({
            "id": "things.list", "operation_id": "listThings", "effect": "read",
            "bounds": {"per_page": {"maximum": 10}}
        }))],
    )
    .unwrap();
}

/// A bounded write is checked in `prepare` before the guard's preflight read.
#[tokio::test]
async fn a_bounded_guarded_write_is_refused_before_its_preflight() {
    let bundle = fixture();
    let engine = Engine::new(
        &bundle,
        "/api/v4",
        &[selection(json!({
            "id": "things.create", "operation_id": "createThing", "effect": "write",
            "bounds": {"per_page": {"maximum": 10}},
            "guard": {
                "preflight": {"operation_id": "listThings", "values": {"id": "id"},
                              "checks": [{"pointer": "/0/id", "expect": {"literal": "1"}}]},
                "postflight": {"checks": []}}
        }))],
    )
    .unwrap();
    for per_page in [json!(11), json!("11"), json!("eleven")] {
        let http = reads(vec![json!([{"id": "1"}])]);
        let error = engine
            .prepare(
                &http,
                "one",
                "things.create",
                json!({"id": "a", "per_page": per_page, "body": {}}),
            )
            .await
            .err()
            .unwrap_or_else(|| panic!("per_page={per_page} prepared"));
        assert_eq!(error.code, ErrorCode::InvalidInput, "per_page={per_page}");
        assert!(http.calls.lock().unwrap().is_empty(), "per_page={per_page}");
    }
    let http = reads(vec![json!([{"id": "1"}])]);
    engine
        .prepare(
            &http,
            "one",
            "things.create",
            json!({"id": "a", "per_page": "10", "body": {}}),
        )
        .await
        .unwrap();
}

/// The key round-trips, refuses unknown keys inside a bound and beside it, and
/// a selection without bounds serialises the fields it did before bounds.
#[test]
fn bounds_round_trip_and_unbounded_selections_serialise_as_before() {
    let bounded = selection(json!({
        "id": "things.list", "operation_id": "listThings", "effect": "read",
        "bounds": {"per_page": {"maximum": 10}}
    }));
    assert_eq!(
        bounded.bounds["per_page"],
        Bound {
            minimum: None,
            maximum: 10
        }
    );
    let written = serde_json::to_value(&bounded).unwrap();
    assert_eq!(written["bounds"], json!({"per_page": {"maximum": 10}}));
    assert_eq!(
        serde_json::from_value::<Selection>(written).unwrap(),
        bounded
    );
    for bad in [
        json!({"bounds": {"per_page": {"maximum": 10, "max": 5}}}),
        json!({"bounds": {"per_page": {"Maximum": 10}}}),
        json!({"bounds": {"per_page": {"maximum": "10"}}}),
        json!({"bounds": {"per_page": {"maximum": 10.5}}}),
        json!({"bounds": {"per_page": 10}}),
        json!({"bound": {"per_page": {"maximum": 10}}}),
    ] {
        let mut value =
            json!({"id": "things.list", "operation_id": "listThings", "effect": "read"});
        for (k, v) in bad.as_object().unwrap() {
            value[k] = v.clone();
        }
        assert!(serde_json::from_value::<Selection>(value).is_err(), "{bad}");
    }
    // Jira carries no bound: every selection serialises exactly the six fields
    // a selection had at the base commit, so its configuration digest is the
    // same bytes as before. The one write, `issue.transition.run`, adds only
    // the `body_keys` that close its body, and no `bounds`.
    let before: BTreeSet<&str> = [
        "id",
        "operation_id",
        "effect",
        "description",
        "guard",
        "response",
    ]
    .into();
    for selection in shipped("jira") {
        assert!(selection.bounds.is_empty(), "{}", selection.id);
        let value = serde_json::to_value(&selection).unwrap();
        let keys: BTreeSet<&str> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        let mut expected = before.clone();
        if selection.id == "issue.transition.run" {
            expected.insert("body_keys");
        }
        assert_eq!(keys, expected, "{}", selection.id);
    }
    let _ = BTreeMap::<String, Bound>::new();
}

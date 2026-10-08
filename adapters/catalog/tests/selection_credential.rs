//! `credential` on a selection: a parameter through which the pinned document
//! passes the credential the connection already sends in its header leaves the
//! declaration even when the document requires it (`docs/catalog-slack.md`,
//! "Credential parameters"). Against the committed Slack bundle and a fixture
//! document for the shapes Slack does not carry. No network, no credential.
use connectors_catalog::{
    bundle::{self, Bundle},
    ingest, inventory,
};
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{path::Path, sync::Mutex};

fn slack(selection: Value) -> std::result::Result<Engine, String> {
    let bundle = bundle::load(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("generated/bundles"),
        "slack",
    )
    .unwrap();
    let selection: Selection = serde_json::from_value(selection).unwrap();
    Engine::new(&bundle, "/api", &[selection]).map_err(|error| error.message)
}

/// `team.info`, `emoji.list` and `search.messages` require a `token` query
/// parameter and `auth.test` a `token` header; named as the credential, each
/// loads, and `token` is neither declared nor required.
#[test]
fn a_required_credential_parameter_leaves_the_declaration() {
    for (id, operation_id) in [
        ("team.info", "team_info"),
        ("emoji.list", "emoji_list"),
        ("search.messages", "search_messages"),
        ("auth.test", "auth_test"),
    ] {
        let engine = slack(json!({
            "id": id, "operation_id": operation_id, "effect": "read",
            "credential": ["token"]
        }))
        .unwrap_or_else(|message| panic!("`{id}`: {message}"));
        let declaration = engine.declarations(&[Effect::Read]).remove(0);
        assert!(
            declaration.input_schema["properties"]
                .get("token")
                .is_none(),
            "`{id}` declares token"
        );
        assert!(
            !declaration.input_schema["required"]
                .as_array()
                .unwrap()
                .contains(&json!("token")),
            "`{id}` requires token"
        );
    }
}

/// `credential` does not change `withhold`: withholding the required `token`
/// is still refused, and without `credential` the required header still
/// refuses `auth.test`.
#[test]
fn withhold_and_the_required_header_refusal_are_unchanged() {
    let message = slack(json!({"id": "team.info", "operation_id": "team_info",
                              "effect": "read", "withhold": ["token"]}))
    .err()
    .expect("a withheld required token loaded");
    assert!(
        message.contains("withholds `token`, which is required"),
        "{message}"
    );
    let message = slack(json!({"id": "auth.test", "operation_id": "auth_test",
                              "effect": "read"}))
    .err()
    .expect("a required header loaded");
    assert!(
        message.contains("needs a header parameter this transport does not carry"),
        "{message}"
    );
}

// ---------------------------------------------------------------------------
// A fixture document: a required credential in the query and in a header, a
// path parameter and a guarded write.

fn fixture() -> Bundle {
    let string = json!({"type": "string"});
    let ok = json!({"200": {"description": "OK", "content": {"application/json": {}}}});
    let bytes = serde_json::to_vec(&json!({
        "openapi": "3.0.0",
        "info": {"title": "credential", "version": "1"},
        "paths": {
            "/v1/items": {"get": {
                "operationId": "listItems",
                "parameters": [
                    {"name": "token", "in": "query", "required": true, "schema": string},
                    {"name": "limit", "in": "query", "schema": {"type": "integer"}}
                ],
                "responses": ok
            }},
            "/v1/whoami": {"get": {
                "operationId": "whoAmI",
                "parameters": [
                    {"name": "token", "in": "header", "required": true, "schema": string}
                ],
                "responses": ok
            }},
            "/v1/things/{id}": {
                "get": {
                    "operationId": "getThing",
                    "parameters": [
                        {"name": "id", "in": "path", "required": true, "schema": string},
                        {"name": "mode", "in": "query", "schema": string}
                    ],
                    "responses": ok
                },
                "put": {
                    "operationId": "putThing",
                    "parameters": [
                        {"name": "id", "in": "path", "required": true, "schema": string},
                        {"name": "mode", "in": "query", "schema": string}
                    ],
                    "requestBody": {"required": true,
                                    "content": {"application/json": {"schema": {"type": "object"}}}},
                    "responses": ok
                }
            }
        }
    }))
    .unwrap();
    Bundle {
        provider: "credential".into(),
        source: ingest("credential.json", &bytes).unwrap(),
        inventory: inventory::extract(&serde_json::from_slice(&bytes).unwrap()),
        auth_profile: "fixture.token".into(),
    }
}
fn load(selection: Value) -> Result<Engine> {
    Engine::new(
        &fixture(),
        "/v1",
        &[serde_json::from_value(selection).unwrap()],
    )
}

type Call = (Vec<String>, Vec<(String, String)>);
/// Records every GET and answers `{}`.
#[derive(Default)]
struct Reads {
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
        Ok(HttpResponse {
            status: 200,
            headers: Default::default(),
            body: b"{}".to_vec(),
        })
    }
}

/// A required credential parameter, in the query or in a header, is never
/// sent: a read without it goes out with the other parameters only, and an
/// input carrying it under any spelling is refused before any request.
#[tokio::test]
async fn a_credential_parameter_reaches_neither_the_declaration_nor_the_request() {
    for (id, operation_id, input, sent) in [
        (
            "items.list",
            "listItems",
            json!({"limit": 5}),
            vec![("limit".to_string(), "5".to_string())],
        ),
        ("whoami", "whoAmI", json!({}), vec![]),
    ] {
        let engine = load(json!({
            "id": id, "operation_id": operation_id, "effect": "read",
            "credential": ["token"]
        }))
        .unwrap_or_else(|error| panic!("`{id}`: {}", error.message));
        let declaration = engine.declarations(&[Effect::Read]).remove(0);
        assert!(
            declaration.input_schema["properties"]
                .get("token")
                .is_none(),
            "`{id}`"
        );
        let http = Reads::default();
        engine.read(&http, "one", id, input).await.unwrap();
        assert_eq!(http.calls.lock().unwrap().clone()[0].1, sent, "`{id}`");
        for spelling in ["token", "query:token", "header:token"] {
            let http = Reads::default();
            let error = engine
                .read(&http, "one", id, json!({spelling: "fixture-token"}))
                .await
                .err()
                .unwrap_or_else(|| panic!("`{id}` accepted `{spelling}`"));
            assert_eq!(error.code, ErrorCode::InvalidInput, "`{id}` `{spelling}`");
            assert!(http.calls.lock().unwrap().is_empty(), "`{id}` `{spelling}`");
        }
    }
}

/// A credential name the operation does not declare in its query or a
/// header, or that the selection also withholds, marks required, bounds or
/// reads through a guard, is refused at load.
#[test]
fn a_credential_name_the_selection_cannot_drop_is_refused() {
    let guard = json!({
        "preflight": {"operation_id": "getThing", "values": {"id": "id", "mode": "mode"},
                      "checks": [{"pointer": "/state", "expect": {"literal": "open"}}]},
        "postflight": {"checks": []}
    });
    for (case, selection) in [
        (
            "undeclared",
            json!({"id": "items.list", "operation_id": "listItems", "effect": "read",
                   "credential": ["secret"]}),
        ),
        (
            "case variant",
            json!({"id": "items.list", "operation_id": "listItems", "effect": "read",
                   "credential": ["Token"]}),
        ),
        (
            "qualified spelling",
            json!({"id": "items.list", "operation_id": "listItems", "effect": "read",
                   "credential": ["query:token"]}),
        ),
        (
            "path parameter",
            json!({"id": "thing.get", "operation_id": "getThing", "effect": "read",
                   "credential": ["id"]}),
        ),
        (
            "also withheld",
            json!({"id": "items.list", "operation_id": "listItems", "effect": "read",
                   "credential": ["token"], "withhold": ["token"]}),
        ),
        (
            "also required",
            json!({"id": "items.list", "operation_id": "listItems", "effect": "read",
                   "credential": ["token"], "required": ["token"]}),
        ),
        (
            "also bounded",
            json!({"id": "items.list", "operation_id": "listItems", "effect": "read",
                   "credential": ["limit"], "bounds": {"limit": {"maximum": 10}}}),
        ),
        (
            "read by a guard",
            json!({"id": "thing.put", "operation_id": "putThing", "effect": "write",
                   "credential": ["mode"], "guard": guard}),
        ),
    ] {
        let error = load(selection)
            .err()
            .unwrap_or_else(|| panic!("`{case}` loaded"));
        assert_eq!(error.code, ErrorCode::InvalidInput, "{case}");
        assert!(
            error.message.contains("as the credential"),
            "{case}: {}",
            error.message
        );
    }
}

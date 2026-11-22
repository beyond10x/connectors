//! Adversary pass on story:catalog-validates-enum-and-date-time.
use connectors_catalog::{bundle, bundle::Bundle, ingest, inventory};
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{collections::VecDeque, path::Path, sync::Mutex};

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
fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped(provider: &str, base: &str) -> Engine {
    let file: Value = serde_json::from_slice(
        &std::fs::read(root().join(format!("providers/{provider}/operations.json"))).unwrap(),
    )
    .unwrap();
    let selections: Vec<Selection> = serde_json::from_value(file["operations"].clone()).unwrap();
    let bundle = bundle::load(&root().join("generated/bundles"), provider).unwrap();
    Engine::new(&bundle, base, &selections).unwrap()
}

/// A pinned 3.0 document with an inline enum, a repeated enum, a date-time and
/// a repeated date-time, as the unit's own `declared_engine` builds it.
fn fixture() -> Engine {
    let bytes = serde_json::to_vec(&json!({
        "openapi": "3.0.0",
        "info": {"title": "things", "version": "1"},
        "paths": {"/v1/things": {"get": {
            "operationId": "getThings",
            "parameters": [
                {"name": "level", "in": "query", "schema": {"type": "integer", "enum": [1, 2]}},
                {"name": "mode", "in": "query", "schema": {"type": "string", "enum": ["x", "y", null], "nullable": true}},
                {"name": "modes", "in": "query", "schema": {"type": "array", "items": {"type": "string", "enum": ["x", "y"]}}},
                {"name": "at", "in": "query", "schema": {"type": "string", "format": "date-time"}},
                {"name": "ats", "in": "query", "schema": {"type": "array", "items": {"type": "string", "format": "date-time"}}}
            ],
            "responses": {"200": {"description": "OK", "content": {"application/json": {}}}}
        }}}
    }))
    .unwrap();
    let source = ingest("things.json", &bytes).unwrap();
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    let bundle = Bundle {
        provider: "things".into(),
        source,
        inventory: inventory::extract(&document),
        auth_profile: "fixture.token".into(),
    };
    let selection: Selection = serde_json::from_value(
        json!({"id": "things.list", "operation_id": "getThings", "effect": "read"}),
    )
    .unwrap();
    Engine::new(&bundle, "/v1", &[selection]).unwrap()
}

async fn refused(engine: &Engine, id: &str, cases: Vec<Value>) -> Vec<String> {
    let mut escaped = Vec::new();
    for input in cases {
        let http = reads();
        let outcome = engine.read(&http, "one", id, input.clone()).await;
        let calls = http.calls.lock().unwrap().clone();
        if !calls.is_empty()
            || outcome.as_ref().err().map(|e| &e.code) != Some(&ErrorCode::InvalidInput)
        {
            escaped.push(format!("{id} {input}: {outcome:?}, sent {calls:?}"));
        }
    }
    escaped
}

async fn sent_query(
    engine: &Engine,
    id: &str,
    input: Value,
) -> std::result::Result<Vec<(String, String)>, String> {
    let http = reads();
    match engine.read(&http, "one", id, input.clone()).await {
        Ok(_) => Ok(http.calls.lock().unwrap()[0].1.clone()),
        Err(error) => Err(format!("{input} refused: {error:?}")),
    }
}

/// Acceptance 1, read literally: Confluence's pinned document gives `sort` and
/// `body-format` an `enum` through a `$ref` to `components/schemas`
/// (`PageSortOrder`, `PrimaryBodyRepresentation[Single]`). The shipped
/// selections `pages.changed`, `space.pages`, `page.get` and `page.comments`
/// expose them, and a value outside the enum is still sent.
#[tokio::test]
async fn adversary_confluence_enum_behind_a_schema_ref_is_refused() {
    let engine = shipped("confluence", "/wiki");
    let escaped = refused(
        &engine,
        "pages.changed",
        vec![json!({"sort": "newest"}), json!({"body-format": "html"})],
    )
    .await;
    let mut escaped = escaped;
    escaped.extend(
        refused(
            &engine,
            "page.get",
            vec![json!({"id": "1001", "body-format": "html"})],
        )
        .await,
    );
    assert!(
        escaped.is_empty(),
        "sent outside the pinned enum: {escaped:#?}"
    );
}

/// The guide (`docs/local-catalog-provider.md`, "Array and required query
/// parameters") says one comma-joined scalar for a repeated parameter is still
/// accepted and sends the same request as before; the unit's own paragraph says
/// the enum check holds "for each element". `x,y` has two elements, both in
/// the enum, and is refused whole.
#[tokio::test]
async fn adversary_comma_joined_repeated_enum_of_allowed_elements_is_sent() {
    let engine = fixture();
    assert_eq!(
        sent_query(&engine, "things.list", json!({"modes": "x,y"})).await,
        Ok(vec![("modes".to_string(), "x,y".to_string())])
    );
}

/// `operations describe` hands callers the declared input schema; a `values`
/// bound is declared there as an `enum`, the pinned document's enum is not, so
/// the declaration admits values the engine now refuses.
#[test]
#[ignore = "story:catalog-declared-schema-publishes-enum-and-date-time"]
fn adversary_declared_input_schema_lists_the_pinned_enum() {
    let engine = fixture();
    let declarations = engine.declarations(&[Effect::Read]);
    let schema = &declarations[0].input_schema["properties"]["level"];
    assert!(
        schema.to_string().contains("enum"),
        "declared `level` admits any integer: {schema}"
    );
}

/// Probes of the hand-written RFC 3339 reader and the enum matching that the
/// unit's tests do not cover; all hold.
#[tokio::test]
async fn adversary_probes_that_hold() {
    let engine = fixture();
    let escaped = refused(
        &engine,
        "things.list",
        vec![
            json!({"at": "2026-09-01T00:00:00+23:60"}),
            json!({"at": "2026-09-01T00:00:00+24:00"}),
            json!({"at": "2026-09-01T00:00:00.5"}),
            json!({"at": "2026-09-01T00:00:00Z\n"}),
            json!({"at": "2026-09-01T00:00:00+02:00x"}),
            json!({"at": "2026-09-01T00:00:00+0200"}),
            json!({"at": "2100-02-29T00:00:00Z"}),
            json!({"at": "2026-02-29T00:00:00Z"}),
            json!({"at": "2026-04-31T00:00:00Z"}),
            json!({"at": "2026-00-10T00:00:00Z"}),
            json!({"at": "2026-09-00T00:00:00Z"}),
            json!({"at": "2026-09-01T00:60:00Z"}),
            json!({"at": "2026-09-01T00:00:61Z"}),
            json!({"at": "２026-09-01T00:00:00Z"}),
            json!({"at": "2026-09-01T00:00:0٠Z"}),
            json!({"at": "+2026-09-01T00:00:00Z"}),
            json!({"at": "2026-09-01T00:00:00ZZ"}),
            json!({"at": "2026-09-01T00:00:00Z "}),
            json!({"at": "2026-09-01T00:00:00+02:00Z"}),
            json!({"ats": "2026-09-01T00:00:00Z,soon"}),
            json!({"level": 1.0}),
            json!({"level": "01"}),
            json!({"level": 3}),
            json!({"mode": "null"}),
            json!({"mode": null}),
            json!({"modes": ["x", "X"]}),
        ],
    )
    .await;
    assert!(escaped.is_empty(), "accepted or sent: {escaped:#?}");
    for (input, name, value) in [
        (
            json!({"at": "2000-02-29T00:00:00Z"}),
            "at",
            "2000-02-29T00:00:00Z",
        ),
        (
            json!({"at": "0000-01-01T00:00:00Z"}),
            "at",
            "0000-01-01T00:00:00Z",
        ),
        (
            json!({"at": "2026-12-31T23:59:59.000000001+23:59"}),
            "at",
            "2026-12-31T23:59:59.000000001+23:59",
        ),
        (json!({"level": "2"}), "level", "2"),
        (json!({"level": 1}), "level", "1"),
    ] {
        let query = sent_query(&engine, "things.list", input.clone())
            .await
            .unwrap();
        assert_eq!(query, [(name.to_string(), value.to_string())], "{input}");
    }
    // deepObject properties reach the check: GitLab's `not[milestone_id]`.
    let gitlab = shipped("gitlab", "/api/v4");
    let escaped = refused(
        &gitlab,
        "issues.list",
        vec![json!({"id": "org/project", "not[milestone_id]": "Bogus"})],
    )
    .await;
    assert!(
        escaped.is_empty(),
        "deepObject enum not checked: {escaped:#?}"
    );
}

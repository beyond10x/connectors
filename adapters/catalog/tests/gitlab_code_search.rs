//! GitLab project code search, `search.blobs`, driven through the engine over
//! the committed bundle with a recording transport
//! (story:catalog-path-correction-and-value-bound).
//!
//! The pinned source declares the route as `/api/v4/projects/{id}/(-/)search`,
//! GitLab's notation for an optional segment. The bundle carries the path GitLab
//! serves, `/api/v4/projects/{id}/search`, from a cited
//! `connectors-source-amendments/1` correction beside the pinned document, which
//! stays byte for byte the one the source record names. The selection holds the
//! required `scope` to `blobs` with a `values` bound, so the search cannot reach
//! issues, users or wiki text: any other scope is refused before a request.
//! Fixtures are hand-written in GitLab's blob search shape with synthetic ids.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
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
fn shipped() -> Vec<Selection> {
    let file: Value = serde_json::from_slice(
        &std::fs::read(root().join("providers/gitlab/operations.json")).unwrap(),
    )
    .unwrap();
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn engine() -> Engine {
    let bundle = bundle::load(&root().join("generated/bundles"), "gitlab").unwrap();
    Engine::new(&bundle, "/api/v4", &shipped()).unwrap()
}
fn sorted(query: &[(String, String)]) -> Vec<(String, String)> {
    let mut query = query.to_vec();
    query.sort();
    query
}
fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    let mut pairs: Vec<(String, String)> = list
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    pairs.sort();
    pairs
}
fn blob(path: &str) -> Value {
    json!({
        "basename": path.trim_end_matches(".rs"),
        "data": "fn main() {\n    connect();\n}\n",
        "path": path,
        "filename": path,
        "id": null,
        "ref": "main",
        "startline": 1,
        "project_id": 7
    })
}

/// `search.blobs` is a declared read of `getApiV4ProjectsIdDashSearch`: `id`,
/// `search` and `scope` required, `scope` declared as the one value `blobs`,
/// `per_page` bounded 1 through 100, and the parameters that only apply to
/// scopes it cannot reach withheld.
#[test]
fn search_blobs_is_a_declared_read_with_scope_held_to_blobs() {
    let engine = engine();
    let selection = shipped()
        .into_iter()
        .find(|s| s.id == "search.blobs")
        .expect("`search.blobs` is shipped");
    assert_eq!(selection.operation_id, "getApiV4ProjectsIdDashSearch");
    assert_eq!(engine.effect("search.blobs"), Some(Effect::Read));
    let declaration = engine
        .declarations(&[Effect::Read])
        .into_iter()
        .find(|o| o.id == "search.blobs")
        .expect("`search.blobs` is a declared read");
    let properties = declaration.input_schema["properties"].as_object().unwrap();
    assert_eq!(properties["scope"]["enum"], json!(["blobs"]));
    assert_eq!(properties["per_page"]["maximum"], json!(100));
    assert_eq!(properties["per_page"]["minimum"], json!(1));
    for parameter in ["id", "search", "scope", "ref", "page", "per_page"] {
        assert!(properties.contains_key(parameter), "`{parameter}`");
    }
    for withheld in ["type", "state", "confidential", "fields"] {
        assert!(!properties.contains_key(withheld), "declares `{withheld}`");
    }
    let mut required: Vec<&str> = declaration.input_schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    required.sort();
    assert_eq!(required, ["id", "scope", "search"]);
    assert!(
        !engine
            .declarations(&[Effect::Write])
            .iter()
            .any(|o| o.id == "search.blobs")
    );
}

/// The request goes to `GET /projects/{id}/search` under the configured
/// `/api/v4` base, never to the literal `(-/)` segment, with `scope=blobs` and
/// the search expression, and the answer is returned unchanged.
#[tokio::test]
async fn search_blobs_sends_get_projects_id_search_with_scope_blobs() {
    let engine = engine();
    let answer = json!([blob("src/main.rs"), blob("src/lib.rs")]);
    let http = reads(vec![answer.clone()]);
    let result = engine
        .read(
            &http,
            "one",
            "search.blobs",
            json!({"id": "org/project", "scope": "blobs", "search": "connect(",
                   "ref": "main", "per_page": 20, "page": 1}),
        )
        .await
        .unwrap();
    assert_eq!(result["status"], 200);
    assert_eq!(result["body"], answer);
    let calls = http.calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, ["projects", "org/project", "search"]);
    assert_eq!(
        sorted(&calls[0].1),
        pairs(&[
            ("scope", "blobs"),
            ("search", "connect("),
            ("ref", "main"),
            ("per_page", "20"),
            ("page", "1"),
        ])
    );
}

/// Every other scope, its spellings and its absence are refused as
/// `invalid_input` before any request, as are a withheld parameter and a
/// `per_page` above GitLab's cap.
#[tokio::test]
async fn any_other_scope_is_refused_before_a_request() {
    let engine = engine();
    let mut inputs: Vec<Value> = [
        json!("issues"),
        json!("work_items"),
        json!("merge_requests"),
        json!("milestones"),
        json!("notes"),
        json!("wiki_blobs"),
        json!("commits"),
        json!("users"),
        json!("Blobs"),
        json!("BLOBS"),
        json!("blobs "),
        json!(" blobs"),
        json!("blobs,issues"),
        json!("blobs\u{0}"),
        json!(""),
        json!(["blobs", "issues"]),
        json!(true),
        json!(1),
        json!(null),
    ]
    .into_iter()
    .map(|scope| json!({"id": "org/project", "scope": scope, "search": "x"}))
    .collect();
    inputs.push(json!({"id": "org/project", "search": "x"}));
    inputs.push(json!({"id": "org/project", "scope": "blobs", "search": "x", "state": "opened"}));
    inputs.push(json!({"id": "org/project", "scope": "blobs", "search": "x", "per_page": 101}));
    for input in inputs {
        let http = reads(vec![json!([])]);
        let error = engine
            .read(&http, "one", "search.blobs", input.clone())
            .await
            .err()
            .unwrap_or_else(|| panic!("{input} was read"));
        assert_eq!(error.code, ErrorCode::InvalidInput, "{input}");
        assert!(
            http.calls.lock().unwrap().is_empty(),
            "{input} reached GitLab"
        );
    }
}

/// The pinned document is unchanged: the bundle's source record names its
/// exact digest, the document still declares the `(-/)` route, and the
/// bundle's corrected path comes from the cited amendment file the record
/// names, whose correction is the only change to that operation.
#[test]
fn the_pinned_source_is_unchanged_and_the_correction_is_cited() {
    let pinned = std::fs::read(root().join("../gitlab/upstream/openapi_v3.yaml")).unwrap();
    let pinned_sha = hex::encode(Sha256::digest(&pinned));
    assert!(
        String::from_utf8_lossy(&pinned).contains("\n  \"/api/v4/projects/{id}/(-/)search\":\n"),
        "the pinned document no longer declares the (-/) route"
    );
    let file = root().join("../gitlab/upstream/openapi_v3.amendments.json");
    let amendments_bytes = std::fs::read(&file).unwrap();
    let amendments: Value = serde_json::from_slice(&amendments_bytes).unwrap();
    assert_eq!(amendments["format"], "connectors-source-amendments/1");
    assert_eq!(amendments["source_sha256"], pinned_sha);
    let correction = amendments["amendments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["operation_id"] == "getApiV4ProjectsIdDashSearch")
        .expect("the search route is corrected by a cited amendment");
    assert_eq!(
        correction["correct_path"],
        json!({"from": "/api/v4/projects/{id}/(-/)search", "to": "/api/v4/projects/{id}/search"})
    );
    assert!(
        correction["cite"]
            .as_str()
            .unwrap()
            .starts_with("https://docs.gitlab.com/")
    );
    assert!(!correction["reason"].as_str().unwrap().trim().is_empty());

    let bundle = bundle::load(&root().join("generated/bundles"), "gitlab").unwrap();
    assert_eq!(bundle.source.file_name, "openapi_v3.yaml");
    assert_eq!(bundle.source.source_sha256, pinned_sha);
    assert_eq!(bundle.source.source_bytes, pinned.len());
    let record = bundle
        .source
        .amendments
        .as_ref()
        .expect("the bundle records the amendment file");
    assert_eq!(record.file_name, "openapi_v3.amendments.json");
    assert_eq!(
        record.sha256,
        hex::encode(Sha256::digest(&amendments_bytes))
    );
    assert_eq!(
        record.count,
        amendments["amendments"].as_array().unwrap().len()
    );
    let operation = bundle
        .inventory
        .operations
        .iter()
        .find(|o| o.operation_id.as_deref() == Some("getApiV4ProjectsIdDashSearch"))
        .unwrap();
    assert_eq!(operation.method, "get");
    assert_eq!(operation.path, "/api/v4/projects/{id}/search");
    // The other routes written with the notation are not corrected here.
    for (id, path) in [
        (
            "getApiV4GroupsIdDashSearch",
            "/api/v4/groups/{id}/(-/)search",
        ),
        (
            "getApiV4ProjectsIdDashSearchSemantic",
            "/api/v4/projects/{id}/(-/)search/semantic",
        ),
    ] {
        let operation = bundle
            .inventory
            .operations
            .iter()
            .find(|o| o.operation_id.as_deref() == Some(id))
            .unwrap();
        assert_eq!(operation.path, path, "`{id}`");
    }
}

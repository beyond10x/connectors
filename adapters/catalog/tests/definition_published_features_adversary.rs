//! Adversary pass on story:definition-lowerer-reads-published-features: the
//! released connectors-definitions 0.5.2 `connectors/github.yaml`, lowered by
//! `connectors_catalog::definition`, loads in the catalog provider's engine, and
//! its `query_fixed`, `query_folds`, `embedded_text` and `rate_limit_headers`
//! act on a scripted provider as the definition writes them. No network, no
//! credential.
#![cfg(unix)]
use connectors_catalog::{bundle, definition};
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Mutex,
};

fn released() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/connectors-catalog/tests/fixtures/definitions-0.5.2")
}

/// The engine the released definition lowers to.
fn engine() -> Engine {
    let temp = tempfile::tempdir().unwrap();
    let scratch = temp.path().join("scratch");
    std::fs::create_dir_all(&scratch).unwrap();
    let out = temp.path().join("out");
    let lowered = definition::lower(&definition::Request {
        definition: &released().join("connectors/github.yaml"),
        root: &released(),
        out: &out,
        scratch: &scratch,
    })
    .expect("the released definition lowers");
    let selections: Vec<Selection> =
        serde_json::from_value(
            serde_json::from_slice::<Value>(&std::fs::read(&lowered.operations).unwrap()).unwrap()
                ["operations"]
                .clone(),
        )
        .expect("the lowered selection reads as the engine's selections");
    let bundle = bundle::load(&out, &lowered.provider).unwrap();
    match Engine::new(&bundle, "", &selections) {
        Ok(engine) => engine,
        Err(error) => panic!("the lowered selection loads: {}", error.message),
    }
}

/// One GET as sent: its path segments and its query pairs.
type Sent = (Vec<String>, Vec<(String, String)>);

struct Scripted {
    answer: Mutex<Option<HttpResponse>>,
    sent: Mutex<Vec<Sent>>,
}

#[async_trait::async_trait]
impl AuthenticatedHttp for Scripted {
    async fn get(&self, path: &[&str], query: &[(&str, String)]) -> Result<HttpResponse> {
        self.sent.lock().unwrap().push((
            path.iter().map(|s| s.to_string()).collect(),
            query
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        ));
        Ok(self
            .answer
            .lock()
            .unwrap()
            .take()
            .expect("one provider read"))
    }
}

fn answering(status: u16, headers: &[(&str, &str)], body: Value) -> Scripted {
    Scripted {
        answer: Mutex::new(Some(HttpResponse {
            status,
            headers: headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<BTreeMap<_, _>>(),
            body: serde_json::to_vec(&body).unwrap(),
        })),
        sent: Mutex::new(Vec::new()),
    }
}

fn declared(engine: &Engine, id: &str) -> connectors_core::Operation {
    engine
        .declarations(&[Effect::Read])
        .into_iter()
        .find(|operation| operation.id == id)
        .unwrap_or_else(|| panic!("`{id}` is declared"))
}

#[test]
fn the_released_github_definition_loads_and_declares_its_ten_reads() {
    let engine = engine();
    let mut ids: Vec<String> = engine
        .declarations(&[Effect::Read])
        .into_iter()
        .map(|operation| operation.id)
        .collect();
    ids.sort();
    assert_eq!(
        ids,
        [
            "file.raw",
            "issues.assigned",
            "notifications.list",
            "readme.raw",
            "release.latest",
            "releases.list",
            "repo.get",
            "repos.list",
            "review_requests.list",
            "user.me",
        ]
    );
    assert!(engine.declarations(&[Effect::Write]).is_empty());
}

/// A fixed query value leaves the declared input and a fold enters it.
#[test]
fn the_released_fixed_values_and_folds_shape_the_declared_input() {
    let engine = engine();
    let review = declared(&engine, "review_requests.list").input_schema;
    let properties = review["properties"].as_object().unwrap();
    for gone in ["q", "sort", "order"] {
        assert!(!properties.contains_key(gone), "`{gone}` is not declared");
    }
    assert_eq!(properties["since"]["type"], "string");
    assert!(properties["since"]["pattern"].is_string());
    let assigned = declared(&engine, "issues.assigned").input_schema;
    let properties = assigned["properties"].as_object().unwrap();
    for gone in ["filter", "state"] {
        assert!(!properties.contains_key(gone), "`{gone}` is not declared");
    }
    assert!(
        properties.contains_key("since"),
        "issues/list's own since stays"
    );
    let readme = declared(&engine, "readme.raw").output_schema;
    assert_eq!(readme["properties"]["body"], json!({"type": "string"}));
}

#[tokio::test]
async fn the_released_review_requests_send_the_fixed_q_with_the_folded_since() {
    let engine = engine();
    let http = answering(
        200,
        &[],
        json!({"total_count": 0, "incomplete_results": false, "items": []}),
    );
    engine
        .read(
            &http,
            "github",
            "review_requests.list",
            json!({"since": "2026-01-02", "per_page": 5}),
        )
        .await
        .unwrap_or_else(|error| panic!("the read is answered: {}", error.message));
    let sent = http.sent.lock().unwrap();
    let (path, query) = &sent[0];
    assert_eq!(path, &["search", "issues"]);
    let q: Vec<&str> = query
        .iter()
        .filter(|(name, _)| name == "q")
        .map(|(_, value)| value.as_str())
        .collect();
    assert_eq!(
        q,
        ["is:pr is:open review-requested:@me updated:>=2026-01-02"]
    );
}

#[tokio::test]
async fn the_released_issues_assigned_refuse_a_caller_state() {
    let engine = engine();
    let http = answering(200, &[], json!([]));
    let error = engine
        .read(
            &http,
            "github",
            "issues.assigned",
            json!({"state": "closed"}),
        )
        .await
        .expect_err("a fixed parameter is no input");
    assert_eq!(error.code, ErrorCode::InvalidInput, "{}", error.message);
    assert!(http.sent.lock().unwrap().is_empty(), "no request was sent");
}

#[tokio::test]
async fn the_released_rate_limit_headers_answer_a_spent_limit_as_rate_limited() {
    let engine = engine();
    let http = answering(
        403,
        &[("x-ratelimit-remaining", "0"), ("x-ratelimit-reset", "1")],
        json!({"message": "API rate limit exceeded"}),
    );
    let error = engine
        .read(&http, "github", "user.me", json!({}))
        .await
        .expect_err("a spent limit is refused");
    assert_eq!(error.code, ErrorCode::RateLimited, "{}", error.message);
}

#[tokio::test]
async fn the_released_readme_answers_the_embedded_file_as_text() {
    let engine = engine();
    let http = answering(
        200,
        &[],
        json!({"content": "aGVs\nbG8K", "encoding": "base64", "size": 6}),
    );
    let answer = engine
        .read(
            &http,
            "github",
            "readme.raw",
            json!({"owner": "example-org", "repo": "example"}),
        )
        .await
        .unwrap_or_else(|error| panic!("the read is answered: {}", error.message));
    assert_eq!(answer["body"], "hello\n");
}

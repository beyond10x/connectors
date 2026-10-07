//! Adversary pass 2, wave 20261007a, U2 (story:gitlab-feed-binding), against the correction
//! 3d50a0740: a provider 403 on the merge request list, after the project lookup answered, is
//! `not_found`.
//!
//! The family's error table (`contracts/datasources/feed/v1alpha1/semantics.md`, Errors) answers
//! "an unknown container, one the connection cannot read, or an undeclared direct conversation,
//! indistinguishably" with `not_found`, and `gitlab.md` (Errors) now says the refused project
//! answers `not_found` "exactly as for a project the token cannot see". It also says a 403 on the
//! project itself stays `forbidden`.
use connectors_catalog::bundle;
use connectors_catalog_provider::{
    Engine,
    feed::{Declaration, ITEMS},
};
use connectors_core::{Error, ErrorCode, Result};
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{collections::VecDeque, path::Path, sync::Mutex};

const INSTANCE: &str = "gitlab-feed-instance";

struct Reads {
    responses: Mutex<VecDeque<(u16, Value)>>,
    paths: Mutex<Vec<Vec<String>>>,
}

impl Reads {
    fn new(responses: Vec<(u16, Value)>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
            paths: Mutex::new(Vec::new()),
        }
    }
    fn paths(&self) -> Vec<Vec<String>> {
        self.paths.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl AuthenticatedHttp for Reads {
    async fn get(&self, path: &[&str], _query: &[(&str, String)]) -> Result<HttpResponse> {
        self.paths
            .lock()
            .unwrap()
            .push(path.iter().map(|s| s.to_string()).collect());
        let (status, body) = self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected provider read");
        Ok(HttpResponse {
            status,
            headers: Default::default(),
            body: serde_json::to_vec(&body).unwrap(),
        })
    }
}

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn engine() -> Engine {
    let shipped: Value = serde_json::from_slice(
        &std::fs::read(root().join("providers/gitlab/operations.json")).unwrap(),
    )
    .unwrap();
    let feed: Declaration = serde_json::from_value(shipped["feed"].clone()).unwrap();
    let bundle = bundle::load(&root().join("generated/bundles"), "gitlab").unwrap();
    Engine::with_feed(&bundle, "/api/v4", &[], Some(&feed)).unwrap()
}

fn project(id: u64) -> Value {
    let mut project: Value = serde_json::from_slice(
        &std::fs::read(root().join("tests/feed/gitlab/project.json")).unwrap(),
    )
    .unwrap();
    project["id"] = json!(id);
    project
}

fn merge_request(iid: u64, updated: &str) -> Value {
    let mut record: Value = serde_json::from_slice(
        &std::fs::read(root().join("tests/feed/gitlab/merge_request.json")).unwrap(),
    )
    .unwrap();
    record["iid"] = json!(iid);
    record["id"] = json!(60_000 + iid);
    record["created_at"] = json!("2026-10-06T08:00:00.000Z");
    record["updated_at"] = json!(updated);
    record
}

async fn items(engine: &Engine, http: &Reads, input: Value) -> std::result::Result<Value, Error> {
    engine.read(http, INSTANCE, ITEMS, input).await
}

/// A watermark the engine issued for project 1002: a full read of its one merge request.
async fn watermark_of_another_project(engine: &Engine) -> String {
    let http = Reads::new(vec![
        (200, project(1002)),
        (200, json!([merge_request(1, "2026-10-06T09:00:00.000Z")])),
    ]);
    let page = items(engine, &http, json!({"container": "1002", "limit": 10}))
        .await
        .unwrap();
    page["next_watermark"].as_str().unwrap().to_owned()
}

/// F4 story:gitlab-feed-binding: the refused project and an unknown one are told apart by the
/// watermark a caller sends. An unknown project is `not_found` before the watermark is read; the
/// refused project's watermark is opened before its merge request list is read, so a foreign or
/// malformed watermark answers `stale_cursor` for it, and the caller learns the project is there
/// and closed to it, which the family's table and `gitlab.md` say it cannot tell.
#[tokio::test]
async fn adversary_u23_pass2_a_refused_project_and_an_unknown_one_answer_alike_whatever_the_watermark()
 {
    let engine = engine();
    let foreign = watermark_of_another_project(&engine).await;
    for watermark in [foreign.as_str(), "not-a-watermark"] {
        let unknown = Reads::new(vec![(404, json!({"message": "404 Project Not Found"}))]);
        let unknown = items(
            &engine,
            &unknown,
            json!({"container": "1003", "limit": 10, "watermark": watermark}),
        )
        .await
        .unwrap_err();
        assert_eq!(unknown.code, ErrorCode::NotFound, "{watermark}");
        let refused = Reads::new(vec![
            (200, project(1001)),
            (403, json!({"message": "403 Forbidden"})),
        ]);
        let answer = items(
            &engine,
            &refused,
            json!({"container": "1001", "limit": 10, "watermark": watermark}),
        )
        .await
        .unwrap_err();
        assert_eq!(
            (answer.code, answer.message.as_str()),
            (unknown.code, unknown.message.as_str()),
            "a project GitLab shows and whose merge requests it refuses is told apart from an \
             unknown one by watermark {watermark:?}; provider reads {:?}",
            refused.paths()
        );
    }
}

/// Probe (expected green), `gitlab.md` Errors: a 403 on the project itself stays `forbidden`,
/// and the merge request list is never read.
#[tokio::test]
async fn adversary_u23_pass2_a_403_on_the_project_itself_stays_forbidden() {
    let engine = engine();
    let http = Reads::new(vec![(403, json!({"message": "403 Forbidden"}))]);
    let error = items(&engine, &http, json!({"container": "1001", "limit": 10}))
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Forbidden, "{}", error.message);
    assert_eq!(
        http.paths(),
        [vec!["projects".to_owned(), "1001".to_owned()]]
    );
}

/// Probe (expected green): a 403 on a later page, resumed with the watermark the first page
/// issued for the same project, is `not_found` like the first-page case, and a 401 there stays
/// `unauthorized`: the mapping does not swallow a credential refusal.
#[tokio::test]
async fn adversary_u23_pass2_a_refusal_on_a_resumed_page_is_not_found_and_a_401_stays_unauthorized()
{
    let engine = engine();
    let first = Reads::new(vec![
        (200, project(1001)),
        (200, json!([merge_request(1, "2026-10-06T09:00:00.000Z")])),
    ]);
    let page = items(&engine, &first, json!({"container": "1001", "limit": 1}))
        .await
        .unwrap();
    let watermark = page["next_watermark"].as_str().unwrap().to_owned();
    for (status, expected) in [(403, ErrorCode::NotFound), (401, ErrorCode::Unauthorized)] {
        let later = Reads::new(vec![
            (200, project(1001)),
            (status, json!({"message": format!("{status}")})),
        ]);
        let error = items(
            &engine,
            &later,
            json!({"container": "1001", "limit": 1, "watermark": watermark}),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code, expected, "{status}: {}", error.message);
        assert_eq!(later.paths().len(), 2, "{status}");
    }
}

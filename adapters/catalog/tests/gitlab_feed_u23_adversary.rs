//! Adversary, wave 20261007a, U2 (story:gitlab-feed-binding).
//!
//! The family's error table (`contracts/datasources/feed/v1alpha1/semantics.md`, Errors) answers
//! "an unknown container, one the connection cannot read, or an undeclared direct conversation,
//! indistinguishably" with `not_found`. The GitLab binding reads a project first and its merge
//! requests second (`gitlab.md`, Containers); GitLab can show the project and refuse its merge
//! request list with `403` (the merge request feature disabled, or kept to members while the
//! project is visible). That container is one the connection cannot read.
use connectors_catalog::bundle;
use connectors_catalog_provider::{
    Engine,
    feed::{Declaration, ITEMS},
};
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{collections::VecDeque, path::Path, sync::Mutex};

const INSTANCE: &str = "gitlab-feed-instance";

struct Reads {
    responses: Mutex<VecDeque<(u16, Value)>>,
    paths: Mutex<Vec<Vec<String>>>,
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

/// F1 story:gitlab-feed-binding: GitLab shows the project (200) and refuses its merge request
/// list (403). The family answers a container the connection cannot read `not_found`; the
/// binding answers `forbidden`, a code the family's table does not hold for a container, and the
/// one a consumer reads as "this credential lacks access" rather than "skip this container".
#[tokio::test]
async fn adversary_u23_a_visible_project_whose_merge_requests_gitlab_refuses_is_not_found() {
    let engine = engine();
    let http = Reads {
        responses: Mutex::new(
            vec![
                (200, project(1001)),
                (403, json!({"message": "403 Forbidden"})),
            ]
            .into(),
        ),
        paths: Mutex::new(Vec::new()),
    };
    let error = engine
        .read(
            &http,
            INSTANCE,
            ITEMS,
            json!({"container": "1001", "limit": 10}),
        )
        .await
        .unwrap_err();
    assert_eq!(
        *http.paths.lock().unwrap(),
        [
            vec!["projects".to_owned(), "1001".to_owned()],
            vec![
                "projects".to_owned(),
                "1001".to_owned(),
                "merge_requests".to_owned()
            ],
        ]
    );
    assert_eq!(
        error.code,
        ErrorCode::NotFound,
        "a container the connection cannot read answered {:?}: {}",
        error.code,
        error.message
    );
}

fn merge_request(iid: u64, updated: &str) -> Value {
    let mut record: Value = serde_json::from_slice(
        &std::fs::read(root().join("tests/feed/gitlab/merge_request.json")).unwrap(),
    )
    .unwrap();
    record["iid"] = json!(iid);
    record["id"] = json!(50_000 + iid);
    record["created_at"] = json!("2026-10-06T08:00:00.000Z");
    record["updated_at"] = json!(updated);
    record
}

/// Probe (expected green): three merge requests share one millisecond across a page boundary,
/// and GitLab orders the tied rows differently on the resumed request. Nothing is lost or
/// repeated.
#[tokio::test]
async fn adversary_u23_ties_reordered_across_a_page_boundary_lose_and_repeat_nothing() {
    let engine = engine();
    let at = "2026-10-06T09:00:00.081Z";
    let later = "2026-10-06T09:00:00.082Z";
    let http = Reads {
        responses: Mutex::new(
            vec![
                (200, project(1001)),
                (200, json!([merge_request(1, at), merge_request(2, at)])),
                (200, project(1001)),
                (
                    200,
                    json!([
                        merge_request(3, at),
                        merge_request(2, at),
                        merge_request(1, at),
                        merge_request(4, later)
                    ]),
                ),
            ]
            .into(),
        ),
        paths: Mutex::new(Vec::new()),
    };
    let first = engine
        .read(
            &http,
            INSTANCE,
            ITEMS,
            json!({"container": "1001", "limit": 2}),
        )
        .await
        .unwrap();
    let second = engine
        .read(
            &http,
            INSTANCE,
            ITEMS,
            json!({"container": "1001", "limit": 2, "watermark": first["next_watermark"]}),
        )
        .await
        .unwrap();
    let ids = |page: &Value| -> Vec<String> {
        page["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["id"].as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(ids(&first), ["1", "2"]);
    assert_eq!(ids(&second), ["3", "4"]);
}

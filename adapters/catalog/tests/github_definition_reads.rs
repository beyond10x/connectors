//! story:catalog-github-reads: the ten reads of the released connectors-definitions
//! 0.5.2 `connectors/github.yaml`, lowered by `connectors_catalog::definition` and
//! loaded in the catalog provider's engine, each against a scripted provider. Every
//! read sends its exact path and query (the definition's fixed values, the `since`
//! fold into `q`, `page` and `per_page`) and returns the provider's answer as `body`
//! unchanged, or, for the two raw reads, the decoded text. Paging is the caller's:
//! `page` and `per_page` pass through, a `per_page` above its bound and a search page
//! past the 1,000-result cap are refused before any request. Synthetic answers only;
//! no network, no credential.
#![cfg(unix)]
use connectors_catalog::{bundle, definition};
use connectors_catalog_provider::{Engine, Selection};
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

fn answering(status: u16, body: &Value) -> Scripted {
    Scripted {
        answer: Mutex::new(Some(HttpResponse {
            status,
            headers: BTreeMap::new(),
            body: serde_json::to_vec(body).unwrap(),
        })),
        sent: Mutex::new(Vec::new()),
    }
}

fn sorted(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    let mut pairs: Vec<(String, String)> = pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    pairs.sort();
    pairs
}

/// Reads `id` with `input` against a provider answering 200 with `answer`; the
/// one request must be `path` with exactly the query pairs `query`, in any
/// order, and the read's `body` is returned.
async fn read_one(
    id: &str,
    input: Value,
    answer: &Value,
    path: &[&str],
    query: &[(&str, &str)],
) -> Value {
    let engine = engine();
    let http = answering(200, answer);
    let result = engine
        .read(&http, "github", id, input)
        .await
        .unwrap_or_else(|error| panic!("`{id}` is answered: {}", error.message));
    let sent = http.sent.lock().unwrap();
    assert_eq!(sent.len(), 1, "`{id}` sends one request");
    let (sent_path, sent_query) = &sent[0];
    assert_eq!(sent_path, path, "`{id}` path");
    let mut sent_query = sent_query.clone();
    sent_query.sort();
    assert_eq!(sent_query, sorted(query), "`{id}` query");
    assert_eq!(result["status"], 200, "`{id}` status");
    result["body"].clone()
}

#[tokio::test]
async fn review_requests_list_sends_the_fixed_q_with_since_folded_and_the_page() {
    let answer = json!({
        "total_count": 1,
        "incomplete_results": false,
        "items": [{
            "number": 7,
            "title": "Example change",
            "html_url": "https://example.com/example-org/example/pull/7",
            "pull_request": {"url": "https://example.com/repos/example-org/example/pulls/7"}
        }]
    });
    let body = read_one(
        "review_requests.list",
        json!({"since": "2026-01-02T03:04:05Z", "page": 2, "per_page": 30}),
        &answer,
        &["search", "issues"],
        &[
            (
                "q",
                "is:pr is:open review-requested:@me updated:>=2026-01-02T03:04:05Z",
            ),
            ("page", "2"),
            ("per_page", "30"),
        ],
    )
    .await;
    assert_eq!(body, answer);
}

#[tokio::test]
async fn review_requests_list_without_since_sends_the_fixed_q_alone() {
    let answer = json!({"total_count": 0, "incomplete_results": false, "items": []});
    let body = read_one(
        "review_requests.list",
        json!({}),
        &answer,
        &["search", "issues"],
        &[("q", "is:pr is:open review-requested:@me")],
    )
    .await;
    assert_eq!(body, answer);
}

#[tokio::test]
async fn issues_assigned_sends_the_fixed_filter_and_state_with_since_and_the_page() {
    let answer = json!([{
        "number": 3,
        "title": "Example issue",
        "state": "open",
        "html_url": "https://example.com/example-org/example/issues/3",
        "updated_at": "2026-01-03T00:00:00Z"
    }]);
    let body = read_one(
        "issues.assigned",
        json!({"since": "2026-01-02T00:00:00Z", "page": 2, "per_page": 50}),
        &answer,
        &["issues"],
        &[
            ("filter", "assigned"),
            ("state", "open"),
            ("since", "2026-01-02T00:00:00Z"),
            ("page", "2"),
            ("per_page", "50"),
        ],
    )
    .await;
    assert_eq!(body, answer);
}

#[tokio::test]
async fn notifications_list_sends_all_since_and_the_page() {
    let answer = json!([{
        "id": "1",
        "unread": true,
        "reason": "review_requested",
        "updated_at": "2026-01-03T00:00:00Z",
        "subject": {"title": "Example change", "type": "PullRequest"},
        "repository": {"full_name": "example-org/example"}
    }]);
    let body = read_one(
        "notifications.list",
        json!({"all": true, "since": "2026-01-02T00:00:00Z", "page": 2, "per_page": 50}),
        &answer,
        &["notifications"],
        &[
            ("all", "true"),
            ("since", "2026-01-02T00:00:00Z"),
            ("page", "2"),
            ("per_page", "50"),
        ],
    )
    .await;
    assert_eq!(body, answer);
}

#[tokio::test]
async fn user_me_reads_the_authenticated_user() {
    let answer =
        json!({"login": "example-user", "id": 1, "html_url": "https://example.com/example-user"});
    let body = read_one("user.me", json!({}), &answer, &["user"], &[]).await;
    assert_eq!(body, answer);
}

#[tokio::test]
async fn repos_list_sends_the_org_path_its_order_and_the_page() {
    let answer = json!([{
        "name": "example",
        "full_name": "example-org/example",
        "pushed_at": "2026-01-03T00:00:00Z",
        "archived": false
    }]);
    let body = read_one(
        "repos.list",
        json!({"org": "example-org", "sort": "pushed", "direction": "desc", "page": 2, "per_page": 100}),
        &answer,
        &["orgs", "example-org", "repos"],
        &[
            ("sort", "pushed"),
            ("direction", "desc"),
            ("page", "2"),
            ("per_page", "100"),
        ],
    )
    .await;
    assert_eq!(body, answer);
}

#[tokio::test]
async fn repo_get_reads_one_repository() {
    let answer = json!({
        "full_name": "example-org/example",
        "description": "An example repository",
        "topics": ["example"],
        "homepage": "https://example.com",
        "stargazers_count": 5,
        "pushed_at": "2026-01-03T00:00:00Z",
        "archived": false,
        "visibility": "public"
    });
    let body = read_one(
        "repo.get",
        json!({"owner": "example-org", "repo": "example"}),
        &answer,
        &["repos", "example-org", "example"],
        &[],
    )
    .await;
    assert_eq!(body, answer);
}

#[tokio::test]
async fn releases_list_sends_the_repository_path_and_the_page() {
    let answer =
        json!([{"tag_name": "v1.0.0", "name": "1.0.0", "draft": false, "prerelease": false}]);
    let body = read_one(
        "releases.list",
        json!({"owner": "example-org", "repo": "example", "page": 2, "per_page": 10}),
        &answer,
        &["repos", "example-org", "example", "releases"],
        &[("page", "2"), ("per_page", "10")],
    )
    .await;
    assert_eq!(body, answer);
}

#[tokio::test]
async fn release_latest_reads_the_latest_release() {
    let answer =
        json!({"tag_name": "v1.0.0", "name": "1.0.0", "draft": false, "prerelease": false});
    let body = read_one(
        "release.latest",
        json!({"owner": "example-org", "repo": "example"}),
        &answer,
        &["repos", "example-org", "example", "releases", "latest"],
        &[],
    )
    .await;
    assert_eq!(body, answer);
}

#[tokio::test]
async fn readme_raw_sends_the_ref_and_answers_the_decoded_text() {
    // "# Example\n" in base64, wrapped as GitHub wraps it.
    let answer = json!({"name": "README.md", "content": "IyBFeGFt\ncGxlCg==\n", "encoding": "base64", "size": 10});
    let body = read_one(
        "readme.raw",
        json!({"owner": "example-org", "repo": "example", "ref": "main"}),
        &answer,
        &["repos", "example-org", "example", "readme"],
        &[("ref", "main")],
    )
    .await;
    assert_eq!(body, "# Example\n");
}

#[tokio::test]
async fn file_raw_sends_the_path_and_ref_and_answers_the_decoded_text() {
    // "hello\n" in base64.
    let answer = json!({"name": "guide.md", "type": "file", "content": "aGVsbG8K", "encoding": "base64", "size": 6});
    let body = read_one(
        "file.raw",
        json!({"owner": "example-org", "repo": "example", "path": "guide.md", "ref": "v1.0.0"}),
        &answer,
        &["repos", "example-org", "example", "contents", "guide.md"],
        &[("ref", "v1.0.0")],
    )
    .await;
    assert_eq!(body, "hello\n");
}

/// The five lists, each with the input that names its repository or org and
/// its `per_page` bound.
fn lists() -> [(&'static str, Value, u64); 5] {
    [
        ("review_requests.list", json!({}), 100),
        ("issues.assigned", json!({}), 100),
        ("notifications.list", json!({}), 50),
        ("repos.list", json!({"org": "example-org"}), 100),
        (
            "releases.list",
            json!({"owner": "example-org", "repo": "example"}),
            100,
        ),
    ]
}

fn with(mut input: Value, page: u64, per_page: u64) -> Value {
    input["page"] = json!(page);
    input["per_page"] = json!(per_page);
    input
}

#[tokio::test]
async fn each_list_passes_page_and_per_page_through_unchanged() {
    for (id, input, bound) in lists() {
        let engine = engine();
        let http = answering(200, &json!([]));
        engine
            .read(&http, "github", id, with(input, 2, bound))
            .await
            .unwrap_or_else(|error| panic!("`{id}` is answered: {}", error.message));
        let sent = http.sent.lock().unwrap();
        let (_, query) = &sent[0];
        let mut paging: Vec<(&str, &str)> = query
            .iter()
            .filter(|(name, _)| name == "page" || name == "per_page")
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect();
        paging.sort();
        let bound = bound.to_string();
        assert_eq!(
            paging,
            [("page", "2"), ("per_page", bound.as_str())],
            "`{id}` paging"
        );
    }
}

#[tokio::test]
async fn each_list_refuses_a_per_page_above_its_bound_before_any_request() {
    for (id, input, bound) in lists() {
        let engine = engine();
        let http = answering(200, &json!([]));
        let error = engine
            .read(&http, "github", id, with(input, 2, bound + 1))
            .await
            .expect_err("a per_page above its bound is refused");
        assert_eq!(
            error.code,
            ErrorCode::InvalidInput,
            "`{id}`: {}",
            error.message
        );
        assert!(
            http.sent.lock().unwrap().is_empty(),
            "`{id}` sent no request"
        );
    }
}

#[tokio::test]
async fn review_requests_list_refuses_page_1001_before_any_request() {
    let engine = engine();
    let http = answering(
        200,
        &json!({"total_count": 0, "incomplete_results": false, "items": []}),
    );
    let error = engine
        .read(
            &http,
            "github",
            "review_requests.list",
            json!({"page": 1001, "per_page": 1}),
        )
        .await
        .expect_err("a page past the search cap is refused");
    assert_eq!(error.code, ErrorCode::InvalidInput, "{}", error.message);
    assert!(http.sent.lock().unwrap().is_empty(), "no request was sent");
}

#[tokio::test]
async fn review_requests_list_refuses_the_search_cap_answer_as_invalid_input() {
    let engine = engine();
    let http = answering(
        422,
        &json!({"message": "Only the first 1000 search results are available"}),
    );
    let error = engine
        .read(
            &http,
            "github",
            "review_requests.list",
            json!({"page": 11, "per_page": 100}),
        )
        .await
        .expect_err("a page past the first 1,000 results is refused");
    assert_eq!(error.code, ErrorCode::InvalidInput, "{}", error.message);
    assert_eq!(
        error.upstream_reason.as_deref(),
        Some("Only the first 1000 search results are available")
    );
    assert_eq!(
        http.sent.lock().unwrap().len(),
        1,
        "sent once, never retried"
    );
}

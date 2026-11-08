//! Adversary pass on story:catalog-github-reads: what `github_definition_reads.rs`
//! leaves unsaid about the released connectors-definitions 0.5.2
//! `connectors/github.yaml`. The edges of each declared bound (the search's last
//! page, `page` and `per_page` below their minimum), the `since` fold refusing a
//! value that would widen the search, the fixed and withheld parameters refused
//! as input, a nested `file.raw` path, the onboarding page's claims about the two
//! raw reads (a directory, a binary file, the 1 MiB bound), the spent rate limit
//! with a past or missing reset, and the search cap's 422 in the shape GitHub
//! answers it. Synthetic answers only; no network, no credential.
#![cfg(unix)]
use base64::Engine as _;
use connectors_catalog::{bundle, definition};
use connectors_catalog_provider::{Engine, Selection};
use connectors_core::{Error, ErrorCode, Result};
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

fn answering_with(status: u16, headers: &[(&str, &str)], body: &Value) -> Scripted {
    Scripted {
        answer: Mutex::new(Some(HttpResponse {
            status,
            headers: headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<BTreeMap<_, _>>(),
            body: serde_json::to_vec(body).unwrap(),
        })),
        sent: Mutex::new(Vec::new()),
    }
}

fn answering(status: u16, body: &Value) -> Scripted {
    answering_with(status, &[], body)
}

fn sent(http: &Scripted) -> Vec<Sent> {
    http.sent.lock().unwrap().clone()
}

/// `id` with `input` is refused `invalid_input` and nothing is sent.
async fn refused_before_any_request(engine: &Engine, id: &str, input: Value) {
    let http = answering(200, &json!([]));
    let error = engine
        .read(&http, "github", id, input.clone())
        .await
        .err()
        .unwrap_or_else(|| panic!("`{id}` with {input} is refused"));
    assert_eq!(
        error.code,
        ErrorCode::InvalidInput,
        "`{id}` with {input}: {}",
        error.message
    );
    assert!(
        sent(&http).is_empty(),
        "`{id}` with {input} sent no request"
    );
}

/// The query value sent under `name`, exactly once.
fn query_value(sent: &Sent, name: &str) -> String {
    let values: Vec<&String> = sent
        .1
        .iter()
        .filter(|(key, _)| key == name)
        .map(|(_, value)| value)
        .collect();
    assert_eq!(values.len(), 1, "`{name}` is sent once: {:?}", sent.1);
    values[0].clone()
}

// Bounds: the edges `github_definition_reads.rs` does not step on. A search
// page maximum of 999 instead of 1000, or a dropped minimum, passes that file.

#[tokio::test]
async fn review_requests_list_sends_page_1000_the_last_page_its_bound_admits() {
    let engine = engine();
    let http = answering(
        200,
        &json!({"total_count": 1000, "incomplete_results": false, "items": []}),
    );
    engine
        .read(
            &http,
            "github",
            "review_requests.list",
            json!({"page": 1000, "per_page": 1}),
        )
        .await
        .unwrap_or_else(|error| panic!("page 1000 is read: {}", error.message));
    let sent = sent(&http);
    assert_eq!(sent.len(), 1);
    assert_eq!(query_value(&sent[0], "page"), "1000");
    assert_eq!(query_value(&sent[0], "per_page"), "1");
}

#[tokio::test]
async fn review_requests_list_refuses_page_0_before_any_request() {
    refused_before_any_request(
        &engine(),
        "review_requests.list",
        json!({"page": 0, "per_page": 10}),
    )
    .await;
}

#[tokio::test]
async fn each_list_refuses_per_page_0_before_any_request() {
    let engine = engine();
    for (id, input) in [
        ("review_requests.list", json!({})),
        ("issues.assigned", json!({})),
        ("notifications.list", json!({})),
        ("repos.list", json!({"org": "example-org"})),
        (
            "releases.list",
            json!({"owner": "example-org", "repo": "example"}),
        ),
    ] {
        let mut input = input;
        input["per_page"] = json!(0);
        refused_before_any_request(&engine, id, input).await;
    }
}

#[tokio::test]
async fn each_list_sends_per_page_1_its_minimum() {
    let engine = engine();
    for (id, input) in [
        ("review_requests.list", json!({})),
        ("issues.assigned", json!({})),
        ("notifications.list", json!({})),
        ("repos.list", json!({"org": "example-org"})),
        (
            "releases.list",
            json!({"owner": "example-org", "repo": "example"}),
        ),
    ] {
        let mut input = input;
        input["per_page"] = json!(1);
        let http = answering(200, &json!([]));
        engine
            .read(&http, "github", id, input)
            .await
            .unwrap_or_else(|error| panic!("`{id}` per_page 1 is read: {}", error.message));
        assert_eq!(query_value(&sent(&http)[0], "per_page"), "1", "`{id}`");
    }
}

// The `since` fold appends caller text to the fixed `q`: anything but a
// timestamp would widen or redirect the search, so it is refused unsent.

#[tokio::test]
async fn review_requests_list_refuses_a_since_that_is_no_timestamp_before_any_request() {
    let engine = engine();
    for since in [
        json!("2026-01-02 repo:example-org/private"),
        json!("2026-01-02 review-requested:example-user"),
        json!("yesterday"),
        json!(""),
        json!(20260102),
    ] {
        refused_before_any_request(&engine, "review_requests.list", json!({ "since": since }))
            .await;
    }
}

#[tokio::test]
async fn review_requests_list_folds_an_offset_date_time() {
    let engine = engine();
    let http = answering(
        200,
        &json!({"total_count": 0, "incomplete_results": false, "items": []}),
    );
    engine
        .read(
            &http,
            "github",
            "review_requests.list",
            json!({"since": "2026-01-02T03:04:05+01:00"}),
        )
        .await
        .unwrap_or_else(|error| panic!("the read is answered: {}", error.message));
    assert_eq!(
        query_value(&sent(&http)[0], "q"),
        "is:pr is:open review-requested:@me updated:>=2026-01-02T03:04:05+01:00"
    );
}

// Fixed and withheld parameters are no input.

#[tokio::test]
async fn review_requests_list_refuses_a_caller_q_sort_or_order_before_any_request() {
    let engine = engine();
    for input in [
        json!({"q": "is:pr repo:example-org/private"}),
        json!({"sort": "created"}),
        json!({"order": "asc"}),
    ] {
        refused_before_any_request(&engine, "review_requests.list", input).await;
    }
}

#[tokio::test]
async fn issues_assigned_refuses_a_caller_filter_before_any_request() {
    refused_before_any_request(&engine(), "issues.assigned", json!({"filter": "all"})).await;
}

// The two raw reads.

#[tokio::test]
async fn file_raw_sends_a_nested_path_as_one_path_value() {
    let engine = engine();
    let http = answering(
        200,
        &json!({"type": "file", "content": "aGVsbG8K", "encoding": "base64", "size": 6}),
    );
    let answer = engine
        .read(
            &http,
            "github",
            "file.raw",
            json!({"owner": "example-org", "repo": "example", "path": "docs/guide.md"}),
        )
        .await
        .unwrap_or_else(|error| panic!("a nested file is read: {}", error.message));
    assert_eq!(answer["body"], "hello\n");
    let sent = sent(&http);
    assert_eq!(
        sent[0].0,
        [
            "repos",
            "example-org",
            "example",
            "contents",
            "docs/guide.md"
        ]
    );
    assert!(sent[0].1.is_empty(), "no ref, no query: {:?}", sent[0].1);
}

async fn raw_read(id: &str, answer: &Value) -> std::result::Result<Value, Error> {
    let engine = engine();
    let http = answering(200, answer);
    let result = engine
        .read(
            &http,
            "github",
            id,
            match id {
                "file.raw" => json!({"owner": "example-org", "repo": "example", "path": "docs"}),
                _ => json!({"owner": "example-org", "repo": "example"}),
            },
        )
        .await;
    assert_eq!(sent(&http).len(), 1, "`{id}` sends one request");
    result
}

/// The onboarding page: "a directory or a binary file is refused".
#[tokio::test]
async fn file_raw_refuses_a_directory() {
    let directory = json!([
        {"name": "guide.md", "path": "docs/guide.md", "type": "file", "size": 6},
        {"name": "img", "path": "docs/img", "type": "dir", "size": 0}
    ]);
    let error = raw_read("file.raw", &directory)
        .await
        .expect_err("a directory is refused");
    assert_eq!(error.code, ErrorCode::UpstreamProtocol, "{}", error.message);
}

#[tokio::test]
async fn file_raw_refuses_a_binary_file() {
    for bytes in [b"PNG\x00\x01\x02".as_slice(), b"\xff\xfe\xfd".as_slice()] {
        let content = base64::engine::general_purpose::STANDARD.encode(bytes);
        let error = raw_read(
            "file.raw",
            &json!({"type": "file", "content": content, "encoding": "base64", "size": bytes.len()}),
        )
        .await
        .expect_err("a binary file is refused");
        assert_eq!(error.code, ErrorCode::Unsupported, "{}", error.message);
    }
}

#[tokio::test]
async fn file_raw_refuses_a_file_github_does_not_inline_as_too_large() {
    // GitHub answers a file of 1 to 100 MB with empty content and encoding `none`.
    let error = raw_read(
        "file.raw",
        &json!({"type": "file", "content": "", "encoding": "none", "size": 2_000_000}),
    )
    .await
    .expect_err("a file GitHub does not inline is refused");
    assert_eq!(error.code, ErrorCode::Capacity, "{}", error.message);
}

const MIB: usize = 1_048_576;

/// The onboarding page: "at most 1 MiB". Exactly 1 MiB is read; one byte more is
/// refused `capacity`, whether `size` says so or understates it.
#[tokio::test]
async fn the_raw_reads_hold_exactly_1_mib() {
    for id in ["readme.raw", "file.raw"] {
        let text = "a".repeat(MIB);
        let content = base64::engine::general_purpose::STANDARD.encode(&text);
        let answer = raw_read(
            id,
            &json!({"type": "file", "content": content, "encoding": "base64", "size": MIB}),
        )
        .await
        .unwrap_or_else(|error| panic!("`{id}` reads 1 MiB: {}", error.message));
        assert_eq!(answer["body"].as_str().map(str::len), Some(MIB), "`{id}`");

        let over = "a".repeat(MIB + 1);
        let content = base64::engine::general_purpose::STANDARD.encode(&over);
        for size in [json!(MIB + 1), json!(6)] {
            let error = raw_read(
                id,
                &json!({"type": "file", "content": content, "encoding": "base64", "size": size}),
            )
            .await
            .expect_err("one byte past 1 MiB is refused");
            assert_eq!(
                error.code,
                ErrorCode::Capacity,
                "`{id}` size {size}: {}",
                error.message
            );
        }
    }
}

// A spent rate limit whose reset is past, absent or unreadable is still
// `rate_limited`, sent once, and names no wait it cannot know.

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

async fn spent_limit(status: u16, headers: &[(&str, &str)]) -> Error {
    let engine = engine();
    let http = answering_with(
        status,
        headers,
        &json!({"message": "API rate limit exceeded for user ID 1.", "documentation_url": "https://docs.github.com/rest/overview/rate-limits-for-the-rest-api", "status": status.to_string()}),
    );
    let error = engine
        .read(&http, "github", "user.me", json!({}))
        .await
        .expect_err("the answer is a refusal");
    assert_eq!(sent(&http).len(), 1, "sent once, never retried");
    error
}

#[tokio::test]
async fn a_spent_limit_with_a_past_missing_or_unreadable_reset_is_rate_limited_without_a_wait() {
    let past = (now() - 60).to_string();
    for headers in [
        vec![
            ("x-ratelimit-remaining", "0"),
            ("x-ratelimit-reset", past.as_str()),
        ],
        vec![("x-ratelimit-remaining", "0")],
        vec![
            ("x-ratelimit-remaining", "0"),
            ("x-ratelimit-reset", "soon"),
        ],
        vec![("x-ratelimit-remaining", "0"), ("x-ratelimit-reset", "")],
    ] {
        for status in [403, 429] {
            let error = spent_limit(status, &headers).await;
            assert_eq!(
                error.code,
                ErrorCode::RateLimited,
                "{status} {headers:?}: {}",
                error.message
            );
            assert_eq!(
                error.retry_after_seconds, None,
                "{status} {headers:?} names no wait"
            );
        }
    }
}

#[tokio::test]
async fn a_429_with_a_future_reset_waits_until_it() {
    let reset = (now() + 90).to_string();
    let error = spent_limit(
        429,
        &[
            ("x-ratelimit-remaining", "0"),
            ("x-ratelimit-reset", &reset),
        ],
    )
    .await;
    assert_eq!(error.code, ErrorCode::RateLimited, "{}", error.message);
    let wait = error.retry_after_seconds.expect("a future reset is a wait");
    assert!((1..=90).contains(&wait), "{wait}");
}

/// GitHub's secondary limit answers 403 with requests left and `Retry-After`.
#[tokio::test]
async fn a_secondary_limit_waits_for_retry_after_whatever_remaining_says() {
    let reset = (now() + 3000).to_string();
    let error = spent_limit(
        403,
        &[
            ("x-ratelimit-remaining", "4321"),
            ("x-ratelimit-reset", &reset),
            ("retry-after", "60"),
        ],
    )
    .await;
    assert_eq!(error.code, ErrorCode::RateLimited, "{}", error.message);
    assert_eq!(error.retry_after_seconds, Some(60));
}

/// A 403 with requests left and no `Retry-After` is a refusal, not a limit.
#[tokio::test]
async fn a_403_with_requests_left_is_forbidden() {
    let reset = (now() + 3000).to_string();
    for remaining in ["4321", "1"] {
        let error = spent_limit(
            403,
            &[
                ("x-ratelimit-remaining", remaining),
                ("x-ratelimit-reset", &reset),
            ],
        )
        .await;
        assert_eq!(
            error.code,
            ErrorCode::Forbidden,
            "remaining {remaining:?}: {}",
            error.message
        );
    }
}

// The search cap's 422, as GitHub answered it on 2026-11-04 (status and body
// observed with a read-only `GET /search/issues?page=11&per_page=100`).

#[tokio::test]
async fn the_search_cap_422_as_github_answers_it_is_invalid_input_with_its_reason() {
    let engine = engine();
    let http = answering(
        422,
        &json!({
            "message": "Only the first 1000 search results are available",
            "documentation_url": "https://docs.github.com/v3/search/",
            "status": "422"
        }),
    );
    let error = engine
        .read(
            &http,
            "github",
            "review_requests.list",
            json!({"page": 11, "per_page": 100}),
        )
        .await
        .expect_err("the page past the cap is refused");
    assert_eq!(error.code, ErrorCode::InvalidInput, "{}", error.message);
    assert_eq!(
        error.upstream_reason.as_deref(),
        Some("Only the first 1000 search results are available")
    );
    let sent = sent(&http);
    assert_eq!(sent.len(), 1, "sent once, never retried");
    assert_eq!(query_value(&sent[0], "page"), "11");
}

/// The cap's wording is GitHub's, carried from the answer: another 422 carries
/// its own reason, never the cap's.
#[tokio::test]
async fn another_422_carries_its_own_reason_not_the_caps() {
    let engine = engine();
    let http = answering(
        422,
        &json!({"message": "Validation Failed", "errors": [{"resource": "Search", "field": "q", "code": "invalid"}]}),
    );
    let error = engine
        .read(
            &http,
            "github",
            "review_requests.list",
            json!({"page": 1, "per_page": 100}),
        )
        .await
        .expect_err("a 422 is refused");
    assert_eq!(error.code, ErrorCode::InvalidInput, "{}", error.message);
    assert_eq!(error.upstream_reason.as_deref(), Some("Validation Failed"));
}

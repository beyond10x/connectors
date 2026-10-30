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

// A definition the lowerer accepts loads in the engine, and a member the engine
// never reads where it lands is refused. Each case below writes one member the
// wrong way into an otherwise valid definition against the released GitHub
// source; the lowerer refuses it, naming the member, before the engine sees it.

const HEAD: &str = r#"schema: urn:beyond10x:connectors-definition:1
source:
  openapi: upstream/github/api.github.com.yaml.gz
  sha256: 5e9edc27c465626b020eb6d9b4d5892e93b4e96f8c7af53ff952ac59b48c0df9
base_url: https://api.github.com
auth:
  github.token:
    scheme: bearer
    label: GitHub token
    identity: {path: user, kind: github.user, subject: /id}
"#;

enum Outcome {
    /// The lowerer refused, with its refusal.
    Unlowered(String),
    /// Lowered, and the engine refused at load, with its refusal.
    Unloaded(String),
    /// Lowered and loaded, with the lowered selection.
    Loaded(Value),
}

fn outcome(rest: &str) -> Outcome {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("github.yaml");
    std::fs::write(&file, format!("{HEAD}{rest}")).unwrap();
    let scratch = temp.path().join("scratch");
    std::fs::create_dir_all(&scratch).unwrap();
    let out = temp.path().join("out");
    let lowered = match definition::lower(&definition::Request {
        definition: &file,
        root: &released(),
        out: &out,
        scratch: &scratch,
    }) {
        Ok(lowered) => lowered,
        Err(refusal) => return Outcome::Unlowered(format!("{refusal:?}")),
    };
    let written: Value =
        serde_json::from_slice(&std::fs::read(&lowered.operations).unwrap()).unwrap();
    let selections: Vec<Selection> = serde_json::from_value(written["operations"].clone())
        .expect("the lowered selection reads as the engine's selections");
    let bundle = bundle::load(&out, &lowered.provider).unwrap();
    match Engine::new(&bundle, "", &selections) {
        Ok(_) => Outcome::Loaded(written),
        Err(error) => Outcome::Unloaded(error.message),
    }
}

/// The control first: without the hostile member the definition lowers and loads.
fn control(rest: &str) -> Value {
    match outcome(rest) {
        Outcome::Loaded(written) => written,
        Outcome::Unlowered(refusal) => panic!("control does not lower: {refusal}"),
        Outcome::Unloaded(refusal) => panic!("control does not load: {refusal}"),
    }
}

/// The lowerer refuses `rest`, and its refusal names `member`; `control_rest`,
/// the same definition without the hostile member, lowers and loads.
fn lowerer_refuses(control_rest: &str, rest: &str, member: &str) {
    control(control_rest);
    match outcome(rest) {
        Outcome::Unlowered(refusal) => assert!(
            refusal.contains(member),
            "story:definition-lowerer-reads-published-features: the lowerer's refusal names `{member}`: {refusal}"
        ),
        Outcome::Unloaded(refusal) => panic!(
            "story:definition-lowerer-reads-published-features: the lowerer accepted a definition the engine refuses at load: {refusal}"
        ),
        Outcome::Loaded(_) => panic!(
            "story:definition-lowerer-reads-published-features: the definition lowered and loaded; the engine never reads `{member}` there"
        ),
    }
}

const ME: &str = "operations:\n  user.me: {op: users/get-authenticated}\n";
const ISSUES: &str = "operations:\n  issues.assigned: {op: issues/list}\n";
const README: &str = "operations:\n  readme.raw: {op: repos/get-readme, embedded_text: {content: /content, encoding: /encoding, max_bytes: 1024}}\n";
const DOWNLOAD: &str = "operations:\n  archive.get: {download: {path_prefix: /example-org/}, response: binary, binary: {max_bytes: 1024, hosts: [\"https://codeload.github.com\"]}}\n";
const SEARCH: &str = "operations:\n  review_requests.list: {op: search/issues-and-pull-requests}\n";

fn download_with(member: &str) -> String {
    format!(
        "operations:\n  archive.get: {{download: {{path_prefix: /example-org/}}, response: binary, binary: {{max_bytes: 1024, hosts: [\"https://codeload.github.com\"]}}, {member}}}\n"
    )
}

/// The lowerer's own copies of the engine's limits are the engine's.
#[test]
fn the_lowerer_limits_are_the_engine_limits() {
    assert_eq!(
        definition::QUERY_FIXED_MAX,
        connectors_catalog_provider::QUERY_FIXED
    );
    assert_eq!(
        definition::EMBEDDED_TEXT_MAX_BYTES,
        connectors_catalog_provider::BINARY_LIMIT
    );
}

// rate_limit_headers: the engine wants two distinct HTTP header tokens.

#[test]
fn rate_limit_headers_naming_one_header_twice_are_refused() {
    lowerer_refuses(
        ME,
        "defaults:\n  rate_limit_headers: {remaining: x-ratelimit-reset, reset: X-RateLimit-Reset}\n\
         operations:\n  user.me: {op: users/get-authenticated}\n",
        "rate_limit_headers",
    );
}

#[test]
fn rate_limit_headers_with_an_empty_name_are_refused() {
    lowerer_refuses(
        ME,
        "operations:\n  user.me: {op: users/get-authenticated, rate_limit_headers: {remaining: \"\", reset: x-ratelimit-reset}}\n",
        "rate_limit_headers",
    );
}

#[test]
fn rate_limit_headers_with_a_name_that_is_no_header_token_are_refused() {
    lowerer_refuses(
        ME,
        "operations:\n  user.me: {op: users/get-authenticated, rate_limit_headers: {remaining: \"x ratelimit remaining\", reset: x-ratelimit-reset}}\n",
        "rate_limit_headers",
    );
}

/// A default no operation takes is still refused as written.
#[test]
fn malformed_default_rate_limit_headers_are_refused_beside_only_a_download() {
    lowerer_refuses(
        DOWNLOAD,
        &format!("defaults:\n  rate_limit_headers: {{remaining: x-a, reset: X-A}}\n{DOWNLOAD}"),
        "rate_limit_headers",
    );
}

/// The default applies to every operation the engine reads it on: a download
/// reads none, so the default does not reach it, and the definition loads.
#[test]
fn default_rate_limit_headers_do_not_reach_a_download() {
    let written = control(&format!(
        "defaults:\n  rate_limit_headers: {{remaining: x-ratelimit-remaining, reset: x-ratelimit-reset}}\n\
         operations:\n  user.me: {{op: users/get-authenticated}}\n{}",
        DOWNLOAD.trim_start_matches("operations:\n")
    ));
    let entry = |id: &str| {
        written["operations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["id"] == id)
            .cloned()
            .unwrap_or_else(|| panic!("`{id}` is in the selection"))
    };
    assert!(entry("archive.get").get("rate_limit_headers").is_none());
    assert_eq!(
        entry("user.me")["rate_limit_headers"],
        serde_json::json!({"remaining": "x-ratelimit-remaining", "reset": "x-ratelimit-reset"})
    );
}

// query_fixed: the engine fixes up to eight non-empty values of query
// parameters it neither bounds, requires, withholds nor passes as the credential.

#[test]
fn query_fixed_on_a_path_parameter_is_refused() {
    lowerer_refuses(
        "operations:\n  repo.get: {op: repos/get}\n",
        "operations:\n  repo.get: {op: repos/get, query_fixed: {owner: example-org}}\n",
        "query_fixed",
    );
}

#[test]
fn query_fixed_on_a_parameter_the_operation_does_not_declare_is_refused() {
    lowerer_refuses(
        ISSUES,
        "operations:\n  issues.assigned: {op: issues/list, query_fixed: {nonexistent: x}}\n",
        "query_fixed",
    );
}

#[test]
fn query_fixed_to_the_empty_value_is_refused() {
    lowerer_refuses(
        ISSUES,
        "operations:\n  issues.assigned: {op: issues/list, query_fixed: {state: \"\"}}\n",
        "query_fixed",
    );
}

#[test]
fn query_fixed_on_more_than_eight_parameters_is_refused() {
    lowerer_refuses(
        ISSUES,
        "operations:\n  issues.assigned:\n    op: issues/list\n    \
         query_fixed: {filter: all, state: open, labels: a, sort: created, direction: asc, collab: \"true\", orgs: \"true\", owned: \"true\", pulls: \"true\"}\n",
        "query_fixed",
    );
}

#[test]
fn query_fixed_on_a_parameter_the_defaults_require_is_refused() {
    lowerer_refuses(
        "defaults:\n  required: [since]\noperations:\n  issues.assigned: {op: issues/list}\n",
        "defaults:\n  required: [since]\n\
         operations:\n  issues.assigned: {op: issues/list, query_fixed: {since: \"2026-01-01T00:00:00Z\"}}\n",
        "query_fixed",
    );
}

#[test]
fn query_fixed_on_a_parameter_it_withholds_is_refused() {
    lowerer_refuses(
        "operations:\n  issues.assigned: {op: issues/list, withhold: [state]}\n",
        "operations:\n  issues.assigned: {op: issues/list, withhold: [state], query_fixed: {state: open}}\n",
        "query_fixed",
    );
}

#[test]
fn query_fixed_on_a_parameter_it_bounds_is_refused() {
    lowerer_refuses(
        "operations:\n  issues.assigned: {op: issues/list, bounds: {per_page: {minimum: 1, maximum: 100}}}\n",
        "operations:\n  issues.assigned: {op: issues/list, bounds: {per_page: {minimum: 1, maximum: 100}}, query_fixed: {per_page: \"100\"}}\n",
        "query_fixed",
    );
}

/// The author declares no bound: the lowerer derives one from the paging
/// style's `limit`, on the parameter the operation fixes.
#[test]
fn query_fixed_on_the_paging_limit_the_lowerer_bounds_itself_is_refused() {
    lowerer_refuses(
        "paging:\n  pages: {request: page, limit: per_page, max: 100}\n\
         operations:\n  issues.assigned: {op: issues/list}\n",
        "paging:\n  pages: {request: page, limit: per_page, max: 100}\n\
         operations:\n  issues.assigned: {op: issues/list, query_fixed: {per_page: \"100\"}}\n",
        "query_fixed",
    );
}

// query_folds: each named by an id that is no parameter of the operation, into a
// value it fixes and no other fold joins, with a prefix free of whitespace.

#[test]
fn query_folds_into_a_value_it_does_not_fix_are_refused() {
    lowerer_refuses(
        SEARCH,
        "operations:\n  review_requests.list:\n    op: search/issues-and-pull-requests\n    \
         query_folds: {since: {into: q, prefix: \"updated:>=\", format: timestamp}}\n",
        "query_folds",
    );
}

#[test]
fn query_folds_named_after_a_parameter_of_the_operation_are_refused() {
    lowerer_refuses(
        ISSUES,
        "operations:\n  issues.assigned:\n    op: issues/list\n    \
         query_fixed: {labels: bug}\n    \
         query_folds: {since: {into: labels, prefix: \"x:\", format: timestamp}}\n",
        "query_folds",
    );
}

#[test]
fn query_folds_named_by_no_id_are_refused() {
    lowerer_refuses(
        SEARCH,
        "operations:\n  review_requests.list:\n    op: search/issues-and-pull-requests\n    \
         query_fixed: {q: \"is:pr\"}\n    \
         query_folds: {\"since date\": {into: q, prefix: \"updated:>=\", format: timestamp}}\n",
        "query_folds",
    );
}

#[test]
fn query_folds_with_an_empty_prefix_are_refused() {
    lowerer_refuses(
        SEARCH,
        "operations:\n  review_requests.list:\n    op: search/issues-and-pull-requests\n    \
         query_fixed: {q: \"is:pr\"}\n    \
         query_folds: {since: {into: q, prefix: \"\", format: timestamp}}\n",
        "query_folds",
    );
}

#[test]
fn query_folds_with_a_prefix_holding_whitespace_are_refused() {
    lowerer_refuses(
        SEARCH,
        "operations:\n  review_requests.list:\n    op: search/issues-and-pull-requests\n    \
         query_fixed: {q: \"is:pr\"}\n    \
         query_folds: {since: {into: q, prefix: \"updated: >=\", format: timestamp}}\n",
        "query_folds",
    );
}

#[test]
fn two_query_folds_into_one_value_are_refused() {
    lowerer_refuses(
        SEARCH,
        "operations:\n  review_requests.list:\n    op: search/issues-and-pull-requests\n    \
         query_fixed: {q: \"is:pr\"}\n    \
         query_folds: {since: {into: q, prefix: \"updated:>=\", format: timestamp}, after: {into: q, prefix: \"created:>=\", format: timestamp}}\n",
        "query_folds",
    );
}

// embedded_text: pointers with a leading `/` and max_bytes from 1 to 2 MiB.

#[test]
fn embedded_text_with_a_pointer_without_a_leading_slash_is_refused() {
    lowerer_refuses(
        README,
        "operations:\n  readme.raw: {op: repos/get-readme, embedded_text: {content: content, encoding: /encoding, max_bytes: 1024}}\n",
        "embedded_text",
    );
}

#[test]
fn embedded_text_with_zero_max_bytes_is_refused() {
    lowerer_refuses(
        README,
        "operations:\n  readme.raw: {op: repos/get-readme, embedded_text: {content: /content, encoding: /encoding, max_bytes: 0}}\n",
        "embedded_text",
    );
}

#[test]
fn embedded_text_with_max_bytes_above_the_binary_limit_is_refused() {
    lowerer_refuses(
        README,
        "operations:\n  readme.raw: {op: repos/get-readme, embedded_text: {content: /content, encoding: /encoding, max_bytes: 2097153}}\n",
        "embedded_text",
    );
}

/// `meta/get-zen` answers only `text/plain`: the engine reads its answer as a
/// string, so an embedded text never reads a file from it and every call fails.
#[test]
fn embedded_text_on_an_operation_answering_only_text_is_refused() {
    lowerer_refuses(
        "operations:\n  zen.get: {op: meta/get-zen}\n",
        "operations:\n  zen.get: {op: meta/get-zen, embedded_text: {content: /content, encoding: /encoding, max_bytes: 1024}}\n",
        "embedded_text",
    );
}

// A download: the engine's download path reads none of query_fixed,
// query_folds or rate_limit_headers, and its load checks none of them.

#[test]
fn query_fixed_on_a_download_is_refused() {
    lowerer_refuses(
        DOWNLOAD,
        &download_with("query_fixed: {ref: main}"),
        "query_fixed",
    );
}

#[test]
fn query_folds_on_a_download_are_refused() {
    lowerer_refuses(
        DOWNLOAD,
        &download_with(
            "query_fixed: {q: x}, query_folds: {since: {into: q, prefix: \"u:\", format: timestamp}}",
        ),
        "query_folds",
    );
}

#[test]
fn rate_limit_headers_on_a_download_are_refused() {
    lowerer_refuses(
        DOWNLOAD,
        &download_with(
            "rate_limit_headers: {remaining: x-ratelimit-remaining, reset: x-ratelimit-reset}",
        ),
        "rate_limit_headers",
    );
}

//! The four optional members a published connectors-definitions release uses
//! (story:definition-lowerer-reads-published-features): `rate_limit_headers`
//! on `defaults` or an operation, and `query_fixed`, `query_folds` and
//! `embedded_text` on an operation, each carried into the selection as the
//! catalog provider reads it. connectors-definitions 0.5.2's
//! `connectors/github.yaml` (`tests/fixtures/definitions-0.5.2/`, its bytes and
//! its pinned source as the release tag holds them) lowers. A member on an
//! operation the catalog provider refuses it on is refused when lowering.
#![cfg(unix)]
use connectors_catalog::definition::{self, Refusal, Request};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// SHA-256 of `connectors/github.yaml` at connectors-definitions tag 0.5.2.
const RELEASED_GITHUB_SHA256: &str =
    "e5408d1307ae01f3a5fbfb32587ecb72288bf0ea873cfb47e3383d1e5ef91cc2";

/// The operation keys of that file, in its order.
const RELEASED_GITHUB_OPERATIONS: [&str; 10] = [
    "review_requests.list",
    "issues.assigned",
    "notifications.list",
    "user.me",
    "repos.list",
    "repo.get",
    "releases.list",
    "release.latest",
    "readme.raw",
    "file.raw",
];

/// The released file's head: its source, base URL and auth profile.
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

const HEADERS: &str = "{remaining: x-ratelimit-remaining, reset: x-ratelimit-reset}";

fn released() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/definitions-0.5.2")
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    hex::encode(sha2::Sha256::digest(bytes))
}

/// Lower `file` against the released definitions root: the selection, or the
/// refusal.
fn lower_file(file: &Path) -> Result<Value, Refusal> {
    let temp = tempfile::tempdir().unwrap();
    let scratch = temp.path().join("scratch");
    std::fs::create_dir_all(&scratch).unwrap();
    let lowered = definition::lower(&Request {
        definition: file,
        root: &released(),
        out: &temp.path().join("out"),
        scratch: &scratch,
    })?;
    Ok(serde_json::from_slice(&std::fs::read(&lowered.operations).unwrap()).unwrap())
}

/// Lower `HEAD` followed by `rest` as `github.yaml`.
fn lower(rest: &str) -> Result<Value, Refusal> {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("github.yaml");
    std::fs::write(&file, format!("{HEAD}{rest}")).unwrap();
    lower_file(&file)
}

fn entry<'a>(selection: &'a Value, id: &str) -> &'a Value {
    selection["operations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["id"] == id)
        .unwrap_or_else(|| panic!("`{id}` is in the selection"))
}

fn malformed(refusal: Refusal) -> String {
    match refusal {
        Refusal::Malformed(detail) => detail,
        other => panic!("refused as {other:?}, not malformed"),
    }
}

fn headers() -> Value {
    json!({"remaining": "x-ratelimit-remaining", "reset": "x-ratelimit-reset"})
}

#[test]
fn defaults_rate_limit_headers_reach_every_operation() {
    let selection = lower(&format!(
        "defaults:\n  rate_limit_headers: {HEADERS}\noperations:\n  \
         user.me: {{op: users/get-authenticated}}\n  \
         readme.raw: {{op: repos/get-readme, embedded_text: {{content: /content, encoding: /encoding, max_bytes: 1024}}}}\n  \
         readme.text: {{op: repos/get-readme, response: text}}\n  \
         issue.create: {{op: issues/create, effect: write, body: {{title: string!}}}}\n"
    ))
    .expect("the definition lowers");
    for id in ["user.me", "readme.raw", "readme.text", "issue.create"] {
        assert_eq!(
            entry(&selection, id)["rate_limit_headers"],
            headers(),
            "`{id}`"
        );
    }
}

#[test]
fn an_operation_rate_limit_headers_replace_the_default() {
    let selection = lower(&format!(
        "defaults:\n  rate_limit_headers: {HEADERS}\noperations:\n  \
         user.me: {{op: users/get-authenticated, rate_limit_headers: {{remaining: x-left, reset: x-again}}}}\n  \
         repo.get: {{op: repos/get}}\n"
    ))
    .expect("the definition lowers");
    assert_eq!(
        entry(&selection, "user.me")["rate_limit_headers"],
        json!({"remaining": "x-left", "reset": "x-again"})
    );
    assert_eq!(
        entry(&selection, "repo.get")["rate_limit_headers"],
        headers()
    );
    let without = lower("operations:\n  user.me: {op: users/get-authenticated}\n")
        .expect("the definition lowers");
    assert!(
        entry(&without, "user.me")
            .get("rate_limit_headers")
            .is_none(),
        "no rate_limit_headers is written where none is declared"
    );
}

#[test]
fn query_fixed_reaches_the_selection() {
    let selection = lower(
        "operations:\n  issues.assigned:\n    op: issues/list\n    \
         query_fixed: {filter: assigned, state: open}\n",
    )
    .expect("the definition lowers");
    assert_eq!(
        entry(&selection, "issues.assigned")["query_fixed"],
        json!({"filter": "assigned", "state": "open"})
    );
    let without = lower("operations:\n  issues.all: {op: issues/list}\n").unwrap();
    assert!(entry(&without, "issues.all").get("query_fixed").is_none());
    let detail = malformed(
        lower(
            "operations:\n  issue.create:\n    op: issues/create\n    effect: write\n    \
             body: {title: string!}\n    query_fixed: {labels: bug}\n",
        )
        .expect_err("query_fixed on a write is refused"),
    );
    assert!(
        detail.contains("issue.create") && detail.contains("query_fixed"),
        "{detail}"
    );
}

#[test]
fn query_folds_reach_the_selection() {
    let selection = lower(
        "operations:\n  review_requests.list:\n    op: search/issues-and-pull-requests\n    \
         query_fixed: {q: \"is:pr is:open review-requested:@me\"}\n    \
         query_folds: {since: {into: q, prefix: \"updated:>=\", format: timestamp}}\n",
    )
    .expect("the definition lowers");
    let entry = entry(&selection, "review_requests.list");
    assert_eq!(
        entry["query_folds"],
        json!({"since": {"into": "q", "prefix": "updated:>=", "format": "timestamp"}})
    );
    assert_eq!(
        entry["query_fixed"],
        json!({"q": "is:pr is:open review-requested:@me"})
    );
    let detail = malformed(
        lower(
            "operations:\n  issue.create:\n    op: issues/create\n    effect: write\n    \
             body: {title: string!}\n    \
             query_folds: {since: {into: q, prefix: \"updated:>=\", format: timestamp}}\n",
        )
        .expect_err("query_folds on a write is refused"),
    );
    assert!(
        detail.contains("issue.create") && detail.contains("query_folds"),
        "{detail}"
    );
    let unknown = lower(
        "operations:\n  review_requests.list:\n    op: search/issues-and-pull-requests\n    \
         query_fixed: {q: open}\n    \
         query_folds: {since: {into: q, prefix: \"updated:>=\", format: date}}\n",
    )
    .expect_err("a fold format the engine does not know is refused");
    assert!(matches!(unknown, Refusal::Malformed(_)), "{unknown:?}");
}

#[test]
fn embedded_text_reaches_the_selection() {
    let selection = lower(
        "operations:\n  readme.raw:\n    op: repos/get-readme\n    \
         embedded_text: {content: /content, encoding: /encoding, size: /size, max_bytes: 1048576}\n  \
         file.raw:\n    op: repos/get-content\n    \
         embedded_text: {content: /content, encoding: /encoding, max_bytes: 4096}\n",
    )
    .expect("the definition lowers");
    assert_eq!(
        entry(&selection, "readme.raw")["embedded_text"],
        json!({"content": "/content", "encoding": "/encoding", "size": "/size", "max_bytes": 1_048_576})
    );
    assert_eq!(
        entry(&selection, "file.raw")["embedded_text"],
        json!({"content": "/content", "encoding": "/encoding", "max_bytes": 4096})
    );
}

/// The catalog provider admits `embedded_text` only on a read answering JSON
/// that reads its answer no other way; a default `envelope` applies only to
/// reads answering JSON, so it does not reach an embedded-text read.
#[test]
fn embedded_text_where_the_engine_refuses_it_is_refused() {
    const TEXT: &str = "embedded_text: {content: /content, encoding: /encoding, max_bytes: 4096}";
    for (case, operation) in [
        (
            "a write",
            format!("{{op: issues/create, effect: write, body: {{title: string!}}, {TEXT}}}"),
        ),
        (
            "a text response",
            format!("{{op: repos/get-readme, response: text, {TEXT}}}"),
        ),
        (
            "an own envelope",
            format!(
                "{{op: repos/get-readme, envelope: {{ok: /ok, error: /error, otherwise: internal}}, {TEXT}}}"
            ),
        ),
        (
            "a binary bound",
            format!("{{op: repos/get-readme, binary: {{max_bytes: 10}}, {TEXT}}}"),
        ),
    ] {
        let detail = malformed(
            lower(&format!("operations:\n  readme.raw: {operation}\n"))
                .expect_err("embedded_text there is refused"),
        );
        assert!(
            detail.contains("readme.raw") && detail.contains("embedded_text"),
            "{case}: {detail}"
        );
    }
    let selection = lower(&format!(
        "defaults:\n  envelope: {{ok: /ok, error: /error, otherwise: internal}}\noperations:\n  \
         readme.raw: {{op: repos/get-readme, {TEXT}}}\n  user.me: {{op: users/get-authenticated}}\n"
    ))
    .expect("a default envelope leaves an embedded-text read alone");
    assert!(entry(&selection, "readme.raw").get("envelope").is_none());
    assert!(entry(&selection, "user.me").get("envelope").is_some());
}

#[test]
fn the_released_github_definition_lowers() {
    let file = released().join("connectors/github.yaml");
    let bytes = std::fs::read(&file).unwrap();
    assert_eq!(
        sha256(&bytes),
        RELEASED_GITHUB_SHA256,
        "the fixture is connectors-definitions 0.5.2's connectors/github.yaml byte for byte"
    );
    let parsed = definition::parse(&bytes).expect("the released definition parses");
    let keys: Vec<&str> = parsed.operations.keys().map(String::as_str).collect();
    assert_eq!(keys, RELEASED_GITHUB_OPERATIONS);
    let selection = lower_file(&file).expect("the released definition lowers");
    assert_eq!(selection["provider"], "github");
    let ids: Vec<&str> = selection["operations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, RELEASED_GITHUB_OPERATIONS);
    for id in RELEASED_GITHUB_OPERATIONS {
        assert_eq!(
            entry(&selection, id)["rate_limit_headers"],
            headers(),
            "`{id}`"
        );
    }
    assert_eq!(
        entry(&selection, "issues.assigned")["query_fixed"],
        json!({"filter": "assigned", "state": "open"})
    );
    assert_eq!(
        entry(&selection, "review_requests.list")["query_folds"]["since"]["into"],
        "q"
    );
    for id in ["readme.raw", "file.raw"] {
        assert_eq!(
            entry(&selection, id)["embedded_text"]["max_bytes"],
            1_048_576,
            "`{id}`"
        );
    }
}

//! Adversary pass on D247 (story:definition-lowerer-reads-published-features).
//! `lowerer_features.rs` proves each FEATURES member against the released GitHub
//! definition, which writes `rate_limit_headers` only under `defaults`: a lowerer
//! that read it there and dropped it on an operation would pass. Here every
//! member install admits is written on an operation alone, with no defaults, and
//! must reach that operation's entry as written.
#![cfg(unix)]
use connectors_catalog::definition::{self, Request};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

fn released() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../connectors-catalog/tests/fixtures/definitions-0.5.2")
}

const DEFINITION: &str = r#"schema: urn:beyond10x:connectors-definition:1
source:
  openapi: upstream/github/api.github.com.yaml.gz
  sha256: 5e9edc27c465626b020eb6d9b4d5892e93b4e96f8c7af53ff952ac59b48c0df9
base_url: https://api.github.com
auth:
  github.token:
    scheme: bearer
    label: GitHub token
    identity: {path: user, kind: github.user, subject: /id}
operations:
  user.me:
    op: users/get-authenticated
    rate_limit_headers: {remaining: x-left, reset: x-again}
  issues.assigned:
    op: issues/list
    query_fixed: {filter: assigned}
  review_requests.list:
    op: search/issues-and-pull-requests
    query_fixed: {q: "is:pr"}
    query_folds: {since: {into: q, prefix: "updated:>=", format: timestamp}}
  readme.raw:
    op: repos/get-readme
    embedded_text: {content: /content, encoding: /encoding, max_bytes: 64}
"#;

#[test]
fn each_admitted_feature_written_on_an_operation_reaches_that_operation() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("github.yaml");
    std::fs::write(&file, DEFINITION).unwrap();
    let scratch = temp.path().join("scratch");
    std::fs::create_dir_all(&scratch).unwrap();
    let lowered = definition::lower(&Request {
        definition: &file,
        root: &released(),
        out: &temp.path().join("out"),
        scratch: &scratch,
    })
    .expect("the definition lowers");
    let selection: Value =
        serde_json::from_slice(&std::fs::read(&lowered.operations).unwrap()).unwrap();
    let entry = |id: &str| {
        selection["operations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["id"] == id)
            .cloned()
            .unwrap_or_else(|| panic!("`{id}` is in the selection"))
    };
    let expected = [
        (
            "embedded_text",
            "readme.raw",
            json!({"content": "/content", "encoding": "/encoding", "max_bytes": 64}),
        ),
        (
            "query_fixed",
            "issues.assigned",
            json!({"filter": "assigned"}),
        ),
        (
            "query_folds",
            "review_requests.list",
            json!({"since": {"into": "q", "prefix": "updated:>=", "format": "timestamp"}}),
        ),
        (
            "rate_limit_headers",
            "user.me",
            json!({"remaining": "x-left", "reset": "x-again"}),
        ),
    ];
    let named: Vec<&str> = expected.iter().map(|(feature, _, _)| *feature).collect();
    assert_eq!(
        connectors_definitions_install::FEATURES,
        named.as_slice(),
        "this case covers every feature install admits"
    );
    for (feature, id, value) in expected {
        assert_eq!(entry(id)[feature], value, "`{feature}` on `{id}`");
    }
}

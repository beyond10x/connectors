//! Adversary pass on story:definition-lowerer-reads-published-features: the
//! lowering of `rate_limit_headers`, `query_fixed`, `query_folds` and
//! `embedded_text` held to determinism and to the members' shapes. The cases
//! that load the lowered selection through the catalog provider's engine live
//! in `adapters/catalog/tests/definition_published_features_adversary.rs`.
#![cfg(unix)]
use connectors_catalog::definition::{self, Refusal, Request};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

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

fn released() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/definitions-0.5.2")
}

/// The bytes a lowering wrote: the selection, the declaration and the bundle.
struct Written {
    operations: Vec<u8>,
    connector: Vec<u8>,
    bundle: Vec<u8>,
    entry: connectors_catalog::bundle::Entry,
}

fn lower_file(file: &Path) -> Result<Written, Refusal> {
    let temp = tempfile::tempdir().unwrap();
    let scratch = temp.path().join("scratch");
    std::fs::create_dir_all(&scratch).unwrap();
    let lowered = definition::lower(&Request {
        definition: file,
        root: &released(),
        out: &temp.path().join("out"),
        scratch: &scratch,
    })?;
    Ok(Written {
        operations: std::fs::read(&lowered.operations).unwrap(),
        connector: std::fs::read(&lowered.connector).unwrap(),
        bundle: std::fs::read(&lowered.bundle).unwrap(),
        entry: lowered.entry,
    })
}

fn lower(rest: &str) -> Result<Written, Refusal> {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("github.yaml");
    std::fs::write(&file, format!("{HEAD}{rest}")).unwrap();
    lower_file(&file)
}

fn entry(written: &Written, id: &str) -> Value {
    let selection: Value = serde_json::from_slice(&written.operations).unwrap();
    selection["operations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["id"] == id)
        .cloned()
        .unwrap_or_else(|| panic!("`{id}` is in the selection"))
}

/// The released definition lowers to the same bytes every time: the selection,
/// the declaration and the bundle, and the index entry naming the bundle's digest.
#[test]
fn lowering_the_released_definition_twice_writes_the_same_bytes() {
    let file = released().join("connectors/github.yaml");
    let first = lower_file(&file).expect("the released definition lowers");
    let second = lower_file(&file).expect("the released definition lowers again");
    assert_eq!(first.operations, second.operations, "operations.json");
    assert_eq!(first.connector, second.connector, "connector.json");
    assert_eq!(first.bundle, second.bundle, "bundle");
    assert_eq!(first.entry, second.entry, "index entry");
}

/// `query_fixed` and `query_folds` are maps: the order a definition writes their
/// keys in does not reach the selection's bytes.
#[test]
fn query_fixed_and_fold_key_order_does_not_change_the_selection() {
    let one = lower(
        "operations:\n  issues.assigned:\n    op: issues/list\n    \
         query_fixed: {filter: assigned, state: open, labels: bug}\n  \
         review_requests.list:\n    op: search/issues-and-pull-requests\n    \
         query_fixed: {q: \"is:pr\", order: desc}\n    \
         query_folds: {since: {into: q, prefix: \"updated:>=\", format: timestamp}, until: {into: order, prefix: \"x:\", format: timestamp}}\n",
    )
    .expect("the definition lowers");
    let other = lower(
        "operations:\n  issues.assigned:\n    op: issues/list\n    \
         query_fixed: {labels: bug, state: open, filter: assigned}\n  \
         review_requests.list:\n    op: search/issues-and-pull-requests\n    \
         query_fixed: {order: desc, q: \"is:pr\"}\n    \
         query_folds: {until: {format: timestamp, prefix: \"x:\", into: order}, since: {prefix: \"updated:>=\", into: q, format: timestamp}}\n",
    )
    .expect("the reordered definition lowers");
    assert_eq!(
        String::from_utf8(one.operations).unwrap(),
        String::from_utf8(other.operations).unwrap()
    );
}

/// A key written twice in `query_fixed` names two values for one parameter; the
/// lowering does not pick one of them silently.
#[test]
fn a_repeated_query_fixed_key_is_refused() {
    let lowered = lower(
        "operations:\n  issues.assigned:\n    op: issues/list\n    \
         query_fixed: {state: open, state: closed}\n",
    );
    match lowered {
        Err(_) => {}
        Ok(written) => panic!(
            "a repeated query_fixed key lowered to {}",
            entry(&written, "issues.assigned")["query_fixed"]
        ),
    }
}

/// An operation's `rate_limit_headers` replaces the default whole: one naming a
/// single header is refused, not completed from the default.
#[test]
fn a_partial_operation_rate_limit_headers_is_refused_not_merged_with_the_default() {
    let lowered = lower(
        "defaults:\n  rate_limit_headers: {remaining: x-ratelimit-remaining, reset: x-ratelimit-reset}\n\
         operations:\n  user.me: {op: users/get-authenticated, rate_limit_headers: {remaining: x-left}}\n",
    );
    match lowered {
        Err(Refusal::Malformed(_)) => {}
        Err(other) => panic!("refused as {other:?}, not malformed"),
        Ok(written) => panic!(
            "a partial rate_limit_headers lowered to {}",
            entry(&written, "user.me")["rate_limit_headers"]
        ),
    }
}

/// `embedded_text` without `size` keeps `size` out of the selection, and with
/// it carries the pointer as written; `max_bytes` stays a number.
#[test]
fn embedded_text_members_reach_the_selection_as_written() {
    let written = lower(
        "operations:\n  readme.raw:\n    op: repos/get-readme\n    \
         embedded_text: {content: /a/b, encoding: /a/c, size: /a/d, max_bytes: 1}\n",
    )
    .expect("the definition lowers");
    assert_eq!(
        entry(&written, "readme.raw")["embedded_text"],
        json!({"content": "/a/b", "encoding": "/a/c", "size": "/a/d", "max_bytes": 1})
    );
}

/// A default `envelope` reaches the plain reads beside an embedded-text read and
/// no write, and an operation's own `rate_limit_headers` on a write replaces the
/// default there as on a read.
#[test]
fn defaults_reach_writes_and_reads_beside_an_embedded_text_read_as_documented() {
    let written = lower(
        "defaults:\n  envelope: {ok: /ok, error: /error, otherwise: internal}\n  \
         rate_limit_headers: {remaining: x-ratelimit-remaining, reset: x-ratelimit-reset}\n\
         operations:\n  \
         readme.raw: {op: repos/get-readme, embedded_text: {content: /content, encoding: /encoding, max_bytes: 10}}\n  \
         repo.get: {op: repos/get}\n  \
         issue.create: {op: issues/create, effect: write, body: {title: string!}, rate_limit_headers: {remaining: x-w-left, reset: x-w-reset}}\n",
    )
    .expect("the definition lowers");
    assert!(entry(&written, "readme.raw").get("envelope").is_none());
    assert_eq!(
        entry(&written, "repo.get")["envelope"],
        json!({"ok": "/ok", "error": "/error", "otherwise": "internal"})
    );
    assert!(entry(&written, "issue.create").get("envelope").is_none());
    assert_eq!(
        entry(&written, "issue.create")["rate_limit_headers"],
        json!({"remaining": "x-w-left", "reset": "x-w-reset"})
    );
    assert_eq!(
        entry(&written, "readme.raw")["rate_limit_headers"],
        json!({"remaining": "x-ratelimit-remaining", "reset": "x-ratelimit-reset"})
    );
}

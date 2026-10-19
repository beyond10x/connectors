//! story:write-postflight-present-check: a write's postflight can require a
//! member of the answer to be present (`{"present": true}`), and Jira
//! `issue.create` (`/id` and `/key`) and `issue.comment.add` (`/id`) use it, so
//! a 2xx that names no new id is `unknown`, not `applied`.
//!
//! The shipped selection and bundle are driven through the engine against a
//! scripted provider: the preflight read answers as Jira does, and the write is
//! answered with the status and bytes each case names. No network, no
//! credential.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Engine, Guard, Selection};
use connectors_core::Result;
use connectors_sdk::{
    AuthenticatedHttp, AuthenticatedWrite, HttpResponse, WriteMethod, WriteOutcome,
};
use serde_json::{Value, json};
use std::{collections::VecDeque, path::Path, sync::Mutex};

const BASE: &str = "/rest/api/3";
const INSTANCE: &str = "fixture-jira";
const CREATE: &str = "issue.create";
const COMMENT: &str = "issue.comment.add";

struct Scripted {
    responses: Mutex<VecDeque<Value>>,
}
#[async_trait::async_trait]
impl AuthenticatedHttp for Scripted {
    async fn get(&self, _: &[&str], _: &[(&str, String)]) -> Result<HttpResponse> {
        let answer = self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected provider read");
        Ok(json_response(200, Some(&answer)))
    }
}

fn json_response(status: u16, body: Option<&Value>) -> HttpResponse {
    match body {
        Some(body) => HttpResponse {
            status,
            headers: [("content-type".to_owned(), "application/json".to_owned())].into(),
            body: serde_json::to_vec(body).unwrap(),
        },
        None => HttpResponse {
            status,
            headers: Default::default(),
            body: Vec::new(),
        },
    }
}

/// The write's answer: a status and, unless `None`, a JSON body.
struct Answer {
    status: u16,
    body: Option<Value>,
}
#[async_trait::async_trait]
impl AuthenticatedWrite for Answer {
    async fn send_json(
        self: Box<Self>,
        _: WriteMethod,
        _: &[&str],
        _: &[(&str, String)],
        _: &Value,
    ) -> Result<HttpResponse> {
        Ok(json_response(self.status, self.body.as_ref()))
    }
}

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn selection_file() -> Value {
    serde_json::from_slice(&std::fs::read(root().join("providers/jira/operations.json")).unwrap())
        .unwrap()
}
fn engine_of(operations: Value) -> Result<Engine> {
    let bundle = bundle::load(&root().join("generated/bundles"), "jira").unwrap();
    let selections: Vec<Selection> = serde_json::from_value(operations).unwrap();
    Engine::new(&bundle, BASE, &selections)
}
fn engine() -> Engine {
    engine_of(selection_file()["operations"].clone()).unwrap()
}

fn create_input() -> Value {
    json!({"body": {"fields": {"project": {"key": "PROJ"},
                               "issuetype": {"id": "10001"},
                               "summary": "Fixture summary"}}})
}
fn comment_input() -> Value {
    json!({"issueIdOrKey": "PROJ-1",
           "body": {"body": {"content": [{"type": "paragraph",
                                          "content": [{"type": "text", "text": "fixture comment"}]}]}}})
}
fn project() -> Value {
    json!({"id": "10000", "key": "PROJ", "name": "Fixture project"})
}
fn issue() -> Value {
    json!({"id": "10001", "key": "PROJ-1", "fields": {"summary": "fixture issue"}})
}
fn comment_document() -> Value {
    json!({"type": "doc", "version": 1,
           "content": [{"type": "paragraph",
                        "content": [{"type": "text", "text": "fixture comment"}]}]})
}

/// What the engine classifies one write as: `applied`, `refused` or `unknown`.
async fn classify(id: &str, input: Value, preflight: Value, answer: Answer) -> &'static str {
    let http = Scripted {
        responses: Mutex::new([preflight].into()),
    };
    let prepared = engine()
        .prepare(&http, INSTANCE, id, input)
        .await
        .unwrap_or_else(|error| panic!("`{id}` was refused before dispatch: {error:?}"));
    match prepared.execute(Box::new(answer)).await {
        WriteOutcome::Applied(_) => "applied",
        WriteOutcome::Refused(_) => "refused",
        WriteOutcome::Unknown(_) => "unknown",
    }
}

/// The scratch cases of the 0.54.0 review, and the partial answers beside them:
/// a create answered 2xx without both the new issue's `id` and `key`, or with
/// either `null`, leaves the effect possible.
#[tokio::test]
async fn a_create_answered_without_its_id_and_key_is_unknown() {
    for (case, status, body) in [
        ("201 with no body", 201, None),
        ("201 {}", 201, Some(json!({}))),
        ("204", 204, None),
        ("201 null", 201, Some(Value::Null)),
        ("201 key only", 201, Some(json!({"key": "PROJ-10"}))),
        ("201 id only", 201, Some(json!({"id": "10010"}))),
        (
            "201 id null",
            201,
            Some(json!({"id": null, "key": "PROJ-10"})),
        ),
        (
            "201 key null",
            201,
            Some(json!({"id": "10010", "key": null})),
        ),
    ] {
        assert_eq!(
            classify(CREATE, create_input(), project(), Answer { status, body }).await,
            "unknown",
            "{case}"
        );
    }
}

/// Jira's documented `CreatedIssue` answer, `id`, `key` and `self`, is applied;
/// `present` holds for any non-null value, so an `id` Jira wrote as a number
/// holds as well as its string.
#[tokio::test]
async fn a_create_answered_with_its_id_and_key_is_applied() {
    for body in [
        json!({"id": "10010", "key": "PROJ-10",
               "self": "https://tracker.example.test/rest/api/3/issue/10010"}),
        json!({"id": 10010, "key": "PROJ-10"}),
    ] {
        assert_eq!(
            classify(
                CREATE,
                create_input(),
                project(),
                Answer {
                    status: 201,
                    body: Some(body.clone())
                }
            )
            .await,
            "applied",
            "{body}"
        );
    }
}

/// A comment answered 2xx with the document but no `id`, or a `null` one, is
/// unknown; so is one with no body at all, as before.
#[tokio::test]
async fn a_comment_answered_without_its_id_is_unknown() {
    for (case, status, body) in [
        (
            "201 doc without id",
            201,
            Some(json!({"body": comment_document()})),
        ),
        (
            "201 doc with id null",
            201,
            Some(json!({"id": null, "body": comment_document()})),
        ),
        ("201 with no body", 201, None),
        ("201 {}", 201, Some(json!({}))),
        ("204", 204, None),
    ] {
        assert_eq!(
            classify(COMMENT, comment_input(), issue(), Answer { status, body }).await,
            "unknown",
            "{case}"
        );
    }
}

/// Jira's documented `Comment` answer, its `id` among it and its body a `doc`,
/// is applied.
#[tokio::test]
async fn a_comment_answered_with_its_id_is_applied() {
    let body = json!({"id": "10100",
                      "self": "https://tracker.example.test/rest/api/3/issue/PROJ-1/comment/10100",
                      "body": comment_document()});
    assert_eq!(
        classify(
            COMMENT,
            comment_input(),
            issue(),
            Answer {
                status: 201,
                body: Some(body)
            }
        )
        .await,
        "applied"
    );
}

/// The shipped postflights, as reviewed: create requires `/id` and `/key`,
/// comment `/id` beside its `/body/type`.
#[test]
fn create_and_comment_require_the_new_id_after_the_write() {
    let file = selection_file();
    let postflight = |id: &str| {
        file["operations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["id"] == id)
            .unwrap()["guard"]["postflight"]
            .clone()
    };
    assert_eq!(
        postflight(CREATE),
        json!({"checks": [{"pointer": "/id", "expect": {"present": true}},
                          {"pointer": "/key", "expect": {"present": true}}]})
    );
    assert_eq!(
        postflight(COMMENT),
        json!({"checks": [{"pointer": "/body/type", "expect": {"literal": "doc"}},
                          {"pointer": "/id", "expect": {"present": true}}]})
    );
}

/// `present` is only `true`, as the model holds it; `false` would read as
/// "absent", which a postflight proves with `absent` and a read, so the engine
/// refuses it when the selection loads. A `present` beside another expectation
/// is not a value of the model and does not read.
#[test]
fn a_present_expectation_other_than_true_is_refused_when_the_selection_loads() {
    let mut operations = selection_file()["operations"].clone();
    let create = operations
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|entry| entry["id"] == CREATE)
        .unwrap();
    create["guard"]["postflight"]["checks"][0]["expect"] = json!({"present": false});
    let refused = engine_of(operations.clone())
        .err()
        .expect("a presence of false loads");
    assert!(
        format!("{refused:?}").contains("presence other than true"),
        "{refused:?}"
    );

    let both: std::result::Result<Vec<Selection>, _> = serde_json::from_value({
        let mut operations = operations;
        operations
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|entry| entry["id"] == CREATE)
            .unwrap()["guard"]["postflight"]["checks"][0]["expect"] =
            json!({"present": true, "literal": "x"});
        operations
    });
    assert!(both.is_err());
}

/// A guard with `present` reads and writes back unchanged, so it keeps its
/// bytes and its configuration revision.
#[test]
fn a_present_expectation_reads_and_writes_back_unchanged() {
    let file = selection_file();
    for id in [CREATE, COMMENT] {
        let guard = file["operations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["id"] == id)
            .unwrap()["guard"]
            .clone();
        let read: Guard = serde_json::from_value(guard.clone()).unwrap();
        assert_eq!(serde_json::to_value(&read).unwrap(), guard, "`{id}`");
    }
}

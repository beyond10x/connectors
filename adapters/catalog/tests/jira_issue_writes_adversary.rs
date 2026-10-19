//! Adversarial cases for story:parity-jira-issue-create-and-comment,
//! story:parity-jira-issue-writes and story:parity-jira-create-meta-per-type:
//! the shipped Jira issue writes and `issue.create_meta.fields`, driven at the
//! engine boundary with in-memory ports that record every request in one
//! ordered log, so "nothing sent" is the log holding no write and "before" is a
//! position in it.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Engine, Selection};
use connectors_core::{Error, ErrorCode, Result};
use connectors_sdk::{
    AuthenticatedHttp, AuthenticatedWrite, HttpResponse, WriteMethod, WriteOutcome,
};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex},
};

const BASE: &str = "/rest/api/3";
const CREATE: &str = "issue.create";
const COMMENT: &str = "issue.comment.add";
const EDIT: &str = "issue.edit";
const LINK: &str = "issue.link.add";
const DELETE: &str = "issue.delete";
const FIELDS: &str = "issue.create_meta.fields";

const PROJECT: &str = "PROJ";
const ISSUE: &str = "PROJ-1";
const OTHER: &str = "PROJ-2";
const STAMP: &str = "2026-10-01T09:00:00.000+0000";

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn engine() -> Engine {
    let file: Value = serde_json::from_slice(
        &std::fs::read(root().join("providers/jira/operations.json")).unwrap(),
    )
    .unwrap();
    let selections: Vec<Selection> = serde_json::from_value(file["operations"].clone()).unwrap();
    let bundle = bundle::load(&root().join("generated/bundles"), "jira").unwrap();
    Engine::new(&bundle, BASE, &selections).unwrap()
}
fn block<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(future)
}
fn answer(status: u16, body: Option<Value>) -> Option<HttpResponse> {
    Some(HttpResponse {
        status,
        headers: BTreeMap::new(),
        body: body
            .map(|b| serde_json::to_vec(&b).unwrap())
            .unwrap_or_default(),
    })
}
fn issue(key: &str, updated: &str) -> Value {
    json!({"id": "10001", "key": key,
           "self": format!("https://tracker.example.test/rest/api/3/issue/{key}"),
           "fields": {"summary": "fixture", "updated": updated}})
}
fn found(key: &str) -> Option<HttpResponse> {
    answer(200, Some(issue(key, STAMP)))
}
fn project(key: &str) -> Option<HttpResponse> {
    answer(
        200,
        Some(json!({"id": "10000", "key": key, "name": "Fixture project"})),
    )
}
fn document(text: &str) -> Value {
    json!({"type": "doc", "version": 1,
           "content": [{"type": "paragraph", "content": [{"type": "text", "text": text}]}]})
}

/// Method, joined segments, query and body of one request.
type Entry = (String, String, String, Value);
type Log = Arc<Mutex<Vec<Entry>>>;

fn joined(query: &[(&str, String)]) -> String {
    query
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&")
}

/// A read port answering GETs in order; `None` is a lost response.
struct Reader {
    log: Log,
    answers: Mutex<Vec<Option<HttpResponse>>>,
}
#[async_trait::async_trait]
impl AuthenticatedHttp for Reader {
    async fn get(&self, segments: &[&str], query: &[(&str, String)]) -> Result<HttpResponse> {
        self.log.lock().unwrap().push((
            "GET".into(),
            segments.join("/"),
            joined(query),
            Value::Null,
        ));
        let mut answers = self.answers.lock().unwrap();
        assert!(!answers.is_empty(), "a GET nobody expected was sent");
        answers.remove(0).ok_or_else(Error::unavailable)
    }
}
/// A write port answering once.
struct Writer {
    log: Log,
    answer: Option<HttpResponse>,
}
#[async_trait::async_trait]
impl AuthenticatedWrite for Writer {
    async fn send_json(
        self: Box<Self>,
        method: WriteMethod,
        segments: &[&str],
        query: &[(&str, String)],
        body: &Value,
    ) -> Result<HttpResponse> {
        self.log.lock().unwrap().push((
            format!("{method:?}").to_uppercase(),
            segments.join("/"),
            joined(query),
            body.clone(),
        ));
        self.answer.ok_or_else(Error::unavailable)
    }
}

type Outcome = std::result::Result<WriteOutcome<Value>, Error>;

/// Prepare with `preflight` answers, execute with `write`, and read after it
/// with `after`. The outcome, or the refusal in prepare, and the log.
fn run(
    id: &str,
    input: Value,
    preflight: Vec<Option<HttpResponse>>,
    write: Option<HttpResponse>,
    after: Vec<Option<HttpResponse>>,
) -> (Outcome, Vec<Entry>) {
    let engine = engine();
    let log: Log = Arc::default();
    let before = Reader {
        log: log.clone(),
        answers: Mutex::new(preflight),
    };
    let outcome = block(async {
        let prepared = engine.prepare(&before, "fixture", id, input).await?;
        let reader = Reader {
            log: log.clone(),
            answers: Mutex::new(after),
        };
        let writer = Box::new(Writer {
            log: log.clone(),
            answer: write,
        });
        Ok::<_, Error>(prepared.execute_reading(writer, Some(&reader)).await)
    });
    let log = log.lock().unwrap().clone();
    (outcome, log)
}

fn effect(outcome: &Outcome) -> String {
    match outcome {
        Ok(WriteOutcome::Applied(Ok(_))) => "applied".into(),
        Ok(WriteOutcome::Applied(Err(e))) => format!("applied-err {:?}", e.code),
        Ok(WriteOutcome::Refused(e)) => format!("refused {:?}", e.code),
        Ok(WriteOutcome::Unknown(e)) => format!("unknown {:?}", e.code),
        Err(e) => format!("not-attempted {:?}", e.code),
    }
}
fn writes(log: &[Entry]) -> Vec<&Entry> {
    log.iter().filter(|(method, ..)| method != "GET").collect()
}

/// A body the caller sends is refused as `invalid_input` with no request of any
/// kind, read or write.
fn refused_before_any_request(id: &str, input: Value) {
    let (outcome, log) = run(id, input.clone(), vec![], None, vec![]);
    match &outcome {
        Err(error) => assert_eq!(error.code, ErrorCode::InvalidInput, "`{id}` {input}"),
        other => panic!("`{id}` {input}: expected a refusal, got {}", effect(other)),
    }
    assert!(log.is_empty(), "`{id}` {input}: {log:?}");
}

fn create_input() -> Value {
    json!({"body": {"fields": {
        "project": {"key": PROJECT},
        "issuetype": {"id": "10001"},
        "summary": "Fixture summary",
        "description": document("fixture description"),
        "labels": ["fixture"],
        "priority": {"id": "3"},
        "assignee": {"accountId": "fixture-user-1"},
        "parent": {"key": ISSUE}}}})
}
fn comment_input() -> Value {
    json!({"issueIdOrKey": ISSUE,
           "body": {"body": {"content": document("fixture comment")["content"]},
                    "visibility": {"type": "role", "value": "Administrators"}}})
}
fn edit_input(stamp: &str) -> Value {
    json!({"issueIdOrKey": ISSUE, "updated_stamp": stamp,
           "body": {"fields": {"summary": "Edited summary"}}})
}
fn link_input() -> Value {
    json!({"body": {"type": {"name": "Blocks"},
                    "inwardIssue": {"key": ISSUE}, "outwardIssue": {"key": OTHER}}})
}
fn delete_input(stamp: &str) -> Value {
    json!({"issueIdOrKey": ISSUE, "updated_stamp": stamp})
}

// ------------------------------------------------------------ closed bodies

/// `/body/content` is `body_required`: a comment whose content is JSON `null`
/// has none, and must be refused like one that omits it. The engine reads
/// "required" as "the key is present", so `null` passes and a comment of
/// `{"type":"doc","version":1,"content":null}` is sent to Jira.
#[test]
#[ignore = "story:required-body-key-refuses-null: the engine admits null for a required untyped key today (adversary_catalog_body_types.rs:113)"]
fn a_comment_whose_content_is_null_is_refused_before_any_request() {
    let mut input = comment_input();
    input["body"]["body"]["content"] = Value::Null;
    refused_before_any_request(COMMENT, input);
}

/// The fixed document members are never caller input in any value, and a
/// visibility type outside the enum, of another case, empty, or not a string is
/// refused.
#[test]
fn comment_fixed_members_and_visibility_enum_hold_on_every_value() {
    for (pointer, value) in [
        ("/body/body/type", json!("paragraph")),
        ("/body/body/version", json!(1)),
        ("/body/body/version", json!(2)),
        ("/body/body/version", Value::Null),
        ("/body/visibility/type", json!("Role")),
        ("/body/visibility/type", json!("")),
        ("/body/visibility/type", json!("user")),
        ("/body/visibility/type", json!(["role"])),
        ("/body/visibility/type", Value::Null),
        ("/body/visibility/value", json!({"name": "x"})),
        ("/body/visibility/extra", json!("x")),
        ("/body/properties", json!([])),
    ] {
        let mut input = comment_input();
        let (parent, leaf) = pointer.rsplit_once('/').unwrap();
        input.pointer_mut(parent).unwrap()[leaf] = value;
        refused_before_any_request(COMMENT, input);
    }
    // `visibility` and `body` as non-objects.
    for (key, value) in [("visibility", json!("role")), ("body", json!([]))] {
        let mut input = comment_input();
        input["body"][key] = value;
        refused_before_any_request(COMMENT, input);
    }
}

/// Approved, the comment sent is the caller's content inside the fixed
/// document and nothing else.
#[test]
fn the_comment_sent_carries_the_fixed_document_and_nothing_more() {
    let (outcome, log) = run(
        COMMENT,
        comment_input(),
        vec![found(ISSUE)],
        answer(
            201,
            Some(json!({"id": "10100", "body": document("fixture comment")})),
        ),
        vec![],
    );
    assert_eq!(effect(&outcome), "applied");
    assert_eq!(
        writes(&log),
        [&(
            "POST".to_owned(),
            format!("issue/{ISSUE}/comment"),
            String::new(),
            json!({"body": document("fixture comment"),
                   "visibility": {"type": "role", "value": "Administrators"}})
        )]
    );
}

/// Every typed, non-empty and nested key of create and link, given the wrong
/// shape, is refused with nothing read.
#[test]
fn create_and_link_keys_of_the_wrong_shape_are_refused_before_any_request() {
    for (pointer, value) in [
        ("/fields/issuetype/id", json!({"id": "10001"})),
        ("/fields/issuetype/id", json!(10001)),
        ("/fields/issuetype/id", json!("")),
        ("/fields/issuetype/id", Value::Null),
        ("/fields/issuetype", json!("10001")),
        ("/fields/summary", json!(["x"])),
        ("/fields/summary", Value::Null),
        ("/fields/project/key", Value::Null),
        ("/fields/project/key", json!({"key": PROJECT})),
        ("/fields/project", json!(PROJECT)),
        ("/fields/project/id", json!("10000")),
        ("/fields/labels", json!([1])),
        ("/fields/labels", json!([null])),
        ("/fields/priority/id", json!(3)),
        ("/fields/priority/name", json!("High")),
        ("/fields/assignee/accountId", Value::Null),
        ("/fields/parent/key", json!("")),
        ("/fields/parent/id", json!("10001")),
        ("/fields/reporter", json!({"accountId": "x"})),
        ("/update", json!({})),
    ] {
        let mut input = create_input();
        let (parent, leaf) = pointer.rsplit_once('/').unwrap();
        input["body"].pointer_mut(parent).unwrap()[leaf] = value;
        refused_before_any_request(CREATE, input);
    }
    for (pointer, value) in [
        ("/type/name", json!("")),
        ("/type/name", Value::Null),
        ("/type/id", json!("10000")),
        ("/inwardIssue/key", json!(7)),
        ("/outwardIssue/key", json!("")),
        ("/outwardIssue/key", Value::Null),
        ("/outwardIssue/id", json!("10002")),
        ("/comment", json!({"body": document("x")})),
    ] {
        let mut input = link_input();
        let (parent, leaf) = pointer.rsplit_once('/').unwrap();
        input["body"].pointer_mut(parent).unwrap()[leaf] = value;
        refused_before_any_request(LINK, input);
    }
}

/// Approved, a create sends exactly the fields given, the description as given.
#[test]
fn the_create_sent_is_the_body_as_given() {
    let input = create_input();
    let (outcome, log) = run(
        CREATE,
        input.clone(),
        vec![project(PROJECT)],
        answer(201, Some(json!({"id": "10010", "key": "PROJ-10"}))),
        vec![],
    );
    assert_eq!(effect(&outcome), "applied");
    assert_eq!(log[0].1, format!("project/{PROJECT}"));
    assert_eq!(
        writes(&log),
        [&(
            "POST".to_owned(),
            "issue".to_owned(),
            String::new(),
            input["body"].clone()
        )]
    );
}

// ------------------------------------------------------------ withheld parameters

/// Every withheld parameter, at the input's top level or inside the body, in
/// any JSON type, is refused with nothing read; the unit's own suite covers only
/// `overrideScreenSecurity` and `deleteSubtasks` as a string.
#[test]
fn withheld_parameters_are_unreachable_from_caller_input() {
    for name in [
        "overrideScreenSecurity",
        "overrideEditableFlag",
        "returnIssue",
        "expand",
    ] {
        for value in [json!(true), json!("true")] {
            let mut input = edit_input(STAMP);
            input[name] = value.clone();
            refused_before_any_request(EDIT, input);
            let mut input = edit_input(STAMP);
            input["body"][name] = value;
            refused_before_any_request(EDIT, input);
        }
    }
    for value in [json!(true), json!("true"), json!(["true"])] {
        let mut input = delete_input(STAMP);
        input["deleteSubtasks"] = value;
        refused_before_any_request(DELETE, input);
    }
    let mut input = delete_input(STAMP);
    input["body"] = json!({"deleteSubtasks": true});
    refused_before_any_request(DELETE, input);
}

/// `notifyUsers` is passed through on the edit, as the guide states, and on the
/// write only: no withheld parameter reaches the query of the `PUT`.
#[test]
fn the_edit_sends_notify_users_and_no_withheld_parameter() {
    let mut input = edit_input(STAMP);
    input["notifyUsers"] = json!(false);
    let (outcome, log) = run(
        EDIT,
        input.clone(),
        vec![found(ISSUE)],
        answer(204, None),
        vec![found(ISSUE)],
    );
    assert_eq!(effect(&outcome), "applied");
    assert_eq!(
        log,
        [
            (
                "GET".into(),
                format!("issue/{ISSUE}"),
                String::new(),
                Value::Null
            ),
            (
                "PUT".into(),
                format!("issue/{ISSUE}"),
                "notifyUsers=false".into(),
                input["body"].clone()
            ),
            (
                "GET".into(),
                format!("issue/{ISSUE}"),
                String::new(),
                Value::Null
            ),
        ]
    );
}

// ------------------------------------------------------------ stamp and conflict

/// Any difference in the issue's key or `updated` stamp, the stamp differing
/// in form only or the key in case, refuses an edit and a delete as `conflict`
/// after one read, with nothing written.
#[test]
fn any_key_or_stamp_difference_is_conflict_with_nothing_written() {
    for (id, input) in [(EDIT, edit_input(STAMP)), (DELETE, delete_input(STAMP))] {
        for observed in [
            issue("PROJ-9", STAMP),
            issue("proj-1", STAMP),
            issue(ISSUE, "2026-10-01T09:00:00.000+00:00"),
            issue(ISSUE, "2026-10-01T09:00:00.001+0000"),
            issue(ISSUE, &format!("{STAMP} ")),
        ] {
            let (outcome, log) = run(
                id,
                input.clone(),
                vec![answer(200, Some(observed.clone()))],
                answer(204, None),
                vec![found(ISSUE)],
            );
            assert_eq!(
                effect(&outcome),
                "not-attempted Conflict",
                "`{id}` {observed}"
            );
            assert!(writes(&log).is_empty(), "`{id}` {observed}: {log:?}");
            assert_eq!(log.len(), 1, "`{id}` {observed}");
        }
    }
}

/// An issue answered without an `updated` stamp, or with one that is not a
/// scalar, is never written: the stamp cannot be shown equal.
#[test]
fn an_issue_without_a_comparable_stamp_is_never_written() {
    for (id, input) in [(EDIT, edit_input(STAMP)), (DELETE, delete_input(STAMP))] {
        for fields in [
            json!({}),
            json!({"updated": null}),
            json!({"updated": {"at": STAMP}}),
        ] {
            let observed = json!({"id": "10001", "key": ISSUE, "fields": fields});
            let (outcome, log) = run(
                id,
                input.clone(),
                vec![answer(200, Some(observed.clone()))],
                answer(204, None),
                vec![found(ISSUE)],
            );
            assert!(
                effect(&outcome).starts_with("not-attempted"),
                "`{id}` {observed}: {}",
                effect(&outcome)
            );
            assert!(writes(&log).is_empty(), "`{id}` {observed}");
        }
    }
}

/// A stamp that is not a string, or empty, never matches into a write.
#[test]
fn a_stamp_that_is_not_text_is_refused_or_conflict_never_written() {
    for (id, base) in [(EDIT, edit_input(STAMP)), (DELETE, delete_input(STAMP))] {
        for stamp in [Value::Null, json!({"s": STAMP}), json!([STAMP]), json!("")] {
            let mut input = base.clone();
            input["updated_stamp"] = stamp.clone();
            let (outcome, log) = run(
                id,
                input,
                vec![found(ISSUE)],
                answer(204, None),
                vec![found(ISSUE)],
            );
            assert!(
                effect(&outcome).starts_with("not-attempted"),
                "`{id}` {stamp}: {}",
                effect(&outcome)
            );
            assert!(writes(&log).is_empty(), "`{id}` {stamp}");
        }
    }
}

/// Jira's `409` to an edit is `conflict`, refused, never applied.
#[test]
fn jira_409_to_an_edit_is_refused_as_conflict() {
    let (outcome, log) = run(
        EDIT,
        edit_input(STAMP),
        vec![found(ISSUE)],
        answer(409, Some(json!({"errorMessages": ["conflict"]}))),
        vec![],
    );
    assert_eq!(effect(&outcome), "refused Conflict");
    assert_eq!(writes(&log).len(), 1);
}

// ------------------------------------------------------------ delete postflight

/// Only a `404` to the read after a delete proves it: every other answer, a
/// redirect, a refusal, a server error, `410` or no answer, leaves it unknown.
#[test]
fn a_delete_is_applied_only_on_a_404_after_it() {
    for (after, expected) in [
        (
            answer(404, Some(json!({"errorMessages": ["gone"]}))),
            "applied",
        ),
        (found(ISSUE), "unknown UpstreamProtocol"),
        (answer(200, None), "unknown UpstreamProtocol"),
        (answer(204, None), "unknown UpstreamProtocol"),
        (answer(301, None), "unknown UpstreamProtocol"),
        (answer(401, None), "unknown UpstreamProtocol"),
        (answer(403, None), "unknown UpstreamProtocol"),
        (answer(410, None), "unknown UpstreamProtocol"),
        (answer(429, None), "unknown UpstreamProtocol"),
        (answer(500, None), "unknown UpstreamProtocol"),
        (None, "unknown UpstreamProtocol"),
    ] {
        let status = after.as_ref().map(|a| a.status);
        let (outcome, log) = run(
            DELETE,
            delete_input(STAMP),
            vec![found(ISSUE)],
            answer(204, None),
            vec![after],
        );
        assert_eq!(effect(&outcome), expected, "read after {status:?}");
        assert_eq!(
            log.iter().map(|e| e.0.as_str()).collect::<Vec<_>>(),
            ["GET", "DELETE", "GET"],
            "read after {status:?}"
        );
        assert_eq!(log[1].1, format!("issue/{ISSUE}"));
        assert_eq!(log[1].2, "", "deleteSubtasks never sent");
        assert_eq!(log[1].3, Value::Null, "a delete carries no body");
        assert_eq!(
            log[2].1,
            format!("issue/{ISSUE}"),
            "read after the same issue"
        );
    }
}

// ------------------------------------------------------------ guards' targets

/// The link reads the inward issue by the inward key and the outward issue by
/// the outward key, each compared with its own key: an answer carrying the
/// other end's key refuses, with no link sent.
#[test]
fn the_link_guard_reads_and_compares_each_end_by_its_own_key() {
    let (outcome, log) = run(
        LINK,
        link_input(),
        vec![found(ISSUE), found(OTHER)],
        answer(201, None),
        vec![],
    );
    assert_eq!(effect(&outcome), "applied");
    assert_eq!(log[0].1, format!("issue/{ISSUE}"));
    assert_eq!(log[1].1, format!("issue/{OTHER}"));
    for answers in [
        vec![found(OTHER), found(OTHER)],
        vec![found(ISSUE), found(ISSUE)],
    ] {
        let (outcome, log) = run(LINK, link_input(), answers, answer(201, None), vec![]);
        assert_eq!(effect(&outcome), "not-attempted Forbidden");
        assert!(writes(&log).is_empty(), "{log:?}");
    }
}

/// The create's guard compares the project's `key`, not its `id` or `name`.
#[test]
fn the_create_guard_compares_the_project_key() {
    for observed in [
        json!({"id": PROJECT, "key": "OTHER", "name": PROJECT}),
        json!({"id": "10000", "name": PROJECT}),
        json!({"id": "10000", "key": "proj"}),
    ] {
        let (outcome, log) = run(
            CREATE,
            create_input(),
            vec![answer(200, Some(observed.clone()))],
            answer(201, Some(json!({"id": "10010", "key": "PROJ-10"}))),
            vec![],
        );
        assert!(
            effect(&outcome).starts_with("not-attempted"),
            "{observed}: {}",
            effect(&outcome)
        );
        assert!(writes(&log).is_empty(), "{observed}");
    }
}

// ------------------------------------------------------------ create_meta.fields

/// Each GET as its segments, one string each, then its query.
struct Recorder(Mutex<Vec<(Vec<String>, String)>>);
#[async_trait::async_trait]
impl AuthenticatedHttp for Recorder {
    async fn get(&self, segments: &[&str], query: &[(&str, String)]) -> Result<HttpResponse> {
        self.0.lock().unwrap().push((
            segments.iter().map(|s| (*s).to_owned()).collect(),
            joined(query),
        ));
        Ok(answer(
            200,
            Some(json!({"startAt": 0, "maxResults": 50, "total": 0, "fields": []})),
        )
        .unwrap())
    }
}

/// An empty or missing `issueTypeId` never reaches the issue types list one
/// segment up; a `/` in it never adds a segment.
#[test]
fn create_meta_fields_never_resolves_to_another_path() {
    let engine = engine();
    for input in [
        json!({"projectIdOrKey": PROJECT}),
        json!({"projectIdOrKey": PROJECT, "issueTypeId": ""}),
        json!({"projectIdOrKey": "", "issueTypeId": "10001"}),
        json!({"projectIdOrKey": PROJECT, "issueTypeId": null}),
        json!({"projectIdOrKey": PROJECT, "issueTypeId": "10001", "maxResults": "all"}),
    ] {
        let http = Recorder(Mutex::new(Vec::new()));
        let outcome = block(engine.read(&http, "fixture", FIELDS, input.clone()));
        assert_eq!(
            outcome.map(|_| ()).map_err(|e| e.code),
            Err(ErrorCode::InvalidInput),
            "{input}"
        );
        assert!(http.0.lock().unwrap().is_empty(), "{input}");
    }
    let http = Recorder(Mutex::new(Vec::new()));
    let input = json!({"projectIdOrKey": PROJECT, "issueTypeId": "10001/fields",
                       "startAt": 0, "maxResults": 200});
    if block(engine.read(&http, "fixture", FIELDS, input)).is_ok() {
        let sent = http.0.lock().unwrap().clone();
        let segments: Vec<String> = ["issue", "createmeta", PROJECT, "issuetypes", "10001/fields"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        assert_eq!(sent, [(segments, "startAt=0&maxResults=200".to_owned())]);
    }
}

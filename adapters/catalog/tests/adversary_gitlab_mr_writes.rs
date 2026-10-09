//! Adversary pass against story:parity-gitlab-mr-writes: the shipped GitLab
//! merge-request note, reply and resolve selections, driven through the engine
//! against a scripted provider. No network, no credential.
//!
//! The pinned GitLab document (`adapters/gitlab/upstream/openapi_v3.yaml`)
//! types the resolve body (`RequestBody_b5c6ef66b3c0`) as `resolved: boolean`,
//! required, and the note bodies (`RequestBody_ac6f9367f3de`,
//! `RequestBody_c1925678aad5`) with `body: string` required. The cases below
//! hold the selections to that document, and the resolve guard to the
//! discussion it was approved for.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Engine, Selection};
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{
    AuthenticatedHttp, AuthenticatedWrite, HttpResponse, WriteMethod, WriteOutcome,
};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    path::Path,
    sync::{Arc, Mutex},
};

const THREAD: &str = "d15c0000000000000000000000000000000000a1";
const OTHER: &str = "d15c0000000000000000000000000000000000b2";

struct Reads {
    responses: Mutex<VecDeque<HttpResponse>>,
    calls: Mutex<Vec<Vec<String>>>,
}
#[async_trait::async_trait]
impl AuthenticatedHttp for Reads {
    async fn get(&self, path: &[&str], _query: &[(&str, String)]) -> Result<HttpResponse> {
        self.calls
            .lock()
            .unwrap()
            .push(path.iter().map(|s| s.to_string()).collect());
        Ok(self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected provider read"))
    }
}
fn reads(answers: Vec<HttpResponse>) -> Reads {
    Reads {
        responses: Mutex::new(answers.into()),
        calls: Mutex::new(Vec::new()),
    }
}

type Sent = Arc<Mutex<Vec<(WriteMethod, Vec<String>, Value)>>>;
struct Write {
    sent: Sent,
    answer: HttpResponse,
}
#[async_trait::async_trait]
impl AuthenticatedWrite for Write {
    async fn send_json(
        self: Box<Self>,
        method: WriteMethod,
        path: &[&str],
        _query: &[(&str, String)],
        body: &Value,
    ) -> Result<HttpResponse> {
        self.sent.lock().unwrap().push((
            method,
            path.iter().map(|s| s.to_string()).collect(),
            body.clone(),
        ));
        Ok(self.answer)
    }
}

fn json_answer(status: u16, body: &Value) -> HttpResponse {
    HttpResponse {
        status,
        headers: Default::default(),
        body: serde_json::to_vec(body).unwrap(),
    }
}
fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn engine() -> Engine {
    let file: Value = serde_json::from_slice(
        &std::fs::read(root().join("providers/gitlab/operations.json")).unwrap(),
    )
    .unwrap();
    let selections: Vec<Selection> = serde_json::from_value(file["operations"].clone()).unwrap();
    let bundle = bundle::load(&root().join("generated/bundles"), "gitlab").unwrap();
    Engine::new(&bundle, "/api/v4", &selections).unwrap()
}
/// A resolvable discussion as GitLab answers it (`APIEntitiesDiscussion`).
fn discussion(id: &str, resolved: Value) -> Value {
    json!({"id": id, "individual_note": false, "resolvable": true, "resolved": resolved,
           "notes": [{"id": 901, "type": "DiscussionNote", "body": "Fixture thread",
                      "resolvable": true, "resolved": resolved, "noteable_iid": 7}]})
}
fn resolve_input(resolved: Value) -> Value {
    json!({"id": "org/project", "noteable_id": 7, "discussion_id": THREAD,
           "body": {"resolved": resolved}})
}

/// The pinned document types `resolved` as a boolean. The selection declares
/// it as any scalar (`declared_type(None)`: string, integer or boolean), so
/// `"true"`, `1`, `0` and `"yes"` pass the declared schema, the preflight
/// `GET` runs and the `PUT` would carry a value the document does not admit.
/// A GitLab that coerces `1` to `true` resolves the discussion and answers
/// `"resolved": true`, which the postflight compares with the text `"1"` and
/// reports as unknown — an applied write the operator approved, reported as
/// possibly not applied. Each must be refused before any request.
#[tokio::test]
async fn adversary_resolve_refuses_a_non_boolean_resolved_before_any_request() {
    let engine = engine();
    let mut admitted = Vec::new();
    for resolved in [
        json!("true"),
        json!("false"),
        json!(1),
        json!(0),
        json!("yes"),
    ] {
        let http = reads(vec![json_answer(200, &discussion(THREAD, json!(false)))]);
        let outcome = engine
            .prepare(
                &http,
                "fixture",
                "merge_request.discussion.resolve",
                resolve_input(resolved.clone()),
            )
            .await;
        let reads = http.calls.lock().unwrap().len();
        match outcome {
            Err(error) if error.code == ErrorCode::InvalidInput && reads == 0 => {}
            Err(error) => admitted.push(format!(
                "{resolved}: {:?} after {reads} read(s)",
                error.code
            )),
            Ok(_) => admitted.push(format!("{resolved}: prepared after {reads} read(s)")),
        }
    }
    assert!(
        admitted.is_empty(),
        "a non-boolean `resolved` was admitted: {admitted:?}"
    );
}

/// The pinned document requires `body` (the note text) on a note and on a
/// reply. The selections close their bodies to `body` (and `internal`) but
/// require nothing, so a note carrying only `internal: true`, or no key at
/// all, prepares and is sent as an approved write GitLab can only refuse.
#[tokio::test]
async fn adversary_a_note_or_reply_without_its_text_is_refused_before_any_request() {
    let engine = engine();
    let mut admitted = Vec::new();
    for (operation, input) in [
        (
            "merge_request.note.create",
            json!({"id": "org/project", "noteable_id": 7, "body": {}}),
        ),
        (
            "merge_request.note.create",
            json!({"id": "org/project", "noteable_id": 7, "body": {"internal": true}}),
        ),
        (
            "merge_request.discussion.reply",
            json!({"id": "org/project", "noteable_id": 7, "discussion_id": THREAD, "body": {}}),
        ),
    ] {
        let http = reads(vec![]);
        match engine
            .prepare(&http, "fixture", operation, input.clone())
            .await
        {
            Err(error) if error.code == ErrorCode::InvalidInput => {}
            Err(error) => admitted.push(format!("{operation} {input}: {:?}", error.code)),
            Ok(_) => admitted.push(format!("{operation} {input}: prepared")),
        }
    }
    assert!(
        admitted.is_empty(),
        "a note without its text was prepared: {admitted:?}"
    );
}

/// The resolve guard compares only `/resolved` after the write. An answer for
/// a different discussion that happens to carry the requested state is
/// reported as applied, although nothing shows the approved discussion
/// changed. `/id` equal to `discussion_id` is not checked before or after.
#[tokio::test]
async fn adversary_a_resolve_answer_for_another_discussion_is_not_applied() {
    let engine = engine();
    let http = reads(vec![json_answer(200, &discussion(THREAD, json!(false)))]);
    let prepared = engine
        .prepare(
            &http,
            "fixture",
            "merge_request.discussion.resolve",
            resolve_input(json!(true)),
        )
        .await
        .unwrap();
    let sent: Sent = Arc::default();
    let outcome = prepared
        .execute(Box::new(Write {
            sent: sent.clone(),
            answer: json_answer(200, &discussion(OTHER, json!(true))),
        }))
        .await;
    assert_eq!(sent.lock().unwrap().len(), 1);
    assert!(
        !matches!(outcome, WriteOutcome::Applied(_)),
        "an answer for discussion {OTHER} proved the resolve of {THREAD}"
    );
}

/// Could not break, kept as evidence: a preflight answer without
/// `/resolvable` refuses before the write; a `PUT` answer without
/// `/resolved`, with an empty body, or a redirect, is never applied; each
/// write's redirect is never applied.
#[tokio::test]
async fn adversary_resolve_guard_holds_on_missing_values_empty_bodies_and_redirects() {
    let engine = engine();
    let mut answer = discussion(THREAD, json!(false));
    answer.as_object_mut().unwrap().remove("resolvable");
    let http = reads(vec![json_answer(200, &answer)]);
    let refused = engine
        .prepare(
            &http,
            "fixture",
            "merge_request.discussion.resolve",
            resolve_input(json!(true)),
        )
        .await;
    assert_eq!(
        refused.err().map(|e| e.code),
        Some(ErrorCode::UpstreamProtocol)
    );

    let mut without = discussion(THREAD, json!(true));
    without.as_object_mut().unwrap().remove("resolved");
    for answer in [
        json_answer(200, &without),
        HttpResponse {
            status: 200,
            headers: Default::default(),
            body: Vec::new(),
        },
        HttpResponse {
            status: 302,
            headers: vec![("location".into(), "https://elsewhere.test/".into())]
                .into_iter()
                .collect(),
            body: Vec::new(),
        },
    ] {
        let status = answer.status;
        let http = reads(vec![json_answer(200, &discussion(THREAD, json!(false)))]);
        let prepared = engine
            .prepare(
                &http,
                "fixture",
                "merge_request.discussion.resolve",
                resolve_input(json!(true)),
            )
            .await
            .unwrap();
        let outcome = prepared
            .execute(Box::new(Write {
                sent: Arc::default(),
                answer,
            }))
            .await;
        assert!(
            matches!(outcome, WriteOutcome::Unknown(_)),
            "resolve answered {status}"
        );
    }

    for (operation, input) in [
        (
            "merge_request.note.create",
            json!({"id": "org/project", "noteable_id": 7, "body": {"body": "x"}}),
        ),
        (
            "merge_request.discussion.reply",
            json!({"id": "org/project", "noteable_id": 7, "discussion_id": THREAD,
                   "body": {"body": "x"}}),
        ),
    ] {
        let http = reads(vec![]);
        let prepared = engine
            .prepare(&http, "fixture", operation, input)
            .await
            .unwrap();
        let outcome = prepared
            .execute(Box::new(Write {
                sent: Arc::default(),
                answer: HttpResponse {
                    status: 302,
                    headers: Default::default(),
                    body: Vec::new(),
                },
            }))
            .await;
        assert!(
            matches!(outcome, WriteOutcome::Unknown(_)),
            "{operation} answered 302"
        );
        assert!(http.calls.lock().unwrap().is_empty(), "{operation} read");
    }
}

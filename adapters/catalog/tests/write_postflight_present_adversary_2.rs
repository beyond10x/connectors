//! Second adversary pass on story:write-postflight-present-check: what the
//! correction `Pinned::Present => true` in `Pinned::present` reaches beyond the
//! main preflight pass 1 tested: a further preflight and its own `mismatch`,
//! the other comparison kinds the same function reads, a preflight answer that
//! carries no JSON value at all, and a postflight presence beside a declared
//! refusal or after a read.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Engine, Selection};
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{
    AuthenticatedHttp, AuthenticatedWrite, HttpResponse, WriteMethod, WriteOutcome,
};
use serde_json::{Value, json};
use std::{collections::VecDeque, path::Path, sync::Mutex};

const BASE: &str = "/rest/api/3";
const INSTANCE: &str = "fixture-jira";
const CREATE: &str = "issue.create";
const COMMENT: &str = "issue.comment.add";
const EDIT: &str = "issue.edit";
const STAMP: &str = "2026-10-01T00:00:00.000+0000";

/// Answers each read in turn: `Some(value)` a 200 JSON answer, `None` a 200
/// with an empty body.
struct Scripted {
    responses: Mutex<VecDeque<Option<Value>>>,
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
        Ok(json_response(200, answer.as_ref()))
    }
}

fn scripted(answers: Vec<Option<Value>>) -> Scripted {
    Scripted {
        responses: Mutex::new(answers.into()),
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

/// The shipped Jira selections, the guard of `id` changed by `edit`.
fn engine_with(id: &str, edit: impl FnOnce(&mut Value)) -> Engine {
    let mut file: Value = serde_json::from_slice(
        &std::fs::read(root().join("providers/jira/operations.json")).unwrap(),
    )
    .unwrap();
    let entry = file["operations"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|entry| entry["id"] == id)
        .unwrap();
    edit(&mut entry["guard"]);
    let bundle = bundle::load(&root().join("generated/bundles"), "jira").unwrap();
    let selections: Vec<Selection> = serde_json::from_value(file["operations"].clone()).unwrap();
    Engine::new(&bundle, BASE, &selections).expect("the edited selection loads")
}

fn comment_input() -> Value {
    json!({"issueIdOrKey": "PROJ-1",
           "body": {"body": {"content": [{"type": "paragraph",
                                          "content": [{"type": "text", "text": "c"}]}]}}})
}
fn edit_input() -> Value {
    json!({"issueIdOrKey": "PROJ-1", "updated_stamp": STAMP,
           "body": {"fields": {"summary": "changed"}}})
}
fn create_input() -> Value {
    json!({"body": {"fields": {"project": {"key": "PROJ"},
                               "issuetype": {"id": "10001"},
                               "summary": "s"}}})
}

/// The issue the shipped preflights read, `extra` merged into its `fields`.
fn issue(extra: Value) -> Value {
    let mut fields = json!({"summary": "s", "updated": STAMP});
    for (key, value) in extra.as_object().unwrap() {
        fields[key] = value.clone();
    }
    json!({"id": "10001", "key": "PROJ-1", "fields": fields})
}

/// The code the preflights refuse with, or `None` when the write is prepared.
async fn preflight_code(
    engine: &Engine,
    id: &str,
    input: Value,
    reads: Vec<Option<Value>>,
) -> Option<ErrorCode> {
    let http = scripted(reads);
    match engine.prepare(&http, INSTANCE, id, input).await {
        Ok(_) => None,
        Err(error) => Some(error.code),
    }
}

fn outcome<T>(outcome: WriteOutcome<T>) -> String {
    match outcome {
        WriteOutcome::Applied(_) => "applied".into(),
        WriteOutcome::Refused(error) => format!("refused {:?}", error.code),
        WriteOutcome::Unknown(_) => "unknown".into(),
    }
}

/// A presence in a further preflight is read with that preflight's own
/// `mismatch`: an absent member and a `null` one are refused alike, as
/// `forbidden` without `mismatch` and as `conflict` with it, and the first
/// preflight's lack of `mismatch` does not leak into the second.
#[tokio::test]
async fn a_further_preflight_presence_refuses_absent_and_null_with_its_own_code() {
    let further = |mismatch: Option<&str>| {
        let mut preflight = json!({
            "operation_id": "getIssue",
            "values": {"issueIdOrKey": "issueIdOrKey"},
            "checks": [{"pointer": "/fields/assignee", "expect": {"present": true}}]
        });
        if let Some(mismatch) = mismatch {
            preflight["mismatch"] = json!(mismatch);
        }
        engine_with(COMMENT, |guard| {
            guard["further_preflights"] = json!([preflight]);
        })
    };
    let plain = further(None);
    let conflict = further(Some("conflict"));
    let first = || Some(issue(json!({})));
    for (case, engine, second, expected) in [
        (
            "assigned, no mismatch",
            &plain,
            issue(json!({"assignee": {"accountId": "a"}})),
            None,
        ),
        (
            "assignee null, no mismatch",
            &plain,
            issue(json!({"assignee": null})),
            Some(ErrorCode::Forbidden),
        ),
        (
            "assignee absent, no mismatch",
            &plain,
            issue(json!({})),
            Some(ErrorCode::Forbidden),
        ),
        (
            "assigned, mismatch conflict",
            &conflict,
            issue(json!({"assignee": {"accountId": "a"}})),
            None,
        ),
        (
            "assignee null, mismatch conflict",
            &conflict,
            issue(json!({"assignee": null})),
            Some(ErrorCode::Conflict),
        ),
        (
            "assignee absent, mismatch conflict",
            &conflict,
            issue(json!({})),
            Some(ErrorCode::Conflict),
        ),
    ] {
        assert_eq!(
            preflight_code(
                engine,
                COMMENT,
                comment_input(),
                vec![first(), Some(second)]
            )
            .await,
            expected,
            "{case}"
        );
    }
}

/// The correction changed only the presence arm of `Pinned::present`: an
/// absent member a text or a boolean comparison reads is still the preflight
/// answering without a value (`upstream_protocol`), while a `null` boolean is
/// the comparison failing (`conflict` under the edit's `mismatch`).
#[tokio::test]
async fn the_correction_leaves_an_absent_text_or_boolean_member_a_protocol_failure() {
    let with_check = |check: Value| {
        engine_with(EDIT, move |guard| {
            guard["preflight"]["checks"]
                .as_array_mut()
                .unwrap()
                .push(check)
        })
    };
    let literal =
        with_check(json!({"pointer": "/fields/reporter", "expect": {"literal": "someone"}}));
    let boolean = with_check(json!({"pointer": "/fields/flagged", "expect": {"boolean": true}}));
    for (case, engine, extra, expected) in [
        (
            "literal absent",
            &literal,
            json!({}),
            Some(ErrorCode::UpstreamProtocol),
        ),
        (
            "literal null",
            &literal,
            json!({"reporter": null}),
            Some(ErrorCode::UpstreamProtocol),
        ),
        (
            "literal differs",
            &literal,
            json!({"reporter": "other"}),
            Some(ErrorCode::Conflict),
        ),
        (
            "boolean absent",
            &boolean,
            json!({}),
            Some(ErrorCode::UpstreamProtocol),
        ),
        (
            "boolean null",
            &boolean,
            json!({"flagged": null}),
            Some(ErrorCode::Conflict),
        ),
        ("boolean true", &boolean, json!({"flagged": true}), None),
    ] {
        assert_eq!(
            preflight_code(engine, EDIT, edit_input(), vec![Some(issue(extra))]).await,
            expected,
            "{case}"
        );
    }
}

/// A preflight answered 200 with an empty body carries no JSON value at all,
/// so it shows nothing about the target. With a text check first it is the
/// preflight answering without a value (`upstream_protocol`). With a presence
/// check first, the same empty answer must not become the `conflict` the edit's
/// `"mismatch": "conflict"` reserves for "a read that proves the target changed
/// since the caller read it" (`docs/local-catalog-provider.md`, guard).
#[tokio::test]
async fn an_empty_preflight_answer_is_not_read_as_a_changed_target_by_a_presence() {
    let text_first = engine_with(EDIT, |_| {});
    let presence_first = engine_with(EDIT, |guard| {
        guard["preflight"]["checks"].as_array_mut().unwrap().insert(
            0,
            json!({"pointer": "/fields/assignee", "expect": {"present": true}}),
        )
    });
    assert_eq!(
        preflight_code(&text_first, EDIT, edit_input(), vec![None]).await,
        Some(ErrorCode::UpstreamProtocol),
        "empty answer, text check first"
    );
    assert_eq!(
        preflight_code(&presence_first, EDIT, edit_input(), vec![None]).await,
        Some(ErrorCode::UpstreamProtocol),
        "empty answer, presence check first"
    );
}

/// A presence beside a declared refusal: an answer without the new `id` and
/// `key` that names a declared refusal is refused with its code; one with them
/// is applied even when it carries the refusal's value, as the refusal is read
/// only from an answer that fails a check.
#[tokio::test]
async fn a_declared_refusal_beside_a_presence_reads_only_an_answer_that_fails_it() {
    let create = engine_with(CREATE, |guard| {
        guard["postflight"]["refusals"] = json!([
            {"pointer": "/errorMessages/0", "values": ["denied"], "code": "forbidden"}
        ]);
    });
    for (case, body, expected) in [
        (
            "no id, refusal named",
            json!({"errorMessages": ["denied"]}),
            "refused Forbidden",
        ),
        (
            "no id, other message",
            json!({"errorMessages": ["other"]}),
            "unknown",
        ),
        (
            "id and key, refusal named",
            json!({"id": "1", "key": "PROJ-1", "errorMessages": ["denied"]}),
            "applied",
        ),
    ] {
        let http = scripted(vec![Some(json!({"id": "10000", "key": "PROJ"}))]);
        let prepared = create
            .prepare(&http, INSTANCE, CREATE, create_input())
            .await
            .unwrap_or_else(|error| panic!("{case}: refused before dispatch: {error:?}"));
        let got = outcome(
            prepared
                .execute(Box::new(Answer {
                    status: 201,
                    body: Some(body),
                }))
                .await,
        );
        assert_eq!(got, expected, "{case}");
    }
}

/// A presence after a read after the write: a reread answered 200 with an
/// empty body names nothing, so the edit is unknown, never applied.
#[tokio::test]
async fn a_presence_after_a_read_reads_an_empty_reread_as_unknown() {
    let edit = engine_with(EDIT, |guard| {
        guard["postflight"]["checks"] = json!([{"pointer": "/key", "expect": {"present": true}}]);
    });
    let http = scripted(vec![Some(issue(json!({}))), None]);
    let prepared = edit
        .prepare(&http, INSTANCE, EDIT, edit_input())
        .await
        .unwrap_or_else(|error| panic!("refused before dispatch: {error:?}"));
    let got = outcome(
        prepared
            .execute_reading(
                Box::new(Answer {
                    status: 204,
                    body: None,
                }),
                Some(&http),
            )
            .await,
    );
    assert_eq!(got, "unknown");
}

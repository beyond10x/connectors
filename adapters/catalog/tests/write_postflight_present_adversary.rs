//! Adversary cases for story:write-postflight-present-check: the `present`
//! expectation outside the two shipped postflights it was written for.
//!
//! The guide (`docs/local-catalog-provider.md`, "configure the provider") says
//! a check that differs before the write refuses it unsent as `forbidden`, or
//! as `conflict` under `"mismatch": "conflict"`, and that `{"present": true}`
//! holds when the member exists and is not `null`. The definition guide offers
//! `"?"` in `before.expect` as well as in `after.expect`. So before the write,
//! a member that is absent and one that is `null` fail the same comparison and
//! are refused the same way.
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

/// The code the preflight refuses with, or `None` when the write is prepared.
async fn preflight_code(engine: &Engine, id: &str, input: Value, read: Value) -> Option<ErrorCode> {
    let http = Scripted {
        responses: Mutex::new([read].into()),
    };
    match engine.prepare(&http, INSTANCE, id, input).await {
        Ok(_) => None,
        Err(error) => Some(error.code),
    }
}

/// `{"present": true}` before the write: an absent member is the comparison
/// failing, exactly as a `null` one is, so both are refused with the code the
/// preflight declares for a check that does not hold: `forbidden` without
/// `mismatch`, `conflict` with `"mismatch": "conflict"`.
#[tokio::test]
async fn a_preflight_presence_refuses_an_absent_member_as_it_refuses_a_null_one() {
    let check = json!({"pointer": "/fields/assignee", "expect": {"present": true}});
    let comment = engine_with(COMMENT, |guard| {
        guard["preflight"]["checks"]
            .as_array_mut()
            .unwrap()
            .push(check.clone())
    });
    let edit = engine_with(EDIT, |guard| {
        guard["preflight"]["checks"]
            .as_array_mut()
            .unwrap()
            .push(check.clone())
    });
    let issue = |assignee: Option<Value>| {
        let mut fields = json!({"summary": "s", "updated": STAMP});
        if let Some(assignee) = assignee {
            fields["assignee"] = assignee;
        }
        json!({"id": "10001", "key": "PROJ-1", "fields": fields})
    };

    assert_eq!(
        preflight_code(
            &comment,
            COMMENT,
            comment_input(),
            issue(Some(json!({"accountId": "a"})))
        )
        .await,
        None,
        "an assigned issue is prepared"
    );
    assert_eq!(
        preflight_code(&comment, COMMENT, comment_input(), issue(Some(Value::Null))).await,
        Some(ErrorCode::Forbidden),
        "assignee null, no mismatch"
    );
    assert_eq!(
        preflight_code(&comment, COMMENT, comment_input(), issue(None)).await,
        Some(ErrorCode::Forbidden),
        "assignee absent, no mismatch"
    );
    assert_eq!(
        preflight_code(&edit, EDIT, edit_input(), issue(Some(Value::Null))).await,
        Some(ErrorCode::Conflict),
        "assignee null, mismatch conflict"
    );
    assert_eq!(
        preflight_code(&edit, EDIT, edit_input(), issue(None)).await,
        Some(ErrorCode::Conflict),
        "assignee absent, mismatch conflict"
    );
}

/// `present` after a read after the write (`postflight.read`): the reread
/// answer without the member, or with it `null`, leaves the edit unknown; with
/// it, applied.
#[tokio::test]
async fn a_presence_after_a_read_after_the_write_reads_the_reread_answer() {
    let edit = engine_with(EDIT, |guard| {
        guard["postflight"]["checks"] =
            json!([{"pointer": "/fields/summary", "expect": {"present": true}}]);
    });
    let issue =
        json!({"id": "10001", "key": "PROJ-1", "fields": {"summary": "s", "updated": STAMP}});
    for (case, reread, expected) in [
        (
            "summary present",
            json!({"key": "PROJ-1", "fields": {"summary": "changed"}}),
            "applied",
        ),
        (
            "summary null",
            json!({"key": "PROJ-1", "fields": {"summary": null}}),
            "unknown",
        ),
        (
            "summary absent",
            json!({"key": "PROJ-1", "fields": {}}),
            "unknown",
        ),
    ] {
        let http = Scripted {
            responses: Mutex::new([issue.clone(), reread].into()),
        };
        let prepared = edit
            .prepare(&http, INSTANCE, EDIT, edit_input())
            .await
            .unwrap_or_else(|error| panic!("{case}: refused before dispatch: {error:?}"));
        let outcome = match prepared
            .execute_reading(
                Box::new(Answer {
                    status: 204,
                    body: None,
                }),
                Some(&http),
            )
            .await
        {
            WriteOutcome::Applied(_) => "applied",
            WriteOutcome::Refused(_) => "refused",
            WriteOutcome::Unknown(_) => "unknown",
        };
        assert_eq!(outcome, expected, "{case}");
    }
}

/// Nested pointers and array members: `/fields/0/id` style pointers resolve as
/// RFC 6901 says, and an empty array or object at the pointer is a value, not
/// `null`, as the guide documents ("whatever its value").
#[tokio::test]
async fn a_presence_reads_nested_and_array_pointers() {
    let create = engine_with(CREATE, |guard| {
        guard["postflight"]["checks"] = json!([
            {"pointer": "/issues/0/id", "expect": {"present": true}},
            {"pointer": "/meta/key", "expect": {"present": true}}
        ]);
    });
    for (case, body, expected) in [
        (
            "both present",
            json!({"issues": [{"id": "1"}], "meta": {"key": "PROJ-1"}}),
            "applied",
        ),
        (
            "array empty",
            json!({"issues": [], "meta": {"key": "PROJ-1"}}),
            "unknown",
        ),
        (
            "array member null",
            json!({"issues": [null], "meta": {"key": "PROJ-1"}}),
            "unknown",
        ),
        (
            "nested key an empty object",
            json!({"issues": [{"id": "1"}], "meta": {"key": {}}}),
            "applied",
        ),
        (
            "parent an array, not an object",
            json!({"issues": [{"id": "1"}], "meta": ["PROJ-1"]}),
            "unknown",
        ),
    ] {
        let http = Scripted {
            responses: Mutex::new([json!({"id": "10000", "key": "PROJ"})].into()),
        };
        let prepared = create
            .prepare(&http, INSTANCE, CREATE, create_input())
            .await
            .unwrap_or_else(|error| panic!("{case}: refused before dispatch: {error:?}"));
        let outcome = match prepared
            .execute(Box::new(Answer {
                status: 201,
                body: Some(body),
            }))
            .await
        {
            WriteOutcome::Applied(_) => "applied",
            WriteOutcome::Refused(_) => "refused",
            WriteOutcome::Unknown(_) => "unknown",
        };
        assert_eq!(outcome, expected, "{case}");
    }
}

/// `present: false` is refused when the selection loads wherever a check
/// sits: in a further preflight and in `any_of`, not only in `checks`.
#[test]
fn a_presence_of_false_is_refused_in_any_of_and_further_preflights() {
    let load = |edit: &dyn Fn(&mut Value)| {
        let mut file: Value = serde_json::from_slice(
            &std::fs::read(root().join("providers/jira/operations.json")).unwrap(),
        )
        .unwrap();
        let entry = file["operations"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|entry| entry["id"] == CREATE)
            .unwrap();
        edit(&mut entry["guard"]);
        let bundle = bundle::load(&root().join("generated/bundles"), "jira").unwrap();
        let selections: Vec<Selection> =
            serde_json::from_value(file["operations"].clone()).unwrap();
        Engine::new(&bundle, BASE, &selections).err()
    };
    let any_of = load(&|guard| {
        guard["postflight"]["any_of"] = json!([
            {"pointer": "/self", "expect": {"present": false}},
            {"pointer": "/self", "expect": {"present": true}}
        ]);
    })
    .expect("a presence of false in any_of loads");
    assert!(
        format!("{any_of:?}").contains("presence other than true"),
        "{any_of:?}"
    );
    let further = load(&|guard| {
        guard["further_preflights"] = json!([{
            "operation_id": "getProject",
            "values": {"projectIdOrKey": "body.fields.project.key"},
            "checks": [{"pointer": "/id", "expect": {"present": false}}]
        }]);
    })
    .expect("a presence of false in a further preflight loads");
    assert!(
        format!("{further:?}").contains("presence other than true"),
        "{further:?}"
    );
}

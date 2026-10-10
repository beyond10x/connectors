//! Adversarial cases for story:catalog-guard-postflight-read and
//! story:catalog-selection-fixed-body-value, driven at the engine boundary with
//! in-memory ports that record every request in one ordered log, so "before",
//! "after" and "exactly once" are positions in that log.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Engine, Selection};
use connectors_core::{Error, ErrorCode, Result};
use connectors_sdk::{
    AuthenticatedHttp, AuthenticatedWrite, HttpResponse, WriteMethod, WriteOutcome,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex},
};

const HEAD: &str = "0123456789abcdef0123456789abcdef01234567";
const OTHER: &str = "89abcdef0123456789abcdef0123456789abcdef";

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped(provider: &str) -> Vec<Selection> {
    let file: Value = serde_json::from_slice(
        &std::fs::read(root().join(format!("providers/{provider}/operations.json"))).unwrap(),
    )
    .unwrap();
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn engine_with(provider: &str, base: &str, selections: &[Selection]) -> Result<Engine> {
    let bundle = bundle::load(&root().join("generated/bundles"), provider).unwrap();
    Engine::new(&bundle, base, selections)
}
fn block<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(future)
}
fn answer(status: u16, body: Option<Value>) -> HttpResponse {
    HttpResponse {
        status,
        headers: BTreeMap::new(),
        body: body
            .map(|b| serde_json::to_vec(&b).unwrap())
            .unwrap_or_default(),
    }
}

/// Every request, in order: method, joined segments, query, body.
type Entry = (String, String, String, Value);
type Log = Arc<Mutex<Vec<Entry>>>;

/// A read port answering GETs from a list of answers, in order; one answer per
/// GET, and `Err(unavailable)` for an answer of `None`.
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
            query
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("&"),
            Value::Null,
        ));
        let mut answers = self.answers.lock().unwrap();
        assert!(!answers.is_empty(), "a GET nobody expected was sent");
        answers.remove(0).ok_or_else(Error::unavailable)
    }
}
/// A write port answering once; `None` is a lost response.
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
            query
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("&"),
            body.clone(),
        ));
        self.answer.ok_or_else(Error::unavailable)
    }
}

/// Prepare with `preflight` answers, then execute with the write answer and
/// `after` answers for the read after the write. Returns the outcome (or the
/// prepare refusal) and the log.
fn run(
    provider: &str,
    base: &str,
    id: &str,
    input: Value,
    preflight: Vec<Option<HttpResponse>>,
    write: Option<HttpResponse>,
    after: Vec<Option<HttpResponse>>,
) -> (std::result::Result<WriteOutcome<Value>, Error>, Vec<Entry>) {
    let engine = engine_with(provider, base, &shipped(provider)).unwrap();
    let log: Log = Arc::default();
    let before = Reader {
        log: log.clone(),
        answers: Mutex::new(preflight),
    };
    let outcome = block(async {
        let prepared = engine.prepare(&before, "fixture", id, input).await?;
        assert!(
            before.answers.lock().unwrap().is_empty(),
            "a preflight answer was left unread"
        );
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

fn effect(outcome: &std::result::Result<WriteOutcome<Value>, Error>) -> &'static str {
    match outcome {
        Ok(WriteOutcome::Applied(Ok(_))) => "applied",
        Ok(WriteOutcome::Applied(Err(_))) => "applied-err",
        Ok(WriteOutcome::Refused(_)) => "refused",
        Ok(WriteOutcome::Unknown(_)) => "unknown",
        Err(_) => "not-attempted",
    }
}

// ---------------------------------------------------------------- GitLab reopen

fn mr(state: &str, sha: &str) -> Value {
    json!({"id": 510, "iid": 10, "project_id": 7, "state": state, "sha": sha,
           "merge_when_pipeline_succeeds": false})
}

/// GitLab's reopen reloads the merge request's diff (`ReopenService` calls
/// `reload_diff`), so a source branch pushed while the merge request was closed
/// gives the reopened merge request another head than the closed one showed.
/// The selection promises "Reopen a closed merge request whose head is the
/// pinned sha"; the preflight pins the head of the *closed* merge request, and
/// the postflight checks only `/state`. An answer `opened` at another head is
/// then reported applied, where the auto-merge variant beside it (whose
/// postflight compares `/sha`) leaves the same divergence unknown.
#[test]
fn a_reopen_answered_at_another_head_is_not_applied() {
    let (outcome, log) = run(
        "gitlab",
        "/api/v4",
        "merge_request.reopen",
        json!({"id": "org/project", "merge_request_iid": 10, "sha": HEAD}),
        vec![Some(answer(200, Some(mr("closed", HEAD))))],
        Some(answer(200, Some(mr("opened", OTHER)))),
        vec![],
    );
    assert_eq!(log.len(), 2, "{log:?}");
    assert_eq!(effect(&outcome), "unknown", "{log:?}");
}

/// Regression guard: the reopen sends exactly the fixed body, whatever the
/// caller omits, and a reopen at the pinned head is applied.
#[test]
fn a_reopen_sends_exactly_the_fixed_state_event() {
    for body in [None, Some(json!({}))] {
        let mut input = json!({"id": "org/project", "merge_request_iid": 10, "sha": HEAD});
        if let Some(body) = body {
            input["body"] = body;
        }
        let (outcome, log) = run(
            "gitlab",
            "/api/v4",
            "merge_request.reopen",
            input,
            vec![Some(answer(200, Some(mr("closed", HEAD))))],
            Some(answer(200, Some(mr("opened", HEAD)))),
            vec![],
        );
        assert_eq!(effect(&outcome), "applied");
        assert_eq!(log[1].0, "PUT");
        assert_eq!(log[1].3, json!({"state_event": "reopen"}));
    }
}

// ------------------------------------------------------------ GitLab auto-merge

/// Regression guard: `any_of` holding does not excuse an ordinary check. A merge
/// answered `merged` (an accepted alternative) at another head is unknown.
#[test]
fn an_any_of_that_holds_does_not_excuse_a_differing_ordinary_check() {
    let (outcome, log) = run(
        "gitlab",
        "/api/v4",
        "merge_request.auto_merge",
        json!({"id": "org/project", "merge_request_iid": 10, "body": {"sha": HEAD}}),
        vec![Some(answer(200, Some(mr("opened", HEAD))))],
        Some(answer(200, Some(mr("merged", OTHER)))),
        vec![],
    );
    assert_eq!(effect(&outcome), "unknown", "{log:?}");
    // And the fixed value is what was sent.
    assert_eq!(log[1].3, json!({"auto_merge": true, "sha": HEAD}));
}

/// Regression guard: no caller shape reaches the fixed key. A differently cased
/// key, a nested object, a `null`, a body given as a string or as an array,
/// and `null` for the whole body are each refused before any request.
#[test]
fn no_caller_body_shape_reaches_the_fixed_value() {
    let engine = engine_with("gitlab", "/api/v4", &shipped("gitlab")).unwrap();
    let bodies = [
        json!({"sha": HEAD, "Auto_Merge": false}),
        json!({"sha": HEAD, "AUTO_MERGE": false}),
        json!({"sha": HEAD, "auto_merge": {"value": false}}),
        json!({"sha": HEAD, "auto_merge": null}),
        json!({"sha": HEAD, "auto_merge ": false}),
        json!({"sha": HEAD, "merge_when_pipeline_succeeds": false}),
        json!(r#"{"sha":"0123","auto_merge":false}"#),
        json!([{"auto_merge": false}]),
        Value::Null,
    ];
    for body in bodies {
        let input = json!({"id": "org/project", "merge_request_iid": 10, "body": body.clone()});
        let reader = Reader {
            log: Arc::default(),
            answers: Mutex::new(vec![]),
        };
        let refusal = block(engine.prepare(&reader, "fixture", "merge_request.auto_merge", input));
        assert!(refusal.is_err(), "{body} was admitted");
        assert_eq!(
            refusal.err().unwrap().code,
            ErrorCode::InvalidInput,
            "{body}"
        );
    }
    // The same for the reopen, whose body may be omitted.
    for body in [
        json!({"State_Event": "close"}),
        json!("state_event=close"),
        Value::Null,
        json!({"title": "x"}),
    ] {
        let input = json!({"id": "org/project", "merge_request_iid": 10, "sha": HEAD, "body": body.clone()});
        let reader = Reader {
            log: Arc::default(),
            answers: Mutex::new(vec![]),
        };
        let refusal = block(engine.prepare(&reader, "fixture", "merge_request.reopen", input));
        assert!(refusal.is_err(), "{body} was admitted");
    }
}

// ------------------------------------------------------------ Jira transition

fn issue(status: &str) -> Value {
    json!({"id": "10001", "key": "FIX-1", "fields": {"status": {"id": status}}})
}
fn transitions(id: &str, to: &str) -> Value {
    json!({"transitions": [{"id": id, "isAvailable": true, "to": {"id": to}}]})
}
fn run_input(key: &str) -> Value {
    json!({"issueIdOrKey": key, "current_status": "3", "target_status": "10002",
           "body": {"transition": {"id": "31"}}})
}

/// Regression guard: the read after the write is never issued when the write
/// is refused, answered with a non-2xx or lost; it is issued exactly once,
/// after the write, at the write's own issue, when the write answers `204`.
#[test]
fn the_read_after_is_issued_once_after_a_2xx_and_never_otherwise() {
    let preflight = || {
        vec![
            Some(answer(200, Some(issue("3")))),
            Some(answer(200, Some(transitions("31", "10002")))),
        ]
    };
    for (write, expected) in [
        (
            Some(answer(400, Some(json!({"errorMessages": ["no"]})))),
            "refused",
        ),
        (Some(answer(409, None)), "refused"),
        (Some(answer(500, None)), "unknown"),
        (Some(answer(302, None)), "unknown"),
        (None, "unknown"),
    ] {
        let (outcome, log) = run(
            "jira",
            "/rest/api/3",
            "issue.transition.run",
            run_input("FIX-1"),
            preflight(),
            write,
            vec![],
        );
        assert_eq!(effect(&outcome), expected, "{log:?}");
        assert_eq!(log.len(), 3, "{log:?}");
        assert_eq!(log[2].0, "POST");
    }
    // 2xx: exactly one read after, at the issue written.
    for key in ["FIX-1", "10001", "fix-1"] {
        let (outcome, log) = run(
            "jira",
            "/rest/api/3",
            "issue.transition.run",
            run_input(key),
            preflight(),
            Some(answer(204, None)),
            vec![Some(answer(200, Some(issue("10002"))))],
        );
        assert_eq!(effect(&outcome), "applied", "{log:?}");
        assert_eq!(log.len(), 4, "{log:?}");
        assert_eq!(log[2].0, "POST");
        assert_eq!(log[2].1, format!("issue/{key}/transitions"));
        assert_eq!(log[3].0, "GET");
        assert_eq!(log[3].1, format!("issue/{key}"));
        assert_eq!(log[0].1, log[3].1);
    }
    // 2xx and the read after fails, is refused or answers unreadably: unknown.
    for after in [
        None,
        Some(answer(404, None)),
        Some(answer(429, None)),
        Some(answer(200, None)),
        Some(HttpResponse {
            status: 200,
            headers: BTreeMap::new(),
            body: b"<html>".to_vec(),
        }),
        Some(answer(
            200,
            Some(json!({"fields": {"status": {"id": 10002}}})),
        )),
    ] {
        let numeric = after
            .as_ref()
            .is_some_and(|a| a.body.starts_with(b"{\"fields\""));
        let (outcome, log) = run(
            "jira",
            "/rest/api/3",
            "issue.transition.run",
            run_input("FIX-1"),
            preflight(),
            Some(answer(204, None)),
            vec![after],
        );
        // A numeric status id compares as its text: applied, never refused.
        let expected = if numeric { "applied" } else { "unknown" };
        assert_eq!(effect(&outcome), expected, "{log:?}");
        assert_eq!(log.len(), 4, "{log:?}");
    }
}

/// Regression guard: a further preflight that refuses, errors, is lost or
/// answers without the checked value stops the write: no POST is sent.
#[test]
fn a_further_preflight_that_refuses_or_fails_never_dispatches() {
    for (second, code) in [
        (
            Some(answer(200, Some(transitions("31", "10000")))),
            ErrorCode::Forbidden,
        ),
        (
            Some(answer(
                200,
                Some(
                    json!({"transitions": [{"id": "31", "isAvailable": false, "to": {"id": "10002"}}]}),
                ),
            )),
            ErrorCode::Forbidden,
        ),
        (
            Some(answer(200, Some(json!({"transitions": []})))),
            ErrorCode::UpstreamProtocol,
        ),
        (Some(answer(404, None)), ErrorCode::Forbidden),
        (Some(answer(500, None)), ErrorCode::Unavailable),
        (Some(answer(429, None)), ErrorCode::RateLimited),
        (None, ErrorCode::Unavailable),
    ] {
        let (outcome, log) = run(
            "jira",
            "/rest/api/3",
            "issue.transition.run",
            run_input("FIX-1"),
            vec![Some(answer(200, Some(issue("3")))), second],
            Some(answer(204, None)),
            vec![],
        );
        assert_eq!(effect(&outcome), "not-attempted", "{log:?}");
        assert_eq!(outcome.err().unwrap().code, code, "{log:?}");
        assert!(log.iter().all(|(m, ..)| m == "GET"), "{log:?}");
        assert_eq!(log.len(), 2, "{log:?}");
    }
}

// ------------------------------------------------------------ load-time limits

fn jira_with(change: &dyn Fn(&mut Value)) -> Result<Engine> {
    let mut selections: Vec<Value> = shipped("jira")
        .iter()
        .map(|s| serde_json::to_value(s).unwrap())
        .collect();
    let run = selections
        .iter_mut()
        .find(|s| s["id"] == "issue.transition.run")
        .unwrap();
    change(&mut run["guard"]);
    let selections: Vec<Selection> = serde_json::from_value(Value::Array(selections))
        .map_err(|e| Error::invalid(e.to_string()))?;
    engine_with("jira", "/rest/api/3", &selections)
}

/// Regression guard at the boundaries: exactly three further preflights load,
/// and sixteen checks in all load, counting `any_of`; seventeen do not, whether
/// the seventeenth is an ordinary check or an `any_of` comparison.
#[test]
fn the_further_preflight_and_check_limits_hold_at_their_boundaries() {
    let check = json!({"pointer": "/fields/status/id", "expect": {"input": "target_status"}});
    // Shipped: 1 + 3 + 1 = 5 checks.
    assert!(
        jira_with(&|guard| {
            let one = guard["further_preflights"][0].clone();
            guard["further_preflights"] = json!([one.clone(), one.clone(), one]);
        })
        .is_ok(),
        "three further preflights were refused"
    );
    // 1 + 3 + 10 + 2 any_of = 16.
    let sixteen = |guard: &mut Value| {
        guard["postflight"]["checks"] = json!(vec![check.clone(); 10]);
        guard["postflight"]["any_of"] = json!([check.clone(), check.clone()]);
    };
    assert!(jira_with(&sixteen).is_ok(), "sixteen checks were refused");
    assert!(
        jira_with(&|guard| {
            sixteen(guard);
            guard["postflight"]["any_of"] = json!([check.clone(), check.clone(), check.clone()]);
        })
        .is_err(),
        "seventeen checks, the last in any_of, loaded"
    );
    assert!(
        jira_with(&|guard| {
            sixteen(guard);
            guard["postflight"]["checks"] = json!(vec![check.clone(); 11]);
        })
        .is_err(),
        "seventeen checks loaded"
    );
}

/// The model (`adapters/catalog/spec/ess/domains/guard.yaml`, Postflight)
/// holds `not defined(any_of) or any_of.count >= 2`: an `any_of` that is
/// present and empty is not a value of it. The engine refuses only an `any_of`
/// of exactly one (`src/lib.rs`, `any_of.len() == 1`) and loads an empty one,
/// then writes it back without the member.
#[test]
fn an_empty_any_of_loads_as_an_absent_one() {
    assert!(
        jira_with(&|guard| guard["postflight"]["any_of"] = json!([])).is_ok(),
        "an empty any_of, which the model admits as absent, was refused"
    );
}

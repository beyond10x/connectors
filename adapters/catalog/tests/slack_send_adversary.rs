//! Adversarial cases for story:slack-message-send, driven at the engine
//! boundary with in-memory ports that record every request in one ordered log,
//! so "before", "after" and "exactly once" are positions in that log. Every
//! id, token and message is synthetic.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Engine, Selection};
use connectors_core::{Error, Result};
use connectors_sdk::{
    AuthenticatedHttp, AuthenticatedWrite, HttpResponse, WriteMethod, WriteOutcome,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex},
};

const BASE: &str = "/api";

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped() -> Vec<Selection> {
    let file: Value = serde_json::from_slice(
        &std::fs::read(root().join("providers/slack/operations.json")).unwrap(),
    )
    .unwrap();
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn engine() -> Engine {
    let bundle = bundle::load(&root().join("generated/bundles"), "slack").unwrap();
    Engine::new(&bundle, BASE, &shipped()).unwrap()
}
fn block<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(future)
}
fn raw(status: u16, headers: &[(&str, &str)], body: &[u8]) -> HttpResponse {
    HttpResponse {
        status,
        headers: headers
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect::<BTreeMap<_, _>>(),
        body: body.to_vec(),
    }
}
fn answer(status: u16, body: Value) -> HttpResponse {
    raw(status, &[], &serde_json::to_vec(&body).unwrap())
}
fn auth_ok() -> HttpResponse {
    answer(
        200,
        json!({"ok": true, "url": "https://fixture.slack.com/", "team": "fixture",
               "user": "fixture-bot", "team_id": "T0FIXTURE01", "user_id": "U0FIXTURE01"}),
    )
}
fn posted() -> Value {
    json!({"ok": true, "channel": "C0FIXTURE01", "ts": "1780000700.000700",
           "message": {"type": "message", "text": "fixture", "ts": "1780000700.000700"}})
}

/// Every request, in order: method, joined segments, query, body.
type Entry = (String, String, String, Value);
type Log = Arc<Mutex<Vec<Entry>>>;

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

/// Prepare with the `auth.test` answer, then execute with the write answer.
fn run(
    id: &str,
    input: Value,
    preflight: Option<HttpResponse>,
    write: Option<HttpResponse>,
) -> (std::result::Result<WriteOutcome<Value>, Error>, Vec<Entry>) {
    let engine = engine();
    let log: Log = Arc::default();
    let before = Reader {
        log: log.clone(),
        answers: Mutex::new(vec![preflight]),
    };
    let outcome = block(async {
        let prepared = engine.prepare(&before, "fixture", id, input).await?;
        let writer = Box::new(Writer {
            log: log.clone(),
            answer: write,
        });
        Ok::<_, Error>(prepared.execute(writer).await)
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
fn post(channel: &str, text: &str) -> Value {
    json!({"body": {"channel": channel, "text": text}})
}
fn writes(log: &[Entry]) -> usize {
    log.iter().filter(|(method, ..)| method != "GET").count()
}

// ───────────────────────────── `ok` read as a string ─────────────────────────

/// Slack's success schema pins `ok` to the boolean `true` (`defs_ok_true`).
/// The guard compares through `scalar`, which turns the boolean and the string
/// alike into `"true"`, so an acknowledgement carrying `"ok": "true"` beside
/// Slack's refusal `error` is recorded as applied.
#[test]
fn a_post_answered_ok_as_the_string_true_is_not_applied() {
    let (outcome, log) = run(
        "chat.postMessage",
        post("C0FIXTURE01", "fixture"),
        Some(auth_ok()),
        Some(answer(
            200,
            json!({"ok": "true", "error": "channel_not_found"}),
        )),
    );
    assert_eq!(writes(&log), 1, "{log:?}");
    assert_eq!(effect(&outcome), "unknown", "{log:?}");
}

/// The same comparison before the write: an `auth.test` answering
/// `"ok": "true"` with Slack's `invalid_auth` passes the preflight, and the
/// post is sent.
#[test]
fn an_auth_test_answered_ok_as_the_string_true_stops_the_post() {
    let (outcome, log) = run(
        "chat.postMessage",
        post("C0FIXTURE01", "fixture"),
        Some(answer(200, json!({"ok": "true", "error": "invalid_auth"}))),
        Some(answer(200, posted())),
    );
    assert_eq!(writes(&log), 0, "{log:?}");
    assert_eq!(effect(&outcome), "not-attempted", "{log:?}");
}

// ───────────────────────── the unit's own ESS invariants ─────────────────────

/// `connectors_slack.records.MessagePost` declares `text.count: {gte: 1}` and
/// `conversation_id.count: {gte: 1}`. An empty `text` or `channel` is outside
/// the model and must be refused before anything is sent; sent, Slack answers
/// `no_text` / `channel_not_found` as `200` with `ok: false`, which the
/// provider records as outcome_unknown, an attempt somebody must reconcile.
#[test]
fn a_post_with_an_empty_text_or_channel_is_refused_before_dispatch() {
    for input in [post("C0FIXTURE01", ""), post("", "fixture")] {
        let (outcome, log) = run(
            "chat.postMessage",
            input.clone(),
            Some(auth_ok()),
            Some(answer(200, json!({"ok": false, "error": "no_text"}))),
        );
        assert_eq!(effect(&outcome), "not-attempted", "{input}: {log:?}");
        assert!(log.is_empty(), "{input}: {log:?}");
    }
}

/// `connectors_slack.records.DirectMessageOpen` declares `users.count:
/// {gte: 1}`; an empty `users` must be refused before anything is sent.
#[test]
fn an_open_with_empty_users_is_refused_before_dispatch() {
    let input = json!({"body": {"users": ""}});
    let (outcome, log) = run(
        "conversations.open",
        input,
        Some(auth_ok()),
        Some(answer(
            200,
            json!({"ok": false, "error": "users_list_not_supplied"}),
        )),
    );
    assert_eq!(effect(&outcome), "not-attempted", "{log:?}");
    assert!(log.is_empty(), "{log:?}");
}

// ───────────────────────────── attacks that held ────────────────────────────

/// Every ambiguous acknowledgement stays unknown and is sent once: `ok`
/// missing, `ok: false`, `ok` as a number, a non-JSON body, an empty body,
/// a `429` with `Retry-After`, a `502`, and a lost response.
#[test]
fn every_ambiguous_acknowledgement_is_unknown_and_sent_once() {
    let cases: Vec<(&str, Option<HttpResponse>)> = vec![
        (
            "ok missing",
            Some(answer(200, json!({"channel": "C0FIXTURE01", "ts": "1.2"}))),
        ),
        (
            "ok false",
            Some(answer(200, json!({"ok": false, "error": "not_in_channel"}))),
        ),
        ("ok one", Some(answer(200, json!({"ok": 1})))),
        ("ok null", Some(answer(200, json!({"ok": null})))),
        (
            "not json",
            Some(raw(
                200,
                &[("content-type", "text/html")],
                b"<html>ok</html>",
            )),
        ),
        ("empty", Some(raw(200, &[], b""))),
        (
            "429",
            Some(raw(
                429,
                &[("retry-after", "30")],
                br#"{"ok":false,"error":"ratelimited"}"#,
            )),
        ),
        ("502", Some(answer(502, json!({"ok": true})))),
        ("lost", None),
    ];
    for (name, write) in cases {
        let (outcome, log) = run(
            "chat.postMessage",
            post("C0FIXTURE01", "fixture"),
            Some(auth_ok()),
            write,
        );
        assert_eq!(effect(&outcome), "unknown", "{name}: {log:?}");
        assert_eq!(writes(&log), 1, "{name}: {log:?}");
        assert_eq!(log[0].1, "auth.test", "{name}");
        assert_eq!(log[1].1, "chat.postMessage", "{name}");
    }
}

/// A failed or refusing `auth.test` stops the write with nothing sent.
#[test]
fn a_failed_auth_test_refuses_before_anything_is_sent() {
    let cases = || -> Vec<(&'static str, Option<HttpResponse>)> {
        vec![
            (
                "ok false",
                Some(answer(200, json!({"ok": false, "error": "invalid_auth"}))),
            ),
            (
                "ok missing",
                Some(answer(200, json!({"user_id": "U0FIXTURE01"}))),
            ),
            ("401", Some(answer(401, json!({"ok": false})))),
            ("429", Some(raw(429, &[("retry-after", "5")], b"{}"))),
            ("500", Some(answer(500, json!({"ok": true})))),
            ("not json", Some(raw(200, &[], b"ok"))),
            ("lost", None),
        ]
    };
    for id in ["chat.postMessage", "conversations.open"] {
        for (name, preflight) in cases() {
            let input = if id == "chat.postMessage" {
                post("C0FIXTURE01", "fixture")
            } else {
                json!({"body": {"users": "U0FIXTURE11"}})
            };
            let (outcome, log) = run(id, input, preflight, Some(answer(200, posted())));
            assert_eq!(effect(&outcome), "not-attempted", "{id} {name}: {log:?}");
            assert_eq!(writes(&log), 0, "{id} {name}: {log:?}");
        }
    }
}

/// No admitted path carries a key outside the closed body, or the token, to
/// Slack: smuggled members and a top-level `token` are refused with nothing
/// sent, and an applied post sends exactly the body supplied.
#[test]
fn no_unadmitted_member_or_token_reaches_the_request() {
    for extra in [
        "blocks",
        "attachments",
        "as_user",
        "username",
        "icon_url",
        "icon_emoji",
        "link_names",
        "reply_broadcast",
        "parse",
        "unfurl_media",
        "token",
        "Token",
        "TOKEN",
        "channel ",
        "metadata",
    ] {
        let mut input = post("C0FIXTURE01", "fixture");
        input["body"][extra] = json!(true);
        let (outcome, log) = run(
            "chat.postMessage",
            input,
            Some(auth_ok()),
            Some(answer(200, posted())),
        );
        assert_eq!(effect(&outcome), "not-attempted", "`{extra}`: {log:?}");
        assert!(log.is_empty(), "`{extra}`: {log:?}");
    }
    for extra in ["channel", "token", "prevent_creation"] {
        let mut input = json!({"body": {"users": "U0FIXTURE11"}});
        input["body"][extra] = json!("D0FIXTURE11");
        let (outcome, log) = run(
            "conversations.open",
            input,
            Some(auth_ok()),
            Some(answer(200, posted())),
        );
        assert_eq!(effect(&outcome), "not-attempted", "`{extra}`: {log:?}");
        assert!(log.is_empty(), "`{extra}`: {log:?}");
    }
    for top in [
        json!({"token": "xoxb-fixture"}),
        json!({"channel": "C0FIXTURE02"}),
    ] {
        let mut input = post("C0FIXTURE01", "fixture");
        for (k, v) in top.as_object().unwrap() {
            input[k] = v.clone();
        }
        let (outcome, log) = run(
            "chat.postMessage",
            input,
            Some(auth_ok()),
            Some(answer(200, posted())),
        );
        assert_eq!(effect(&outcome), "not-attempted", "{log:?}");
        assert!(log.is_empty(), "{log:?}");
    }
    let input = json!({"body": {"channel": "C0FIXTURE01", "text": "fixture",
                                "thread_ts": "1.2", "mrkdwn": false, "unfurl_links": true}});
    let (outcome, log) = run(
        "chat.postMessage",
        input.clone(),
        Some(auth_ok()),
        Some(answer(200, posted())),
    );
    assert_eq!(effect(&outcome), "applied", "{log:?}");
    assert_eq!(log.len(), 2);
    assert_eq!(
        log[0],
        ("GET".into(), "auth.test".into(), String::new(), Value::Null)
    );
    assert_eq!(
        log[1],
        (
            "POST".into(),
            "chat.postMessage".into(),
            String::new(),
            input["body"].clone()
        )
    );
}

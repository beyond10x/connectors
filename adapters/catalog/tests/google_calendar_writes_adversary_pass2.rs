//! Adversary pass 2 against the Google Calendar event writes, after correction
//! round 1: the guide's new claims about `sendUpdates`, rate limits and the
//! truncated guest list, read against the code and the pinned Discovery
//! document, and driven through the engine against a scripted provider. No
//! network, no credential.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Engine, Selection};
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{
    AuthenticatedHttp, AuthenticatedWrite, HttpResponse, WriteMethod, WriteOutcome,
};
use serde_json::{Value, json};
use std::{collections::VecDeque, fs, path::Path, sync::Arc, sync::Mutex};

const INSTANCE: &str = "fixture-google-calendar";
const ETAG: &str = "\"fixture-etag-2\"";

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped() -> Vec<Selection> {
    let file: Value = serde_json::from_slice(
        &fs::read(root().join("providers/google-calendar/operations.json")).unwrap(),
    )
    .unwrap();
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn engine() -> Engine {
    let bundle = bundle::load(&root().join("generated/bundles"), "google-calendar").unwrap();
    Engine::new(&bundle, "/calendar/v3", &shipped()).unwrap()
}
fn pinned() -> Value {
    serde_json::from_slice(
        &fs::read(root().join("../google/upstream/calendar/calendar-api.json")).unwrap(),
    )
    .unwrap()
}
fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
fn guide() -> String {
    flat(&fs::read_to_string(root().join("../../docs/catalog-google-calendar.md")).unwrap())
}
/// The guide text from the bullet opening with `start` up to the next bullet
/// opening with `end`.
fn bullet(guide: &str, start: &str, end: &str) -> String {
    let from = guide.find(start).expect(start);
    let to = from + guide[from..].find(end).expect(end);
    guide[from..to].to_owned()
}

type Call = (Vec<String>, Vec<(String, String)>);
struct Reads {
    responses: Mutex<VecDeque<HttpResponse>>,
    calls: Mutex<Vec<Call>>,
}
#[async_trait::async_trait]
impl AuthenticatedHttp for Reads {
    async fn get(&self, path: &[&str], query: &[(&str, String)]) -> Result<HttpResponse> {
        self.calls.lock().unwrap().push((
            path.iter().map(|s| s.to_string()).collect(),
            query
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        ));
        Ok(self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected provider read"))
    }
}
fn response(status: u16, value: Option<Value>) -> HttpResponse {
    HttpResponse {
        status,
        headers: [(
            "content-type".to_owned(),
            "application/json; charset=UTF-8".to_owned(),
        )]
        .into(),
        body: value
            .map(|v| serde_json::to_vec(&v).unwrap())
            .unwrap_or_default(),
    }
}
type Sent = Arc<Mutex<Vec<(WriteMethod, Vec<String>, Vec<(String, String)>, Value)>>>;
struct Write {
    sent: Sent,
    response: HttpResponse,
}
#[async_trait::async_trait]
impl AuthenticatedWrite for Write {
    async fn send_json(
        self: Box<Self>,
        method: WriteMethod,
        path: &[&str],
        query: &[(&str, String)],
        body: &Value,
    ) -> Result<HttpResponse> {
        self.sent.lock().unwrap().push((
            method,
            path.iter().map(|s| s.to_string()).collect(),
            query
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
            body.clone(),
        ));
        Ok(self.response)
    }
}
fn classify(outcome: WriteOutcome<Value>) -> String {
    match outcome {
        WriteOutcome::Applied(Ok(_)) => "applied".into(),
        WriteOutcome::Applied(Err(error)) => format!("applied, projection {:?}", error.code),
        WriteOutcome::Refused(error) => format!("refused {:?}", error.code),
        WriteOutcome::Unknown(error) => format!("unknown {:?}", error.code),
    }
}
fn stored() -> Value {
    json!({"kind": "calendar#event", "etag": ETAG, "id": "fixture-event-1", "status": "confirmed",
           "summary": "first", "start": {"dateTime": "2026-09-21T10:00:00+02:00"},
           "end": {"dateTime": "2026-09-21T11:00:00+02:00"},
           "attendees": [{"email": "caller@example.test", "self": true, "organizer": true},
                         {"email": "guest-a@example.test"}, {"email": "guest-b@example.test"}]})
}
fn inputs() -> [(&'static str, Value); 3] {
    [
        (
            "events.insert",
            json!({"calendarId": "primary", "sendUpdates": "none",
                   "body": {"summary": "Fixture review",
                            "start": {"dateTime": "2026-10-05T10:00:00+02:00"},
                            "end": {"dateTime": "2026-10-05T11:00:00+02:00"},
                            "attendees": [{"email": "guest-a@example.test"}]}}),
        ),
        (
            "events.patch",
            json!({"calendarId": "primary", "eventId": "fixture-event-1", "etag": ETAG,
                   "sendUpdates": "none", "body": {"summary": "first, moved"}}),
        ),
        (
            "events.delete",
            json!({"calendarId": "primary", "eventId": "fixture-event-1", "etag": ETAG,
                   "sendUpdates": "none"}),
        ),
    ]
}
/// Prepare `input` (a guarded write's preflight answers the stored, confirmed
/// event), send it through a capability answering `answer`, and return how it
/// was classified and what was sent; `Err` is a refusal in prepare.
async fn run(
    operation: &str,
    input: Value,
    answer: HttpResponse,
) -> (
    std::result::Result<String, ErrorCode>,
    Vec<(WriteMethod, Vec<String>, Vec<(String, String)>, Value)>,
) {
    let preflight = (operation != "events.insert").then(|| response(200, Some(stored())));
    let http = Reads {
        responses: Mutex::new(preflight.into_iter().collect()),
        calls: Mutex::new(Vec::new()),
    };
    let sent: Sent = Arc::default();
    let outcome = match engine().prepare(&http, INSTANCE, operation, input).await {
        Err(error) => Err(error.code),
        Ok(prepared) => Ok(classify(
            prepared
                .execute(Box::new(Write {
                    sent: sent.clone(),
                    response: answer,
                }))
                .await,
        )),
    };
    let sent = sent.lock().unwrap().clone();
    (outcome, sent)
}

/// Correction round 1 made `sendUpdates` required. The pinned document gives
/// the parameter exactly three values, but the declaration types it as any
/// string or integer and nothing compares it with them: an input whose
/// `sendUpdates` is the empty string, a miscased `NONE` or the integer `0`
/// passes the requirement and is sent as it was given. The engine records no
/// enumeration, so this pins today's behaviour and the guide sentence that
/// leaves the check to the issuer.
#[tokio::test]
async fn send_updates_outside_its_three_values_is_sent_as_given() {
    let sentence = "The issuer checks that it is one of `all`, `externalOnly` or `none` before \
                    approving.";
    assert!(
        guide().contains(sentence),
        "the guide no longer says the issuer checks the sendUpdates value: {sentence}"
    );
    let pinned = pinned();
    for method in ["insert", "patch", "delete"] {
        assert_eq!(
            pinned["resources"]["events"]["methods"][method]["parameters"]["sendUpdates"]["enum"],
            json!(["all", "externalOnly", "none"]),
            "`{method}`"
        );
    }
    let mut accepted = Vec::new();
    for (operation, base) in inputs() {
        for value in [json!(""), json!("NONE"), json!(0)] {
            let mut input = base.clone();
            input["sendUpdates"] = value.clone();
            let (outcome, sent) = run(operation, input, response(200, Some(stored()))).await;
            let verbatim = match &value {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            };
            let as_given = sent.len() == 1
                && sent[0]
                    .2
                    .contains(&("sendUpdates".to_owned(), verbatim.clone()));
            if outcome.as_deref() != Ok("applied") || !as_given {
                accepted.push(format!(
                    "{operation}: sendUpdates={value} {outcome:?}, sent query {:?}",
                    sent.first().map(|s| s.2.clone())
                ));
            }
        }
    }
    assert!(
        accepted.is_empty(),
        "a sendUpdates value outside all, externalOnly, none was not sent as given, which the \
         guide sentence \"{sentence}\" relies on: {accepted:#?}"
    );
}

/// A write that Google answers `429` is classified `unknown`, not a
/// `rate_limited` refusal (`adapters/catalog/src/lib.rs` `Prepared::execute`
/// refuses only 400, 401, 403, 404, 405, 409, 410, 412, 415 and 422), as
/// `docs/local-catalog-provider.md` says; a `403` naming a rate-limit reason is
/// a `rate_limited` refusal. The guide's Limits line states both.
#[tokio::test]
async fn a_write_google_answers_429_is_unknown_as_the_guide_says() {
    let line = "A write that Google answers `429` is reported `unknown`";
    assert!(
        guide().contains(line),
        "the guide's Limits line no longer says: {line}"
    );
    let body = |code: u16| {
        json!({"error": {"code": code, "message": "Rate Limit Exceeded",
                         "errors": [{"domain": "usageLimits", "reason": "rateLimitExceeded",
                                     "message": "Rate Limit Exceeded"}]}})
    };
    let mut classified = Vec::new();
    for (operation, input) in inputs() {
        for status in [403, 429] {
            let (outcome, sent) = run(
                operation,
                input.clone(),
                response(status, Some(body(status))),
            )
            .await;
            assert_eq!(sent.len(), 1, "`{operation}` {status}");
            let documented = if status == 429 {
                "unknown UpstreamProtocol"
            } else {
                "refused RateLimited"
            };
            if outcome.as_deref() != Ok(documented) {
                classified.push(format!(
                    "{operation}: {status} classified {outcome:?}, documented {documented}"
                ));
            }
        }
    }
    assert!(
        classified.is_empty(),
        "a rate-limited write is not classified as the guide's Limits line \"{line}\" \
         says: {classified:#?}"
    );
}

/// The new warning names `events.get` and `events.list` as the reads that
/// truncate the guest list, and "The answers" bullet tells a caller that insert
/// and patch return the stored event "with its new `etag` to pin next". The
/// pinned document gives `events.insert` and `events.patch` the same
/// `maxAttendees` with the same truncation, the declaration exposes it, and a
/// patch carrying it is sent and its truncated answer returned as `applied`.
/// A next patch whose `body.attendees` is built from that answer passes the
/// guard (its `etag` is the pin) and removes every other guest, and neither
/// bullet says so.
#[tokio::test]
async fn the_truncated_guest_list_warning_covers_the_answers_of_insert_and_patch() {
    let pinned = pinned();
    let truncates = "If there are more than the specified number of attendees, only the participant is returned.";
    for method in ["get", "list", "insert", "patch"] {
        let text = pinned["resources"]["events"]["methods"][method]["parameters"]["maxAttendees"]
            ["description"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains(truncates), "`events.{method}` maxAttendees");
    }
    let mut input = inputs()[1].1.clone();
    input["maxAttendees"] = json!(1);
    let mut truncated = stored();
    truncated["etag"] = json!("\"fixture-etag-3\"");
    truncated["attendees"] =
        json!([{"email": "caller@example.test", "self": true, "organizer": true}]);
    truncated["attendeesOmitted"] = json!(true);
    let (outcome, sent) = run("events.patch", input, response(200, Some(truncated))).await;
    assert_eq!(outcome.as_deref(), Ok("applied"));
    assert!(
        sent[0]
            .2
            .contains(&("maxAttendees".to_owned(), "1".to_owned())),
        "{:?}",
        sent[0].2
    );
    let guide = guide();
    assert!(guide.contains("with its new `etag` to pin next"));
    let warning = bullet(
        &guide,
        "**A truncated guest list.**",
        "**Recurring events.**",
    );
    let answers = bullet(&guide, "**The answers.**", "**Race boundary.**");
    assert!(
        (warning.contains("events.insert") && warning.contains("events.patch"))
            || answers.contains("maxAttendees"),
        "neither the truncated-guest-list warning nor the answers bullet says that an insert \
         or patch sent with maxAttendees answers a truncated guest list:\n{warning}\n{answers}"
    );
}

//! Adversary pass against the Google Calendar event writes: the guide's and the
//! selection descriptions' claims about what reaches guests, read against the
//! pinned Discovery document and driven through the engine against a scripted
//! provider. No network, no credential.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Effect, Engine, Selection};
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
fn doc(name: &str) -> String {
    fs::read_to_string(root().join("../../docs").join(name)).unwrap()
}
fn description(id: &str) -> String {
    shipped()
        .into_iter()
        .find(|s| s.id == id)
        .and_then(|s| s.description)
        .unwrap_or_default()
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
fn reads(responses: Vec<HttpResponse>) -> Reads {
    Reads {
        responses: Mutex::new(responses.into()),
        calls: Mutex::new(Vec::new()),
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
/// What the one-use write capability was asked to send: method, path, query,
/// body.
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
fn stored(status: &str) -> Value {
    json!({"kind": "calendar#event", "etag": ETAG, "id": "fixture-event-1", "status": status,
           "summary": "first", "start": {"dateTime": "2026-09-21T10:00:00+02:00"},
           "end": {"dateTime": "2026-09-21T11:00:00+02:00"},
           "attendees": [{"email": "caller@example.test", "self": true, "organizer": true},
                         {"email": "guest-a@example.test"}, {"email": "guest-b@example.test"}]})
}
fn patch_input() -> Value {
    json!({"calendarId": "primary", "eventId": "fixture-event-1", "etag": ETAG,
           "sendUpdates": "none", "body": {"summary": "first, moved"}})
}
fn delete_input() -> Value {
    json!({"calendarId": "primary", "eventId": "fixture-event-1", "etag": ETAG,
           "sendUpdates": "none"})
}
fn insert_input() -> Value {
    json!({"calendarId": "primary", "sendUpdates": "none",
           "body": {"summary": "Fixture review",
                    "start": {"dateTime": "2026-10-05T10:00:00+02:00"},
                    "end": {"dateTime": "2026-10-05T11:00:00+02:00"},
                    "attendees": [{"email": "guest-a@example.test"}]}})
}
/// Prepare `input` against a preflight answering `preflight` (when the write is
/// guarded), send it through a capability answering `answer`, and return what
/// was sent and how it was classified; `Err` is a refusal in prepare.
async fn run(
    operation: &str,
    input: Value,
    preflight: Option<HttpResponse>,
    answer: HttpResponse,
) -> (
    std::result::Result<String, ErrorCode>,
    Vec<(WriteMethod, Vec<String>, Vec<(String, String)>, Value)>,
    usize,
) {
    let http = reads(preflight.into_iter().collect());
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
    let reads = http.calls.lock().unwrap().len();
    let sent = sent.lock().unwrap().clone();
    (outcome, sent, reads)
}

/// The guide: "An event the preflight cannot find — a `404`, or a `410` for an
/// event Google has deleted — is refused with no write sent." The pinned
/// document says the opposite of the `410` half: a deleted event is
/// `status: cancelled`, and "The get method always returns them" — a `200`
/// with an `etag`. `events.list` returns them too on the incremental sync the
/// guide teaches (`syncToken`) and with `showDeleted`, so a pin read there is a
/// deleted event's `etag`. The preflight compares `/etag` only, so a patch or a
/// delete of a deleted event passes the guard and is sent.
///
/// Correction round 1: the guard now also requires `/status` to be the literal
/// `confirmed`, and the guide's claim under test is the one that says so.
#[tokio::test]
async fn a_write_to_an_event_google_deleted_sends_no_write() {
    let guide = flat(&doc("catalog-google-calendar.md"));
    assert!(
        guide.contains(
            "The `status` check refuses a patch or delete of a cancelled event with no write sent."
        ),
        "the guide no longer makes the claim under test"
    );
    let pinned = pinned();
    let status = pinned["schemas"]["Event"]["properties"]["status"]["description"]
        .as_str()
        .unwrap();
    assert!(status.contains("\"cancelled\" - The event is cancelled (deleted)."));
    assert!(status.contains("The get method always returns them."));
    let mut written = Vec::new();
    for (operation, input, answer) in [
        (
            "events.patch",
            patch_input(),
            response(200, Some(stored("cancelled"))),
        ),
        // Google's answer to a delete of an event it already deleted.
        (
            "events.delete",
            delete_input(),
            response(
                410,
                Some(
                    json!({"error": {"code": 410, "message": "Resource has been deleted",
                                  "errors": [{"domain": "global", "reason": "deleted"}]}}),
                ),
            ),
        ),
    ] {
        let (outcome, sent, reads) = run(
            operation,
            input,
            Some(response(200, Some(stored("cancelled")))),
            answer,
        )
        .await;
        assert_eq!(reads, 1, "`{operation}` preflight");
        if !sent.is_empty() {
            written.push(format!(
                "{operation}: sent {:?} {}, classified {outcome:?}",
                sent[0].0,
                sent[0].1.join("/")
            ));
        }
    }
    assert!(
        written.is_empty(),
        "a write was sent to an event Google reports as deleted: {written:#?}"
    );
}

/// Each write description says "name it explicitly" of `sendUpdates`, and the
/// guide says "Name it in every write input" and that the pinned document gives
/// no default for patch and delete. The declaration an agent reads from
/// `operations describe` does not require it, and an input without it is
/// prepared and sent with no `sendUpdates` at all — so an approved input does
/// not say whom Google emails; Google's unstated default does.
#[tokio::test]
async fn a_write_without_send_updates_is_refused_before_any_request() {
    let guide = flat(&doc("catalog-google-calendar.md"));
    assert!(guide.contains("Name it in every write input."));
    let engine = engine();
    let declarations = engine.declarations(&[Effect::Write]);
    let mut accepted = Vec::new();
    for (operation, mut input) in [
        ("events.insert", insert_input()),
        ("events.patch", patch_input()),
        ("events.delete", delete_input()),
    ] {
        assert!(
            description(operation).contains("name it explicitly"),
            "`{operation}`"
        );
        let declaration = declarations.iter().find(|o| o.id == operation).unwrap();
        let required = declaration.input_schema["required"].as_array().unwrap();
        if !required.contains(&json!("sendUpdates")) {
            accepted.push(format!(
                "{operation}: declared required {required:?} omits sendUpdates"
            ));
        }
        input.as_object_mut().unwrap().remove("sendUpdates");
        let preflight =
            (operation != "events.insert").then(|| response(200, Some(stored("confirmed"))));
        let (outcome, sent, _) = run(
            operation,
            input,
            preflight,
            response(200, Some(stored("confirmed"))),
        )
        .await;
        if outcome != Err(ErrorCode::InvalidInput) || !sent.is_empty() {
            accepted.push(format!(
                "{operation}: without sendUpdates {outcome:?}, sent query {:?}",
                sent.first().map(|s| s.2.clone())
            ));
        }
    }
    assert!(
        accepted.is_empty(),
        "a write that does not name sendUpdates: {accepted:#?}"
    );
}

/// Nothing refuses an input naming `sendUpdates: none` and
/// `sendNotifications: true`: it is sent with both, and which one Google obeys
/// is not stated by the pinned document. A selection cannot make two
/// parameters exclusive without an engine change, so this pins today's
/// behaviour and the guide sentence that tells the reader so.
#[tokio::test]
async fn a_write_naming_both_notification_parameters_sends_both() {
    let guide = flat(&doc("catalog-google-calendar.md"));
    let sentence = "nothing refuses an input that names both: both are sent, and the pinned \
                    document does not say which one Google obeys when they disagree. Do not \
                    send both.";
    assert!(
        guide.contains(sentence),
        "the guide no longer says that both notification parameters are sent: {sentence}"
    );
    let mut both = Vec::new();
    for (operation, mut input) in [
        ("events.insert", insert_input()),
        ("events.patch", patch_input()),
        ("events.delete", delete_input()),
    ] {
        input["sendNotifications"] = json!(true);
        let preflight =
            (operation != "events.insert").then(|| response(200, Some(stored("confirmed"))));
        let (outcome, sent, _) = run(
            operation,
            input,
            preflight,
            response(200, Some(stored("confirmed"))),
        )
        .await;
        let expected = vec![
            ("sendNotifications".to_owned(), "true".to_owned()),
            ("sendUpdates".to_owned(), "none".to_owned()),
        ];
        if outcome != Ok("applied".into()) || sent.len() != 1 || sent[0].2 != expected {
            both.push(format!(
                "{operation}: {outcome:?} with query {:?}",
                sent.first().map(|s| s.2.clone())
            ));
        }
    }
    assert!(
        both.is_empty(),
        "an input naming both notification parameters was not sent with both, as the guide \
         sentence \"{sentence}\" says: {both:#?}"
    );
}

/// `events.get` and `events.list` both take `maxAttendees`, and the pinned
/// document says an event read with it lists "only the participant" and sets
/// `attendeesOmitted`. The pin is the event's `etag`, which the preflight reads
/// in full and compares; a truncated read does not change what it compares.
/// So a patch whose `body.attendees` was built from such a read passes the
/// guard and replaces the whole guest list with the caller alone. Neither the
/// patch description an agent reads nor the guide names `maxAttendees` or
/// `attendeesOmitted`.
#[tokio::test]
async fn patch_warns_that_a_truncated_guest_list_replaces_every_guest() {
    let pinned = pinned();
    let methods = &pinned["resources"]["events"]["methods"];
    for method in ["get", "list"] {
        let limit = methods[method]["parameters"]["maxAttendees"]["description"]
            .as_str()
            .unwrap();
        assert!(
            limit.contains("only the participant is returned"),
            "`{method}`"
        );
    }
    assert!(
        pinned["schemas"]["Event"]["properties"]["attendeesOmitted"]["description"]
            .as_str()
            .unwrap()
            .contains("maxAttendee query parameter")
    );
    // The participant-only read of the stored event, pinned and patched back.
    let mut input = patch_input();
    input["body"] = json!({"attendees": [{"email": "caller@example.test"}]});
    let (outcome, sent, _) = run(
        "events.patch",
        input,
        Some(response(200, Some(stored("confirmed")))),
        response(200, Some(stored("confirmed"))),
    )
    .await;
    assert_eq!(outcome, Ok("applied".into()), "the guard passes");
    assert_eq!(
        sent[0].3["attendees"],
        json!([{"email": "caller@example.test"}])
    );
    let guide = doc("catalog-google-calendar.md");
    let patch = description("events.patch");
    let named: Vec<&str> = ["maxAttendees", "attendeesOmitted"]
        .into_iter()
        .filter(|term| patch.contains(term) || guide.contains(term))
        .collect();
    assert!(
        !named.is_empty(),
        "neither the events.patch description nor the guide warns that a guest list read with maxAttendees is truncated"
    );
}

/// Every Google configuration in the Calendar guide — the read instance and
/// the write instance — names one authorization endpoint,
/// `https://accounts.google.com/o/oauth2/auth`. Only the Calendar guide is
/// checked in this tree: the Drive, Slides and catalog provider guides are
/// moved to the same endpoint on the integration branch (34dab8a31), not on
/// this unit's base.
#[test]
fn every_google_guide_names_one_authorize_url() {
    let mut urls = Vec::new();
    for line in doc("catalog-google-calendar.md").lines() {
        if let Some((_, rest)) = line.split_once("\"authorize_url\": \"")
            && rest.starts_with("https://accounts.google.com/")
        {
            urls.push(rest.split('"').next().unwrap().to_owned());
        }
    }
    assert_eq!(
        urls.len(),
        2,
        "the read and the write configuration: {urls:?}"
    );
    assert!(
        urls.iter()
            .all(|url| url == "https://accounts.google.com/o/oauth2/auth"),
        "the Calendar guide names another authorize_url than \
         https://accounts.google.com/o/oauth2/auth (only this guide is checked here; the \
         other Google guides are fixed on the integration branch at 34dab8a31): {urls:#?}"
    );
}

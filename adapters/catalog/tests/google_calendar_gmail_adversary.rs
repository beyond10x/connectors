//! Adversary pass against the Google Calendar and Gmail read selections: the
//! guides and the selection descriptions an agent reads through
//! `operations describe`, checked against the pinned Discovery documents and
//! driven through the engine against a scripted provider. No network, no
//! credential.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{collections::VecDeque, path::Path, sync::Mutex};

type Call = (Vec<String>, Vec<(String, String)>);

struct Scripted {
    responses: Mutex<VecDeque<HttpResponse>>,
    calls: Mutex<Vec<Call>>,
}
#[async_trait::async_trait]
impl AuthenticatedHttp for Scripted {
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
fn scripted(status: u16, body: &Value) -> Scripted {
    Scripted {
        responses: Mutex::new(VecDeque::from([HttpResponse {
            status,
            headers: [(
                "content-type".to_owned(),
                "application/json; charset=UTF-8".to_owned(),
            )]
            .into(),
            body: serde_json::to_vec(body).unwrap(),
        }])),
        calls: Mutex::new(Vec::new()),
    }
}
fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn operations_file(provider: &str) -> Value {
    serde_json::from_slice(
        &std::fs::read(root().join(format!("providers/{provider}/operations.json"))).unwrap(),
    )
    .unwrap()
}
fn shipped(provider: &str) -> Vec<Selection> {
    serde_json::from_value(operations_file(provider)["operations"].clone()).unwrap()
}
fn engine(provider: &str, base: &str) -> Engine {
    let bundle = bundle::load(&root().join("generated/bundles"), provider).unwrap();
    Engine::new(&bundle, base, &shipped(provider)).unwrap()
}
fn calendar() -> Engine {
    engine("google-calendar", "/calendar/v3")
}
fn gmail() -> Engine {
    engine("google-gmail", "")
}
fn guide(name: &str) -> String {
    std::fs::read_to_string(root().join(format!("../../docs/{name}"))).unwrap()
}
/// The paragraph of `guide` that starts with `lead`.
fn paragraph(guide: &str, lead: &str) -> String {
    let start = guide.find(lead).unwrap_or_else(|| panic!("no `{lead}`"));
    let rest = &guide[start..];
    rest[..rest.find("\n\n").unwrap_or(rest.len())].to_owned()
}
fn description(provider: &str, id: &str) -> String {
    operations_file(provider)["operations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["id"] == id)
        .unwrap()["description"]
        .as_str()
        .unwrap()
        .to_owned()
}

/// The Gmail reset rule orders the steps: "do a full sync: walk
/// `users.messages.list` … from the first page, read what is needed, and take a
/// new baseline from `users.getProfile`" (and the `users.history.list`
/// description: "do a full sync and take a new baseline from
/// users.getProfile"). A baseline read after the walk is newer than the state
/// the walk saw: a message that arrives or is relabelled while the walk runs is
/// in neither the walk nor any later `users.history.list` from that baseline,
/// so the mirror silently loses it. The baseline has to be taken before the
/// walk starts (or from the newest message the walk returns), so a change
/// during the walk is replayed by the next delta.
#[test]
fn gmail_reset_rule_takes_the_baseline_before_the_full_walk() {
    let mut late = Vec::new();
    let rule = paragraph(&guide("catalog-google-gmail.md"), "**Reset rule.**");
    let history = description("google-gmail", "users.history.list");
    for (source, text) in [
        ("docs/catalog-google-gmail.md reset rule", rule.as_str()),
        ("users.history.list description", history.as_str()),
    ] {
        let walk = text
            .find("full sync")
            .unwrap_or_else(|| panic!("{source}: no full sync"));
        let baseline = text
            .find("users.getProfile")
            .unwrap_or_else(|| panic!("{source}: no baseline"));
        if baseline > walk {
            late.push(format!("{source}: {text}"));
        }
    }
    assert!(
        late.is_empty(),
        "the baseline is taken after the full walk, so changes during the walk are lost: \
         {late:#?}"
    );
}

/// Drive's paging selections say "that end condition holds only if fields,
/// when given, includes nextPageToken, so include it", because a `fields` that
/// omits the token ends the walk silently after one page. The Calendar and
/// Gmail lists declare the same document-wide `fields` (the guides: "accepted
/// by name, including the document-wide `fields`"), end on the same absent
/// token, and — for Calendar — hand over the delta cursor in `nextSyncToken`,
/// which such a `fields` drops as well. The Gmail guide even recommends
/// `fields` to narrow an answer. An agent reading `operations describe` for
/// these lists is not told.
#[test]
fn every_google_paging_description_warns_that_fields_must_keep_the_end_condition() {
    let mut silent = Vec::new();
    for (provider, engine) in [
        ("google-drive", engine("google-drive", "/drive/v3")),
        ("google-calendar", calendar()),
        ("google-gmail", gmail()),
    ] {
        for declaration in engine.declarations(&[Effect::Read]) {
            let paged = declaration.description.contains("nextPageToken");
            let narrowed = declaration.input_schema["properties"]
                .get("fields")
                .is_some();
            if paged && narrowed && !declaration.description.contains("fields") {
                silent.push(format!("{provider} {}", declaration.id));
            }
        }
    }
    assert!(
        silent.is_empty(),
        "paging reads that accept `fields` without saying it must keep the end \
         condition: {silent:#?}"
    );
}

/// Calendar's and Gmail's published error guides both list a `403` whose
/// `error.errors[].reason` is `dailyLimitExceeded` (domain `usageLimits`) as
/// the answer to an exhausted project quota, beside `rateLimitExceeded` and
/// `userRateLimitExceeded`. It is a quota, not a permission, and resumable
/// later; the selections name only the other two, so it reaches the caller as
/// `forbidden` — the answer the guides reserve for "every other `403`", and
/// the one an agent reads as a missing scope.
#[tokio::test]
async fn a_daily_quota_403_is_rate_limited_not_forbidden() {
    let body = json!({"error": {"code": 403, "message": "Daily Limit Exceeded",
        "errors": [{"domain": "usageLimits", "reason": "dailyLimitExceeded",
                    "message": "Daily Limit Exceeded"}]}});
    let mut codes = Vec::new();
    let http = scripted(403, &body);
    let refusal = gmail()
        .read(
            &http,
            "fixture-google-gmail",
            "users.messages.list",
            json!({"userId": "me"}),
        )
        .await
        .expect_err("a 403 is a refusal");
    codes.push(("users.messages.list", refusal.code));
    let http = scripted(403, &body);
    let refusal = calendar()
        .read(
            &http,
            "fixture-google-calendar",
            "events.list",
            json!({"calendarId": "primary"}),
        )
        .await
        .expect_err("a 403 is a refusal");
    codes.push(("events.list", refusal.code));
    assert!(
        codes
            .iter()
            .all(|(_, code)| *code == ErrorCode::RateLimited),
        "{codes:?}"
    );
}

/// A `410` `fullSyncRequired` (an expired sync token) and a `404` (a
/// `calendarId` that does not exist or is no longer shared) reach the caller as
/// the same code with the same message; the engine maps both to `not_found`.
/// Telling them apart is an engine change. Until then the `events.list`
/// description and the guide's reset rule say that a `not_found` on a
/// sync-token request means either, and that the caller re-checks
/// `calendarList.list` before a full resync. This case pins today's identical
/// answers and that sentence, so the day they differ is seen.
#[tokio::test]
async fn an_expired_sync_token_and_an_unknown_calendar_are_documented_as_one_answer() {
    let expired = json!({"error": {"code": 410,
        "message": "Sync token is no longer valid, a full sync is required.",
        "errors": [{"domain": "calendar", "reason": "fullSyncRequired",
                    "message": "Sync token is no longer valid, a full sync is required."}]}});
    let missing = json!({"error": {"code": 404, "message": "Not Found",
        "errors": [{"domain": "global", "reason": "notFound", "message": "Not Found"}]}});
    let mut seen = Vec::new();
    for (status, body) in [(410, &expired), (404, &missing)] {
        let http = scripted(status, body);
        let refusal = calendar()
            .read(
                &http,
                "fixture-google-calendar",
                "events.list",
                json!({"calendarId": "fixture-team@group.example.test",
                       "syncToken": "fixture-events-sync-1"}),
            )
            .await
            .expect_err("a refusal");
        seen.push((refusal.code, refusal.message.clone()));
    }
    assert_eq!(
        seen[0], seen[1],
        "a 410 fullSyncRequired and a 404 notFound now reach the caller differently: \
         revise the sentence \"either the sync token expired or the calendar is gone\" in \
         docs/catalog-google-calendar.md (Reset rule) and the events.list description"
    );
    assert_eq!(seen[0].0, ErrorCode::NotFound);
    let rule = paragraph(&guide("catalog-google-calendar.md"), "**Reset rule.**");
    let events = description("google-calendar", "events.list");
    for (source, text) in [
        ("docs/catalog-google-calendar.md reset rule", rule.as_str()),
        ("events.list description", events.as_str()),
    ] {
        assert!(
            text.contains("either the sync token expired or the calendar is gone")
                && text.contains("calendarList.list"),
            "{source} does not say that not_found on a sync-token request means either an \
             expired token or a gone calendar, re-checked with calendarList.list: {text}"
        );
    }
}

/// The pinned Calendar document, on `syncToken`: "it is not allowed to set
/// showDeleted to False" for `events.list`, and "not allowed to set
/// showDeleted neither showHidden to False" for `calendarList.list`. The
/// guide lists the parameters that "cannot be sent beside `syncToken`" and
/// tells the caller to send the token "with the same other parameters"; a walk
/// made with `showDeleted: false` then has a delta Google refuses, and the
/// guide does not say so.
#[test]
fn the_calendar_sync_token_section_names_the_show_deleted_restriction() {
    let guide = guide("catalog-google-calendar.md");
    let start = guide.find("## Sync tokens").unwrap();
    let rest = &guide[start + 3..];
    let section = &rest[..rest.find("\n## ").unwrap()];
    let mut missing = Vec::new();
    for term in ["`showDeleted`", "`showHidden`"] {
        if !section.contains(term) {
            missing.push(term);
        }
    }
    let pinned: Value = serde_json::from_slice(
        &std::fs::read(root().join("../google/upstream/calendar/calendar-api.json")).unwrap(),
    )
    .unwrap();
    assert!(
        pinned["resources"]["events"]["methods"]["list"]["parameters"]["syncToken"]["description"]
            .as_str()
            .unwrap()
            .contains("not allowed to set showDeleted to False")
    );
    assert!(
        missing.is_empty(),
        "the Sync tokens section omits {missing:?}"
    );
}

/// The pinned Calendar document declares `maxAttendees` with `"minimum": "1"`
/// on `events.list` and `events.get`, and the projection carries it. A
/// selection bound needs a `maximum`, which the pinned document does not give,
/// so the declared input schema an agent reads has no minimum and `0` is sent.
/// Until the engine carries a minimum-only bound, the guide says so: `0` is
/// sent and Google answers `400`. This case pins today's state and that
/// sentence, so the day the minimum is declared is seen.
#[tokio::test]
async fn max_attendees_below_its_discovery_minimum_is_documented_as_sent() {
    let declarations = calendar().declarations(&[Effect::Read]);
    let mut unbounded = Vec::new();
    for id in ["events.list", "events.get"] {
        let schema = &declarations
            .iter()
            .find(|o| o.id == id)
            .unwrap()
            .input_schema["properties"]["maxAttendees"];
        if schema["minimum"] != json!(1) {
            unbounded.push(format!("{id}: {schema}"));
        }
    }
    let http = scripted(
        200,
        &json!({"kind": "calendar#event", "id": "fixture-event-1"}),
    );
    let sent = calendar()
        .read(
            &http,
            "fixture-google-calendar",
            "events.get",
            json!({"calendarId": "primary", "eventId": "fixture-event-1", "maxAttendees": 0}),
        )
        .await
        .is_ok();
    assert!(
        unbounded.len() == 2 && sent,
        "maxAttendees now carries its Discovery minimum (declared without one: \
         {unbounded:#?}, 0 sent={sent}): revise the sentence \"`maxAttendees` 0 is sent\" in \
         docs/catalog-google-calendar.md"
    );
    let calendar_guide = guide("catalog-google-calendar.md");
    assert!(
        calendar_guide.contains("`maxAttendees` 0 is sent") && calendar_guide.contains("`400`"),
        "docs/catalog-google-calendar.md does not say that `maxAttendees` 0 is sent and Google \
         answers `400`"
    );
}

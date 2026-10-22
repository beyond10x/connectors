//! The time position's `utc_offsets` (`contracts/catalog/v1alpha1/semantics.md` §3.3,
//! Watermark: `time`): a provider reading `{since}` in a zone between the declared offsets. The
//! cases drive the generic query fixture in `tests/feed/` with an in-process provider; no live
//! provider is called.
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use connectors_catalog::{bundle::Bundle, ingest, inventory};
use connectors_catalog_provider::{
    Engine,
    feed::{Declaration, ITEMS},
};
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

const SERVER: &str = "https://feed.fixture.test/v1";
const INSTANCE: &str = "offsets-instance";
const QUERY: &str = "time-query.operations.json";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/feed")
}

fn selection() -> Value {
    serde_json::from_slice(&fs::read(fixtures().join(QUERY)).unwrap()).unwrap()
}

/// The query fixture's feed with `utc_offsets` set to `offsets`.
fn offset(offsets: Value) -> Value {
    let mut feed = selection()["feed"].clone();
    feed["items"]["position"]["value"]["query"]["utc_offsets"] = offsets;
    feed
}

fn bundle() -> Bundle {
    let bytes = fs::read(fixtures().join("api.json")).unwrap();
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    Bundle {
        provider: selection()["provider"].as_str().unwrap().into(),
        source: ingest("api.json", &bytes).unwrap(),
        inventory: inventory::extract(&document),
        auth_profile: "fixture.token".into(),
    }
}

fn build(feed: &Value) -> Result<Engine> {
    let feed: Declaration = serde_json::from_value(feed.clone()).unwrap();
    Engine::with_feed(&bundle(), "/v1", &[], Some(&feed))?.with_link_origin(SERVER)
}

type Call = (Vec<String>, Vec<(String, String)>);

/// A provider admitting any room and answering every search with `listing`.
struct Provider {
    listing: Value,
    calls: Mutex<Vec<Call>>,
}

impl Provider {
    fn new(listing: Value) -> Self {
        Self {
            listing,
            calls: Mutex::new(Vec::new()),
        }
    }

    fn searches(&self) -> Vec<Call> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(path, _)| path.first().map(String::as_str) != Some("rooms"))
            .cloned()
            .collect()
    }
}

#[async_trait::async_trait]
impl AuthenticatedHttp for Provider {
    async fn get(&self, path: &[&str], query: &[(&str, String)]) -> Result<HttpResponse> {
        let path: Vec<String> = path.iter().map(|s| s.to_string()).collect();
        let body = if path.first().map(String::as_str) == Some("rooms") {
            json!({"room": {"id": "room1", "title": "Room", "type": "room", "access": "open"}})
        } else {
            self.listing.clone()
        };
        self.calls.lock().unwrap().push((
            path,
            query
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        ));
        Ok(HttpResponse {
            status: 200,
            headers: Default::default(),
            body: serde_json::to_vec(&body).unwrap(),
        })
    }
}

fn record(id: &str, at: &str) -> Value {
    json!({"id": id, "fields": {"version": "1", "created": at, "updated": at, "text": "body"}})
}

fn listing(records: Vec<Value>) -> Value {
    json!({"issues": records})
}

/// A watermark in the engine's own format at `time`, holding nothing seen.
fn watermark(time: &str) -> String {
    let mark = json!({"format": "catalog-feed/1", "profile": "feed-fixture-query/1",
                      "instance": INSTANCE, "container": "room1", "time": time});
    URL_SAFE_NO_PAD.encode(serde_json::to_vec(&mark).unwrap())
}

fn resume(engine: &Engine, http: &Provider, time: &str, limit: u64) -> Result<Value> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(engine.read(
            http,
            INSTANCE,
            ITEMS,
            json!({"container": "room1", "limit": limit, "watermark": watermark(time)}),
        ))
}

fn sent<'a>(call: &'a Call, name: &str) -> Option<&'a str> {
    call.1
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
}

fn ids(page: &Value) -> Vec<&str> {
    page["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["id"].as_str().unwrap())
        .collect()
}

/// `least` and `greatest` lie within a day of UTC, `least` is not above `greatest`, the query
/// is read further by `page`, and the field holds nothing else.
#[test]
fn utc_offsets_load_within_a_day_in_order_and_only_with_a_page() {
    for loads in [
        json!({"least": -720, "greatest": 840}),
        json!({"least": -1439, "greatest": 1439}),
        json!({"least": 0, "greatest": 0}),
        json!({"least": 330, "greatest": 330}),
    ] {
        assert!(build(&offset(loads.clone())).is_ok(), "{loads}");
    }
    for refused in [
        json!({"least": -1440, "greatest": 0}),
        json!({"least": 0, "greatest": 1440}),
        json!({"least": 60, "greatest": -60}),
    ] {
        let error = build(&offset(refused.clone())).err().unwrap();
        assert!(
            error.message.contains("`utc_offsets`"),
            "{refused}: {error:?}"
        );
    }
    let mut unpaged = offset(json!({"least": -720, "greatest": 840}));
    unpaged["items"]["position"]["value"]
        .as_object_mut()
        .unwrap()
        .remove("page");
    let error = build(&unpaged).err().unwrap();
    assert!(error.message.contains("`page`"), "{error:?}");
    for strict in [
        json!({"least": -720, "greatest": 840, "zone": "UTC"}),
        json!({"least": -720}),
        json!({"greatest": 840}),
        json!({"least": "-12:00", "greatest": 840}),
    ] {
        assert!(
            serde_json::from_value::<Declaration>(offset(strict.clone())).is_err(),
            "{strict}"
        );
    }
}

/// The field is optional and round-trips; a declaration without it serializes without it.
#[test]
fn utc_offsets_are_optional_and_round_trip() {
    for feed in [
        selection()["feed"].clone(),
        offset(json!({"least": -720, "greatest": 840})),
    ] {
        let declaration: Declaration = serde_json::from_value(feed.clone()).unwrap();
        assert_eq!(serde_json::to_value(&declaration).unwrap(), feed);
    }
}

/// Without `utc_offsets` a resume sends the kept UTC minute and asks for the limit plus the
/// items the watermark holds; with them it sends the minute `least` before and full pages.
#[test]
fn a_widened_resume_sends_the_least_offset_minute_and_asks_for_full_pages() {
    for (feed, since, requested) in [
        (selection()["feed"].clone(), "2026-10-06 09:00", "10"),
        (
            offset(json!({"least": -120, "greatest": 180})),
            "2026-10-06 07:00",
            "100",
        ),
        (
            offset(json!({"least": 90, "greatest": 180})),
            "2026-10-06 10:30",
            "100",
        ),
    ] {
        let engine = build(&feed).unwrap();
        let http = Provider::new(listing(vec![]));
        resume(&engine, &http, "2026-10-06T09:00:30Z", 10).unwrap();
        let searches = http.searches();
        assert_eq!(
            sent(&searches[0], "q"),
            Some(format!(r#"room = "room1" AND edited >= "{since}" ORDER BY edited ASC"#).as_str())
        );
        assert_eq!(sent(&searches[0], "maxResults"), Some(requested));
    }
}

/// What a widened filter answers before the kept minute is the watermark's and is dropped; the
/// order is checked from the earliest start the offsets allow, `least - greatest` before the
/// kept minute, and an answer before that start is `unavailable`.
#[test]
fn a_widened_resume_drops_what_precedes_the_kept_minute_and_checks_order_from_the_earliest_start() {
    let engine = build(&offset(json!({"least": -120, "greatest": 180}))).unwrap();
    let http = Provider::new(listing(vec![
        record("early", "2026-10-06T04:00:00Z"),
        record("before", "2026-10-06T08:59:59Z"),
        record("kept", "2026-10-06T09:00:10Z"),
        record("after", "2026-10-06T09:10:00Z"),
    ]));
    let page = resume(&engine, &http, "2026-10-06T09:00:30Z", 10).unwrap();
    assert_eq!(ids(&page), ["kept", "after"]);
    assert_eq!(page["complete"], json!(true));

    let http = Provider::new(listing(vec![
        record("too-early", "2026-10-06T03:59:59Z"),
        record("kept", "2026-10-06T09:00:10Z"),
    ]));
    let error = resume(&engine, &http, "2026-10-06T09:00:30Z", 10).unwrap_err();
    assert_eq!(error.code, ErrorCode::Unavailable);

    // Unwidened, the same early answer is before the filter's start.
    let engine = build(&selection()["feed"]).unwrap();
    let http = Provider::new(listing(vec![
        record("before", "2026-10-06T08:59:59Z"),
        record("kept", "2026-10-06T09:00:10Z"),
    ]));
    let error = resume(&engine, &http, "2026-10-06T09:00:30Z", 10).unwrap_err();
    assert_eq!(error.code, ErrorCode::Unavailable);
}

/// The literal keeps the `YYYY-MM-DD HH:MM` shape after the shift: a watermark whose shifted
/// minute leaves the years 0000 to 9999 is stale before any search.
#[test]
fn a_shift_past_the_literal_years_is_a_stale_watermark() {
    for (offsets, time) in [
        (
            json!({"least": -120, "greatest": 0}),
            "0000-01-01T01:00:00Z",
        ),
        (
            json!({"least": 120, "greatest": 120}),
            "9999-12-31T23:00:00Z",
        ),
    ] {
        let engine = build(&offset(offsets.clone())).unwrap();
        let http = Provider::new(listing(vec![]));
        let error = resume(&engine, &http, time, 10).unwrap_err();
        assert_eq!(error.code, ErrorCode::StaleCursor, "{offsets} {time}");
        assert!(http.searches().is_empty(), "{offsets} {time}");
    }
    let engine = build(&selection()["feed"]).unwrap();
    let http = Provider::new(listing(vec![]));
    resume(&engine, &http, "0000-01-01T01:00:00Z", 10).unwrap();
    assert_eq!(
        sent(&http.searches()[0], "q"),
        Some(r#"room = "room1" AND edited >= "0000-01-01 01:00" ORDER BY edited ASC"#)
    );
}

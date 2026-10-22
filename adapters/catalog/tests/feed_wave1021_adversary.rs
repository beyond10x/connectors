//! Adversarial pass over wave 20261021: the year bound of the `{since}` literal against the
//! family's admission order (`contracts/catalog/v1alpha1/semantics.md` §3.3, Watermark scope:
//! "`stale_cursor` is answered only when that listing is answered, its records discarded, and a
//! container the lookup or the listing refuses answers as the Errors row says, whatever the
//! watermark"). The cases drive the fixture declarations in `tests/feed/` with an in-process
//! provider; no live provider is called.
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
const INSTANCE: &str = "adversary-instance";
const QUERY: &str = "time-query.operations.json";
const NEXT: &str = "time-next.operations.json";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/feed")
}

fn selection(name: &str) -> Value {
    serde_json::from_slice(&fs::read(fixtures().join(name)).unwrap()).unwrap()
}

/// The query fixture's feed with `utc_offsets` set to `offsets`.
fn offset(offsets: Value) -> Value {
    let mut feed = selection(QUERY)["feed"].clone();
    feed["items"]["position"]["value"]["query"]["utc_offsets"] = offsets;
    feed
}

fn bundle() -> Bundle {
    let bytes = fs::read(fixtures().join("api.json")).unwrap();
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    Bundle {
        provider: selection(QUERY)["provider"].as_str().unwrap().into(),
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

/// A provider whose container lookup admits `room1` and whose item listing answers `status`
/// with `listing`.
struct Provider {
    status: u16,
    listing: Value,
    calls: Mutex<Vec<Call>>,
}

impl Provider {
    fn new(status: u16, listing: Value) -> Self {
        Self {
            status,
            listing,
            calls: Mutex::new(Vec::new()),
        }
    }

    fn listings(&self) -> Vec<Call> {
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
        let (status, body) = if path.first().map(String::as_str) == Some("rooms") {
            (
                200,
                json!({"room": {"id": "room1", "title": "Room", "type": "room", "access": "open"}}),
            )
        } else {
            (self.status, self.listing.clone())
        };
        self.calls.lock().unwrap().push((
            path,
            query
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        ));
        Ok(HttpResponse {
            status,
            headers: Default::default(),
            body: serde_json::to_vec(&body).unwrap(),
        })
    }
}

/// A watermark in the engine's own format for `room1` at `time`, as any caller can write one.
fn mark(profile: &str, time: &str) -> String {
    let mark = json!({"format": "catalog-feed/1", "profile": profile, "instance": INSTANCE,
                      "container": "room1", "time": time});
    URL_SAFE_NO_PAD.encode(serde_json::to_vec(&mark).unwrap())
}

fn read(engine: &Engine, http: &Provider, watermark: &str) -> Result<Value> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(engine.read(
            http,
            INSTANCE,
            ITEMS,
            json!({"container": "room1", "limit": 10, "watermark": watermark}),
        ))
}

fn sent<'a>(call: &'a Call, name: &str) -> Option<&'a str> {
    call.1
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
}

/// Each fixture feed, the parameter its query is sent in, and a watermark time whose `{since}`
/// literal (after the declared shift) leaves the UTC years 0000 to 9999, though the instant
/// itself is one the engine opens.
fn year_bound_cases() -> Vec<(&'static str, Value, &'static str, &'static str)> {
    let query = selection(QUERY)["feed"].clone();
    let next = selection(NEXT)["feed"].clone();
    vec![
        ("query", query.clone(), "q", "0000-01-01T00:30:00+01:00"),
        ("query", query, "q", "9999-12-31T23:59:00-01:00"),
        (
            "query, utc_offsets -120..0",
            offset(json!({"least": -120, "greatest": 0})),
            "q",
            "0000-01-01T01:00:00Z",
        ),
        (
            "query, utc_offsets 120..120",
            offset(json!({"least": 120, "greatest": 120})),
            "q",
            "9999-12-31T23:00:00Z",
        ),
        ("next", next.clone(), "filter", "0000-01-01T00:30:00+01:00"),
        ("next", next, "filter", "9999-12-31T23:59:00-01:00"),
    ]
}

/// The lookup answers `room1`, so the connection sees it, but the provider refuses its items
/// with a 403: the family answers that `not_found`, exactly as an unknown room, "whatever the
/// watermark". A caller-written watermark whose `{since}` would leave the literal's years must
/// not turn that into `stale_cursor`, which tells the caller the container's listing was never
/// the deciding answer.
#[test]
fn adversary_a_year_bound_watermark_on_a_container_whose_listing_refuses_is_not_found() {
    let refusal = json!({"message": "403 Forbidden"});
    let mut disclosed = Vec::new();
    for (name, feed, _, time) in year_bound_cases() {
        let engine = build(&feed).unwrap();
        let profile = feed["profile"].as_str().unwrap().to_owned();
        // Controls: a watermark that resumes and one that is no watermark both answer
        // `not_found` here.
        let reference = read(
            &engine,
            &Provider::new(403, refusal.clone()),
            &mark(&profile, "2026-10-06T09:00:30Z"),
        )
        .unwrap_err();
        assert_eq!(reference.code, ErrorCode::NotFound, "{name}: control");
        let garbage = read(
            &engine,
            &Provider::new(403, refusal.clone()),
            "not-a-watermark",
        )
        .unwrap_err();
        assert_eq!(
            (&garbage.code, &garbage.message),
            (&reference.code, &reference.message),
            "{name}: control"
        );

        let http = Provider::new(403, refusal.clone());
        match read(&engine, &http, &mark(&profile, time)) {
            Err(error)
                if (&error.code, &error.message) == (&reference.code, &reference.message) => {}
            other => disclosed.push(format!(
                "{name} {time}: {:?}, {} listing(s) sent",
                other.map_err(|error| (error.code, error.message)),
                http.listings().len()
            )),
        }
    }
    assert!(
        disclosed.is_empty(),
        "a container whose listing refuses answered other than `not_found`: {disclosed:#?}"
    );
}

/// The same watermark on a container whose listing answers is `stale_cursor`, but only after
/// that listing was sent, as a first read (no `{since}`), its records discarded.
#[test]
fn adversary_a_year_bound_watermark_is_stale_only_after_the_listing_answered() {
    let mut wrong = Vec::new();
    for (name, feed, parameter, time) in year_bound_cases() {
        let engine = build(&feed).unwrap();
        let profile = feed["profile"].as_str().unwrap().to_owned();
        let http = Provider::new(200, json!({"issues": [], "results": []}));
        let answer = read(&engine, &http, &mark(&profile, time));
        let listings = http.listings();
        let first = listings.first().and_then(|call| sent(call, parameter));
        let stale = matches!(&answer, Err(error) if error.code == ErrorCode::StaleCursor);
        if !stale || listings.len() != 1 || first != Some(r#"room = "room1" ORDER BY edited ASC"#) {
            wrong.push(format!(
                "{name} {time}: {:?}, listings sent {listings:?}",
                answer.map_err(|error| (error.code, error.message))
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "a year-bound watermark was not judged after a first-read listing: {wrong:#?}"
    );
}

//! Security review of story:catalog-feed-engine-extensions: integrity conformance checks of the
//! query, next-link, relative-link, paging and compatibility rules the unit added to the catalog
//! engine's `datasource.feed/v1alpha1` binding (`contracts/catalog/v1alpha1/semantics.md`). Each
//! case drives the fixture declarations in `tests/feed/` with an in-process provider whose
//! answers the case states; no live provider is called.
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use connectors_catalog::{bundle::Bundle, ingest, inventory};
use connectors_catalog_provider::{
    Engine,
    feed::{CONTAINERS, Declaration, ITEMS, PAGES},
};
use connectors_core::{ErrorCode, Result};
use connectors_host::local::filesystem;
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
};

const SERVER: &str = "https://feed.fixture.test/v1";
const ORIGIN_HOST: &str = "feed.fixture.test";
const INSTANCE: &str = "review-instance";
const QUERY: &str = "time-query.operations.json";
const NEXT: &str = "time-next.operations.json";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/feed")
}

fn selection(name: &str) -> Value {
    serde_json::from_slice(&fs::read(fixtures().join(name)).unwrap()).unwrap()
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

/// The engine as the local service builds it: the feed, its links under the API base's origin.
fn build(feed: &Value) -> Result<Engine> {
    let feed: Declaration = serde_json::from_value(feed.clone()).unwrap();
    Engine::with_feed(&bundle(), "/v1", &[], Some(&feed))?.with_link_origin(SERVER)
}

/// The parameter each fixture sends its query in.
fn query_parameter(name: &str) -> &'static str {
    if name == QUERY { "q" } else { "filter" }
}

type Call = (Vec<String>, Vec<(String, String)>);
type Answer = Box<dyn Fn(usize, &[String]) -> (u16, Value) + Send + Sync>;

/// A provider answering each GET as `answer` says, given the index of the call and its path.
struct Provider {
    answer: Answer,
    calls: Mutex<Vec<Call>>,
}

impl Provider {
    fn new(answer: impl Fn(usize, &[String]) -> (u16, Value) + Send + Sync + 'static) -> Self {
        Self {
            answer: Box::new(answer),
            calls: Mutex::new(Vec::new()),
        }
    }

    /// A provider whose container lookup admits any id and whose every listing is `listing`.
    fn listing(listing: Value) -> Self {
        Self::new(move |_, path| {
            if lookup(path) {
                (200, room_answer())
            } else {
                (200, listing.clone())
            }
        })
    }

    fn calls(&self) -> Vec<Call> {
        self.calls.lock().unwrap().clone()
    }

    fn listings(&self) -> Vec<Call> {
        self.calls()
            .into_iter()
            .filter(|(p, _)| !lookup(p))
            .collect()
    }
}

#[async_trait::async_trait]
impl AuthenticatedHttp for Provider {
    async fn get(&self, path: &[&str], query: &[(&str, String)]) -> Result<HttpResponse> {
        let path: Vec<String> = path.iter().map(|s| s.to_string()).collect();
        let query: Vec<(String, String)> = query
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect();
        let index = self.calls.lock().unwrap().len();
        let (status, body) = (self.answer)(index, &path);
        self.calls.lock().unwrap().push((path, query));
        Ok(HttpResponse {
            status,
            headers: Default::default(),
            body: serde_json::to_vec(&body).unwrap(),
        })
    }
}

fn lookup(path: &[String]) -> bool {
    path.first().map(String::as_str) == Some("rooms")
}

fn room(id: &str) -> Value {
    json!({"id": id, "title": "Room", "type": "room", "access": "open"})
}

fn room_answer() -> Value {
    json!({"room": room("admitted")})
}

/// One item in the shape both fixtures read: the query fixture under `/fields`, the next-link
/// fixture at the top level.
fn record(id: &str, at: &str, link: Value) -> Value {
    json!({
        "id": id, "version": "1", "created": at, "edited": at, "text": "body",
        "_links": {"webui": link},
        "fields": {"version": "1", "created": at, "updated": at, "text": "body", "link": link},
    })
}

/// A listing answer both fixtures read: `/issues` for one, `/results` for the other.
fn page(records: Vec<Value>) -> Value {
    json!({"issues": records, "results": records})
}

fn read(engine: &Engine, http: &Provider, id: &str, input: Value) -> Result<Value> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(engine.read(http, INSTANCE, id, input))
}

fn sent<'a>(call: &'a Call, name: &str) -> Option<&'a str> {
    call.1
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
}

/// A watermark or listing cursor in the engine's own format, as any caller can write one.
fn mark(
    profile: &str,
    container: Option<&str>,
    cursor: Option<&str>,
    time: Option<&str>,
) -> String {
    let mut mark = json!({"format": "catalog-feed/1", "profile": profile, "instance": INSTANCE});
    if let Some(container) = container {
        mark["container"] = json!(container);
    }
    if let Some(cursor) = cursor {
        mark["cursor"] = json!(cursor);
    }
    if let Some(time) = time {
        mark["time"] = json!(time);
    }
    URL_SAFE_NO_PAD.encode(serde_json::to_vec(&mark).unwrap())
}

/// What a watermark or listing cursor the engine issued carries.
fn opened(token: &str) -> Value {
    serde_json::from_slice(&URL_SAFE_NO_PAD.decode(token).unwrap()).unwrap()
}

/// One double-quoted literal of the query languages at the start of `text`, read as the
/// contract states them (`\` escapes the next character): its value and the text after it.
fn literal(text: &str) -> Option<(String, &str)> {
    let mut chars = text.char_indices();
    if chars.next()?.1 != '"' {
        return None;
    }
    let mut value = String::new();
    while let Some((at, c)) = chars.next() {
        match c {
            '\\' => value.push(chars.next()?.1),
            '"' => return Some((value, &text[at + 1..])),
            c => value.push(c),
        }
    }
    None
}

/// The fixtures' templates as sent: the container literal and, on a resume, the since literal,
/// when `q` is exactly that template with one literal in each slot.
fn parse_query(q: &str) -> Option<(String, Option<String>)> {
    let (container, rest) = literal(q.strip_prefix("room = ")?)?;
    if rest == " ORDER BY edited ASC" {
        return Some((container, None));
    }
    let (since, rest) = literal(rest.strip_prefix(" AND edited >= ")?)?;
    (rest == " ORDER BY edited ASC").then_some((container, Some(since)))
}

/// `YYYY-MM-DD HH:MM`, the shape the contract gives `{since}`.
fn minute_shaped(text: &str) -> bool {
    let b = text.as_bytes();
    b.len() == 16
        && [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15]
            .iter()
            .all(|&i| b[i].is_ascii_digit())
        && b[4] == b'-'
        && b[7] == b'-'
        && b[10] == b' '
        && b[13] == b':'
}

/// The host a WHATWG URL parser reads in `link` (the `url` crate, through the host's own
/// endpoint canonicalization), its query and fragment set aside.
fn host(link: &str) -> Option<String> {
    let link = &link[..link.find(['?', '#']).unwrap_or(link.len())];
    let parsed = connectors_host::http::canonical_base(link).ok()?;
    let rest = parsed
        .strip_prefix("https://")
        .or_else(|| parsed.strip_prefix("http://"))?;
    let authority = &rest[..rest.find('/').unwrap_or(rest.len())];
    Some(
        authority
            .rsplit_once(':')
            .map_or(authority, |(h, _)| h)
            .to_owned(),
    )
}

// ---- invariant 1: a value cannot add a clause ------------------------------------------------

/// The contract: each slot's value is a quoted literal of the language, "never spliced raw"
/// (semantics.md, Watermark: `time`). A template whose slot sits inside a literal the template
/// itself opened, but not next to its quote, puts the value's own quotes against the
/// template's: the id closes the literal and its text is read as query. Such a template must be
/// refused when the selection file loads, as the slot next to a quote is.
#[test]
fn review_a_slot_inside_a_template_literal_is_refused_when_the_selection_loads() {
    let cases = [
        (
            r#"text ~ "x {container} y" ORDER BY edited ASC"#,
            r#"text ~ "x {container} y" AND edited >= {since} ORDER BY edited ASC"#,
            "OR room = other OR text ~",
        ),
        (
            "text ~ 'x {container} y' ORDER BY edited ASC",
            "text ~ 'x {container} y' AND edited >= {since} ORDER BY edited ASC",
            "' OR room = 'other",
        ),
    ];
    let mut loaded = Vec::new();
    for (first, resume, id) in cases {
        let mut raw = selection(QUERY);
        raw["feed"]["items"]["position"]["value"]["query"]["first"] = json!(first);
        raw["feed"]["items"]["position"]["value"]["query"]["resume"] = json!(resume);
        let Ok(engine) = build(&raw["feed"]) else {
            continue;
        };
        let http = Provider::listing(page(vec![]));
        let answer = read(&engine, &http, ITEMS, json!({"container": id, "limit": 10}));
        let q = http
            .listings()
            .first()
            .and_then(|call| sent(call, "q").map(str::to_owned));
        loaded.push((first, id, answer.is_ok(), q));
    }
    assert!(
        loaded.is_empty(),
        "templates with a slot inside their own literal loaded; (template, container id, read ok, q sent): {loaded:#?}"
    );
}

/// Through the shipped fixture templates, every container id that holds no control character
/// reaches the query as exactly one literal whose value is the id, on a first read and on a
/// resume, in both languages; an id with a control character is `invalid_input` before any
/// request, the lookup included, and an empty id is refused by the input schema.
#[test]
fn review_hostile_container_ids_stay_one_literal_and_control_characters_send_nothing() {
    let long = "\"\\".repeat(2048);
    let hostile = [
        r#"x" OR room = "other"#,
        r#"x\" OR room = \"other"#,
        "\\",
        "x\\",
        "\\\"",
        "\u{5c}u0022 OR room = \u{5c}u0022other",
        "x ORDER BY created DESC",
        "a AND b OR c",
        "\u{201c}smart\u{201d} \u{2018}single\u{2019} \u{ff02}full\u{ff02}",
        "x\u{2028}OR room = other",
        "{since} {container}",
        long.as_str(),
    ];
    for name in [QUERY, NEXT] {
        let parameter = query_parameter(name);
        let engine = build(&selection(name)["feed"]).unwrap();
        for id in hostile {
            let http = Provider::listing(page(vec![record(
                "i1",
                "2026-10-06T09:00:30Z",
                Value::Null,
            )]));
            let first = read(&engine, &http, ITEMS, json!({"container": id, "limit": 10}))
                .unwrap_or_else(|e| panic!("{name}: {id:?}: {e}"));
            let watermark = first["next_watermark"].clone();
            read(
                &engine,
                &http,
                ITEMS,
                json!({"container": id, "limit": 10, "watermark": watermark}),
            )
            .unwrap_or_else(|e| panic!("{name}: {id:?}: resume: {e}"));
            let listings = http.listings();
            assert_eq!(listings.len(), 2, "{name}: {id:?}");
            let q0 = sent(&listings[0], parameter).unwrap();
            let q1 = sent(&listings[1], parameter).unwrap();
            assert_eq!(parse_query(q0), Some((id.to_owned(), None)), "{name}: {q0}");
            assert_eq!(
                parse_query(q1),
                Some((id.to_owned(), Some("2026-10-06 09:00".to_owned()))),
                "{name}: {q1}"
            );
        }
        for id in ["\0", "\n", "\r", "\t", "\u{85}", "\u{1b}", "x\nAND y", ""] {
            let http = Provider::listing(page(vec![]));
            let error =
                read(&engine, &http, ITEMS, json!({"container": id, "limit": 10})).unwrap_err();
            assert_eq!(error.code, ErrorCode::InvalidInput, "{name}: {id:?}");
            assert!(
                http.calls().is_empty(),
                "{name}: {id:?}: {:?}",
                http.calls()
            );
        }
    }
}

// ---- invariant 2: the time literal comes from the parsed instant -------------------------------

/// A watermark is unsigned, so any caller can write one. Its time reaches the query only as
/// the UTC minute of the instant it parses to; a time that does not parse is `stale_cursor`,
/// and the listing sent first carries no `{since}` and none of the forged text.
#[test]
fn review_a_forged_watermark_time_reaches_the_query_only_as_its_utc_minute() {
    for name in [QUERY, NEXT] {
        let parameter = query_parameter(name);
        let raw = selection(name);
        let profile = raw["feed"]["profile"].as_str().unwrap();
        let engine = build(&raw["feed"]).unwrap();
        for (time, minute) in [
            ("2026-10-06T10:30:15+01:00", Some("2026-10-06 09:30")),
            ("2026-10-06T09:00:60Z", Some("2026-10-06 09:01")),
            (
                "2026-10-06T09:00:00.1234567890123456789012345Z",
                Some("2026-10-06 09:00"),
            ),
            ("2026-10-06T09:00:00.5\" OR room = \"otherZ", None),
            ("2026-10-06 09:00\" OR room = \"other", None),
            ("2026-10-06T09:00:00Z\" OR room = \"other", None),
        ] {
            let http = Provider::listing(page(vec![]));
            let watermark = mark(profile, Some("room1"), None, Some(time));
            let answer = read(
                &engine,
                &http,
                ITEMS,
                json!({"container": "room1", "limit": 10, "watermark": watermark}),
            );
            let listings = http.listings();
            let q = sent(&listings[0], parameter).unwrap();
            match minute {
                Some(minute) => {
                    answer.unwrap_or_else(|e| panic!("{name}: {time}: {e}"));
                    assert_eq!(
                        parse_query(q),
                        Some(("room1".to_owned(), Some(minute.to_owned()))),
                        "{name}: {time}: {q}"
                    );
                }
                None => {
                    assert_eq!(
                        answer.unwrap_err().code,
                        ErrorCode::StaleCursor,
                        "{name}: {time}"
                    );
                    assert_eq!(
                        parse_query(q),
                        Some(("room1".to_owned(), None)),
                        "{name}: {q}"
                    );
                }
            }
        }
    }
}

/// The contract gives `{since}` the shape `"YYYY-MM-DD HH:MM"` in UTC. An instant the engine
/// accepts (years 0000 to 9999 with any offset) can lie outside the years that shape holds once
/// it is in UTC; whether it comes from a forged watermark or a provider's `updated_at`, the
/// literal sent must keep the shape, or the read must be refused.
#[test]
fn review_the_since_literal_keeps_its_shape_at_the_instant_bounds() {
    let mut malformed = Vec::new();
    for name in [QUERY, NEXT] {
        let parameter = query_parameter(name);
        let raw = selection(name);
        let profile = raw["feed"]["profile"].as_str().unwrap();
        let engine = build(&raw["feed"]).unwrap();
        for time in ["9999-12-31T23:59:00-01:00", "0000-01-01T00:30:00+01:00"] {
            let http = Provider::listing(page(vec![]));
            let watermark = mark(profile, Some("room1"), None, Some(time));
            let answer = read(
                &engine,
                &http,
                ITEMS,
                json!({"container": "room1", "limit": 10, "watermark": watermark}),
            );
            if answer.is_err() {
                continue;
            }
            let listings = http.listings();
            let q = sent(&listings[0], parameter).unwrap().to_owned();
            let since = parse_query(&q).and_then(|(_, since)| since);
            if !since.as_deref().is_some_and(minute_shaped) {
                malformed.push((name, time, q));
            }
        }
    }
    assert!(
        malformed.is_empty(),
        "a `{{since}}` literal outside `YYYY-MM-DD HH:MM` was sent: {malformed:#?}"
    );
}

// ---- invariant 3: one parameter of a next link, never requested ------------------------------

/// A next link is read for its one named parameter, decoded, and never requested: whatever its
/// host, scheme or the URL its value carries, the next request goes to the declared listing
/// with that value, and a link without exactly one non-empty decodable value is `unavailable`.
#[test]
fn review_a_next_link_yields_one_parameter_and_is_never_requested() {
    let engine = build(&selection(NEXT)["feed"]).unwrap();
    let read_with = |link: &str| {
        let link = link.to_owned();
        let http = Provider::new(move |index, _| {
            if index == 0 {
                (
                    200,
                    json!({"results": [room("r1")], "_links": {"next": link}}),
                )
            } else {
                (200, json!({"results": []}))
            }
        });
        let first = read(&engine, &http, CONTAINERS, json!({"limit": 1}));
        (http, first)
    };
    for (link, value) in [
        ("https://other.test/elsewhere?start=abc", "abc"),
        ("javascript:alert(1)//?start=abc", "abc"),
        ("/rooms?start=a%26b%3Dc&limit=5", "a&b=c"),
        (
            "/rooms?start=https%3A%2F%2Fother.test%2Frooms%3Fstart%3Dzzz",
            "https://other.test/rooms?start=zzz",
        ),
        ("/rooms?other=1&start=x+y", "x y"),
        ("/rooms?start=abc#start=def", "abc"),
    ] {
        let (http, first) = read_with(link);
        let first = first.unwrap_or_else(|e| panic!("{link}: {e}"));
        let cursor = first["next_cursor"].as_str().unwrap();
        assert_eq!(opened(cursor)["cursor"], json!(value), "{link}");
        read(
            &engine,
            &http,
            CONTAINERS,
            json!({"limit": 1, "cursor": cursor}),
        )
        .unwrap();
        let calls = http.calls();
        assert_eq!(calls.len(), 2, "{link}: {calls:?}");
        assert!(
            calls.iter().all(|(path, _)| path == &["room-directory"]),
            "{link}: {calls:?}"
        );
        assert_eq!(sent(&calls[1], "start"), Some(value), "{link}");
        assert_eq!(calls[1].1.iter().filter(|(k, _)| k == "start").count(), 1);
    }
    for link in [
        "/rooms?x=1#start=abc",
        "/rooms?start=a&start=b",
        "/rooms?start=a&st%61rt=b",
        "/rooms?start=",
        "/rooms?start=%ZZ",
        "/rooms?start=%FF",
        "/rooms",
    ] {
        let (http, first) = read_with(link);
        assert_eq!(first.unwrap_err().code, ErrorCode::Unavailable, "{link}");
        assert_eq!(http.calls().len(), 1, "{link}");
    }
}

// ---- invariant 4: relative item links stay under their base ----------------------------------

fn item_link(name: &str, base: Option<&str>, link: &str) -> Value {
    let engine = build(&selection(name)["feed"]).unwrap();
    let mut answer = page(vec![record("i1", "2026-10-06T09:00:30Z", json!(link))]);
    if let Some(base) = base {
        answer["_links"] = json!({"base": base});
    }
    let http = Provider::listing(answer);
    let page = read(
        &engine,
        &http,
        ITEMS,
        json!({"container": "room1", "limit": 10}),
    )
    .unwrap_or_else(|e| panic!("{name}: {link}: {e}"));
    page["items"][0]["url"].clone()
}

/// Under the API base's origin and under a declared `url_base`, a link that is not absolute
/// resolves to the base's host as a WHATWG parser reads it, or to no link.
#[test]
fn review_a_relative_link_never_reaches_another_host_under_a_usable_base() {
    let hostile = [
        "//evil.test/x",
        "\\\\evil.test\\x",
        "/\\evil.test/x",
        "/\\\\evil.test",
        "https:evil.test/x",
        "https:/evil.test/x",
        "https:\\\\evil.test",
        "http:/evil.test",
        "/x@evil.test",
        "/@evil.test/x",
        "/..//evil.test/x",
        "/../../../evil.test",
        "/%2F%2Fevil.test",
        "/%5C%5Cevil.test",
        "/\t/evil.test",
        "/\n//evil.test",
        "@evil.test/x",
        "evil.test/x",
        " //evil.test",
        ":evil.test",
        "/:@evil.test",
        "HTTPS://evil.test/x",
    ];
    for (name, base, expected, control) in [
        (QUERY, None, ORIGIN_HOST, "https://feed.fixture.test/a/b"),
        (
            NEXT,
            Some("https://wiki.fixture.test/wiki/"),
            "wiki.fixture.test",
            "https://wiki.fixture.test/wiki/a/b",
        ),
    ] {
        assert_eq!(item_link(name, base, "/a/b"), json!(control), "{name}");
        for link in hostile {
            let url = item_link(name, base, link);
            if let Some(text) = url.as_str() {
                assert_eq!(
                    host(text).as_deref(),
                    Some(expected),
                    "{name}: {link:?} -> {text}"
                );
            }
        }
    }
}

/// The contract: with `url_base`, a relative link resolves under "the `http(s)://` URL at that
/// pointer", and under an absent or unusable one it is `null`. A base that names no host is not
/// such a URL; the link under it must be `null`, never a URL whose host the link chose.
#[test]
fn review_a_url_base_that_names_no_host_gives_no_link() {
    let mut reached = Vec::new();
    for base in ["https://", "http://", "https:////"] {
        let url = item_link(NEXT, Some(base), "/evil.test/x");
        if let Some(text) = url.as_str() {
            reached.push((base, text.to_owned(), host(text)));
        }
    }
    assert!(
        reached.is_empty(),
        "a relative link under a host-less url_base resolved; (url_base, link, WHATWG host): {reached:#?}"
    );
}

// ---- invariant 6: paging terminates within the cap -------------------------------------------

/// A page continuation the provider repeats forever, with pages that hold no new item, stops at
/// `PAGES` listing requests and is `unavailable`; with a new item on each page it stops at the
/// same count, incomplete.
#[test]
fn review_a_repeated_page_token_stops_at_the_page_cap() {
    for name in [QUERY, NEXT] {
        let engine = build(&selection(name)["feed"]).unwrap();
        let continued = |records: Vec<Value>| {
            let mut answer = page(records);
            answer["nextPageToken"] = json!("same");
            answer["_links"] = json!({"next": "/message-query?after=same"});
            answer
        };
        let http = Provider::listing(continued(vec![]));
        let error = read(
            &engine,
            &http,
            ITEMS,
            json!({"container": "room1", "limit": 50}),
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::Unavailable, "{name}");
        assert_eq!(http.listings().len(), PAGES, "{name}");
        let http = Provider::new(move |index, path| {
            if lookup(path) {
                return (200, room_answer());
            }
            let at = format!("2026-10-06T09:{:02}:00Z", index);
            let mut answer = page(vec![record(&format!("i{index}"), &at, Value::Null)]);
            answer["nextPageToken"] = json!("same");
            answer["_links"] = json!({"next": "/message-query?after=same"});
            (200, answer)
        });
        let read_page = read(
            &engine,
            &http,
            ITEMS,
            json!({"container": "room1", "limit": 50}),
        )
        .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(read_page["complete"], json!(false), "{name}");
        assert_eq!(
            read_page["items"].as_array().unwrap().len(),
            PAGES,
            "{name}"
        );
        assert_eq!(http.listings().len(), PAGES, "{name}");
    }
}

/// Offset paging makes one request per call whatever the answer: a page with the flag false or
/// missing continues after its records, and an empty page without the flag is `unavailable`.
#[test]
fn review_offset_paging_makes_one_request_per_call_and_refuses_an_empty_unflagged_page() {
    let engine = build(&selection(QUERY)["feed"]).unwrap();
    let http = Provider::new(|index, _| match index {
        0 => (200, json!({"values": [room("r1")], "isLast": false})),
        1 => (200, json!({"values": [room("r2")]})),
        _ => (200, json!({"values": [], "isLast": false})),
    });
    let first = read(&engine, &http, CONTAINERS, json!({"limit": 1})).unwrap();
    let second = read(
        &engine,
        &http,
        CONTAINERS,
        json!({"limit": 1, "cursor": first["next_cursor"]}),
    )
    .unwrap();
    let third = read(
        &engine,
        &http,
        CONTAINERS,
        json!({"limit": 1, "cursor": second["next_cursor"]}),
    );
    assert_eq!(third.unwrap_err().code, ErrorCode::Unavailable);
    let calls = http.calls();
    assert_eq!(calls.len(), 3);
    assert_eq!(
        calls.iter().map(|c| sent(c, "startAt")).collect::<Vec<_>>(),
        [None, Some("1"), Some("2")]
    );
}

/// A listing cursor this binding did not issue is `stale_cursor` (the family's error table:
/// "a foreign, malformed or no-longer-resumable watermark"). An offset at the top of `u64`
/// parses, so the cursor is accepted and sent; the next offset then overflows.
#[test]
fn review_a_forged_offset_cursor_at_the_top_of_u64_is_stale_cursor() {
    let engine = build(&selection(QUERY)["feed"]).unwrap();
    let profile = selection(QUERY)["feed"]["profile"]
        .as_str()
        .unwrap()
        .to_owned();
    let http = Provider::new(|_, _| (200, json!({"values": [room("r1")], "isLast": false})));
    let cursor = mark(&profile, None, Some(&u64::MAX.to_string()), None);
    let error = read(
        &engine,
        &http,
        CONTAINERS,
        json!({"limit": 1, "cursor": cursor}),
    )
    .unwrap_err();
    assert_eq!(
        error.code,
        ErrorCode::StaleCursor,
        "{error}; requests sent: {:?}",
        http.calls()
    );
}

// ---- invariant 5: backward compatibility ---------------------------------------------------

fn shipped_gitlab() -> Value {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    serde_json::from_slice(&fs::read(repository.join("providers/gitlab/operations.json")).unwrap())
        .unwrap()
}

/// The local service's bootstrap of a GitLab configuration under `api_base`, with the shipped
/// selection file or, without `feed`, the same file with its `feed` removed.
fn bootstrap(api_base: &str, feed: bool) -> std::process::Output {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let scratch = tempfile::tempdir().unwrap();
    fs::set_permissions(scratch.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let private = scratch.path().join("private");
    filesystem::directory(&private, true, true).unwrap();
    let operations = if feed {
        repository
            .join("providers/gitlab/operations.json")
            .canonicalize()
            .unwrap()
    } else {
        let mut selection = shipped_gitlab();
        selection.as_object_mut().unwrap().remove("feed");
        let path = private.join("operations.json");
        fs::write(&path, serde_json::to_vec(&selection).unwrap()).unwrap();
        path
    };
    let path = private.join("catalog.json");
    let config = json!({
        "format": "connectors-catalog-local/2",
        "instance": "review-gitlab",
        "provider": "gitlab",
        "bundle_directory": repository.join("generated/bundles").canonicalize().unwrap(),
        "api_base": api_base,
        "auth": {
            "profile": "gitlab.pat",
            "header": "PRIVATE-TOKEN",
            "bearer": false,
            "label": "GitLab personal access token",
            "identity": {"path": "user", "kind": "gitlab.user", "subject_pointer": "/id"},
            "scopes": {"path": "personal_access_tokens/self", "pointer": "/scopes"},
            "minimum_scopes": ["api"]
        },
        "operations_file": operations,
    });
    fs::write(&path, serde_json::to_vec(&config).unwrap()).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
        .arg("--local-config")
        .arg(&path)
        .arg("--print-local-bootstrap")
        .output()
        .unwrap()
}

/// A GitLab configuration with the shipped feed loads under every `https` API base the host's
/// endpoint rule admits, as before the unit: the GitLab feed declares no item `url`, so the
/// origin its links would resolve under cannot be a reason to refuse it. The same configuration
/// without the feed loads under each base, so a refusal is the feed's; the engine's own refusal
/// is recorded beside it.
#[test]
fn review_the_shipped_gitlab_feed_loads_under_every_admitted_https_base() {
    let control = bootstrap("https://gitlab.example.test/api/v4", true);
    assert!(
        control.status.success(),
        "{}",
        String::from_utf8_lossy(&control.stderr)
    );
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let gitlab =
        connectors_catalog::bundle::load(&repository.join("generated/bundles"), "gitlab").unwrap();
    let feed: Declaration = serde_json::from_value(shipped_gitlab()["feed"].clone()).unwrap();
    let mut refused = Vec::new();
    for base in [
        "https://[fd00::1]/api/v4",
        "https://gitlab_host.example.test/api/v4",
        "https://gitlab.example.test./api/v4",
    ] {
        assert!(
            connectors_host::http::canonical_base(base).is_ok(),
            "{base}"
        );
        let without = bootstrap(base, false);
        assert!(
            without.status.success(),
            "{base} without the feed: {}",
            String::from_utf8_lossy(&without.stderr)
        );
        let output = bootstrap(base, true);
        if !output.status.success() {
            let engine = Engine::with_feed(&gitlab, "/api/v4", &[], Some(&feed))
                .unwrap()
                .with_link_origin(base)
                .err()
                .map(|e| e.to_string());
            refused.push((
                base,
                String::from_utf8_lossy(&output.stderr).trim().to_owned(),
                engine,
            ));
        }
    }
    assert!(
        refused.is_empty(),
        "a GitLab feed configuration the endpoint rule admits is refused: {refused:#?}"
    );
}

/// The new declaration types refuse unknown keys and values, and the shipped GitLab feed
/// serializes back to exactly its file, so the revision digest over it does not move.
#[test]
fn review_new_declaration_fields_are_strict_and_the_gitlab_feed_round_trips() {
    let refused = |name: &str, change: &dyn Fn(&mut Value)| {
        let mut raw = selection(name);
        change(&mut raw["feed"]);
        assert!(
            serde_json::from_value::<Declaration>(raw["feed"].clone()).is_err(),
            "{name}: {}",
            raw["feed"]
        );
    };
    refused(QUERY, &|f| {
        f["items"]["position"]["value"]["query"]["zone"] = json!("UTC")
    });
    refused(QUERY, &|f| {
        f["items"]["position"]["value"]["query"]["language"] = json!("sql")
    });
    refused(QUERY, &|f| {
        f["items"]["position"]["value"]["precision"] = json!("second")
    });
    refused(QUERY, &|f| {
        f["items"]["position"]["value"]["page"]["limit"] = json!(5)
    });
    refused(QUERY, &|f| {
        f["containers"]["cursor"]["offset"]["start"] = json!(0)
    });
    refused(NEXT, &|f| {
        f["containers"]["cursor"]["url_parameters"] = json!("start")
    });
    refused(NEXT, &|f| f["items"]["url_bases"] = json!("/_links/base"));
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let shipped: Value = serde_json::from_slice(
        &fs::read(repository.join("providers/gitlab/operations.json")).unwrap(),
    )
    .unwrap();
    let feed: Declaration = serde_json::from_value(shipped["feed"].clone()).unwrap();
    assert_eq!(serde_json::to_value(&feed).unwrap(), shipped["feed"]);
}

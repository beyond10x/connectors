//! story:confluence-feed-binding: a Confluence REST v1 declaration binds
//! `datasource.feed/v1alpha1` as data (`adapters/catalog/contracts/feed/v1alpha1/confluence-v1.md`),
//! and the catalog engine reads it over the committed `confluence-v1` bundle. The declaration is
//! the fixture `tests/feed/confluence/feed.json`, not part of the shipped `confluence-v1`
//! selection file, which carries no `feed` until the connector definition format every shipped
//! selection round-trips through can express one. A container is a space the user can view; an
//! item is one of its pages. Every answer here is a Confluence record in the shape of
//! `tests/feed/confluence/*.json`, taken from the public v1 reference, not recorded from a running
//! Confluence; the requests the engine sends are recorded and checked. The family's suite runs
//! against the same declaration in `crates/connectors-build/src/confluence_feed_conformance.rs`.
//!
//! The binding expects Confluence to read a CQL date literal in the calling user's zone, which it
//! is not told. [`Site`] answers the content search as Confluence would in one such zone: it reads
//! the `lastmodified >=` literal as wall time there. Confluence writes `version.when` and
//! `history.createdDate` in UTC. The zone cases hold that no zone from UTC-12:00 to UTC+14:00
//! makes a resume skip a page or answer `unavailable`.
use connectors_catalog::bundle;
use connectors_catalog_provider::{
    Effect, Engine, Selection,
    feed::{CONTAINERS, Declaration, FEED_CONTRACT, ITEMS},
};
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{collections::VecDeque, path::Path, sync::Mutex};

const PROFILE: &str = "confluence-pages/1";
const PROVIDER: &str = "confluence-v1";
const INSTANCE: &str = "confluence-v1-feed-instance";
/// The configured `api_base` path: the site's `/wiki` root.
const BASE: &str = "/wiki";
const SPACE: &str = "DOCS";
const SITE: &str = "https://site.example.test/wiki";
const EXPAND: &str = "history,version";
const FIRST: &str = "space = \"DOCS\" and type = page order by lastmodified asc";

type Call = (Vec<String>, Vec<(String, String)>);

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn recorded(name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(root().join("tests/feed/confluence").join(name)).unwrap())
        .unwrap()
}

fn declaration() -> Value {
    recorded("feed.json")
}

fn engine() -> Engine {
    let feed: Declaration = serde_json::from_value(declaration()).unwrap();
    let bundle = bundle::load(&root().join("generated/bundles"), PROVIDER).unwrap();
    Engine::with_feed(&bundle, BASE, &[], Some(&feed)).unwrap()
}

fn record(path: &[&str], query: &[(&str, String)]) -> Call {
    let mut query: Vec<(String, String)> = query
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect();
    query.sort();
    (path.iter().map(|s| s.to_string()).collect(), query)
}

fn respond(status: u16, body: &Value) -> HttpResponse {
    HttpResponse {
        status,
        headers: Default::default(),
        body: serde_json::to_vec(body).unwrap(),
    }
}

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    let mut pairs: Vec<(String, String)> = list
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    pairs.sort();
    pairs
}

fn segments(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

fn ids(page: &Value) -> Vec<&str> {
    page["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_str().unwrap())
        .collect()
}

/// The links every v1 page carries; `next` only while another page follows.
fn links(next: Option<&str>) -> Value {
    let mut links = json!({"base": SITE, "context": "/wiki"});
    if let Some(next) = next {
        links["next"] = json!(next);
    }
    links
}

/// A space as v1 lists and reads it, with the fields the binding reads replaced.
fn space(id: u64, key: &str, name: &str, kind: &str) -> Value {
    let mut space = recorded("space.json");
    space["id"] = json!(id);
    space["key"] = json!(key);
    space["name"] = json!(name);
    space["type"] = json!(kind);
    space["_links"]["webui"] = json!(format!("/spaces/{key}"));
    space["_links"]["self"] = json!(format!("{SITE}/rest/api/space/{key}"));
    space
}

// ---- recorded answers, replayed --------------------------------------------------------------

struct Reads {
    responses: Mutex<VecDeque<(u16, Value)>>,
    calls: Mutex<Vec<Call>>,
}

#[async_trait::async_trait]
impl AuthenticatedHttp for Reads {
    async fn get(&self, path: &[&str], query: &[(&str, String)]) -> Result<HttpResponse> {
        self.calls.lock().unwrap().push(record(path, query));
        let (status, body) = self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected provider read");
        Ok(respond(status, &body))
    }
}

fn answers(responses: Vec<(u16, Value)>) -> Reads {
    Reads {
        responses: Mutex::new(responses.into()),
        calls: Mutex::new(Vec::new()),
    }
}

// ---- civil time ------------------------------------------------------------------------------

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year.rem_euclid(400);
    let doy = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// Milliseconds since the epoch of the wall time `YYYY-MM-DD HH:MM` or `YYYY-MM-DD HH:MM:SS`
/// read in UTC; `None` for any other text.
fn wall(text: &str) -> Option<i64> {
    let bytes = text.as_bytes();
    let number = |range: std::ops::Range<usize>| -> Option<i64> {
        let digits = text.get(range)?;
        digits
            .bytes()
            .all(|b| b.is_ascii_digit())
            .then(|| digits.parse().ok())?
    };
    let shaped = bytes.len() >= 16
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b' '
        && bytes[13] == b':';
    if !shaped {
        return None;
    }
    let second = match bytes.len() {
        16 => 0,
        19 if bytes[16] == b':' => number(17..19)?,
        _ => return None,
    };
    let (year, month, day) = (number(0..4)?, number(5..7)?, number(8..10)?);
    let (hour, minute) = (number(11..13)?, number(14..16)?);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour > 23 || minute > 59 {
        return None;
    }
    Some(((days_from_civil(year, month, day) * 24 + hour) * 60 + minute) * 60_000 + second * 1000)
}

/// `at` as Confluence writes `version.when` and `history.createdDate`:
/// `YYYY-MM-DDTHH:MM:SS.mmmZ`.
fn utc_time(at: i64) -> String {
    let (year, month, day) = civil_from_days(at.div_euclid(86_400_000));
    let rem = at.rem_euclid(86_400_000);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        rem / 3_600_000,
        rem / 60_000 % 60,
        rem / 1000 % 60,
        rem % 1000
    )
}

/// The `{since}` literal a resume sends, read back out of its CQL.
fn since_of(cql: &str) -> Option<&str> {
    let (_, rest) = cql.split_once("and lastmodified >= \"")?;
    Some(rest.split_once('"')?.0)
}

fn at(text: &str) -> i64 {
    wall(text).expect("a wall time")
}

// ---- a Confluence site that reads CQL in one zone --------------------------------------------

/// One page of the simulated space: its content id, its version number, and its creation and
/// last version, in milliseconds since the epoch.
#[derive(Clone, Copy)]
struct Page {
    id: u64,
    version: u64,
    created: i64,
    updated: i64,
}

/// The v1 space lookup and content search of one site, for a user whose zone is `offset`
/// minutes east of UTC: the `lastmodified >=` literal is read as wall time there. At most `page`
/// pages an answer, as Confluence may answer fewer than asked for; it continues by a `cursor`
/// carried in `_links.next`, percent-encoded as Confluence writes one.
struct Site {
    offset: i64,
    page: usize,
    pages: Mutex<Vec<Page>>,
    calls: Mutex<Vec<Call>>,
}

impl Site {
    fn new(offset: i64, pages: Vec<Page>) -> Self {
        Self {
            offset,
            page: 100,
            pages: Mutex::new(pages),
            calls: Mutex::new(Vec::new()),
        }
    }

    fn record(page: &Page) -> Value {
        let mut record = recorded("page.json");
        record["id"] = json!(page.id.to_string());
        record["title"] = json!(format!("page {}", page.id));
        record["history"]["createdDate"] = json!(utc_time(page.created));
        record["version"]["number"] = json!(page.version);
        record["version"]["when"] = json!(utc_time(page.updated));
        record["_links"]["webui"] = json!(format!("/spaces/{SPACE}/pages/{}", page.id));
        record["_links"]["self"] = json!(format!("{SITE}/rest/api/content/{}", page.id));
        record
    }

    /// The CQL the binding sends, read strictly: the space, and the instant a resume's literal
    /// names in this zone. Anything else is no query this simulation answers.
    fn parse(&self, cql: &str) -> Option<(String, Option<i64>)> {
        let rest = cql.strip_prefix("space = \"")?;
        let (space, rest) = rest.split_once('"')?;
        let rest = rest.strip_prefix(" and type = page")?;
        if let Some(rest) = rest.strip_prefix(" and lastmodified >= \"") {
            let (literal, rest) = rest.split_once('"')?;
            if rest != " order by lastmodified asc" || literal.len() != 16 {
                return None;
            }
            let since = wall(literal)? - self.offset * 60_000;
            return Some((space.to_owned(), Some(since)));
        }
        (rest == " order by lastmodified asc").then(|| (space.to_owned(), None))
    }

    fn answer(&self, path: &[&str], query: &[(&str, String)]) -> (u16, Value) {
        let get = |name: &str| {
            query
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.as_str())
        };
        let refused = |status: u16, message: String| {
            (
                status,
                json!({"statusCode": status,
                       "data": {"authorized": status != 404, "valid": status == 404,
                                "errors": [], "successful": false},
                       "message": message}),
            )
        };
        match path {
            ["rest", "api", "space", key] if *key == SPACE => {
                (200, space(65_538, SPACE, "Fixture Space", "global"))
            }
            ["rest", "api", "space", key] => refused(404, format!("No space with key : {key}")),
            ["rest", "api", "content", "search"] => {
                let Some((space, since)) = get("cql").and_then(|cql| self.parse(cql)) else {
                    return refused(
                        400,
                        "The CQL query is not one this simulation answers.".into(),
                    );
                };
                if space != SPACE || get("expand") != Some(EXPAND) {
                    return refused(
                        400,
                        "Only the fixture space, expanded, is simulated.".into(),
                    );
                }
                let Some(asked) = get("limit").map_or(Some(25), |n| n.parse::<usize>().ok()) else {
                    return refused(400, "limit is not a number.".into());
                };
                let start = match get("cursor") {
                    None => 0,
                    Some(token) => match token
                        .strip_prefix("_f_")
                        .and_then(|rest| rest.strip_suffix('='))
                        .and_then(|n| n.parse().ok())
                    {
                        Some(start) => start,
                        None => return refused(400, "The cursor is invalid.".into()),
                    },
                };
                let mut listed: Vec<Page> = self
                    .pages
                    .lock()
                    .unwrap()
                    .iter()
                    .filter(|page| since.is_none_or(|since| page.updated >= since))
                    .copied()
                    .collect();
                listed.sort_by_key(|page| (page.updated, page.id));
                let end = listed.len().min(start + asked.min(self.page));
                let results: Vec<Value> = listed[start.min(end)..end]
                    .iter()
                    .map(Self::record)
                    .collect();
                let next = (end < listed.len()).then(|| {
                    format!(
                        "/rest/api/content/search?next=true&cursor=_f_{end}%3D&expand=history%2Cversion&limit={asked}&start={end}"
                    )
                });
                (
                    200,
                    json!({"results": results, "start": start, "limit": asked,
                           "size": end - start.min(end), "_links": links(next.as_deref())}),
                )
            }
            _ => refused(404, "Not found".into()),
        }
    }

    /// The CQL of every content search this simulation answered, in order.
    fn searches(&self) -> Vec<String> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(path, _)| path == &segments(&["rest", "api", "content", "search"]))
            .map(|(_, query)| {
                query
                    .iter()
                    .find(|(key, _)| key == "cql")
                    .map(|(_, value)| value.clone())
                    .expect("a cql")
            })
            .collect()
    }
}

#[async_trait::async_trait]
impl AuthenticatedHttp for Site {
    async fn get(&self, path: &[&str], query: &[(&str, String)]) -> Result<HttpResponse> {
        self.calls.lock().unwrap().push(record(path, query));
        let (status, body) = self.answer(path, query);
        Ok(respond(status, &body))
    }
}

/// Three pages around a watermark at 09:00 UTC: one hours before it, the one at it, and one two
/// and a half hours after it, which a resume must return in every zone.
fn around_the_watermark() -> Vec<Page> {
    let created = at("2026-10-01 08:00:00");
    vec![
        Page {
            id: 1001,
            version: 2,
            created,
            updated: at("2026-10-06 05:00:00"),
        },
        Page {
            id: 1002,
            version: 4,
            created,
            updated: at("2026-10-06 09:00:10"),
        },
        Page {
            id: 1003,
            version: 1,
            created,
            updated: at("2026-10-06 11:30:00"),
        },
    ]
}

/// A first read of two pages, then a resume from the watermark it returned, both in the zone
/// `offset` minutes east of UTC: the resume's page and the site's record of the searches.
async fn first_then_resume(offset: i64) -> (Result<Value>, Site) {
    let engine = engine();
    let site = Site::new(offset, around_the_watermark());
    let first = engine
        .read(
            &site,
            INSTANCE,
            ITEMS,
            json!({"container": SPACE, "limit": 2}),
        )
        .await
        .unwrap();
    assert_eq!(ids(&first), ["1001", "1002"], "offset {offset}");
    assert_eq!(first["complete"], json!(false), "offset {offset}");
    let resumed = engine
        .read(
            &site,
            INSTANCE,
            ITEMS,
            json!({"container": SPACE, "limit": 10, "watermark": first["next_watermark"]}),
        )
        .await;
    (resumed, site)
}

// ---- the cases -------------------------------------------------------------------------------

/// The declaration names Confluence v1's own operations and is carried by the committed
/// `confluence-v1` bundle: the engine adds exactly the family's two reads, under the binding's
/// profile. The profile states what v1 lets it observe: no deletions, the one kind word `space`,
/// the version number as an opaque revision, and every space private, so the declaration carries
/// no visibility rule. Spaces are listed whatever their type, global and personal alike. The
/// content search reads the `lastmodified` literal in a zone from UTC-12:00 to UTC+14:00 and
/// continues by the `cursor` of its next link.
#[test]
fn the_confluence_v1_feed_declaration_binds_the_family_under_its_profile() {
    let feed = &declaration();
    assert_eq!(feed["profile"], PROFILE);
    assert_eq!(
        feed["capabilities"],
        json!({"deletions": "not-observed", "kind": "fixed-word",
               "revision": "opaque", "visibility": "all-private"})
    );
    assert_eq!(feed["containers"]["operation_id"], "getSpaces");
    assert_eq!(
        feed["containers"]["lookup"],
        json!({"operation_id": "getSpace", "parameter": "spaceKey", "record": ""})
    );
    assert_eq!(feed["containers"]["query"], json!({"status": "current"}));
    assert_eq!(
        feed["containers"]["cursor"],
        json!({"parameter": "start", "next": "/_links/next", "url_parameter": "start"})
    );
    assert_eq!(feed["containers"]["id"], "/key");
    assert_eq!(feed["containers"]["kind_word"], "space");
    assert!(feed["containers"].get("kind").is_none());
    assert!(feed["containers"].get("visibility").is_none());
    let items = &feed["items"];
    assert_eq!(items["operation_id"], "searchContentByCQL");
    assert_eq!(items["query"], json!({"expand": EXPAND}));
    assert!(items.get("container").is_none());
    assert!(items.get("parent").is_none());
    assert!(items.get("deleted").is_none());
    assert_eq!(items["revision"], "/version/number");
    assert_eq!(items["updated_at"], "/version/when");
    assert_eq!(items["created_at"], "/history/createdDate");
    assert_eq!(items["body"]["pointer"], "/title");
    assert_eq!(items["url"], "/_links/webui");
    assert_eq!(items["url_base"], "/_links/base");
    let time = &items["position"]["value"];
    assert_eq!(items["position"]["kind"], "time");
    assert_eq!(time["precision"], "minute");
    assert_eq!(
        time["page"],
        json!({"parameter": "cursor", "next": "/_links/next", "url_parameter": "cursor"})
    );
    assert_eq!(time["query"]["language"], "cql");
    assert_eq!(time["query"]["parameter"], "cql");
    assert_eq!(
        time["query"]["utc_offsets"],
        json!({"least": -720, "greatest": 840})
    );
    let engine = engine();
    let declared = engine.declarations(&[Effect::Read]);
    let ids: Vec<&str> = declared.iter().map(|o| o.id.as_str()).collect();
    assert_eq!(ids, [CONTAINERS, ITEMS]);
    for operation in &declared {
        assert_eq!(operation.contract, FEED_CONTRACT);
        assert_eq!(operation.profile, PROFILE);
    }
    assert_eq!(
        declared[1].input_schema["properties"]["limit"]["maximum"],
        json!(100)
    );
    assert!(engine.declarations(&[Effect::Write]).is_empty());
}

/// The binding is not shipped: neither the `confluence-v1` selection file nor the REST v2
/// `confluence` one carries a `feed`, and the engine over each declares no read of the family.
/// The connector definition format every shipped selection round-trips through
/// (`crates/connectors-catalog/tests/definition_import.rs`) has no next-link parameter, time query
/// or `utc_offsets` to carry the declaration in
/// (`adapters/catalog/contracts/feed/v1alpha1/confluence-v1.md`). REST v2 has no time filter for
/// a feed to resume by at all.
#[test]
fn the_shipped_confluence_v1_selection_declares_no_feed() {
    for provider in [PROVIDER, "confluence"] {
        let shipped: Value = serde_json::from_slice(
            &std::fs::read(
                root()
                    .join("providers")
                    .join(provider)
                    .join("operations.json"),
            )
            .unwrap(),
        )
        .unwrap();
        assert!(shipped.get("feed").is_none(), "{provider}");
        let selections: Vec<Selection> =
            serde_json::from_value(shipped["operations"].clone()).unwrap();
        let bundle = bundle::load(&root().join("generated/bundles"), provider).unwrap();
        let engine = Engine::new(&bundle, BASE, &selections).unwrap();
        let declared = engine.declarations(&[Effect::Read]);
        assert!(!declared.is_empty(), "{provider}");
        assert!(
            declared.iter().all(|o| o.contract != FEED_CONTRACT),
            "{provider}"
        );
    }
}

/// `feed.containers` lists the current spaces the user can view, global and personal alike, one
/// v1 page a call, and continues at the `start` of the answer's `_links.next` until an answer
/// carries none. A space is `space`, identified by its key, named by its name, and listed
/// `private` whatever its type.
#[tokio::test]
async fn containers_are_current_spaces_paged_by_the_start_of_the_next_link() {
    let engine = engine();
    let http = answers(vec![
        (
            200,
            json!({"results": [space(65_538, SPACE, "Fixture Space", "global"),
                               space(65_539, "~fixture-user-1", "Fixture User 1", "personal")],
                   "start": 0, "limit": 2, "size": 2,
                   "_links": links(Some("/rest/api/space?status=current&limit=2&start=2"))}),
        ),
        (
            200,
            json!({"results": [space(65_540, "OPS", "Operations", "global")],
                   "start": 2, "limit": 2, "size": 1, "_links": links(None)}),
        ),
    ]);
    let first = engine
        .read(&http, INSTANCE, CONTAINERS, json!({"limit": 2}))
        .await
        .unwrap();
    assert_eq!(
        first["containers"],
        json!([
            {"id": "DOCS", "name": "Fixture Space", "kind": "space", "visibility": "private"},
            {"id": "~fixture-user-1", "name": "Fixture User 1", "kind": "space",
             "visibility": "private"},
        ])
    );
    assert_eq!(first["complete"], json!(false));
    let second = engine
        .read(
            &http,
            INSTANCE,
            CONTAINERS,
            json!({"limit": 2, "cursor": first["next_cursor"]}),
        )
        .await
        .unwrap();
    assert_eq!(
        second["containers"],
        json!([{"id": "OPS", "name": "Operations", "kind": "space", "visibility": "private"}])
    );
    assert_eq!(second["complete"], json!(true));
    assert_eq!(second["next_cursor"], Value::Null);
    let listing = |extra: &[(&str, &str)]| {
        let mut query = vec![("status", "current"), ("limit", "2")];
        query.extend_from_slice(extra);
        (segments(&["rest", "api", "space"]), pairs(&query))
    };
    assert_eq!(
        *http.calls.lock().unwrap(),
        [listing(&[]), listing(&[("start", "2")])]
    );
}

/// A next link that lost its `start` cannot be continued: the listing says so rather than
/// claiming it is complete.
#[tokio::test]
async fn a_space_next_link_without_its_start_is_unavailable() {
    let engine = engine();
    let http = answers(vec![(
        200,
        json!({"results": [space(65_538, SPACE, "Fixture Space", "global")],
               "start": 0, "limit": 1, "size": 1,
               "_links": links(Some("/rest/api/space?status=current&limit=1"))}),
    )]);
    let error = engine
        .read(&http, INSTANCE, CONTAINERS, json!({"limit": 1}))
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Unavailable);
}

/// `feed.items` reads the space first, then its pages by CQL, oldest `lastmodified` first,
/// expanding the history and the version only. A first read names the space and no time; one
/// call reads further v1 pages by the `cursor` of `_links.next`, decoded, until it holds `limit`
/// pages. The item is the page's content `id`; its revision is its version number; its times are
/// `history.createdDate` and `version.when`; its body is the title as plain text; its author is
/// the page's creator; its url is `_links.webui` under the answer's `_links.base`.
#[tokio::test]
async fn items_are_pages_read_oldest_modification_first_across_cursor_pages() {
    let engine = engine();
    let mut site = Site::new(0, around_the_watermark());
    site.page = 2;
    let first = engine
        .read(
            &site,
            INSTANCE,
            ITEMS,
            json!({"container": SPACE, "limit": 3}),
        )
        .await
        .unwrap();
    assert_eq!(ids(&first), ["1001", "1002", "1003"]);
    assert_eq!(
        first["items"][1],
        json!({
            "id": "1002",
            "revision": "4",
            "created_at": "2026-10-01T08:00:00.000Z",
            "updated_at": "2026-10-06T09:00:10.000Z",
            "author": {"id": "fixture-user-1", "display_name": "Fixture User 1"},
            "body": {"representation": "text/plain", "bytes": 9, "content": "page 1002",
                     "truncated": false, "truncation": []},
            "url": "https://site.example.test/wiki/spaces/DOCS/pages/1002",
            "parent": null,
            "deleted": false,
        })
    );
    assert_eq!(first["complete"], json!(true));
    assert_eq!(first["provenance"]["profile"], json!(PROFILE));
    let search = |extra: &[(&str, &str)]| {
        let mut query = vec![("cql", FIRST), ("expand", EXPAND), ("limit", "3")];
        query.extend_from_slice(extra);
        (
            segments(&["rest", "api", "content", "search"]),
            pairs(&query),
        )
    };
    assert_eq!(
        *site.calls.lock().unwrap(),
        [
            (segments(&["rest", "api", "space", SPACE]), Vec::new()),
            search(&[]),
            search(&[("cursor", "_f_2=")]),
        ]
    );
}

/// In a zone behind UTC, Confluence reads the `lastmodified` literal hours later than the
/// watermark it was formed from. The binding sends the kept minute less 12 hours, so the page
/// changed two and a half hours after the watermark is returned, not skipped; the one before it
/// is dropped, since the watermark already holds it.
#[tokio::test]
async fn a_zone_behind_utc_skips_no_page_after_the_watermark() {
    let (resumed, site) = first_then_resume(-300).await;
    let resumed = resumed.unwrap();
    assert_eq!(ids(&resumed), ["1003"]);
    assert_eq!(resumed["complete"], json!(true));
    assert_eq!(
        site.searches()[1],
        "space = \"DOCS\" and type = page and lastmodified >= \"2026-10-05 21:00\" order by lastmodified asc"
    );
    let resume = &site.calls.lock().unwrap()[3];
    assert!(resume.1.contains(&("limit".to_owned(), "100".to_owned())));
}

/// In a zone ahead of UTC, Confluence reads the literal hours earlier and answers pages the
/// watermark already holds before the one it was formed from. They are dropped, not taken for an
/// answer out of order, and the page changed after the watermark is returned.
#[tokio::test]
async fn a_zone_ahead_of_utc_resumes_and_does_not_answer_unavailable() {
    let (resumed, site) = first_then_resume(840).await;
    let resumed = resumed.unwrap();
    assert_eq!(ids(&resumed), ["1003"]);
    assert_eq!(resumed["complete"], json!(true));
    assert_eq!(
        since_of(&site.searches()[1]),
        Some("2026-10-05 21:00"),
        "the literal does not depend on the zone"
    );
}

/// Every zone from UTC-12:00 to UTC+14:00, in quarter hours: the resume returns exactly the page
/// changed after the watermark, its version and its time as Confluence wrote them.
#[tokio::test]
async fn every_zone_from_utc_minus_12_to_utc_plus_14_resumes_without_a_skip() {
    let mut failed = Vec::new();
    for offset in (-720..=840).step_by(15) {
        let (resumed, _) = first_then_resume(offset).await;
        let expected = (
            vec!["1003".to_owned()],
            json!("1"),
            json!(utc_time(at("2026-10-06 11:30:00"))),
        );
        match resumed {
            Ok(page) => {
                let got = (
                    ids(&page)
                        .into_iter()
                        .map(str::to_owned)
                        .collect::<Vec<_>>(),
                    page["items"][0]["revision"].clone(),
                    page["items"][0]["updated_at"].clone(),
                );
                if got != expected {
                    failed.push(format!("{offset}: {got:?}"));
                }
            }
            Err(error) => failed.push(format!("{offset}: {:?} {}", error.code, error.message)),
        }
    }
    assert!(
        failed.is_empty(),
        "zones that skipped or refused: {failed:#?}"
    );
}

/// The revision is the version number, not the time: a page given a new version within the kept
/// minute is returned once more under its new number, its time later in that minute, while the
/// version the watermark holds is not returned again.
#[tokio::test]
async fn a_new_version_within_the_kept_minute_is_returned_again() {
    let engine = engine();
    let site = Site::new(0, around_the_watermark());
    let first = engine
        .read(
            &site,
            INSTANCE,
            ITEMS,
            json!({"container": SPACE, "limit": 2}),
        )
        .await
        .unwrap();
    assert_eq!(ids(&first), ["1001", "1002"]);
    assert_eq!(first["items"][1]["revision"], "4");
    {
        let mut pages = site.pages.lock().unwrap();
        pages[1].version = 5;
        pages[1].updated = at("2026-10-06 09:00:40");
    }
    let resumed = engine
        .read(
            &site,
            INSTANCE,
            ITEMS,
            json!({"container": SPACE, "limit": 10, "watermark": first["next_watermark"]}),
        )
        .await
        .unwrap();
    assert_eq!(ids(&resumed), ["1002", "1003"]);
    assert_eq!(resumed["items"][0]["revision"], "5");
    assert_eq!(
        resumed["items"][0]["updated_at"],
        "2026-10-06T09:00:40.000Z"
    );
    assert_eq!(resumed["complete"], json!(true));
}

/// The window a resume re-reads reaches 26 hours before the kept minute in a zone at UTC+14:00.
/// When more pages changed within it than the 10 v1 pages one call reads hold, the resume answers
/// `unavailable` rather than move past a page it has not read: here 1,100 pages one minute apart,
/// read 100 a call, stop at the watermark the 10th call returned.
#[tokio::test]
async fn a_resume_whose_answers_hold_only_re_read_pages_is_unavailable_not_a_skip() {
    let engine = engine();
    let start = at("2026-10-05 00:00:00");
    let pages: Vec<Page> = (0..1100)
        .map(|n| Page {
            id: 20_000 + n,
            version: 1,
            created: start,
            updated: start + n as i64 * 60_000,
        })
        .collect();
    let site = Site::new(840, pages);
    let mut watermark = Value::Null;
    let mut returned = 0;
    let refusal = loop {
        let mut input = json!({"container": SPACE, "limit": 100});
        if !watermark.is_null() {
            input["watermark"] = watermark.clone();
        }
        match engine.read(&site, INSTANCE, ITEMS, input).await {
            Ok(page) => {
                returned += page["items"].as_array().unwrap().len();
                assert_eq!(page["complete"], json!(false), "after {returned}");
                watermark = page["next_watermark"].clone();
            }
            Err(error) => break error,
        }
    };
    assert_eq!(refusal.code, ErrorCode::Unavailable);
    assert_eq!(
        refusal.message,
        "more items changed in the re-read window than the pages one read may request hold"
    );
    assert_eq!(returned, 1000);
}

/// A space Confluence does not show this user is `not_found`, and nothing else is read.
#[tokio::test]
async fn a_space_confluence_does_not_show_is_not_found() {
    let engine = engine();
    let site = Site::new(0, around_the_watermark());
    let error = engine
        .read(
            &site,
            INSTANCE,
            ITEMS,
            json!({"container": "HIDDEN", "limit": 10}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::NotFound);
    assert_eq!(site.calls.lock().unwrap().len(), 1);
}

/// A watermark in the engine's own format for this instance and `space`, kept at `time`.
fn kept(space: &str, time: &str) -> String {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    let mark = json!({"format": "catalog-feed/1", "profile": PROFILE, "instance": INSTANCE,
                      "container": space, "time": time, "seen": [["1002", "4"]]});
    URL_SAFE_NO_PAD.encode(serde_json::to_vec(&mark).unwrap())
}

/// A watermark kept in the first hours of the year 0000 is one the binding cannot resume: the
/// literal it would send, 12 hours before the kept minute, falls before the year 0000. It is
/// `stale_cursor` only once the space lookup and the search, sent as a first read with no
/// `lastmodified` clause, have answered; a space Confluence does not show this user stays
/// `not_found`, and nothing is searched.
#[tokio::test]
async fn a_watermark_before_the_year_zero_is_stale_only_after_the_search_answered() {
    let engine = engine();
    let site = Site::new(0, around_the_watermark());
    let error = engine
        .read(
            &site,
            INSTANCE,
            ITEMS,
            json!({"container": SPACE, "limit": 10,
                   "watermark": kept(SPACE, "0000-01-01T05:00:00.000Z")}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::StaleCursor);
    assert_eq!(site.searches(), [FIRST]);

    let site = Site::new(0, around_the_watermark());
    let error = engine
        .read(
            &site,
            INSTANCE,
            ITEMS,
            json!({"container": "HIDDEN", "limit": 10,
                   "watermark": kept("HIDDEN", "0000-01-01T05:00:00.000Z")}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::NotFound);
    assert!(site.searches().is_empty());

    let site = Site::new(0, around_the_watermark());
    let resumed = engine
        .read(
            &site,
            INSTANCE,
            ITEMS,
            json!({"container": SPACE, "limit": 10,
                   "watermark": kept(SPACE, "0000-01-01T12:00:00.000Z")}),
        )
        .await
        .unwrap();
    assert_eq!(ids(&resumed), ["1001", "1002", "1003"]);
    assert_eq!(since_of(&site.searches()[0]), Some("0000-01-01 00:00"));
}

/// A bulk change of 900 pages within one UTC minute, 66 milliseconds apart, in a zone at
/// UTC+14:00: read 100 a call, every page comes back exactly once and the last call is
/// `complete`. The watermark holds only the pages at its kept instant, so it stays far below its
/// 16 KiB ceiling however many pages share the kept minute, and the bound is the 10 v1 pages one
/// call re-reads.
#[tokio::test]
async fn nine_hundred_pages_changed_in_one_minute_are_all_returned_once() {
    let engine = engine();
    let start = at("2026-10-06 09:00:00");
    let pages: Vec<Page> = (0..900)
        .map(|n| Page {
            id: 20_000 + n,
            version: 1,
            created: start,
            updated: start + n as i64 * 66,
        })
        .collect();
    let site = Site::new(840, pages);
    let mut watermark = Value::Null;
    let mut returned: Vec<String> = Vec::new();
    let mut reads = 0;
    loop {
        reads += 1;
        assert!(reads <= 11, "no complete page after {reads} reads");
        let mut input = json!({"container": SPACE, "limit": 100});
        if !watermark.is_null() {
            input["watermark"] = watermark.clone();
        }
        let page = engine
            .read(&site, INSTANCE, ITEMS, input)
            .await
            .unwrap_or_else(|error| {
                panic!(
                    "read {reads}, after {} pages (watermark {} bytes): {:?} {}",
                    returned.len(),
                    watermark.as_str().map_or(0, str::len),
                    error.code,
                    error.message
                )
            });
        returned.extend(ids(&page).into_iter().map(str::to_owned));
        watermark = page["next_watermark"].clone();
        if page["complete"] == json!(true) {
            break;
        }
    }
    let unique: std::collections::BTreeSet<&String> = returned.iter().collect();
    assert_eq!(unique.len(), 900, "pages returned: {}", returned.len());
    assert_eq!(returned.len(), 900, "a page was returned twice");
}

/// `count` pages with ids from `first`, each at version 1, whose last version is one
/// millisecond, read 100 a call from a first read until a call is `complete` or refused: the
/// ids returned, and the refusal.
async fn one_millisecond(first: u64, count: u64) -> (Vec<String>, Option<(ErrorCode, String)>) {
    let engine = engine();
    let at = at("2026-10-06 09:00:00");
    let pages: Vec<Page> = (0..count)
        .map(|n| Page {
            id: first + n,
            version: 1,
            created: at,
            updated: at,
        })
        .collect();
    let site = Site::new(0, pages);
    let mut watermark = Value::Null;
    let mut returned = Vec::new();
    for _ in 0..11 {
        let mut input = json!({"container": SPACE, "limit": 100});
        if !watermark.is_null() {
            input["watermark"] = watermark.clone();
        }
        match engine.read(&site, INSTANCE, ITEMS, input).await {
            Ok(page) => {
                returned.extend(ids(&page).into_iter().map(str::to_owned));
                watermark = page["next_watermark"].clone();
                if page["complete"] == json!(true) {
                    return (returned, None);
                }
            }
            Err(error) => return (returned, Some((error.code, error.message))),
        }
    }
    panic!("no complete page after 11 reads of {count} pages");
}

/// The watermark holds the pairs of the pages sharing its kept instant, so the 16 KiB ceiling
/// bounds how many pages one millisecond may hold: 866 with five-digit ids and 638 with
/// ten-digit ones here, all returned once. One more is `unavailable` at the read whose
/// watermark would pass the ceiling, after the pages earlier reads returned, none skipped.
#[tokio::test]
async fn pages_sharing_one_millisecond_are_bounded_by_the_watermark_ceiling() {
    let ceiling = Some((
        ErrorCode::Unavailable,
        "the watermark would exceed its ceiling; too many items share one instant".to_owned(),
    ));
    for (first, held, before) in [(20_000, 866, 800), (1_000_000_000, 638, 600)] {
        let expected: Vec<String> = (0..held).map(|n| (first + n).to_string()).collect();
        let (returned, refusal) = one_millisecond(first, held).await;
        assert_eq!(refusal, None, "{first}");
        assert_eq!(returned, expected, "{first}");
        let (returned, refusal) = one_millisecond(first, held + 1).await;
        assert_eq!(refusal, ceiling, "{first}");
        assert_eq!(returned, expected[..before], "{first}");
    }
}

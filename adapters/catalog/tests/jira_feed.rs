//! story:jira-feed-binding: a Jira declaration binds `datasource.feed/v1alpha1` as data
//! (`adapters/catalog/contracts/feed/v1alpha1/jira.md`), and the catalog engine reads it over the
//! committed Jira bundle. The declaration is the fixture `tests/feed/jira/feed.json`, not part of
//! the shipped Jira selection file, which carries no `feed` until the connector definition format
//! every shipped selection round-trips through can express one. A container is a project the user
//! can view; an item is one of its issues. Every answer here is a Jira record in the shape of
//! `tests/feed/jira/*.json`, taken from the pinned upstream description, not recorded from a
//! running Jira; the requests the engine sends are recorded and checked. The family's suite runs against the same declaration in
//! `crates/connectors-build/src/jira_feed_conformance.rs`.
//!
//! Jira reads a JQL date literal in the calling user's profile zone, which the binding is not
//! told. [`Zoned`] answers the issue search as Jira does in one such zone: it reads the
//! `updated >=` literal as wall time there and writes `created` and `updated` with the `±hhmm`
//! offset of the zone it writes times in, by default the same one. The zone cases hold that no
//! profile zone from UTC-12:00 to UTC+14:00 makes a resume skip an issue or answer `unavailable`.
use connectors_catalog::bundle;
use connectors_catalog_provider::{
    Effect, Engine, Selection,
    feed::{CONTAINERS, Declaration, FEED_CONTRACT, ITEMS},
};
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{collections::VecDeque, path::Path, sync::Mutex};

const PROFILE: &str = "jira-issues/1";
const INSTANCE: &str = "jira-feed-instance";
const BASE: &str = "/rest/api/3";
const PROJECT: &str = "10000";
const FIELDS: &str = "summary,created,updated,creator";

type Call = (Vec<String>, Vec<(String, String)>);

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn recorded(name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(root().join("tests/feed/jira").join(name)).unwrap())
        .unwrap()
}

fn declaration() -> Value {
    recorded("feed.json")
}

fn engine() -> Engine {
    let feed: Declaration = serde_json::from_value(declaration()).unwrap();
    let bundle = bundle::load(&root().join("generated/bundles"), "jira").unwrap();
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

/// A project as Jira lists and reads it, with the fields the binding reads replaced.
fn project(id: &str, name: &str, private: bool) -> Value {
    let mut project = recorded("project.json");
    project["id"] = json!(id);
    project["self"] = json!(format!(
        "https://tracker.example.test/rest/api/3/project/{id}"
    ));
    project["name"] = json!(name);
    project["isPrivate"] = json!(private);
    project
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

/// `at` as Jira writes it in a zone `offset` minutes east of UTC:
/// `YYYY-MM-DDTHH:MM:SS.mmm±hhmm`.
fn jira_time(at: i64, offset: i64) -> String {
    let local = at + offset * 60_000;
    let (year, month, day) = civil_from_days(local.div_euclid(86_400_000));
    let rem = local.rem_euclid(86_400_000);
    let sign = if offset < 0 { '-' } else { '+' };
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}{sign}{:02}{:02}",
        rem / 3_600_000,
        rem / 60_000 % 60,
        rem / 1000 % 60,
        rem % 1000,
        offset.abs() / 60,
        offset.abs() % 60
    )
}

/// `at` as the engine returns an instant Jira wrote: in UTC, `Z`, the milliseconds kept.
fn utc_time(at: i64) -> String {
    jira_time(at, 0).replace("+0000", "Z")
}

/// The `{since}` literal a resume sends, read back out of its JQL.
fn since_of(jql: &str) -> Option<&str> {
    let (_, rest) = jql.split_once("AND updated >= \"")?;
    Some(rest.split_once('"')?.0)
}

// ---- a Jira that answers in one profile zone -------------------------------------------------

/// One issue of the simulated project: its number, and its creation and last update, in
/// milliseconds since the epoch.
#[derive(Clone, Copy)]
struct Issue {
    number: u64,
    created: i64,
    updated: i64,
}

/// Jira's issue search for one project, answering as Jira does for a user whose profile zone is
/// `offset` minutes east of UTC, and writing times `written` minutes east of it (the pinned
/// upstream writes them "in the system default user time zone", which need not be the
/// profile's); at most `page` issues an answer, as Jira may answer fewer than asked for.
struct Zoned {
    offset: i64,
    written: i64,
    page: usize,
    issues: Vec<Issue>,
    calls: Mutex<Vec<Call>>,
}

impl Zoned {
    fn new(offset: i64, issues: Vec<Issue>) -> Self {
        Self {
            offset,
            written: offset,
            page: 100,
            issues,
            calls: Mutex::new(Vec::new()),
        }
    }

    fn issue(&self, issue: &Issue) -> Value {
        let mut record = recorded("issue.json");
        record["id"] = json!(issue.number.to_string());
        record["key"] = json!(format!("FIX-{}", issue.number - 10_000));
        record["self"] = json!(format!(
            "https://tracker.example.test/rest/api/3/issue/{}",
            issue.number
        ));
        record["fields"]["summary"] = json!(format!("issue {}", issue.number));
        record["fields"]["created"] = json!(jira_time(issue.created, self.written));
        record["fields"]["updated"] = json!(jira_time(issue.updated, self.written));
        record
    }

    /// The JQL the binding sends, read strictly: the project, and the instant a resume's
    /// literal names in this zone. Anything else is no query this simulation answers.
    fn parse(&self, jql: &str) -> Option<(String, Option<i64>)> {
        let rest = jql.strip_prefix("project = \"")?;
        let (project, rest) = rest.split_once('"')?;
        if let Some(rest) = rest.strip_prefix(" AND updated >= \"") {
            let (literal, rest) = rest.split_once('"')?;
            if rest != " ORDER BY updated ASC" || literal.len() != 16 {
                return None;
            }
            let since = wall(literal)? - self.offset * 60_000;
            return Some((project.to_owned(), Some(since)));
        }
        (rest == " ORDER BY updated ASC").then(|| (project.to_owned(), None))
    }

    fn answer(&self, path: &[&str], query: &[(&str, String)]) -> (u16, Value) {
        let get = |name: &str| {
            query
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.as_str())
        };
        match path {
            ["project", id] if *id == PROJECT => (200, project(PROJECT, "Fixture Project", false)),
            ["project", id] => (
                404,
                json!({"errorMessages": [format!("No project could be found with key '{id}'.")],
                       "errors": {}}),
            ),
            ["search", "jql"] => {
                let refused =
                    |message: &str| (400, json!({"errorMessages": [message], "errors": {}}));
                let Some((project, since)) = get("jql").and_then(|jql| self.parse(jql)) else {
                    return refused("The JQL query is not one this simulation answers.");
                };
                if project != PROJECT {
                    return refused("The value does not exist for the field 'project'.");
                }
                let Some(asked) = get("maxResults").map_or(Some(50), |n| n.parse::<usize>().ok())
                else {
                    return refused("maxResults is not a number.");
                };
                let start = match get("nextPageToken") {
                    None => 0,
                    Some(token) => match token.strip_prefix("page-").and_then(|n| n.parse().ok()) {
                        Some(start) => start,
                        None => return refused("The provided next page token is invalid."),
                    },
                };
                let mut listed: Vec<Issue> = self
                    .issues
                    .iter()
                    .filter(|issue| since.is_none_or(|since| issue.updated >= since))
                    .copied()
                    .collect();
                listed.sort_by_key(|issue| (issue.updated, issue.number));
                let end = listed.len().min(start + asked.min(self.page));
                let issues: Vec<Value> = listed[start.min(end)..end]
                    .iter()
                    .map(|issue| self.issue(issue))
                    .collect();
                let mut body = json!({"issues": issues, "isLast": end == listed.len()});
                if end < listed.len() {
                    body["nextPageToken"] = json!(format!("page-{end}"));
                }
                (200, body)
            }
            _ => (404, json!({"errorMessages": ["Not found"], "errors": {}})),
        }
    }

    /// The JQL of every issue search this simulation answered, in order.
    fn searches(&self) -> Vec<String> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(path, _)| path == &segments(&["search", "jql"]))
            .map(|(_, query)| {
                query
                    .iter()
                    .find(|(key, _)| key == "jql")
                    .map(|(_, value)| value.clone())
                    .expect("a jql")
            })
            .collect()
    }
}

#[async_trait::async_trait]
impl AuthenticatedHttp for Zoned {
    async fn get(&self, path: &[&str], query: &[(&str, String)]) -> Result<HttpResponse> {
        self.calls.lock().unwrap().push(record(path, query));
        let (status, body) = self.answer(path, query);
        Ok(respond(status, &body))
    }
}

fn at(text: &str) -> i64 {
    wall(text).expect("a wall time")
}

/// Three issues around a watermark at 09:00 UTC: one hours before it, the one at it, and one
/// two and a half hours after it, which a resume must return in every zone.
fn around_the_watermark() -> Vec<Issue> {
    vec![
        Issue {
            number: 10_001,
            created: at("2026-10-01 08:00:00"),
            updated: at("2026-10-06 05:00:00"),
        },
        Issue {
            number: 10_002,
            created: at("2026-10-01 08:00:00"),
            updated: at("2026-10-06 09:00:00"),
        },
        Issue {
            number: 10_003,
            created: at("2026-10-01 08:00:00"),
            updated: at("2026-10-06 11:30:00"),
        },
    ]
}

/// A first read of two issues, then a resume from the watermark it returned, both in the zone
/// `offset` minutes east of UTC: the resume's page and the provider's record of the searches.
async fn first_then_resume(offset: i64) -> (Result<Value>, Zoned) {
    let engine = engine();
    let jira = Zoned::new(offset, around_the_watermark());
    let first = engine
        .read(
            &jira,
            INSTANCE,
            ITEMS,
            json!({"container": PROJECT, "limit": 2}),
        )
        .await
        .unwrap();
    assert_eq!(ids(&first), ["10001", "10002"], "offset {offset}");
    assert_eq!(first["complete"], json!(false), "offset {offset}");
    let resumed = engine
        .read(
            &jira,
            INSTANCE,
            ITEMS,
            json!({"container": PROJECT, "limit": 10, "watermark": first["next_watermark"]}),
        )
        .await;
    (resumed, jira)
}

// ---- the cases -------------------------------------------------------------------------------

/// The declaration names Jira's own operations and is carried by the committed bundle: the
/// engine adds exactly the family's two reads, under the binding's profile. The profile
/// states what Jira lets it observe: no deletions, the one kind word `project`, the update time
/// as the revision, and every project private, so the declaration carries no visibility rule and
/// no `url`. The issue search reads the `updated` literal in a profile zone from UTC-12:00 to
/// UTC+14:00.
#[test]
fn the_jira_feed_declaration_binds_the_family_under_its_profile() {
    let feed = &declaration();
    assert_eq!(feed["profile"], PROFILE);
    assert_eq!(
        feed["capabilities"],
        json!({"deletions": "not-observed", "kind": "fixed-word",
               "revision": "update-time", "visibility": "all-private"})
    );
    assert_eq!(feed["containers"]["kind_word"], "project");
    assert!(feed["containers"].get("kind").is_none());
    assert!(feed["containers"].get("visibility").is_none());
    assert_eq!(
        feed["containers"]["query"],
        json!({"action": "view", "orderBy": "key", "status": "live"})
    );
    assert_eq!(
        feed["containers"]["cursor"],
        json!({"parameter": "startAt",
               "offset": {"complete": {"pointer": "/isLast", "equals": true}}})
    );
    assert!(feed["items"].get("container").is_none());
    assert!(feed["items"].get("url").is_none());
    assert!(feed["items"].get("deleted").is_none());
    assert_eq!(feed["items"]["revision"], feed["items"]["updated_at"]);
    assert_eq!(feed["items"]["body"]["pointer"], "/fields/summary");
    let query = &feed["items"]["position"]["value"]["query"];
    assert_eq!(query["language"], "jql");
    assert_eq!(
        query["utc_offsets"],
        json!({"least": -720, "greatest": 840})
    );
    assert_eq!(feed["containers"]["operation_id"], "searchProjects");
    assert_eq!(feed["containers"]["lookup"]["operation_id"], "getProject");
    assert_eq!(
        feed["items"]["operation_id"],
        "searchAndReconsileIssuesUsingJql"
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

/// The binding is not shipped: the Jira selection file carries no `feed`, and the engine over it
/// declares no read of the family. The connector definition format every shipped selection
/// round-trips through (`crates/connectors-catalog/tests/definition_import.rs`) has no offset
/// listing, time query or `utc_offsets` to carry the declaration in
/// (`adapters/catalog/contracts/feed/v1alpha1/jira.md`).
#[test]
fn the_shipped_jira_selection_declares_no_feed() {
    let shipped: Value = serde_json::from_slice(
        &std::fs::read(root().join("providers/jira/operations.json")).unwrap(),
    )
    .unwrap();
    assert!(shipped.get("feed").is_none());
    let selections: Vec<Selection> = serde_json::from_value(shipped["operations"].clone()).unwrap();
    let bundle = bundle::load(&root().join("generated/bundles"), "jira").unwrap();
    let engine = Engine::new(&bundle, BASE, &selections).unwrap();
    let declared = engine.declarations(&[Effect::Read]);
    assert!(!declared.is_empty());
    assert!(declared.iter().all(|o| o.contract != FEED_CONTRACT));
}

/// `feed.containers` lists the live projects the user can view, by key, one Jira page a call,
/// and continues at the index after the records the page carried (`startAt`) until Jira answers
/// `isLast: true`. A project is `project`, named by its name, and listed `private` whatever its
/// `isPrivate` says.
#[tokio::test]
async fn containers_are_viewable_live_projects_paged_by_offset_until_the_last() {
    let engine = engine();
    let http = answers(vec![
        (
            200,
            json!({"startAt": 0, "maxResults": 2, "total": 3, "isLast": false,
                   "values": [project("10000", "Fixture Project", false),
                              project("10010", "Second Project", true)]}),
        ),
        (
            200,
            json!({"startAt": 2, "maxResults": 2, "total": 3, "isLast": true,
                   "values": [project("10020", "Third Project", false)]}),
        ),
    ]);
    let first = engine
        .read(&http, INSTANCE, CONTAINERS, json!({"limit": 2}))
        .await
        .unwrap();
    assert_eq!(
        first["containers"],
        json!([
            {"id": "10000", "name": "Fixture Project", "kind": "project", "visibility": "private"},
            {"id": "10010", "name": "Second Project", "kind": "project", "visibility": "private"},
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
        json!([{"id": "10020", "name": "Third Project", "kind": "project",
                "visibility": "private"}])
    );
    assert_eq!(second["complete"], json!(true));
    assert_eq!(second["next_cursor"], Value::Null);
    let listing = |extra: &[(&str, &str)]| {
        let mut query = vec![
            ("action", "view"),
            ("orderBy", "key"),
            ("status", "live"),
            ("maxResults", "2"),
        ];
        query.extend_from_slice(extra);
        (segments(&["project", "search"]), pairs(&query))
    };
    assert_eq!(
        *http.calls.lock().unwrap(),
        [listing(&[]), listing(&[("startAt", "2")])]
    );
}

/// An empty page Jira does not call the last cannot be continued: the listing says so rather
/// than claiming it is complete.
#[tokio::test]
async fn an_empty_project_page_jira_does_not_call_the_last_is_unavailable() {
    let engine = engine();
    let http = answers(vec![(
        200,
        json!({"startAt": 0, "maxResults": 2, "total": 3, "isLast": false, "values": []}),
    )]);
    let error = engine
        .read(&http, INSTANCE, CONTAINERS, json!({"limit": 2}))
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Unavailable);
}

/// `feed.items` reads the project first, then its issues by JQL, oldest `updated` first, asking
/// for the summary, the two times and the creator only. A first read names the project and no
/// time; one call reads further Jira pages by `nextPageToken` until it holds `limit` issues. The
/// item is the issue's `id`; its revision is `updated` as Jira wrote it; its times are returned
/// in UTC; its body is the summary as plain text; its author is the issue's creator.
#[tokio::test]
async fn items_are_issues_read_oldest_update_first_across_jira_pages() {
    let engine = engine();
    let mut jira = Zoned::new(0, around_the_watermark());
    jira.page = 2;
    let first = engine
        .read(
            &jira,
            INSTANCE,
            ITEMS,
            json!({"container": PROJECT, "limit": 3}),
        )
        .await
        .unwrap();
    assert_eq!(ids(&first), ["10001", "10002", "10003"]);
    assert_eq!(
        first["items"][0],
        json!({
            "id": "10001",
            "revision": "2026-10-06T05:00:00.000+0000",
            "created_at": "2026-10-01T08:00:00.000Z",
            "updated_at": "2026-10-06T05:00:00.000Z",
            "author": {"id": "fixture-user-1", "display_name": "Fixture User 1"},
            "body": {"representation": "text/plain", "bytes": 11, "content": "issue 10001",
                     "truncated": false, "truncation": []},
            "url": null,
            "parent": null,
            "deleted": false,
        })
    );
    assert_eq!(first["complete"], json!(true));
    assert_eq!(first["provenance"]["profile"], json!(PROFILE));
    let search = |extra: &[(&str, &str)]| {
        let mut query = vec![
            ("fields", FIELDS),
            ("jql", "project = \"10000\" ORDER BY updated ASC"),
            ("maxResults", "3"),
        ];
        query.extend_from_slice(extra);
        (segments(&["search", "jql"]), pairs(&query))
    };
    assert_eq!(
        *jira.calls.lock().unwrap(),
        [
            (segments(&["project", PROJECT]), Vec::new()),
            search(&[]),
            search(&[("nextPageToken", "page-2")]),
        ]
    );
}

/// In a profile zone behind UTC, Jira reads the `updated` literal hours later than the watermark
/// it was formed from. The binding sends the kept minute less 12 hours, so the issue changed two
/// and a half hours after the watermark is returned, not skipped; the one before it is dropped,
/// since the watermark already holds it.
#[tokio::test]
async fn a_profile_zone_behind_utc_skips_no_issue_after_the_watermark() {
    let (resumed, jira) = first_then_resume(-300).await;
    let resumed = resumed.unwrap();
    assert_eq!(ids(&resumed), ["10003"]);
    assert_eq!(resumed["complete"], json!(true));
    let searches = jira.searches();
    assert_eq!(
        searches[1],
        "project = \"10000\" AND updated >= \"2026-10-05 21:00\" ORDER BY updated ASC"
    );
    let resume = &jira.calls.lock().unwrap()[3];
    assert!(
        resume
            .1
            .contains(&("maxResults".to_owned(), "100".to_owned()))
    );
}

/// In a profile zone ahead of UTC, Jira reads the literal hours earlier and answers issues the
/// watermark already holds before the one it was formed from. They are dropped, not taken for an
/// answer out of order, and the issue changed after the watermark is returned.
#[tokio::test]
async fn a_profile_zone_ahead_of_utc_resumes_and_does_not_answer_unavailable() {
    let (resumed, jira) = first_then_resume(840).await;
    let resumed = resumed.unwrap();
    assert_eq!(ids(&resumed), ["10003"]);
    assert_eq!(resumed["complete"], json!(true));
    assert_eq!(
        since_of(&jira.searches()[1]),
        Some("2026-10-05 21:00"),
        "the literal does not depend on the zone"
    );
}

/// The zone Jira writes times in need not be the one it reads the literal in: each written time
/// is read at its own offset, so a resume returns exactly the issue changed after the watermark
/// whichever two zones of UTC-12:00 to UTC+14:00 the two are.
#[tokio::test]
async fn the_zone_jira_writes_times_in_need_not_be_the_one_it_reads_the_literal_in() {
    for (offset, written) in [(-720, 840), (840, -720), (-570, 0), (345, -300)] {
        let engine = engine();
        let mut jira = Zoned::new(offset, around_the_watermark());
        jira.written = written;
        let first = engine
            .read(
                &jira,
                INSTANCE,
                ITEMS,
                json!({"container": PROJECT, "limit": 2}),
            )
            .await
            .unwrap();
        assert_eq!(ids(&first), ["10001", "10002"], "{offset}/{written}");
        let resumed = engine
            .read(
                &jira,
                INSTANCE,
                ITEMS,
                json!({"container": PROJECT, "limit": 10, "watermark": first["next_watermark"]}),
            )
            .await
            .unwrap();
        assert_eq!(ids(&resumed), ["10003"], "{offset}/{written}");
        assert_eq!(
            resumed["items"][0]["revision"],
            json!(jira_time(at("2026-10-06 11:30:00"), written)),
            "{offset}/{written}"
        );
        assert_eq!(resumed["complete"], json!(true), "{offset}/{written}");
    }
}

/// The zone Jira writes times in, changed between two reads: Jira then writes the same instants
/// with the new zone's offset, so the issue at the kept minute carries another revision text and
/// is returned once more, its time and body unchanged; nothing before the kept minute is returned
/// again and the issue after it is not skipped.
#[tokio::test]
async fn a_zone_change_between_reads_returns_the_kept_minute_again_under_its_new_text() {
    let engine = engine();
    let before = Zoned::new(120, around_the_watermark());
    let first = engine
        .read(
            &before,
            INSTANCE,
            ITEMS,
            json!({"container": PROJECT, "limit": 2}),
        )
        .await
        .unwrap();
    assert_eq!(ids(&first), ["10001", "10002"]);
    assert_eq!(
        first["items"][1]["revision"],
        "2026-10-06T11:00:00.000+0200"
    );
    let after = Zoned::new(-300, around_the_watermark());
    let resumed = engine
        .read(
            &after,
            INSTANCE,
            ITEMS,
            json!({"container": PROJECT, "limit": 10, "watermark": first["next_watermark"]}),
        )
        .await
        .unwrap();
    assert_eq!(ids(&resumed), ["10002", "10003"]);
    assert_eq!(
        resumed["items"][0]["revision"],
        "2026-10-06T04:00:00.000-0500"
    );
    assert_eq!(
        resumed["items"][0]["updated_at"],
        first["items"][1]["updated_at"]
    );
    assert_eq!(resumed["items"][0]["body"], first["items"][1]["body"]);
    assert_eq!(resumed["complete"], json!(true));
}

/// Every profile zone from UTC-12:00 to UTC+14:00, in quarter hours: the resume returns exactly
/// the issue changed after the watermark, its times in UTC and its revision as Jira wrote it in
/// that zone.
#[tokio::test]
async fn every_profile_zone_from_utc_minus_12_to_utc_plus_14_resumes_without_a_skip() {
    let mut failed = Vec::new();
    for offset in (-720..=840).step_by(15) {
        let (resumed, _) = first_then_resume(offset).await;
        let updated = at("2026-10-06 11:30:00");
        let expected = (
            vec!["10003".to_owned()],
            json!(jira_time(updated, offset)),
            json!(utc_time(updated)),
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

/// The window a resume re-reads reaches 26 hours before the kept minute in a zone at UTC+14:00.
/// When more issues changed within it than the 10 pages one call reads hold, the resume answers
/// `unavailable` rather than move past an issue it has not read: here 1,100 issues one minute
/// apart, read 100 a call, stop at the watermark the 10th call returned.
#[tokio::test]
async fn a_resume_whose_pages_hold_only_re_read_issues_is_unavailable_not_a_skip() {
    let engine = engine();
    let start = at("2026-10-05 00:00:00");
    let issues: Vec<Issue> = (0..1100)
        .map(|n| Issue {
            number: 20_000 + n,
            created: start,
            updated: start + n as i64 * 60_000,
        })
        .collect();
    let jira = Zoned::new(840, issues);
    let mut watermark = Value::Null;
    let mut returned = 0;
    let refusal = loop {
        let mut input = json!({"container": PROJECT, "limit": 100});
        if !watermark.is_null() {
            input["watermark"] = watermark.clone();
        }
        match engine.read(&jira, INSTANCE, ITEMS, input).await {
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

/// A project Jira does not show this user is `not_found`, and nothing else is read.
#[tokio::test]
async fn a_project_jira_does_not_show_is_not_found() {
    let engine = engine();
    let jira = Zoned::new(0, around_the_watermark());
    let error = engine
        .read(
            &jira,
            INSTANCE,
            ITEMS,
            json!({"container": "10099", "limit": 10}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::NotFound);
    assert_eq!(jira.calls.lock().unwrap().len(), 1);
}

/// `count` issues numbered from 20000 whose `updated` is one millisecond, read 100 a call from
/// a first read until a call is `complete` or refused: the ids returned, and the refusal.
async fn one_millisecond(count: u64) -> (Vec<String>, Option<(ErrorCode, String)>) {
    let engine = engine();
    let at = at("2026-10-06 09:00:00");
    let issues: Vec<Issue> = (0..count)
        .map(|n| Issue {
            number: 20_000 + n,
            created: at,
            updated: at,
        })
        .collect();
    let jira = Zoned::new(0, issues);
    let mut watermark = Value::Null;
    let mut returned = Vec::new();
    for _ in 0..10 {
        let mut input = json!({"container": PROJECT, "limit": 100});
        if !watermark.is_null() {
            input["watermark"] = watermark.clone();
        }
        match engine.read(&jira, INSTANCE, ITEMS, input).await {
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
    panic!("no complete page after 10 reads of {count} issues");
}

/// The watermark holds the pairs of the issues sharing its kept instant, and a revision is the
/// 28-byte `updated` text, so the 16 KiB ceiling bounds how many issues one millisecond may
/// hold: 296 with five-digit ids here, all returned once. One more is `unavailable` at the
/// read whose watermark would pass the ceiling, after the 200 issues earlier reads returned,
/// none skipped. How many share the kept minute does not count.
#[tokio::test]
async fn issues_sharing_one_millisecond_are_bounded_by_the_watermark_ceiling() {
    let (returned, refusal) = one_millisecond(296).await;
    assert_eq!(refusal, None);
    let expected: Vec<String> = (0..296).map(|n| (20_000 + n).to_string()).collect();
    assert_eq!(returned, expected);

    let (returned, refusal) = one_millisecond(297).await;
    assert_eq!(
        refusal,
        Some((
            ErrorCode::Unavailable,
            "the watermark would exceed its ceiling; the container id is too long or too many \
             items share one instant"
                .into()
        ))
    );
    assert_eq!(returned, expected[..200]);
}

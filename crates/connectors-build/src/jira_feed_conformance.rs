//! A Jira `datasource.feed/v1alpha1` binding for the catalog provider, held to the
//! `feed-binding` suite its profile's capabilities select, in five profile zones from UTC-12:00
//! to UTC+14:00.
//!
//! The binding is the declaration `adapters/catalog/tests/feed/jira/feed.json`, realized by the
//! catalog engine over the committed Jira bundle; its profile statement is
//! `adapters/catalog/contracts/feed/v1alpha1/jira.md`. It is not in the shipped Jira selection
//! file `adapters/catalog/providers/jira/operations.json`, since the connector definition format
//! every shipped selection round-trips through cannot yet express it. Nothing here maps a field
//! or forms a watermark: [`Jira`] only answers the requests the engine sends, from the suite's
//! provider state, the way the upstream description says Jira answers them.
//!
//! [`Jira`] is the remote service, for a user whose profile zone is `offset` minutes east of
//! UTC. A suite container is a project and a suite item one of its issues, rendered over the
//! shapes in `adapters/catalog/tests/feed/jira/`, which are taken from the pinned upstream
//! description, not recorded from a running Jira. It answers where the binding depends on it:
//! - `GET /project/search` lists the projects the user can view (`action=view`, the default),
//!   live ones only (`status=live`, the default), by key, `startAt` and `maxResults` at a time,
//!   and flags the last page `isLast`;
//! - `GET /project/{projectIdOrKey}` answers 404 for a project the user cannot see;
//! - `GET /search/jql` reads `project = "<id>"`, an optional `updated >= "<YYYY-MM-DD HH:MM>"`
//!   read as wall time in the profile zone, and `ORDER BY updated ASC`; it writes `created` and
//!   `updated` as `YYYY-MM-DDTHH:MM:SS.mmm±hhmm` in that zone and continues by `nextPageToken`;
//! - Jira holds no direct conversation, so a suite container that is one is no project; and
//!   Jira deletes an issue outright, so a removed item is no longer listed.
//!
//! Jira numbers projects and issues itself, and writes an instant in its own form. [`Names`]
//! keeps the provider's numbers for the suite's ids, and the text Jira wrote (and the engine's
//! UTC form of it) for the suite's instants, so the suite adapter names what the binding
//! returned in the suite's terms. An issue's revision is its `updated` text (`revision:
//! update-time`), so the suite holds each revision it reads to the instant the item took it.
use crate::{
    catalog_feed_conformance::claimed,
    feed_conformance::{
        self, Binding, Capabilities, Code, Container, ContainersPage, Item, ItemsPage, Source,
    },
};
use connectors_catalog::bundle;
use connectors_catalog_provider::{Effect, Engine, feed::Declaration};
use connectors_core::{Error, ErrorCode};
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use ess_conformance::CountStatus;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    path::PathBuf,
    sync::Mutex,
    task::{Context, Poll, Waker},
};

const INSTANCE: &str = "jira-feed-instance";
/// The first number Jira gives a project of the suite.
const FIRST_PROJECT: u64 = 10_000;
/// The first number Jira gives an issue of the suite.
const FIRST_ISSUE: u64 = 20_001;
/// Jira may answer fewer than asked: this one lists at most this many projects a page, and
/// at most [`ISSUES_A_PAGE`] issues.
const PROJECTS_A_PAGE: usize = 50;
const ISSUES_A_PAGE: usize = 100;
/// The profile zones the suite runs in, in minutes east of UTC: both ends of the range the
/// declaration's `utc_offsets` admits, UTC, and one zone on each side off the whole hour.
const ZONES: [i64; 5] = [-720, -570, 0, 345, 840];

/// The scenarios the Jira profile's capabilities select, of the 28 the family defines.
const RUN: u64 = 26;
/// The family's scenarios `revision: update-time` leaves out: each restores an item at an
/// instant it already carried another revision at, which an update-time revision cannot tell
/// apart.
const LEFT_OUT: [&str; 2] = [
    "connectors.feed.FeedItem/transition/restore/by/connectors.feed.RestoreItem/restored",
    "connectors.feed.RestoreItem/outcome/restored",
];

fn provider() -> PathBuf {
    feed_conformance::root().join("adapters/catalog")
}

fn recorded(name: &str) -> Value {
    let path = provider().join("tests/feed/jira").join(name);
    serde_json::from_slice(&std::fs::read(&path).expect("upstream shape")).unwrap()
}

fn declaration() -> Declaration {
    serde_json::from_value(recorded("feed.json")).expect("the fixture is a feed declaration")
}

fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    for _ in 0..1000 {
        if let Poll::Ready(value) = future.as_mut().poll(&mut context) {
            return value;
        }
    }
    panic!("the engine waited on a provider that answers at once")
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

fn digits(text: &str, range: std::ops::Range<usize>) -> Option<i64> {
    let part = text.get(range)?;
    part.bytes()
        .all(|b| b.is_ascii_digit())
        .then(|| part.parse().ok())?
}

/// Milliseconds since the epoch of `YYYY-MM-DD<separator>HH:MM` read as UTC wall time.
fn minute(text: &str, separator: u8) -> Option<i64> {
    let b = text.as_bytes();
    if b.len() < 16 || b[4] != b'-' || b[7] != b'-' || b[10] != separator || b[13] != b':' {
        return None;
    }
    let (year, month, day) = (
        digits(text, 0..4)?,
        digits(text, 5..7)?,
        digits(text, 8..10)?,
    );
    let (hour, minute) = (digits(text, 11..13)?, digits(text, 14..16)?);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour > 23 || minute > 59 {
        return None;
    }
    Some(((days_from_civil(year, month, day) * 24 + hour) * 60 + minute) * 60_000)
}

/// A suite instant, `YYYY-MM-DDTHH:MM:SS[.f{1,3}]Z`, in milliseconds since the epoch. Jira
/// writes milliseconds; an instant finer than that, or in another form, is not one this
/// simulation can render, and says so.
fn suite_instant(text: &str) -> i64 {
    let parsed = (|| {
        let at = minute(text, b'T')?;
        let rest = text.get(16..)?.strip_suffix('Z')?;
        let rest = rest.strip_prefix(':')?;
        let second = digits(rest, 0..2)?;
        let fraction = match rest.get(2..)? {
            "" => 0,
            f => {
                let f = f.strip_prefix('.')?;
                if f.is_empty() || f.len() > 3 {
                    return None;
                }
                digits(f, 0..f.len())? * 10_i64.pow(3 - f.len() as u32)
            }
        };
        (second < 60).then_some(at + second * 1000 + fraction)
    })();
    parsed.unwrap_or_else(|| panic!("a suite instant Jira cannot write: {text}"))
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

// ---- the provider's numbers and texts --------------------------------------------------------

/// Jira's numbers for the suite's ids, given in the order Jira first answers them, and the
/// suite's instant for each text Jira wrote or the engine returned for it.
#[derive(Default)]
struct Names {
    projects: BTreeMap<String, u64>,
    issues: BTreeMap<String, u64>,
    times: BTreeMap<String, String>,
}

impl Names {
    fn project(&mut self, container: &str) -> u64 {
        let next = FIRST_PROJECT + self.projects.len() as u64;
        *self.projects.entry(container.to_owned()).or_insert(next)
    }

    fn issue(&mut self, item: &str) -> u64 {
        let next = FIRST_ISSUE + self.issues.len() as u64;
        *self.issues.entry(item.to_owned()).or_insert(next)
    }

    fn container_of(&self, project: &str) -> Option<String> {
        self.projects
            .iter()
            .find(|(_, number)| number.to_string() == project)
            .map(|(id, _)| id.clone())
    }

    fn item_of(&self, issue: &str) -> Option<String> {
        self.issues
            .iter()
            .find(|(_, number)| number.to_string() == issue)
            .map(|(id, _)| id.clone())
    }

    /// `instant` as Jira writes it in the zone, remembered with its UTC form.
    fn write(&mut self, instant: &str, offset: i64) -> String {
        let at = suite_instant(instant);
        let written = jira_time(at, offset);
        for text in [written.clone(), utc_time(at)] {
            let kept = self
                .times
                .entry(text.clone())
                .or_insert_with(|| instant.to_owned());
            assert_eq!(kept, instant, "two suite instants Jira writes as {text}");
        }
        written
    }

    /// The suite's instant for a text the binding returned.
    fn instant_of(&self, text: &str) -> String {
        self.times
            .get(text)
            .cloned()
            .unwrap_or_else(|| panic!("the binding returned a time Jira never wrote: {text}"))
    }
}

// ---- the simulated Jira ----------------------------------------------------------------------

struct Jira<'a> {
    source: &'a Source,
    names: &'a Mutex<Names>,
    offset: i64,
}

fn refused(message: &str) -> (u16, Value) {
    (400, json!({"errorMessages": [message], "errors": {}}))
}

impl Jira<'_> {
    fn project(number: u64, container: &Container) -> Value {
        let mut project = recorded("project.json");
        project["id"] = json!(number.to_string());
        project["key"] = json!(format!("P{number}"));
        project["self"] = json!(format!(
            "https://tracker.example.test/rest/api/3/project/{number}"
        ));
        // Jira names every project; the suite may not.
        project["name"] = json!(
            container
                .name
                .clone()
                .unwrap_or_else(|| format!("Project {number}"))
        );
        project["isPrivate"] = json!(container.visibility != "public");
        project
    }

    /// A project for each suite container that Jira can hold: every one that is not a direct
    /// conversation. Its key sorts as its number does.
    fn projects(&self) -> Vec<(u64, &Container)> {
        let mut names = self.names.lock().unwrap();
        let mut projects: Vec<(u64, &Container)> = self
            .source
            .containers
            .iter()
            .filter(|(_, container)| container.visibility != "direct")
            .map(|(id, container)| (names.project(id), container))
            .collect();
        projects.sort_by_key(|(number, _)| *number);
        projects
    }

    fn container(&self, project: &str) -> Option<(&String, &Container)> {
        let id = self.names.lock().unwrap().container_of(project)?;
        self.source
            .containers
            .get_key_value(&id)
            .filter(|(_, container)| container.visibility != "direct")
    }

    /// The JQL the binding sends, read strictly: the project, and the instant a resume's
    /// literal names in this profile zone. Anything else is no query this simulation answers.
    fn parse(&self, jql: &str) -> Option<(String, Option<i64>)> {
        let rest = jql.strip_prefix("project = \"")?;
        let (project, rest) = rest.split_once('"')?;
        if let Some(rest) = rest.strip_prefix(" AND updated >= \"") {
            let (literal, rest) = rest.split_once('"')?;
            if rest != " ORDER BY updated ASC" || literal.len() != 16 {
                return None;
            }
            let since = minute(literal, b' ')? - self.offset * 60_000;
            return Some((project.to_owned(), Some(since)));
        }
        (rest == " ORDER BY updated ASC").then(|| (project.to_owned(), None))
    }

    fn answer(&self, segments: &[&str], query: &[(&str, String)]) -> (u16, Value) {
        let get = |name: &str| {
            query
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.as_str())
        };
        let Some(asked) = get("maxResults").map_or(Some(50), |n| n.parse::<usize>().ok()) else {
            return refused("maxResults is not a number.");
        };
        match segments {
            ["project", "search"] => {
                if get("action").is_some_and(|action| action != "view")
                    || get("status").is_some_and(|status| status != "live")
                {
                    return refused("Only projects the user can view, live ones, are simulated.");
                }
                let Some(start) = get("startAt").map_or(Some(0), |n| n.parse::<usize>().ok())
                else {
                    return refused("startAt is not a number.");
                };
                let projects = self.projects();
                let end = projects.len().min(start + asked.min(PROJECTS_A_PAGE));
                let values: Vec<Value> = projects[start.min(end)..end]
                    .iter()
                    .map(|(number, container)| Self::project(*number, container))
                    .collect();
                (
                    200,
                    json!({"startAt": start, "maxResults": asked, "total": projects.len(),
                           "isLast": end == projects.len(), "values": values}),
                )
            }
            ["project", project] => match self.container(project) {
                Some((_, container)) => {
                    let number = project.parse::<u64>().expect("a project number");
                    (200, Self::project(number, container))
                }
                None => (
                    404,
                    json!({"errorMessages": [format!("No project could be found with key '{project}'.")],
                           "errors": {}}),
                ),
            },
            ["search", "jql"] => {
                let Some((project, since)) = get("jql").and_then(|jql| self.parse(jql)) else {
                    return refused("The JQL query is not one this simulation answers.");
                };
                let Some((container, _)) = self.container(&project) else {
                    return refused("The value does not exist for the field 'project'.");
                };
                let start = match get("nextPageToken") {
                    None => 0,
                    Some(token) => match token.strip_prefix("page-").and_then(|n| n.parse().ok()) {
                        Some(start) => start,
                        None => return refused("The provided next page token is invalid."),
                    },
                };
                let mut names = self.names.lock().unwrap();
                let mut listed: Vec<(i64, u64, &String, &Item)> = self
                    .source
                    .items
                    .iter()
                    // Jira deletes an issue outright: it is no longer listed.
                    .filter(|(_, item)| &item.container == container && !item.deleted)
                    .map(|(id, item)| (suite_instant(&item.updated_at), names.issue(id), id, item))
                    .filter(|(updated, ..)| since.is_none_or(|since| *updated >= since))
                    .collect();
                listed.sort_by_key(|(updated, number, ..)| (*updated, *number));
                let end = listed.len().min(start + asked.min(ISSUES_A_PAGE));
                let issues: Vec<Value> = listed[start.min(end)..end]
                    .iter()
                    .map(|(_, number, id, item)| {
                        let mut record = recorded("issue.json");
                        record["id"] = json!(number.to_string());
                        record["key"] = json!(format!("P{project}-{number}"));
                        record["self"] = json!(format!(
                            "https://tracker.example.test/rest/api/3/issue/{number}"
                        ));
                        record["fields"]["summary"] = json!(format!("{id} at {}", item.revision));
                        record["fields"]["created"] =
                            json!(names.write(&item.created_at, self.offset));
                        record["fields"]["updated"] =
                            json!(names.write(&item.updated_at, self.offset));
                        record
                    })
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
}

#[async_trait::async_trait]
impl AuthenticatedHttp for Jira<'_> {
    async fn get(
        &self,
        segments: &[&str],
        query: &[(&str, String)],
    ) -> connectors_core::Result<HttpResponse> {
        let (status, body) = self.answer(segments, query);
        Ok(HttpResponse {
            status,
            headers: BTreeMap::new(),
            body: serde_json::to_vec(&body).unwrap(),
        })
    }
}

// ---- the binding under test ------------------------------------------------------------------

/// A Jira declaration, realized by the catalog engine over the committed bundle, read by a user
/// whose profile zone is `offset` minutes east of UTC.
struct Declared {
    engine: Engine,
    profile: String,
    capabilities: Capabilities,
    names: Mutex<Names>,
    offset: i64,
}

impl Declared {
    fn new(feed: &Declaration, offset: i64) -> Self {
        let root = provider();
        let bundle = bundle::load(&root.join("generated/bundles"), "jira").expect("bundle");
        let engine = Engine::with_feed(&bundle, "/rest/api/3", &[], Some(feed)).expect("engine");
        Self {
            engine,
            profile: feed.profile.clone(),
            capabilities: claimed(feed),
            names: Mutex::default(),
            offset,
        }
    }

    fn call(&self, source: &Source, operation: &str, input: Value) -> Result<Value, Error> {
        let http = Jira {
            source,
            names: &self.names,
            offset: self.offset,
        };
        let result = block_on(self.engine.read(&http, INSTANCE, operation, input));
        if let Ok(output) = &result {
            let declaration = self
                .engine
                .declarations(&[Effect::Read])
                .into_iter()
                .find(|o| o.id == operation)
                .expect("declared");
            let validator = jsonschema::validator_for(&declaration.output_schema).unwrap();
            let errors: Vec<String> = validator
                .iter_errors(output)
                .map(|e| format!("{}: {e}", e.instance_path()))
                .collect();
            assert!(errors.is_empty(), "{operation}: {errors:?}\n{output:#}");
        }
        result
    }
}

fn code(error: Error) -> Code {
    match error.code {
        ErrorCode::InvalidInput => Code::InvalidInput,
        ErrorCode::NotFound => Code::NotFound,
        ErrorCode::StaleCursor => Code::StaleCursor,
        other => Code::Other(format!("{other:?}: {}", error.message)),
    }
}

fn text(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}

impl Binding for Declared {
    fn identity(&self) -> (String, String) {
        ("connectors-catalog-feed-jira".into(), self.profile.clone())
    }

    fn capabilities(&self) -> Capabilities {
        self.capabilities
    }

    fn containers(
        &self,
        source: &Source,
        limit: Option<i64>,
        cursor: Option<&str>,
    ) -> Result<ContainersPage, Code> {
        let mut input = json!({});
        if let Some(limit) = limit {
            input["limit"] = json!(limit);
        }
        if let Some(cursor) = cursor {
            input["cursor"] = json!(cursor);
        }
        let page = self.call(source, "feed.containers", input).map_err(code)?;
        let names = self.names.lock().unwrap();
        let containers = page["containers"]
            .as_array()
            .expect("containers")
            .iter()
            .map(|c| {
                let project = text(&c["id"]).expect("id");
                let id = names.container_of(&project).unwrap_or(project);
                (
                    id,
                    Container {
                        connection: String::new(),
                        name: text(&c["name"]),
                        kind: text(&c["kind"]).expect("kind"),
                        visibility: text(&c["visibility"]).expect("visibility"),
                    },
                )
            })
            .collect();
        Ok(ContainersPage {
            containers,
            next_cursor: text(&page["next_cursor"]),
            complete: page["complete"].as_bool().expect("complete"),
        })
    }

    fn items(
        &self,
        source: &Source,
        container: &str,
        watermark: Option<&str>,
        limit: i64,
    ) -> Result<ItemsPage, Code> {
        let project = self.names.lock().unwrap().project(container);
        let mut input = json!({"container": project.to_string(), "limit": limit});
        if let Some(watermark) = watermark {
            input["watermark"] = json!(watermark);
        }
        let page = self.call(source, "feed.items", input).map_err(code)?;
        let names = self.names.lock().unwrap();
        let items = page["items"]
            .as_array()
            .expect("items")
            .iter()
            .map(|i| {
                let issue = text(&i["id"]).expect("id");
                let instant = |field: &str| names.instant_of(&text(&i[field]).expect(field));
                (
                    names.item_of(&issue).unwrap_or(issue),
                    Item {
                        container: container.to_owned(),
                        revision: instant("revision"),
                        created_at: instant("created_at"),
                        updated_at: instant("updated_at"),
                        parent: text(&i["parent"]),
                        deleted: i["deleted"].as_bool().expect("deleted"),
                        changed: 0,
                    },
                )
            })
            .collect();
        Ok(ItemsPage {
            items,
            next_watermark: text(&page["next_watermark"]).expect("next_watermark"),
            complete: page["complete"].as_bool().expect("complete"),
        })
    }
}

// ---- the cases -------------------------------------------------------------------------------

/// The declaration passes every scenario its four capabilities select in each profile zone, and the run names the ones it leaves out: 26 of the family's 28, the two left out by
/// `revision: update-time`. Each of the four capabilities changed at least one expectation, so
/// each claim was exercised.
#[test]
fn the_jira_feed_binding_passes_the_scenarios_its_profile_declares_in_every_zone() {
    for offset in ZONES {
        let run = feed_conformance::run(Declared::new(&declaration(), offset));
        let zone = format!("profile zone {offset:+} minutes");
        println!("{zone}: {}", run.summary());
        assert!(run.accounts_for_the_family(), "{zone}: {}", run.summary());
        assert_eq!(
            run.status,
            CountStatus::Passed,
            "{zone}: {:?}\n{}\n{}",
            run.failed,
            run.summary(),
            run.report
        );
        assert_eq!(
            run.counts.passed, run.counts.total,
            "{zone}: {}",
            run.report
        );
        assert_eq!(run.counts.total, RUN, "{zone}: {}", run.summary());
        assert_eq!(
            run.left_out_ids(),
            BTreeSet::from(LEFT_OUT),
            "{zone}: {}",
            run.summary()
        );
        for left in &run.left_out {
            let by: Vec<&str> = left.by.iter().map(|weaker| weaker.name()).collect();
            assert_eq!(by, ["revision: update-time"], "{zone}: {}", left.scenario);
        }
        let changed_by: BTreeSet<&str> = run.changed.iter().map(|c| c.by.name()).collect();
        assert_eq!(
            changed_by,
            BTreeSet::from([
                "deletions: not-observed",
                "kind: fixed-word",
                "revision: update-time",
                "visibility: all-private",
            ]),
            "{zone}: {}",
            run.summary()
        );
    }
}

/// The simulation writes an instant in the zone's `±hhmm` form and reads a JQL literal as the
/// zone's wall time, the two things the zones are run for.
#[test]
fn the_simulated_jira_writes_and_reads_time_in_its_profile_zone() {
    let at = suite_instant("2026-10-06T09:00:01Z");
    assert_eq!(jira_time(at, 840), "2026-10-06T23:00:01.000+1400");
    assert_eq!(jira_time(at, -720), "2026-10-05T21:00:01.000-1200");
    assert_eq!(jira_time(at, 345), "2026-10-06T14:45:01.000+0545");
    assert_eq!(utc_time(at), "2026-10-06T09:00:01.000Z");
    assert_eq!(suite_instant("2026-10-06T09:00:01.5Z"), at + 500);
    let names = Mutex::default();
    let source = Source::default();
    let jira = Jira {
        source: &source,
        names: &names,
        offset: -300,
    };
    assert_eq!(
        jira.parse(r#"project = "10000" AND updated >= "2026-10-06 04:00" ORDER BY updated ASC"#),
        Some(("10000".into(), Some(suite_instant("2026-10-06T09:00:00Z"))))
    );
    assert_eq!(
        jira.parse(r#"project = "10000" ORDER BY updated ASC"#),
        Some(("10000".into(), None))
    );
    assert_eq!(
        jira.parse(r#"project = "10000" AND updated >= "2026-10-06 04:00" ORDER BY key"#),
        None
    );
}

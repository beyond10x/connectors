//! A Confluence REST v1 `datasource.feed/v1alpha1` binding for the catalog provider, held to the
//! `feed-binding` suite its profile's capabilities select, in five zones from UTC-12:00 to
//! UTC+14:00.
//!
//! The binding is the declaration `adapters/catalog/tests/feed/confluence/feed.json`, realized by
//! the catalog engine over the committed `confluence-v1` bundle; its profile statement is
//! `adapters/catalog/contracts/feed/v1alpha1/confluence-v1.md`. It is not in the shipped
//! selection file `adapters/catalog/providers/confluence-v1/operations.json`, since the connector
//! definition format every shipped selection round-trips through cannot yet express it. Nothing
//! here maps a field or forms a watermark: [`Confluence`] only answers the requests the engine
//! sends, from the suite's provider state, the way the public v1 reference says Confluence answers
//! them.
//!
//! [`Confluence`] is the remote site, for a user whose zone is `offset` minutes east of UTC. A
//! suite container is a space and a suite item one of its pages, rendered over the shapes in
//! `adapters/catalog/tests/feed/confluence/`, which are taken from the v1 reference, not recorded
//! from a running Confluence. It answers where the binding depends on it:
//! - `GET /rest/api/space` lists the current spaces the user can view (`status=current`), by
//!   `start` and `limit`, and links the next page in `_links.next` while one follows;
//! - `GET /rest/api/space/{spaceKey}` answers 404 for a space the user cannot see;
//! - `GET /rest/api/content/search` reads `space = "<key>" and type = page`, an optional
//!   `lastmodified >= "<YYYY-MM-DD HH:MM>"` read as wall time in the user's zone, and
//!   `order by lastmodified asc`; it expands `history,version`, writes `history.createdDate` and
//!   `version.when` as `YYYY-MM-DDTHH:MM:SS.mmmZ` in UTC, answers at most [`PAGES_A_PAGE`]
//!   pages, and continues by the percent-encoded `cursor` of `_links.next`;
//! - Confluence holds no direct conversation, so a suite container that is one is no space; a
//!   private one is a personal space, keyed `~<user>`, and a public one a global space; and a
//!   removed page is in the trash, which the search does not list.
//!
//! Confluence keys spaces, numbers content and versions itself, and writes an instant in its own
//! form. [`Names`] keeps the provider's keys and numbers for the suite's ids and revisions, and the
//! text Confluence wrote for the suite's instants, so the suite adapter names what the binding
//! returned in the suite's terms. A page's revision is its version number (`revision: opaque`):
//! each revision the suite gives an item is the next version of that page.
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

const INSTANCE: &str = "confluence-v1-feed-instance";
const SITE: &str = "https://site.example.test/wiki";
/// The first number Confluence gives a space of the suite.
const FIRST_SPACE: u64 = 65_538;
/// The first number Confluence gives a page of the suite.
const FIRST_PAGE: u64 = 1001;
/// Confluence may answer fewer than asked: this one lists at most this many spaces a page, and
/// at most [`PAGES_A_PAGE`] pages of content.
const SPACES_A_PAGE: usize = 50;
const PAGES_A_PAGE: usize = 25;
/// The only expansion the binding asks for.
const EXPAND: &str = "history,version";
/// The zones the suite runs in, in minutes east of UTC: both ends of the range the
/// declaration's `utc_offsets` admits, UTC, and one zone on each side off the whole hour.
const ZONES: [i64; 5] = [-720, -570, 0, 345, 840];
/// The scenarios the Confluence profile's capabilities select: all 28 the family defines, since an
/// opaque revision tells every revision apart.
const RUN: u64 = 28;

fn provider() -> PathBuf {
    feed_conformance::root().join("adapters/catalog")
}

fn recorded(name: &str) -> Value {
    let path = provider().join("tests/feed/confluence").join(name);
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

/// A suite instant, `YYYY-MM-DDTHH:MM:SS[.f{1,3}]Z`, in milliseconds since the epoch.
/// Confluence writes milliseconds; an instant finer than that, or in another form, is not one
/// this simulation can render, and says so.
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
    parsed.unwrap_or_else(|| panic!("a suite instant Confluence cannot write: {text}"))
}

/// `at` as Confluence writes `history.createdDate` and `version.when`:
/// `YYYY-MM-DDTHH:MM:SS.mmmZ`, in UTC whatever the user's zone.
fn confluence_time(at: i64) -> String {
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

// ---- the provider's keys, numbers and texts --------------------------------------------------

/// Confluence's keys and numbers for the suite's ids and revisions, given in the order
/// Confluence first answers them, and the suite's instant for each text Confluence wrote.
#[derive(Default)]
struct Names {
    /// Each suite container's space: its number and its key.
    spaces: BTreeMap<String, (u64, String)>,
    pages: BTreeMap<String, u64>,
    versions: BTreeMap<String, Vec<String>>,
    times: BTreeMap<String, String>,
}

impl Names {
    /// The number and key of the space a suite container is: a personal space, `~` and its
    /// owner, for a private one, and a global space otherwise.
    fn numbered(&mut self, container: &str, personal: bool) -> (u64, String) {
        let ordinal = self.spaces.len() as u64 + 1;
        self.spaces
            .entry(container.to_owned())
            .or_insert_with(|| {
                let key = match personal {
                    true => format!("~fixture-user-{ordinal}"),
                    false => format!("FIX{ordinal}"),
                };
                (FIRST_SPACE + ordinal - 1, key)
            })
            .clone()
    }

    fn space(&mut self, container: &str, personal: bool) -> String {
        self.numbered(container, personal).1
    }

    fn page(&mut self, item: &str) -> u64 {
        let next = FIRST_PAGE + self.pages.len() as u64;
        *self.pages.entry(item.to_owned()).or_insert(next)
    }

    /// The version number Confluence gives `item` at the suite's `revision`: the next one the
    /// first time it is answered at it.
    fn version(&mut self, item: &str, revision: &str) -> u64 {
        let versions = self.versions.entry(item.to_owned()).or_default();
        let at = match versions.iter().position(|word| word == revision) {
            Some(at) => at,
            None => {
                versions.push(revision.to_owned());
                versions.len() - 1
            }
        };
        at as u64 + 1
    }

    fn container_of(&self, key: &str) -> Option<String> {
        self.spaces
            .iter()
            .find(|(_, (_, kept))| kept.as_str() == key)
            .map(|(id, _)| id.clone())
    }

    fn item_of(&self, page: &str) -> Option<String> {
        self.pages
            .iter()
            .find(|(_, number)| number.to_string() == page)
            .map(|(id, _)| id.clone())
    }

    /// The suite's revision of `item` Confluence numbered `version`.
    fn revision_of(&self, item: &str, version: &str) -> String {
        version
            .parse::<usize>()
            .ok()
            .and_then(|number| self.versions.get(item)?.get(number.checked_sub(1)?))
            .cloned()
            .unwrap_or_else(|| {
                panic!("the binding returned a version Confluence never gave {item}: {version}")
            })
    }

    /// `instant` as Confluence writes it, remembered.
    fn write(&mut self, instant: &str) -> String {
        let written = confluence_time(suite_instant(instant));
        let kept = self
            .times
            .entry(written.clone())
            .or_insert_with(|| instant.to_owned());
        assert_eq!(
            kept, instant,
            "two suite instants Confluence writes as {written}"
        );
        written
    }

    /// The suite's instant for a text the binding returned.
    fn instant_of(&self, text: &str) -> String {
        self.times
            .get(text)
            .cloned()
            .unwrap_or_else(|| panic!("the binding returned a time Confluence never wrote: {text}"))
    }
}

// ---- the simulated Confluence ----------------------------------------------------------------

struct Confluence<'a> {
    source: &'a Source,
    names: &'a Mutex<Names>,
    offset: i64,
}

fn refused(status: u16, message: &str) -> (u16, Value) {
    (
        status,
        json!({"statusCode": status,
               "data": {"authorized": true, "valid": status == 404, "errors": [],
                        "successful": false},
               "message": message}),
    )
}

/// The links every v1 page carries; `next` only while another page follows.
fn links(next: Option<String>) -> Value {
    let mut links = json!({"base": SITE, "context": "/wiki"});
    if let Some(next) = next {
        links["next"] = json!(next);
    }
    links
}

impl Confluence<'_> {
    fn space(number: u64, key: &str, container: &Container) -> Value {
        let mut space = recorded("space.json");
        space["id"] = json!(number);
        space["key"] = json!(key);
        // Confluence names every space; the suite may not.
        space["name"] = json!(
            container
                .name
                .clone()
                .unwrap_or_else(|| format!("Space {number}"))
        );
        space["type"] = json!(match key.starts_with('~') {
            true => "personal",
            false => "global",
        });
        space["_links"]["webui"] = json!(format!("/spaces/{key}"));
        space["_links"]["self"] = json!(format!("{SITE}/rest/api/space/{key}"));
        space
    }

    /// A space for each suite container that Confluence can hold: every one that is not a
    /// direct conversation, with its number and key, in the order Confluence numbered them.
    fn spaces(&self) -> Vec<(u64, String, &Container)> {
        let mut names = self.names.lock().unwrap();
        let mut spaces: Vec<(u64, String, &Container)> = self
            .source
            .containers
            .iter()
            .filter(|(_, container)| container.visibility != "direct")
            .map(|(id, container)| {
                let (number, key) = names.numbered(id, container.visibility == "private");
                (number, key, container)
            })
            .collect();
        spaces.sort_by_key(|(number, ..)| *number);
        spaces
    }

    fn container(&self, key: &str) -> Option<(&String, &Container)> {
        let id = self.names.lock().unwrap().container_of(key)?;
        self.source
            .containers
            .get_key_value(&id)
            .filter(|(_, container)| container.visibility != "direct")
    }

    /// The CQL the binding sends, read strictly: the space, and the instant a resume's literal
    /// names in the user's zone. Anything else is no query this simulation answers.
    fn parse(&self, cql: &str) -> Option<(String, Option<i64>)> {
        let rest = cql.strip_prefix("space = \"")?;
        let (space, rest) = rest.split_once('"')?;
        let rest = rest.strip_prefix(" and type = page")?;
        if let Some(rest) = rest.strip_prefix(" and lastmodified >= \"") {
            let (literal, rest) = rest.split_once('"')?;
            if rest != " order by lastmodified asc" || literal.len() != 16 {
                return None;
            }
            let since = minute(literal, b' ')? - self.offset * 60_000;
            return Some((space.to_owned(), Some(since)));
        }
        (rest == " order by lastmodified asc").then(|| (space.to_owned(), None))
    }

    fn answer(&self, segments: &[&str], query: &[(&str, String)]) -> (u16, Value) {
        let get = |name: &str| {
            query
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.as_str())
        };
        let Some(asked) = get("limit").map_or(Some(25), |n| n.parse::<usize>().ok()) else {
            return refused(400, "limit is not a number.");
        };
        match segments {
            ["rest", "api", "space"] => {
                if get("status") != Some("current") || get("type").is_some() {
                    return refused(400, "Only current spaces of every type are simulated.");
                }
                let Some(start) = get("start").map_or(Some(0), |n| n.parse::<usize>().ok()) else {
                    return refused(400, "start is not a number.");
                };
                let spaces = self.spaces();
                let end = spaces.len().min(start + asked.min(SPACES_A_PAGE));
                let results: Vec<Value> = spaces[start.min(end)..end]
                    .iter()
                    .map(|(number, key, container)| Self::space(*number, key, container))
                    .collect();
                let next = (end < spaces.len())
                    .then(|| format!("/rest/api/space?status=current&limit={asked}&start={end}"));
                (
                    200,
                    json!({"results": results, "start": start, "limit": asked,
                           "size": end - start.min(end), "_links": links(next)}),
                )
            }
            ["rest", "api", "space", key] => match self.container(key) {
                Some(_) => {
                    let spaces = self.spaces();
                    let (number, key, container) = spaces
                        .iter()
                        .find(|(_, kept, _)| kept == key)
                        .expect("a listed space");
                    (200, Self::space(*number, key, container))
                }
                None => refused(404, &format!("No space with key : {key}")),
            },
            ["rest", "api", "content", "search"] => {
                let Some((space, since)) = get("cql").and_then(|cql| self.parse(cql)) else {
                    return refused(400, "The CQL query is not one this simulation answers.");
                };
                if get("expand") != Some(EXPAND) {
                    return refused(
                        400,
                        "Only the history and version expansions are simulated.",
                    );
                }
                let start = match get("cursor") {
                    None => 0,
                    Some(token) => match token
                        .strip_prefix("_f_")
                        .and_then(|rest| rest.strip_suffix('='))
                        .and_then(|n| n.parse().ok())
                    {
                        Some(start) => start,
                        None => return refused(400, "The cursor is invalid."),
                    },
                };
                // A space the user cannot see holds no content the user can find.
                let container = self.container(&space).map(|(id, _)| id.clone());
                let mut names = self.names.lock().unwrap();
                let mut listed: Vec<(i64, u64, &String, &Item)> = self
                    .source
                    .items
                    .iter()
                    // A removed page is in the trash, which the search does not list.
                    .filter(|(_, item)| {
                        Some(&item.container) == container.as_ref() && !item.deleted
                    })
                    .map(|(id, item)| (suite_instant(&item.updated_at), names.page(id), id, item))
                    .filter(|(updated, ..)| since.is_none_or(|since| *updated >= since))
                    .collect();
                listed.sort_by_key(|(updated, number, ..)| (*updated, *number));
                let end = listed.len().min(start + asked.min(PAGES_A_PAGE));
                let results: Vec<Value> = listed[start.min(end)..end]
                    .iter()
                    .map(|(_, number, id, item)| {
                        let mut record = recorded("page.json");
                        record["id"] = json!(number.to_string());
                        record["title"] = json!(format!("{id} at {}", item.revision));
                        record["space"]["key"] = json!(space);
                        record["history"]["createdDate"] = json!(names.write(&item.created_at));
                        record["version"]["number"] = json!(names.version(id, &item.revision));
                        record["version"]["when"] = json!(names.write(&item.updated_at));
                        record["_links"]["webui"] =
                            json!(format!("/spaces/{space}/pages/{number}"));
                        record["_links"]["self"] =
                            json!(format!("{SITE}/rest/api/content/{number}"));
                        record
                    })
                    .collect();
                let next = (end < listed.len()).then(|| {
                    format!(
                        "/rest/api/content/search?next=true&cursor=_f_{end}%3D&expand=history%2Cversion&limit={asked}&start={end}"
                    )
                });
                (
                    200,
                    json!({"results": results, "start": start, "limit": asked,
                           "size": end - start.min(end), "_links": links(next)}),
                )
            }
            _ => refused(404, "Not found"),
        }
    }
}

#[async_trait::async_trait]
impl AuthenticatedHttp for Confluence<'_> {
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

/// A Confluence v1 declaration, realized by the catalog engine over the committed bundle, read by
/// a user whose zone is `offset` minutes east of UTC.
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
        let bundle =
            bundle::load(&root.join("generated/bundles"), "confluence-v1").expect("bundle");
        let engine = Engine::with_feed(&bundle, "/wiki", &[], Some(feed)).expect("engine");
        Self {
            engine,
            profile: feed.profile.clone(),
            capabilities: claimed(feed),
            names: Mutex::default(),
            offset,
        }
    }

    fn call(&self, source: &Source, operation: &str, input: Value) -> Result<Value, Error> {
        let http = Confluence {
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
        (
            "connectors-catalog-feed-confluence-v1".into(),
            self.profile.clone(),
        )
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
                let key = text(&c["id"]).expect("id");
                let id = names.container_of(&key).unwrap_or(key);
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
        let personal = source
            .containers
            .get(container)
            .is_some_and(|c| c.visibility == "private");
        let key = self.names.lock().unwrap().space(container, personal);
        let mut input = json!({"container": key, "limit": limit});
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
                let page = text(&i["id"]).expect("id");
                let item = names.item_of(&page).unwrap_or(page);
                let instant = |field: &str| names.instant_of(&text(&i[field]).expect(field));
                let revision = names.revision_of(&item, &text(&i["revision"]).expect("revision"));
                (
                    item,
                    Item {
                        container: container.to_owned(),
                        revision,
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

/// The declaration passes every scenario its four capabilities select in each zone: all 28 of
/// the family's, since an opaque revision leaves none out. The three capabilities weaker than the
/// family's strongest each changed at least one expectation, so each claim was exercised.
#[test]
fn the_confluence_v1_feed_binding_passes_the_scenarios_its_profile_declares_in_every_zone() {
    for offset in ZONES {
        let run = feed_conformance::run(Declared::new(&declaration(), offset));
        let zone = format!("zone {offset:+} minutes");
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
        assert!(run.left_out.is_empty(), "{zone}: {}", run.summary());
        let changed_by: BTreeSet<&str> = run.changed.iter().map(|c| c.by.name()).collect();
        assert_eq!(
            changed_by,
            BTreeSet::from([
                "deletions: not-observed",
                "kind: fixed-word",
                "visibility: all-private",
            ]),
            "{zone}: {}",
            run.summary()
        );
    }
}

/// The simulation writes an instant in UTC with milliseconds, whatever the zone, and reads a CQL
/// literal as the zone's wall time, the two things the zones are run for; each revision of a page
/// is its next version.
#[test]
fn the_simulated_confluence_writes_utc_and_reads_cql_in_the_users_zone() {
    let at = suite_instant("2026-10-06T09:00:01Z");
    assert_eq!(confluence_time(at), "2026-10-06T09:00:01.000Z");
    assert_eq!(suite_instant("2026-10-06T09:00:01.5Z"), at + 500);
    let names = Mutex::default();
    let source = Source::default();
    let confluence = Confluence {
        source: &source,
        names: &names,
        offset: -300,
    };
    assert_eq!(
        confluence.parse(
            r#"space = "FIX1" and type = page and lastmodified >= "2026-10-06 04:00" order by lastmodified asc"#
        ),
        Some(("FIX1".into(), Some(suite_instant("2026-10-06T09:00:00Z"))))
    );
    assert_eq!(
        confluence.parse(r#"space = "~fixture-user-2" and type = page order by lastmodified asc"#),
        Some(("~fixture-user-2".into(), None))
    );
    assert_eq!(
        confluence.parse(r#"space = "FIX1" order by lastmodified asc"#),
        None
    );
    let mut names = names.into_inner().unwrap();
    assert_eq!(names.space("public", false), "FIX1");
    assert_eq!(names.space("private", true), "~fixture-user-2");
    assert_eq!(names.space("public", true), "FIX1");
    assert_eq!(names.version("item-1", "r1"), 1);
    assert_eq!(names.version("item-1", "r2"), 2);
    assert_eq!(names.version("item-1", "r1"), 1);
    assert_eq!(names.revision_of("item-1", "2"), "r2");
}

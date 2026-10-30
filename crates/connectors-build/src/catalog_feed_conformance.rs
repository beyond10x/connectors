//! The catalog engine's declared feed bindings, held to the `feed-binding` suite.
//!
//! Every `adapters/catalog/tests/feed/*.operations.json` is a provider declared only as data: a
//! `connectors-catalog-operations/1` selection file whose `feed` the catalog engine realizes over
//! the bundle built from `api.json` beside it. Nothing here names a declaration; each one found is
//! run, and the suite adapter is the one the native fixture binding uses
//! ([`feed_conformance::Fixture`]).
//!
//! The provider itself is simulated: [`Simulated`] answers the requests the engine sends from the
//! suite's provider state, as `api.json` documents them. It is the remote service, not the
//! binding: the binding is the declaration, read by the engine.
use crate::feed_conformance::{
    self, Binding, Capabilities, Code, Container, ContainersPage, Item, ItemsPage, Source,
};
use connectors_catalog::{bundle::Bundle, ingest, inventory};
use connectors_catalog_provider::{Effect, Engine, feed::Declaration};
use connectors_core::{Error, ErrorCode};
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use ess_conformance::CountStatus;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    path::{Path, PathBuf},
    sync::Mutex,
    task::{Context, Poll, Waker},
};

/// The instance every read is made through. It names no declaration.
const INSTANCE: &str = "catalog-feed-instance";

fn directory() -> PathBuf {
    feed_conformance::root().join("adapters/catalog/tests/feed")
}

/// One selection file that declares a feed.
struct Declared {
    path: PathBuf,
    raw: Value,
    feed: Declaration,
}

impl Declared {
    fn position(&self) -> &str {
        self.raw["feed"]["items"]["position"]["kind"]
            .as_str()
            .expect("position kind")
    }
}

/// Every declaration in the fixture directory, in name order.
fn declared() -> Vec<Declared> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(directory())
        .expect("feed fixtures")
        .map(|entry| entry.expect("entry").path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".operations.json"))
        })
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let raw: Value =
                serde_json::from_slice(&std::fs::read(&path).expect("declaration")).unwrap();
            assert_eq!(raw["format"], "connectors-catalog-operations/1", "{path:?}");
            let feed = serde_json::from_value(raw["feed"].clone())
                .unwrap_or_else(|error| panic!("{path:?}: {error}"));
            Declared { path, raw, feed }
        })
        .collect()
}

/// The bundle every declaration reads, and the base path its document's server names.
fn bundle(provider: &str) -> (Bundle, String) {
    let bytes = std::fs::read(directory().join("api.json")).expect("api.json");
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    let server = document["servers"][0]["url"].as_str().expect("server");
    let base = server
        .split_once("://")
        .and_then(|(_, rest)| rest.find('/').map(|at| rest[at..].to_owned()))
        .expect("server path");
    (
        Bundle {
            provider: provider.to_owned(),
            source: ingest("api.json", &bytes).expect("ingest"),
            inventory: inventory::extract(&document),
            auth_profile: "fixture.token".into(),
        },
        base,
    )
}

/// Poll a future the simulated provider never leaves pending.
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

// ---- the simulated provider ------------------------------------------------------------------

/// The fixture API over the suite's provider state. Rooms are containers, messages items. A
/// message is listed by edit time (`messages`, `updated_since` inclusive, oldest first only when
/// asked with `order=asc`), through the provider's change cursor (`changes`), which refuses a
/// cursor it never issued with 410, or by a query (`message-search`, `message-query`) of the one
/// form `room = "<room>" [AND edited >= "<YYYY-MM-DD HH:MM>"] ORDER BY edited ASC`, whose literals
/// read `\"` and `\\` back and which answers at most [`QUERY_PAGE`] messages a page. Rooms are
/// listed by cursor (`rooms`), by offset (`room-index`) or by an absolute next link
/// (`room-directory`). Instants compare as text: every instant the suite writes has the one form
/// `YYYY-MM-DDTHH:MM:SSZ`, and `message-search` writes it with a `+0000` offset instead.
struct Simulated<'a> {
    source: &'a Source,
    calls: Mutex<Vec<String>>,
    queries: Mutex<Vec<BTreeMap<String, String>>>,
}

/// The most messages a query answers in one page, so that a minute holding more is read in
/// several.
const QUERY_PAGE: usize = 3;

/// The server every next link the simulated provider writes names.
const SERVER: &str = "https://feed.fixture.test/v1";

/// One quoted literal at the start of `text`, `\"` and `\\` read back, and the text after it;
/// `None` for anything else, a `\` before another character included.
fn literal(text: &str) -> Option<(String, &str)> {
    let body = text.strip_prefix('"')?;
    let mut read = String::new();
    let mut escaped = false;
    for (at, c) in body.char_indices() {
        match (escaped, c) {
            (true, '"' | '\\') => {
                read.push(c);
                escaped = false;
            }
            (true, _) => return None,
            (false, '\\') => escaped = true,
            (false, '"') => return Some((read, &body[at + 1..])),
            (false, _) => read.push(c),
        }
    }
    None
}

/// The room and the minute a simulated query selects; `None` for a query of any other form.
fn selection(query: &str) -> Option<(String, Option<String>)> {
    const ORDER: &str = " ORDER BY edited ASC";
    let (room, rest) = literal(query.strip_prefix("room = ")?)?;
    if rest == ORDER {
        return Some((room, None));
    }
    let (since, rest) = literal(rest.strip_prefix(" AND edited >= ")?)?;
    let minute = since.len() == 16 && since.as_bytes()[10] == b' ';
    (rest == ORDER && minute).then_some((room, Some(since)))
}

/// An instant's minute as a simulated query writes it.
fn minute(at: &str) -> String {
    at[..16].replace('T', " ")
}

/// `text` form-encoded, as a query value in a next link.
fn form(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b' ' => "+".to_owned(),
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

impl<'a> Simulated<'a> {
    fn new(source: &'a Source) -> Self {
        Self {
            source,
            calls: Mutex::default(),
            queries: Mutex::default(),
        }
    }

    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }

    fn queries(&self) -> Vec<BTreeMap<String, String>> {
        self.queries.lock().unwrap().clone()
    }

    /// A message as the query search answers it: its fields apart, instants with `+0000`.
    fn issue(id: &str, item: &Item) -> Value {
        let offset = |at: &str| format!("{}+0000", at.trim_end_matches('Z'));
        json!({
            "id": id,
            "fields": {
                "version": item.revision,
                "created": offset(&item.created_at),
                "updated": offset(&item.updated_at),
                "reporter": {"accountId": "member-1", "displayName": "Member One"},
                "text": if item.deleted { Value::Null } else { json!(format!("{id} at {}", item.revision)) },
                "link": format!("{SERVER}/rooms/{}/{id}", item.container),
                "reply_to": item.parent,
                "removed": item.deleted,
            },
        })
    }

    fn room(id: &str, container: &Container) -> Value {
        let access = match container.visibility.as_str() {
            "public" => "open",
            "private" => "invite",
            "direct" => "im",
            other => other,
        };
        json!({"id": id, "title": container.name, "type": container.kind, "access": access})
    }

    fn message(id: &str, item: &Item) -> Value {
        json!({
            "id": id,
            "version": item.revision,
            "created": item.created_at,
            "edited": item.updated_at,
            "user": {"id": "member-1", "name": "Member One"},
            "text": if item.deleted { Value::Null } else { json!(format!("{id} at {}", item.revision)) },
            "link": format!("https://feed.fixture.test/rooms/{}/{id}", item.container),
            "reply_to": item.parent,
            "deleted": item.deleted,
            "state": if item.deleted { "removed" } else { "present" },
        })
    }

    fn answer(&self, segments: &[&str], query: &[(&str, String)]) -> (u16, Value) {
        let get = |name: &str| {
            query
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.as_str())
        };
        let size = |name: &str| match get(name).map(str::parse::<usize>) {
            None => Some(100),
            Some(Ok(limit)) if (1..=100).contains(&limit) => Some(limit),
            Some(_) => None,
        };
        let Some(limit) = size("limit") else {
            return (400, json!({"error": "limit"}));
        };
        let source = self.source;
        let in_room = |room: &str| -> Vec<(&String, &Item)> {
            source
                .items
                .iter()
                .filter(|(_, item)| item.container == room)
                .collect()
        };
        match segments {
            ["rooms"] => {
                let rooms: Vec<_> = source.containers.iter().collect();
                let start = match get("cursor") {
                    None => 0,
                    Some(cursor) => match cursor
                        .strip_prefix('r')
                        .and_then(|n| n.parse::<usize>().ok())
                    {
                        Some(start) if start <= rooms.len() => start,
                        _ => return (400, json!({"error": "cursor"})),
                    },
                };
                let end = (start + limit).min(rooms.len());
                let page: Vec<Value> = rooms[start..end]
                    .iter()
                    .map(|(id, container)| Self::room(id, container))
                    .collect();
                let next = (end < rooms.len()).then(|| format!("r{end}"));
                (200, json!({"rooms": page, "next_cursor": next}))
            }
            ["rooms", room] => match source.containers.get(*room) {
                Some(container) => (200, json!({"room": Self::room(room, container)})),
                None => (404, json!({"error": "no such room"})),
            },
            ["rooms", room, "messages"] => {
                if !source.containers.contains_key(*room) {
                    return (404, json!({"error": "no such room"}));
                }
                let since = get("updated_since");
                let mut listed: Vec<(&String, &Item)> = in_room(room)
                    .into_iter()
                    .filter(|(_, item)| since.is_none_or(|since| item.updated_at.as_str() >= since))
                    .collect();
                listed.sort_by(|a, b| (&a.1.updated_at, a.0).cmp(&(&b.1.updated_at, b.0)));
                if get("order") != Some("asc") {
                    listed.reverse();
                }
                listed.truncate(limit);
                let messages: Vec<Value> = listed
                    .iter()
                    .map(|(id, item)| Self::message(id, item))
                    .collect();
                (200, json!({"messages": messages}))
            }
            ["rooms", room, "changes"] => {
                if !source.containers.contains_key(*room) {
                    return (404, json!({"error": "no such room"}));
                }
                let after = match get("cursor") {
                    None => 0,
                    Some(cursor) => match cursor.parse::<u64>() {
                        Ok(after) if after <= source.sequence => after,
                        _ => return (410, json!({"error": "cursor expired"})),
                    },
                };
                let mut changed: Vec<(&String, &Item)> = in_room(room)
                    .into_iter()
                    .filter(|(_, item)| item.changed > after)
                    .collect();
                changed.sort_by_key(|(_, item)| item.changed);
                let has_more = changed.len() > limit;
                changed.truncate(limit);
                let next = changed.last().map_or(after, |(_, item)| item.changed);
                let changes: Vec<Value> = changed
                    .iter()
                    .map(|(id, item)| Self::message(id, item))
                    .collect();
                (
                    200,
                    json!({"changes": changes, "cursor": next.to_string(), "has_more": has_more}),
                )
            }
            ["room-index"] => {
                let rooms: Vec<_> = source.containers.iter().collect();
                let (Some(size), Some(start)) = (
                    size("maxResults"),
                    get("startAt").map_or(Some(0), |start| start.parse::<usize>().ok()),
                ) else {
                    return (400, json!({"error": "page"}));
                };
                // A provider that lost its place answers an empty page it does not call the last.
                if start > rooms.len() {
                    return (
                        200,
                        json!({"startAt": start, "values": [], "isLast": false}),
                    );
                }
                let end = (start + size).min(rooms.len());
                let page: Vec<Value> = rooms[start..end]
                    .iter()
                    .map(|(id, container)| Self::room(id, container))
                    .collect();
                (
                    200,
                    json!({"startAt": start, "maxResults": size, "values": page, "isLast": end == rooms.len()}),
                )
            }
            ["room-directory"] => {
                let rooms: Vec<_> = source.containers.iter().collect();
                let start = match get("start") {
                    None => 0,
                    Some(start) => match start
                        .strip_prefix("p:")
                        .and_then(|n| n.parse::<usize>().ok())
                    {
                        Some(start) if start <= rooms.len() => start,
                        _ => return (400, json!({"error": "start"})),
                    },
                };
                let end = (start + limit).min(rooms.len());
                let page: Vec<Value> = rooms[start..end]
                    .iter()
                    .map(|(id, container)| Self::room(id, container))
                    .collect();
                let mut links = json!({"base": SERVER});
                if end < rooms.len() {
                    // A room of this kind ends its page with a link that has lost its position.
                    let lost = rooms[end - 1].1.kind == "link-without-start";
                    links["next"] = json!(if lost {
                        format!("{SERVER}/room-directory?limit={limit}")
                    } else {
                        format!("{SERVER}/room-directory?limit={limit}&start=p%3A{end}")
                    });
                }
                (
                    200,
                    json!({"results": page, "start": start, "size": end - start, "_links": links}),
                )
            }
            [path @ ("message-search" | "message-query")] => {
                let search = *path == "message-search";
                let (filter, page_size, token, prefix) = if search {
                    ("q", "maxResults", "pageToken", "t")
                } else {
                    ("filter", "limit", "after", "c:")
                };
                let (Some(query), Some(size)) = (get(filter), size(page_size)) else {
                    return (400, json!({"error": "query"}));
                };
                let Some((room, since)) = selection(query) else {
                    return (400, json!({"error": "query does not parse"}));
                };
                let start = match get(token) {
                    None => 0,
                    Some(token) => match token
                        .strip_prefix(prefix)
                        .and_then(|n| n.parse::<usize>().ok())
                    {
                        Some(start) => start,
                        None => return (400, json!({"error": "token"})),
                    },
                };
                let mut listed: Vec<(&String, &Item)> = in_room(&room)
                    .into_iter()
                    .filter(|(_, item)| {
                        since
                            .as_ref()
                            .is_none_or(|since| minute(&item.updated_at) >= *since)
                    })
                    .collect();
                listed.sort_by(|a, b| (&a.1.updated_at, a.0).cmp(&(&b.1.updated_at, b.0)));
                let end = (start + size.min(QUERY_PAGE)).min(listed.len());
                let Some(page) = listed.get(start..end) else {
                    return (400, json!({"error": "token"}));
                };
                let more = end < listed.len();
                if search {
                    let issues: Vec<Value> = page
                        .iter()
                        .map(|(id, item)| Self::issue(id, item))
                        .collect();
                    let mut body = json!({"issues": issues, "isLast": !more});
                    if more {
                        body["nextPageToken"] = json!(format!("t{end}"));
                    }
                    (200, body)
                } else {
                    let results: Vec<Value> = page
                        .iter()
                        .map(|(id, item)| {
                            let mut message = Self::message(id, item);
                            message["_links"] =
                                json!({"webui": format!("/rooms/{}/{id}", item.container)});
                            message
                        })
                        .collect();
                    let mut links = json!({"base": SERVER});
                    if more {
                        links["next"] = json!(format!(
                            "/message-query?filter={}&limit={size}&after=c%3A{end}",
                            form(query)
                        ));
                    }
                    (
                        200,
                        json!({"results": results, "size": end - start, "_links": links}),
                    )
                }
            }
            _ => (404, json!({"error": "no such path"})),
        }
    }
}

#[async_trait::async_trait]
impl AuthenticatedHttp for Simulated<'_> {
    async fn get(
        &self,
        segments: &[&str],
        query: &[(&str, String)],
    ) -> connectors_core::Result<HttpResponse> {
        self.calls.lock().unwrap().push(segments.join("/"));
        self.queries.lock().unwrap().push(
            query
                .iter()
                .map(|(key, value)| ((*key).to_owned(), value.clone()))
                .collect(),
        );
        let (status, body) = self.answer(segments, query);
        Ok(HttpResponse {
            status,
            headers: BTreeMap::new(),
            body: serde_json::to_vec(&body).unwrap(),
        })
    }
}

// ---- the binding under test ------------------------------------------------------------------

/// One provider request: its path and the query it sent.
type Request = (String, BTreeMap<String, String>);

/// A declaration realized by the catalog engine. Every answer is checked against the output
/// schema the engine declares for its operation.
struct Catalog {
    engine: Engine,
    profile: String,
    capabilities: Capabilities,
}

impl Catalog {
    fn new(declared: &Declared) -> Self {
        let provider = declared.raw["provider"].as_str().expect("provider");
        let (bundle, base) = bundle(provider);
        // Root-relative item links resolve under the origin of the API base.
        let engine = Engine::with_feed(&bundle, &base, &[], Some(&declared.feed))
            .and_then(|engine| engine.with_link_origin(SERVER))
            .unwrap_or_else(|e| {
                panic!("{:?}: {e}", declared.path);
            });
        Self {
            engine,
            profile: declared.feed.profile.clone(),
            capabilities: claimed(&declared.feed),
        }
    }

    /// One invocation and the provider requests it made.
    fn call(
        &self,
        source: &Source,
        operation: &str,
        input: Value,
    ) -> (Result<Value, Error>, Vec<String>) {
        let (result, requests) = self.requests(source, operation, input);
        (result, requests.into_iter().map(|(path, _)| path).collect())
    }

    /// One invocation and each provider request it made, with the query sent.
    fn requests(
        &self,
        source: &Source,
        operation: &str,
        input: Value,
    ) -> (Result<Value, Error>, Vec<Request>) {
        let http = Simulated::new(source);
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
        (
            result,
            http.calls().into_iter().zip(http.queries()).collect(),
        )
    }
}

/// A declaration's claim, read in the family's vocabulary: a word one side does not know is a
/// vocabulary that moved on one side only. The family's `kind_word` is the declaration's
/// `containers.kind_word`, never repeated under `capabilities`.
pub(crate) fn claimed(feed: &Declaration) -> Capabilities {
    let mut claim = serde_json::to_value(feed.capabilities).unwrap();
    let containers = serde_json::to_value(&feed.containers).unwrap();
    if let Some(word) = containers.get("kind_word") {
        claim["kind_word"] = word.clone();
    }
    serde_json::from_value(claim).unwrap_or_else(|e| panic!("{}: {e}", feed.profile))
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

impl Binding for Catalog {
    fn identity(&self) -> (String, String) {
        ("connectors-catalog-feed".into(), self.profile.clone())
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
        let page = self
            .call(source, "feed.containers", input)
            .0
            .map_err(code)?;
        let containers = page["containers"]
            .as_array()
            .expect("containers")
            .iter()
            .map(|c| {
                (
                    text(&c["id"]).expect("id"),
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
        let mut input = json!({"container": container, "limit": limit});
        if let Some(watermark) = watermark {
            input["watermark"] = json!(watermark);
        }
        let page = self.call(source, "feed.items", input).0.map_err(code)?;
        let items = page["items"]
            .as_array()
            .expect("items")
            .iter()
            .map(|i| {
                (
                    text(&i["id"]).expect("id"),
                    Item {
                        container: container.to_owned(),
                        revision: text(&i["revision"]).expect("revision"),
                        created_at: text(&i["created_at"]).expect("created_at"),
                        updated_at: text(&i["updated_at"]).expect("updated_at"),
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

// ---- seeding ---------------------------------------------------------------------------------

fn room(source: &mut Source, id: &str, visibility: &str) {
    source.containers.insert(
        id.into(),
        Container {
            connection: "workspace-a".into(),
            name: Some(format!("Room {id}")),
            kind: "room".into(),
            visibility: visibility.into(),
        },
    );
}

fn add(source: &mut Source, room: &str, id: &str, revision: &str, at: &str) {
    source.sequence += 1;
    source.items.insert(
        id.into(),
        Item {
            container: room.into(),
            revision: revision.into(),
            created_at: at.into(),
            updated_at: at.into(),
            parent: None,
            deleted: false,
            changed: source.sequence,
        },
    );
}

fn change(source: &mut Source, id: &str, revision: &str, at: &str, deleted: bool) {
    source.sequence += 1;
    let item = source.items.get_mut(id).expect("item");
    item.revision = revision.into();
    item.updated_at = at.into();
    item.deleted = deleted;
    item.changed = source.sequence;
}

fn ids(page: &ItemsPage) -> Vec<(&str, &str, bool)> {
    page.items
        .iter()
        .map(|(id, item)| (id.as_str(), item.revision.as_str(), item.deleted))
        .collect()
}

// ---- the cases -------------------------------------------------------------------------------

/// Both watermark forms the declaration offers are exercised; a fixture directory that lost one
/// would leave that form untested while every remaining run stays green.
#[test]
fn the_fixture_declarations_cover_both_watermark_forms() {
    let forms: BTreeSet<String> = declared().iter().map(|d| d.position().to_owned()).collect();
    assert_eq!(
        forms,
        BTreeSet::from(["cursor".to_owned(), "time".to_owned()])
    );
}

/// Each fixture declaration passes every scenario its capabilities select; both claim the
/// strongest provider's, so each runs all the family defines.
#[test]
fn every_declared_feed_passes_the_feed_binding_suite() {
    let declared = declared();
    assert!(!declared.is_empty());
    for declaration in &declared {
        let run = feed_conformance::run(Catalog::new(declaration));
        println!("{}", run.summary());
        println!(
            "{}: {}",
            declaration.path.display(),
            serde_json::to_string(&run.counts).unwrap()
        );
        assert!(run.accounts_for_the_family(), "{}", run.summary());
        assert!(run.left_out.is_empty(), "{}", run.summary());
        assert_eq!(
            run.counts.total,
            feed_conformance::EXPECTED_SCENARIOS,
            "{:?}: scenario count moved",
            declaration.path
        );
        assert_eq!(
            run.status,
            CountStatus::Passed,
            "{:?}\n{}",
            declaration.path,
            run.report
        );
        assert_eq!(
            run.counts.passed, run.counts.total,
            "{:?}\n{}",
            declaration.path, run.report
        );
    }
}

/// A declaration that carries less claims less, and the engine realizing it passes the suite
/// its claim selects: no `deleted` condition claims no deletions, and a visibility rule that maps
/// nothing to `public` claims every container private. It runs every scenario the family
/// defines: each expected tombstone is held to never being reported, and each container stored
/// `public` to being listed `private`, each change named with the capability that made it.
#[test]
fn a_declaration_that_observes_less_passes_the_scenarios_it_declares() {
    let mut weaker = declared().remove(0);
    let feed = &mut weaker.raw["feed"];
    feed["items"].as_object_mut().unwrap().remove("deleted");
    feed["containers"]["visibility"]["map"] = json!({"im": "direct"});
    feed["capabilities"]["deletions"] = json!("not-observed");
    feed["capabilities"]["visibility"] = json!("all-private");
    weaker.feed = serde_json::from_value(feed.clone()).unwrap();
    let run = feed_conformance::run(Catalog::new(&weaker));
    println!("{}", run.summary());
    assert!(run.accounts_for_the_family(), "{}", run.summary());
    assert_eq!(run.status, CountStatus::Passed, "{}", run.report);
    assert_eq!(run.counts.passed, run.counts.total, "{}", run.report);
    assert_eq!(run.counts.total, feed_conformance::EXPECTED_SCENARIOS);
    assert!(run.left_out.is_empty(), "{}", run.summary());
    let changed: BTreeSet<(&str, &str, &str)> = run
        .changed
        .iter()
        .map(|changed| (changed.field, changed.expect.as_str(), changed.by.name()))
        .collect();
    assert_eq!(
        changed,
        BTreeSet::from([
            ("state", "contains", "deletions: not-observed"),
            ("state", "excludes", "deletions: not-observed"),
            ("visibility", "contains", "visibility: all-private"),
        ])
    );
    assert_eq!(run.changed_ids().len(), 5, "{}", run.summary());
}

/// A declaration whose records carry no word for a container states one, `containers.kind_word`,
/// and claims `kind: fixed-word`; the suite reads that word as the family's `kind_word` and holds
/// every listed container to it. It runs every scenario the family defines.
#[test]
fn a_fixed_kind_word_declaration_passes_the_scenarios_it_declares() {
    let mut fixed = declared().remove(0);
    let feed = &mut fixed.raw["feed"];
    feed["containers"].as_object_mut().unwrap().remove("kind");
    feed["containers"]["kind_word"] = json!("room");
    feed["capabilities"]["kind"] = json!("fixed-word");
    fixed.feed = serde_json::from_value(feed.clone()).unwrap();
    let run = feed_conformance::run(Catalog::new(&fixed));
    println!("{}", run.summary());
    assert_eq!(run.capabilities.kind_word, Some("room"));
    assert!(run.accounts_for_the_family(), "{}", run.summary());
    assert_eq!(run.status, CountStatus::Passed, "{}", run.report);
    assert_eq!(run.counts.total, feed_conformance::EXPECTED_SCENARIOS);
    assert!(run.left_out.is_empty(), "{}", run.summary());
    assert!(
        run.changed
            .iter()
            .any(|changed| changed.field == "kind" && changed.by.name() == "kind: fixed-word"),
        "{}",
        run.summary()
    );
}

/// Every shipped provider selection file that declares a feed, in name order:
/// `adapters/catalog/providers/<provider>/operations.json`.
fn shipped() -> Vec<(PathBuf, Value)> {
    let providers = feed_conformance::root().join("adapters/catalog/providers");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(providers)
        .expect("providers")
        .map(|entry| entry.expect("entry").path().join("operations.json"))
        .filter(|path| path.is_file())
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let raw: Value =
                serde_json::from_slice(&std::fs::read(&path).expect("selection file")).unwrap();
            (path, raw)
        })
        .filter(|(_, raw)| raw.get("feed").is_some())
        .collect()
}

/// Every provider binding held as a fixture, in name order:
/// `adapters/catalog/tests/feed/<provider>/feed.json`, as a selection file would hold it. Each is
/// now carried verbatim by its shipped selection's `feed`.
fn fixture_bindings() -> Vec<(PathBuf, Value)> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(directory())
        .expect("feed fixtures")
        .map(|entry| entry.expect("entry").path().join("feed.json"))
        .filter(|path| path.is_file())
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let feed: Value =
                serde_json::from_slice(&std::fs::read(&path).expect("binding fixture")).unwrap();
            (path, json!({"feed": feed}))
        })
        .collect()
}

/// Every declaration, the fixtures', the provider bindings held as fixtures and the shipped
/// providers', is a value of the adapter's own model (`adapters/catalog/spec/ess`), and the
/// engine's reader carries every field of it: read and written back, it is the same document.
#[test]
fn every_declared_feed_is_a_value_of_the_catalog_feed_model() {
    let root = feed_conformance::root();
    let ess = connectors_spec::toolchain::resolve(None).expect("pinned ess");
    let out = tempfile::tempdir().unwrap();
    let status = std::process::Command::new(ess)
        .current_dir(&root)
        .args([
            "generate",
            "--path",
            "adapters/catalog/spec/ess",
            "--kind",
            "schema",
            "--out",
        ])
        .arg(out.path().join("schemas"))
        .stdout(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());
    let schema: Value = serde_json::from_slice(
        &std::fs::read(
            out.path()
                .join("schemas/schema/types/connectors_catalog.feed.Declaration.schema.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let fixtures = declared().into_iter().map(|d| (d.path, d.raw));
    let shipped = shipped();
    assert!(
        !shipped.is_empty(),
        "no shipped provider declares a feed, so none is checked here"
    );
    let bindings = fixture_bindings();
    let named: Vec<&Path> = bindings.iter().map(|(path, _)| path.as_path()).collect();
    assert_eq!(named.len(), 2, "{named:?}");
    for (path, binding) in &bindings {
        assert!(
            shipped
                .iter()
                .any(|(_, raw)| raw["feed"] == binding["feed"]),
            "{path:?}: no shipped selection carries this binding"
        );
    }
    for (path, raw) in fixtures.chain(bindings).chain(shipped) {
        let errors: Vec<String> = validator
            .iter_errors(&raw["feed"])
            .map(|e| format!("{}: {e}", e.instance_path()))
            .collect();
        assert!(errors.is_empty(), "{path:?}: {errors:?}");
        let feed: Declaration = serde_json::from_value(raw["feed"].clone())
            .unwrap_or_else(|error| panic!("{path:?}: {error}"));
        let written = serde_json::to_value(&feed).unwrap();
        assert_eq!(written, raw["feed"], "{path:?}");
    }
}

/// The acceptance's resume: a first read bounded to one item, a read again from the watermark it
/// returned, an idle read that keeps its position, then a change, a new item and a tombstone
/// under a new revision returned from the kept watermark. A watermark of another container or
/// another profile, or one that is not a watermark at all, is `stale_cursor`, never a first read.
#[test]
fn every_declared_feed_resumes_from_the_watermark_it_returned() {
    let declared = declared();
    let engines: Vec<Catalog> = declared.iter().map(Catalog::new).collect();
    let mut kept = Vec::new();
    for (declaration, catalog) in declared.iter().zip(&engines) {
        let mut source = Source::default();
        room(&mut source, "room", "public");
        room(&mut source, "other", "public");
        add(&mut source, "room", "a", "r1", "2026-10-06T09:00:00Z");
        add(&mut source, "room", "b", "r1", "2026-10-06T09:00:01Z");
        let first = catalog.items(&source, "room", None, 1).unwrap();
        assert_eq!(ids(&first), [("a", "r1", false)], "{:?}", declaration.path);
        assert!(!first.complete, "a page bounded by limit is not complete");
        let second = catalog
            .items(&source, "room", Some(&first.next_watermark), 10)
            .unwrap();
        assert_eq!(ids(&second), [("b", "r1", false)], "{:?}", declaration.path);
        assert!(second.complete);
        let idle = catalog
            .items(&source, "room", Some(&second.next_watermark), 10)
            .unwrap();
        assert!(idle.items.is_empty() && idle.complete);
        assert_eq!(idle.next_watermark, second.next_watermark);

        add(&mut source, "room", "c", "r1", "2026-10-06T09:00:02Z");
        change(&mut source, "a", "r2", "2026-10-06T09:00:03Z", false);
        change(&mut source, "a", "r3-removed", "2026-10-06T09:00:04Z", true);
        let (resumed, calls) = catalog.call(
            &source,
            "feed.items",
            json!({"container": "room", "watermark": idle.next_watermark, "limit": 10}),
        );
        let resumed = resumed.unwrap();
        assert_eq!(calls.len(), 2, "a lookup and one list: {calls:?}");
        let got: Vec<(&str, &str, bool, bool)> = resumed["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| {
                (
                    i["id"].as_str().unwrap(),
                    i["revision"].as_str().unwrap(),
                    i["deleted"].as_bool().unwrap(),
                    i["body"].is_null(),
                )
            })
            .collect();
        assert_eq!(
            got,
            [("c", "r1", false, false), ("a", "r3-removed", true, true)],
            "{:?}",
            declaration.path
        );
        assert_eq!(resumed["provenance"]["profile"], json!(catalog.profile));
        assert_eq!(resumed["provenance"]["instance"], json!(INSTANCE));

        let stale = |watermark: &str| catalog.items(&source, "room", Some(watermark), 10).err();
        assert_eq!(
            catalog
                .items(&source, "other", Some(&idle.next_watermark), 10)
                .err(),
            Some(Code::StaleCursor),
            "{:?}: another container's watermark",
            declaration.path
        );
        assert_eq!(stale("opaque"), Some(Code::StaleCursor));
        assert_eq!(stale(""), Some(Code::StaleCursor));
        for limit in [0, 101] {
            assert_eq!(
                catalog.items(&source, "room", None, limit).err(),
                Some(Code::InvalidInput)
            );
        }
        if declaration.position() == "cursor" {
            // The provider no longer resumes from the cursor this watermark carries.
            let forgotten = Source {
                sequence: 0,
                ..source.clone()
            };
            assert_eq!(
                catalog
                    .items(&forgotten, "room", Some(&idle.next_watermark), 10)
                    .err(),
                Some(Code::StaleCursor)
            );
        }
        kept.push(idle.next_watermark);
    }
    // A watermark another profile issued, for the same container, is foreign.
    let mut source = Source::default();
    room(&mut source, "room", "public");
    for (index, catalog) in engines.iter().enumerate() {
        for (issued, watermark) in kept.iter().enumerate() {
            if issued != index {
                assert_eq!(
                    catalog.items(&source, "room", Some(watermark), 10).err(),
                    Some(Code::StaleCursor)
                );
            }
        }
    }
}

/// More items than one page share one instant: each is delivered once, a page at a time, and a
/// later revision at that same instant is delivered again.
#[test]
fn items_sharing_one_instant_are_each_delivered_page_by_page() {
    for declaration in declared() {
        let catalog = Catalog::new(&declaration);
        let mut source = Source::default();
        room(&mut source, "room", "private");
        let at = "2026-10-06T09:00:00Z";
        for id in ["e", "d", "c", "b", "a"] {
            add(&mut source, "room", id, "r1", at);
        }
        let mut watermark = None;
        let mut delivered = Vec::new();
        for _ in 0..10 {
            let page = catalog
                .items(&source, "room", watermark.as_deref(), 1)
                .unwrap();
            delivered.extend(
                page.items
                    .iter()
                    .map(|(id, item)| (id.clone(), item.revision.clone())),
            );
            watermark = Some(page.next_watermark);
            if page.complete {
                break;
            }
        }
        delivered.sort();
        let expected: Vec<(String, String)> = ["a", "b", "c", "d", "e"]
            .iter()
            .map(|id| ((*id).to_owned(), "r1".to_owned()))
            .collect();
        assert_eq!(delivered, expected, "{:?}", declaration.path);
        change(&mut source, "c", "r2", at, false);
        let again = catalog
            .items(&source, "room", watermark.as_deref(), 10)
            .unwrap();
        assert_eq!(ids(&again), [("c", "r2", false)], "{:?}", declaration.path);
    }
}

/// A page that ends at `limit` with more unseen items in the provider's answer keeps its
/// watermark on the last item it returned: an item delivered earlier that has since changed
/// moves past the ones that did not, and none of them is lost.
#[test]
fn a_page_bounded_by_limit_never_passes_an_item_it_did_not_return() {
    for declaration in declared() {
        let catalog = Catalog::new(&declaration);
        let mut source = Source::default();
        room(&mut source, "room", "public");
        add(&mut source, "room", "x", "r1", "2026-10-06T09:00:00Z");
        add(&mut source, "room", "y", "r1", "2026-10-06T09:00:00Z");
        let first = catalog.items(&source, "room", None, 1).unwrap();
        assert_eq!(ids(&first), [("x", "r1", false)], "{:?}", declaration.path);
        add(&mut source, "room", "z", "r1", "2026-10-06T09:00:01Z");
        change(&mut source, "x", "r2", "2026-10-06T09:00:02Z", false);
        let mut watermark = Some(first.next_watermark);
        let mut delivered = Vec::new();
        for _ in 0..10 {
            let page = catalog
                .items(&source, "room", watermark.as_deref(), 1)
                .unwrap();
            delivered.extend(
                page.items
                    .iter()
                    .map(|(id, item)| format!("{id}@{}", item.revision)),
            );
            watermark = Some(page.next_watermark);
            if page.complete {
                break;
            }
        }
        assert_eq!(
            delivered,
            ["y@r1", "z@r1", "x@r2"],
            "{:?}",
            declaration.path
        );
    }
}

/// A direct conversation is never listed, and reading one answers exactly as reading a container
/// that does not exist.
#[test]
fn direct_conversations_are_neither_listed_nor_readable() {
    for declaration in declared() {
        let catalog = Catalog::new(&declaration);
        let mut source = Source::default();
        for (id, visibility) in [
            ("a-public", "public"),
            ("b-direct", "direct"),
            ("c-private", "private"),
        ] {
            room(&mut source, id, visibility);
        }
        let mut listed = Vec::new();
        let mut cursor = None;
        loop {
            let page = catalog
                .containers(&source, Some(1), cursor.as_deref())
                .unwrap();
            listed.extend(
                page.containers
                    .into_iter()
                    .map(|(id, c)| (id, c.visibility)),
            );
            if page.complete {
                assert!(page.next_cursor.is_none());
                break;
            }
            cursor = page.next_cursor;
        }
        assert_eq!(
            listed,
            [
                ("a-public".to_owned(), "public".to_owned()),
                ("c-private".to_owned(), "private".to_owned())
            ],
            "{:?}",
            declaration.path
        );
        let read = |container: &str| {
            catalog
                .call(
                    &source,
                    "feed.items",
                    json!({"container": container, "limit": 10}),
                )
                .0
                .unwrap_err()
        };
        let (direct, missing) = (read("b-direct"), read("missing"));
        assert_eq!(direct.code, ErrorCode::NotFound);
        assert_eq!(
            (direct.code, direct.message),
            (missing.code, missing.message)
        );
        assert_eq!(
            catalog.containers(&source, Some(1), Some("opaque")).err(),
            Some(Code::StaleCursor)
        );
    }
}

/// The Rust [`no_rust_names_a_declared_provider`] reads: this suite adapter and every `.rs` file
/// under `adapters/catalog/src`, at any depth, in path order.
fn name_check_sources() -> Vec<PathBuf> {
    fn walk(directory: &Path, sources: &mut Vec<PathBuf>) {
        let mut paths: Vec<PathBuf> = std::fs::read_dir(directory)
            .unwrap_or_else(|error| panic!("{directory:?}: {error}"))
            .map(|entry| entry.expect("entry").path())
            .collect();
        paths.sort();
        for path in paths {
            if path.is_dir() {
                walk(&path, sources);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                sources.push(path);
            }
        }
    }
    let root = feed_conformance::root();
    let mut sources = vec![root.join("crates/connectors-build/src/catalog_feed_conformance.rs")];
    walk(&root.join("adapters/catalog/src"), &mut sources);
    sources
}

/// The name check reaches a source file in a subdirectory of the catalog engine, and reads only
/// Rust files.
#[test]
fn the_name_check_reads_nested_sources() {
    let sources = name_check_sources();
    let nested = feed_conformance::root().join("adapters/catalog/src/served/git.rs");
    assert!(sources.contains(&nested), "{sources:?}");
    assert!(
        sources
            .iter()
            .all(|source| source.is_file() && source.extension().is_some_and(|ext| ext == "rs")),
        "{sources:?}"
    );
}

/// Adversary: the story's acceptance says the check "still reads `catalog_feed_conformance.rs`
/// itself", and nothing asserted it: dropping the suite adapter from the list kept every case
/// green. Pin it, with the top-level and nested engine files beside it.
#[test]
fn adversary_the_name_check_still_reads_the_suite_adapter_and_both_levels() {
    let root = feed_conformance::root();
    let sources = name_check_sources();
    for expected in [
        "crates/connectors-build/src/catalog_feed_conformance.rs",
        "adapters/catalog/src/lib.rs",
        "adapters/catalog/src/served.rs",
        "adapters/catalog/src/served/git.rs",
    ] {
        assert!(
            sources.contains(&root.join(expected)),
            "{expected} missing from {sources:?}"
        );
    }
}

/// No Rust in the catalog engine, or in this suite adapter, names a declared provider, a fixture
/// or a shipped one: not its provider id, its profile or an operation it reads, in any letter
/// case. What one provider needs is data in its declaration.
#[test]
fn no_rust_names_a_declared_provider() {
    let sources = name_check_sources();
    let fixtures = declared().into_iter().map(|declaration| declaration.raw);
    for raw in fixtures.chain(shipped().into_iter().map(|(_, raw)| raw)) {
        let feed = &raw["feed"];
        let names = [
            &raw["provider"],
            &feed["profile"],
            &feed["containers"]["operation_id"],
            &feed["containers"]["lookup"]["operation_id"],
            &feed["items"]["operation_id"],
        ];
        for source in &sources {
            let text = std::fs::read_to_string(source).unwrap().to_lowercase();
            for name in names {
                let name = name.as_str().unwrap();
                assert!(
                    !text.contains(&name.to_lowercase()),
                    "{source:?} names `{name}`"
                );
            }
        }
    }
}

// ---- adversary, wave 20261007a U1 ----------------------------------------------------------

/// A declaration whose revision pointer is its `updated_at` pointer must claim
/// `revision: update-time` (the engine refuses `opaque` for it). The harness then leaves out only
/// the scenarios that give one item two revisions at one instant, so every scenario it keeps must
/// be one this honest update-time binding passes.
#[test]
fn adversary_an_update_time_declaration_passes_the_scenarios_it_declares() {
    let mut weaker = declared()
        .into_iter()
        .find(|declared| declared.position() == "cursor")
        .expect("a cursor declaration");
    let feed = &mut weaker.raw["feed"];
    feed["items"]["revision"] = feed["items"]["updated_at"].clone();
    feed["capabilities"]["revision"] = json!("update-time");
    weaker.feed = serde_json::from_value(feed.clone()).unwrap();
    let run = feed_conformance::run(Catalog::new(&weaker));
    println!("{}", run.summary());
    assert!(run.accounts_for_the_family(), "{}", run.summary());
    assert_eq!(
        run.status,
        CountStatus::Passed,
        "an honest update-time binding failed scenarios it is held to: {:?}\n{}",
        run.failed,
        run.summary()
    );
}

/// An all-private declaration that reads `containers.kind` claims `kind: provider-word` (the
/// engine refuses `fixed-word` without `containers.kind_word`), so the suite still holds it to
/// the provider's word. Reading the room's id as its kind is not the provider's word, and must
/// fail.
#[test]
fn adversary_an_all_private_declaration_is_still_held_to_the_providers_kind() {
    let mut weaker = declared().remove(0);
    let feed = &mut weaker.raw["feed"];
    feed["containers"]["visibility"]["map"] = json!({"im": "direct"});
    feed["capabilities"]["visibility"] = json!("all-private");
    feed["containers"]["kind"] = feed["containers"]["id"].clone();
    weaker.feed = serde_json::from_value(feed.clone()).unwrap();
    let run = feed_conformance::run(Catalog::new(&weaker));
    println!("{}", run.summary());
    assert_eq!(
        run.status,
        CountStatus::Failed,
        "a declaration reading the container id as its kind passed under `kind: provider-word`:\n{}",
        run.summary()
    );
}

// ---- the list shapes real providers use, wave 20261021 U1 ----------------------------------

/// The time position of a declaration.
fn time_value(declared: &Declared) -> &Value {
    &declared.raw["feed"]["items"]["position"]["value"]
}

/// Every declaration whose items a query selects, and the parameter the query is sent as.
fn queried() -> Vec<(Declared, String)> {
    declared()
        .into_iter()
        .filter_map(|declared| {
            let parameter = time_value(&declared)["query"]["parameter"]
                .as_str()?
                .to_owned();
            Some((declared, parameter))
        })
        .collect()
}

/// `id@revision` of every item of a `feed.items` answer, in its order.
fn delivered(page: &Value) -> Vec<String> {
    page["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|item| {
            format!(
                "{}@{}",
                item["id"].as_str().unwrap(),
                item["revision"].as_str().unwrap()
            )
        })
        .collect()
}

/// The ids of a `feed.containers` answer, in its order.
fn listed(page: &Value) -> Vec<String> {
    page["containers"]
        .as_array()
        .expect("containers")
        .iter()
        .map(|container| container["id"].as_str().unwrap().to_owned())
        .collect()
}

/// Each listing shape the declaration offers is run by some fixture; a fixture directory that
/// lost one would leave that shape untested while every remaining run stays green.
#[test]
fn the_fixture_declarations_cover_every_listing_shape() {
    let declared = declared();
    type Shape = fn(&Value) -> bool;
    let shapes: [(&str, Shape); 7] = [
        ("containers by a next token", |f| {
            f["containers"]["cursor"]["next"].is_string()
                && f["containers"]["cursor"].get("url_parameter").is_none()
        }),
        ("containers by a next link", |f| {
            f["containers"]["cursor"]["url_parameter"].is_string()
        }),
        ("containers by offset", |f| {
            f["containers"]["cursor"]["offset"]["complete"].is_object()
        }),
        ("items by a jql query", |f| {
            f["items"]["position"]["value"]["query"]["language"] == "jql"
        }),
        ("items by a cql query", |f| {
            f["items"]["position"]["value"]["query"]["language"] == "cql"
        }),
        ("item pages by a token", |f| {
            let page = &f["items"]["position"]["value"]["page"];
            page.is_object() && page.get("url_parameter").is_none()
        }),
        ("item pages by a next link", |f| {
            f["items"]["position"]["value"]["page"]["url_parameter"].is_string()
        }),
    ];
    for (shape, test) in shapes {
        assert!(
            declared.iter().any(|d| test(&d.raw["feed"])),
            "no fixture declaration lists {shape}"
        );
    }
}

/// A query at minute precision re-reads the minute it resumes from and pages within it: seven
/// items in one minute, more than one provider page holds, are each delivered once by one call
/// over several pages, the continuation sent only after the first. A resume sends the minute as
/// a quoted literal and returns only what changed since; reading at a limit below the minute's
/// size delivers each item once over several calls.
#[test]
fn a_minute_query_pages_within_one_minute_and_resumes_by_what_it_holds() {
    let queried = queried();
    assert!(!queried.is_empty());
    for (declaration, parameter) in queried {
        let path = &declaration.path;
        let page_parameter = time_value(&declaration)["page"]["parameter"]
            .as_str()
            .expect("a query declaration pages")
            .to_owned();
        let catalog = Catalog::new(&declaration);
        let mut source = Source::default();
        room(&mut source, "room", "public");
        let seven = ["a", "b", "c", "d", "e", "f", "g"];
        for (second, id) in seven.iter().enumerate() {
            add(
                &mut source,
                "room",
                id,
                "r1",
                &format!("2026-10-06T09:00:{second:02}Z"),
            );
        }
        let (first, requests) = catalog.requests(
            &source,
            "feed.items",
            json!({"container": "room", "limit": 10}),
        );
        let first = first.unwrap_or_else(|e| panic!("{path:?}: {e}"));
        let all: Vec<String> = seven.iter().map(|id| format!("{id}@r1")).collect();
        assert_eq!(delivered(&first), all, "{path:?}");
        assert_eq!(first["complete"], json!(true), "{path:?}");
        assert_eq!(
            first["items"][0]["updated_at"],
            json!("2026-10-06T09:00:00Z")
        );
        assert_eq!(
            requests.len(),
            1 + seven.len().div_ceil(QUERY_PAGE),
            "{path:?}: the lookup, then one request per page: {requests:?}"
        );
        assert!(requests.len() > 2, "{path:?}: {requests:?}");
        assert_eq!(
            requests[1].1[&parameter], r#"room = "room" ORDER BY edited ASC"#,
            "{path:?}"
        );
        assert!(!requests[1].1.contains_key(&page_parameter), "{path:?}");
        for (_, query) in &requests[2..] {
            assert!(query.contains_key(&page_parameter), "{path:?}: {query:?}");
            assert_eq!(query[&parameter], requests[1].1[&parameter], "{path:?}");
        }
        let watermark = first["next_watermark"].as_str().unwrap().to_owned();
        let (idle, requests) = catalog.requests(
            &source,
            "feed.items",
            json!({"container": "room", "watermark": watermark, "limit": 10}),
        );
        let idle = idle.unwrap();
        assert_eq!(idle["items"], json!([]), "{path:?}");
        assert_eq!(idle["complete"], json!(true), "{path:?}");
        assert_eq!(idle["next_watermark"], json!(watermark), "{path:?}");
        assert_eq!(
            requests[1].1[&parameter],
            r#"room = "room" AND edited >= "2026-10-06 09:00" ORDER BY edited ASC"#,
            "{path:?}"
        );

        change(&mut source, "b", "r2", "2026-10-06T09:00:30Z", false);
        add(&mut source, "room", "h", "r1", "2026-10-06T09:00:31Z");
        add(&mut source, "room", "i", "r1", "2026-10-06T09:01:00Z");
        let resumed = catalog
            .call(
                &source,
                "feed.items",
                json!({"container": "room", "watermark": watermark, "limit": 10}),
            )
            .0
            .unwrap();
        assert_eq!(delivered(&resumed), ["b@r2", "h@r1", "i@r1"], "{path:?}");
        assert_eq!(resumed["complete"], json!(true), "{path:?}");

        let mut watermark: Option<String> = None;
        let mut each = Vec::new();
        for _ in 0..20 {
            let mut input = json!({"container": "room", "limit": 2});
            if let Some(watermark) = &watermark {
                input["watermark"] = json!(watermark);
            }
            let page = catalog.call(&source, "feed.items", input).0.unwrap();
            each.extend(delivered(&page));
            watermark = page["next_watermark"].as_str().map(str::to_owned);
            if page["complete"] == json!(true) {
                break;
            }
        }
        each.sort();
        let expected = [
            "a@r1", "b@r2", "c@r1", "d@r1", "e@r1", "f@r1", "g@r1", "h@r1", "i@r1",
        ];
        assert_eq!(each, expected, "{path:?}");
    }
}

/// A container id reaches the provider's query as one quoted literal of the language, its `"`
/// and `\` escaped, on a first read and on a resume: an id that would close the literal and add
/// a clause selects only its own container. An id holding a control character is refused before
/// any item listing is sent.
#[test]
fn a_container_id_reaches_the_query_quoted_and_cannot_add_a_clause() {
    for (declaration, parameter) in queried() {
        let path = &declaration.path;
        let catalog = Catalog::new(&declaration);
        let mut source = Source::default();
        let hostile = r#"x" OR room = "other"#;
        let escaped = r#"back\slash \" quote"#;
        for id in [hostile, escaped, "other"] {
            room(&mut source, id, "public");
        }
        add(&mut source, hostile, "mine", "r1", "2026-10-06T09:00:00Z");
        add(
            &mut source,
            escaped,
            "slashed",
            "r1",
            "2026-10-06T09:00:01Z",
        );
        add(&mut source, "other", "theirs", "r1", "2026-10-06T09:00:02Z");
        for (container, literal, own) in [
            (hostile, r#""x\" OR room = \"other""#, "mine@r1"),
            (escaped, r#""back\\slash \\\" quote""#, "slashed@r1"),
        ] {
            let (page, requests) = catalog.requests(
                &source,
                "feed.items",
                json!({"container": container, "limit": 10}),
            );
            let page = page.unwrap_or_else(|e| panic!("{path:?}: {container}: {e}"));
            assert_eq!(delivered(&page), [own], "{path:?}");
            assert_eq!(
                requests[1].1[&parameter],
                format!("room = {literal} ORDER BY edited ASC"),
                "{path:?}"
            );
            let (resumed, requests) = catalog.requests(
                &source,
                "feed.items",
                json!({"container": container, "watermark": page["next_watermark"], "limit": 10}),
            );
            assert_eq!(delivered(&resumed.unwrap()), Vec::<String>::new());
            assert_eq!(
                requests[1].1[&parameter],
                format!(r#"room = {literal} AND edited >= "2026-10-06 09:00" ORDER BY edited ASC"#),
                "{path:?}"
            );
        }
        let broken = "line\nbreak";
        room(&mut source, broken, "public");
        let (refused, requests) = catalog.requests(
            &source,
            "feed.items",
            json!({"container": broken, "limit": 10}),
        );
        assert_eq!(
            refused.unwrap_err().code,
            ErrorCode::InvalidInput,
            "{path:?}"
        );
        assert!(
            requests
                .iter()
                .all(|(sent, _)| !sent.starts_with("message")),
            "{path:?}: an item listing was sent: {requests:?}"
        );
    }
}

/// A listing continued by a next link reads only the named parameter of it, decoded, and never
/// requests the link; it ends on the answer that carries no link. A link that lost the parameter
/// is `unavailable`, never complete.
#[test]
fn a_next_link_listing_reads_one_parameter_and_ends_without_a_link() {
    let linked: Vec<Declared> = declared()
        .into_iter()
        .filter(|d| d.raw["feed"]["containers"]["cursor"]["url_parameter"].is_string())
        .collect();
    assert!(!linked.is_empty());
    for declaration in linked {
        let path = &declaration.path;
        let parameter = declaration.raw["feed"]["containers"]["cursor"]["parameter"]
            .as_str()
            .unwrap()
            .to_owned();
        let catalog = Catalog::new(&declaration);
        let mut source = Source::default();
        for id in ["a", "b", "c"] {
            room(&mut source, id, "public");
        }
        let (mut cursor, mut ids, mut sent) = (None::<String>, Vec::new(), Vec::new());
        for _ in 0..10 {
            let mut input = json!({"limit": 1});
            if let Some(cursor) = &cursor {
                input["cursor"] = json!(cursor);
            }
            let (page, requests) = catalog.requests(&source, "feed.containers", input);
            let page = page.unwrap_or_else(|e| panic!("{path:?}: {e}"));
            assert_eq!(requests.len(), 1, "{path:?}: {requests:?}");
            sent.push(requests[0].1.get(&parameter).cloned());
            ids.extend(listed(&page));
            if page["complete"] == json!(true) {
                assert!(page["next_cursor"].is_null());
                break;
            }
            cursor = page["next_cursor"].as_str().map(str::to_owned);
        }
        assert_eq!(ids, ["a", "b", "c"], "{path:?}");
        assert_eq!(
            sent,
            [None, Some("p:1".to_owned()), Some("p:2".to_owned())],
            "{path:?}"
        );
        let mut lost = Source::default();
        room(&mut lost, "a", "public");
        room(&mut lost, "b", "public");
        lost.containers.get_mut("a").unwrap().kind = "link-without-start".into();
        let (refused, _) = catalog.requests(&lost, "feed.containers", json!({"limit": 1}));
        assert_eq!(
            refused.unwrap_err().code,
            ErrorCode::Unavailable,
            "{path:?}"
        );
    }
}

/// An offset listing sends the index of the next page's first record, and ends on the answer its
/// declared flag calls the last. An empty answer that is not the last is `unavailable`, never
/// complete.
#[test]
fn an_offset_listing_ends_on_its_last_flag() {
    let offset: Vec<Declared> = declared()
        .into_iter()
        .filter(|d| d.raw["feed"]["containers"]["cursor"]["offset"].is_object())
        .collect();
    assert!(!offset.is_empty());
    for declaration in offset {
        let path = &declaration.path;
        let parameter = declaration.raw["feed"]["containers"]["cursor"]["parameter"]
            .as_str()
            .unwrap()
            .to_owned();
        let catalog = Catalog::new(&declaration);
        let mut source = Source::default();
        for id in ["a", "b", "c"] {
            room(&mut source, id, "public");
        }
        let (first, requests) = catalog.requests(&source, "feed.containers", json!({"limit": 2}));
        let first = first.unwrap();
        assert_eq!(listed(&first), ["a", "b"], "{path:?}");
        assert_eq!(first["complete"], json!(false));
        assert!(!requests[0].1.contains_key(&parameter));
        let cursor = first["next_cursor"].clone();
        let (second, requests) = catalog.requests(
            &source,
            "feed.containers",
            json!({"limit": 2, "cursor": cursor}),
        );
        let second = second.unwrap();
        assert_eq!(requests[0].1[&parameter], "2", "{path:?}");
        assert_eq!(listed(&second), ["c"], "{path:?}");
        assert_eq!(second["complete"], json!(true));
        assert!(second["next_cursor"].is_null());
        let mut fewer = Source::default();
        room(&mut fewer, "a", "public");
        let (lost, _) = catalog.requests(
            &fewer,
            "feed.containers",
            json!({"limit": 2, "cursor": cursor}),
        );
        assert_eq!(lost.unwrap_err().code, ErrorCode::Unavailable, "{path:?}");
    }
}

/// The first item of `room` as `catalog` reads it, and that item's link.
fn first_link(catalog: &Catalog) -> Value {
    let mut source = Source::default();
    room(&mut source, "room", "public");
    add(&mut source, "room", "a", "r1", "2026-10-06T09:00:00Z");
    let page = catalog
        .call(
            &source,
            "feed.items",
            json!({"container": "room", "limit": 1}),
        )
        .0
        .unwrap();
    page["items"][0]["url"].clone()
}

/// An item link the provider writes as a path rooted at `/` resolves under the base its answer
/// names, when the declaration points at one, and otherwise under the origin of the API base;
/// with neither it is no link, never a guess. An absolute link is kept as it is.
#[test]
fn a_relative_item_link_resolves_under_the_answers_base_or_the_origin() {
    let mut relative = 0;
    for declaration in declared() {
        let path = &declaration.path;
        let link = first_link(&Catalog::new(&declaration));
        let text = link.as_str().unwrap_or_else(|| panic!("{path:?}: no link"));
        assert!(
            text.starts_with("https://feed.fixture.test/") && text.ends_with("/rooms/room/a"),
            "{path:?}: {text}"
        );
        if declaration.raw["feed"]["items"].get("url_base").is_none() {
            continue;
        }
        relative += 1;
        assert_eq!(link, json!(format!("{SERVER}/rooms/room/a")), "{path:?}");
        let mut raw = declaration.raw.clone();
        raw["feed"]["items"]
            .as_object_mut()
            .unwrap()
            .remove("url_base");
        let feed: Declaration = serde_json::from_value(raw["feed"].clone()).unwrap();
        let unbased = Declared {
            path: declaration.path.clone(),
            raw,
            feed: feed.clone(),
        };
        assert_eq!(
            first_link(&Catalog::new(&unbased)),
            json!("https://feed.fixture.test/rooms/room/a"),
            "{path:?}"
        );
        let (bundle, base) = bundle(declaration.raw["provider"].as_str().unwrap());
        let unrooted = Catalog {
            engine: Engine::with_feed(&bundle, &base, &[], Some(&feed)).unwrap(),
            profile: feed.profile.clone(),
            capabilities: claimed(&feed),
        };
        assert_eq!(first_link(&unrooted), Value::Null, "{path:?}");
        // An API base whose host is no hostname has no origin to resolve under.
        let literal = Catalog {
            engine: Engine::with_feed(&bundle, &base, &[], Some(&feed))
                .and_then(|engine| engine.with_link_origin("https://[fd00::1]/v1"))
                .unwrap(),
            profile: feed.profile.clone(),
            capabilities: claimed(&feed),
        };
        assert_eq!(first_link(&literal), Value::Null, "{path:?}");
    }
    assert!(
        relative > 0,
        "no fixture writes a relative item link under a declared base"
    );
}

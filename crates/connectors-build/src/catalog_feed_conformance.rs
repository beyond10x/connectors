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
    path::PathBuf,
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
/// asked with `order=asc`) or through the provider's change cursor (`changes`), which refuses a
/// cursor it never issued with 410. Instants compare as text: every instant the suite writes has
/// the one form `YYYY-MM-DDTHH:MM:SSZ`.
struct Simulated<'a> {
    source: &'a Source,
    calls: Mutex<Vec<String>>,
}

impl<'a> Simulated<'a> {
    fn new(source: &'a Source) -> Self {
        Self {
            source,
            calls: Mutex::default(),
        }
    }

    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
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
        let limit = match get("limit").map(str::parse::<usize>) {
            None => 100,
            Some(Ok(limit)) if (1..=100).contains(&limit) => limit,
            Some(_) => return (400, json!({"error": "limit"})),
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
        let (status, body) = self.answer(segments, query);
        Ok(HttpResponse {
            status,
            headers: BTreeMap::new(),
            body: serde_json::to_vec(&body).unwrap(),
        })
    }
}

// ---- the binding under test ------------------------------------------------------------------

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
        let engine =
            Engine::with_feed(&bundle, &base, &[], Some(&declared.feed)).unwrap_or_else(|e| {
                panic!("{:?}: {e}", declared.path);
            });
        // The declaration's claim, read in the family's vocabulary: a word one side does not
        // know is a vocabulary that moved on one side only.
        let capabilities =
            serde_json::from_value(serde_json::to_value(declared.feed.capabilities).unwrap())
                .unwrap_or_else(|e| panic!("{:?}: {e}", declared.path));
        Self {
            engine,
            profile: declared.feed.profile.clone(),
            capabilities,
        }
    }

    /// One invocation and the provider requests it made.
    fn call(
        &self,
        source: &Source,
        operation: &str,
        input: Value,
    ) -> (Result<Value, Error>, Vec<String>) {
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
        (result, http.calls())
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
/// nothing to `public` claims every container private. Each scenario left out is named with the
/// capability that left it out.
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
    assert_eq!(run.counts.total, feed_conformance::EXPECTED_SCENARIOS - 5);
    assert!(
        !run.left_out
            .iter()
            .any(|left| left.scenario.ends_with("listing-omits-direct-conversation"))
    );
    let named: BTreeSet<&str> = run
        .left_out
        .iter()
        .flat_map(|left| left.by.iter().map(|capability| capability.name()))
        .collect();
    assert_eq!(
        named,
        BTreeSet::from(["deletions: not-observed", "visibility: all-private"])
    );
}

/// Every declaration is a value of the adapter's own model (`adapters/catalog/spec/ess`), and the
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
    for declaration in declared() {
        let errors: Vec<String> = validator
            .iter_errors(&declaration.raw["feed"])
            .map(|e| format!("{}: {e}", e.instance_path()))
            .collect();
        assert!(errors.is_empty(), "{:?}: {errors:?}", declaration.path);
        let written = serde_json::to_value(&declaration.feed).unwrap();
        assert_eq!(written, declaration.raw["feed"], "{:?}", declaration.path);
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

/// No Rust in the catalog engine, or in this suite adapter, names a declared provider: not its
/// provider id, its profile or an operation it reads.
#[test]
fn no_rust_names_a_declared_provider() {
    let root = feed_conformance::root();
    let mut sources = vec![root.join("crates/connectors-build/src/catalog_feed_conformance.rs")];
    for entry in std::fs::read_dir(root.join("adapters/catalog/src")).unwrap() {
        sources.push(entry.unwrap().path());
    }
    for declaration in declared() {
        let feed = &declaration.raw["feed"];
        let names = [
            &declaration.raw["provider"],
            &feed["profile"],
            &feed["containers"]["operation_id"],
            &feed["containers"]["lookup"]["operation_id"],
            &feed["items"]["operation_id"],
        ];
        for source in &sources {
            let text = std::fs::read_to_string(source).unwrap();
            for name in names {
                let name = name.as_str().unwrap();
                assert!(!text.contains(name), "{source:?} names `{name}`");
            }
        }
    }
}

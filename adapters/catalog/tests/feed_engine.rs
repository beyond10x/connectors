//! A feed declaration in a selection file adds `feed.containers` and `feed.items` to the
//! engine, under the family's contract and the declaration's profile; a declaration the bundle
//! cannot carry is refused when the engine is built, before any request; and the local service
//! declares both operations from its `operations_file`. The suite run is
//! `crates/connectors-build/src/catalog_feed_conformance.rs`.
#![cfg(unix)]
use connectors_catalog::{
    bundle::{self, Bundle},
    ingest, inventory,
};
use connectors_catalog_provider::{
    Effect, Engine, Selection,
    feed::{CONTAINERS, Declaration, FEED_CONTRACT, ITEMS},
};
use connectors_host::local::filesystem;
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};

fn fixtures() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/feed")
}

fn selection_file(name: &str) -> Value {
    serde_json::from_slice(&fs::read(fixtures().join(name)).unwrap()).unwrap()
}

fn bundle() -> Bundle {
    let bytes = fs::read(fixtures().join("api.json")).unwrap();
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    Bundle {
        provider: selection_file("time.operations.json")["provider"]
            .as_str()
            .unwrap()
            .into(),
        source: ingest("api.json", &bytes).unwrap(),
        inventory: inventory::extract(&document),
        auth_profile: "fixture.token".into(),
    }
}

fn declaration(name: &str) -> Value {
    selection_file(name)["feed"].clone()
}

fn build(feed: &Value, selections: &[Selection]) -> connectors_core::Result<Engine> {
    let feed: Declaration = serde_json::from_value(feed.clone()).expect("a declaration");
    Engine::with_feed(&bundle(), "/v1", selections, Some(&feed))
}

#[test]
fn a_feed_declaration_adds_the_two_family_operations_under_its_profile() {
    for name in [
        "time.operations.json",
        "cursor.operations.json",
        "time-query.operations.json",
        "time-next.operations.json",
    ] {
        let feed = declaration(name);
        let engine = build(&feed, &[]).unwrap_or_else(|e| panic!("{name}: {}", e.message));
        let declared = engine.declarations(&[Effect::Read]);
        let ids: Vec<&str> = declared.iter().map(|o| o.id.as_str()).collect();
        assert_eq!(ids, [CONTAINERS, ITEMS], "{name}");
        for operation in &declared {
            assert_eq!(operation.contract, FEED_CONTRACT);
            assert_eq!(operation.contract, "datasource.feed/v1alpha1");
            assert_eq!(json!(operation.profile), feed["profile"]);
            assert_eq!(engine.effect(&operation.id), Some(Effect::Read));
        }
        let items = &declared[1].input_schema;
        assert_eq!(items["required"], json!(["container", "limit"]));
        assert_eq!(items["properties"]["limit"]["maximum"], feed["max_limit"]);
        assert!(engine.declarations(&[Effect::Write]).is_empty());
    }
}

#[test]
fn without_a_declaration_the_engine_is_unchanged() {
    let selection: Selection = serde_json::from_value(
        json!({"id": "rooms.list", "operation_id": "listRooms", "effect": "read"}),
    )
    .unwrap();
    let engine = Engine::new(&bundle(), "/v1", std::slice::from_ref(&selection)).unwrap();
    let ids: Vec<String> = engine
        .declarations(&[Effect::Read, Effect::Write])
        .into_iter()
        .map(|o| o.id)
        .collect();
    assert_eq!(ids, ["rooms.list"]);
    assert_eq!(engine.effect(ITEMS), None);
    // An engine with neither a selection nor a feed declares nothing and is refused.
    assert!(Engine::with_feed(&bundle(), "/v1", &[], None).is_err());
}

/// Each case changes one field of a valid declaration; each is refused when the engine is built.
#[test]
fn a_declaration_the_bundle_cannot_carry_is_refused_before_any_request() {
    let base = declaration("time.operations.json");
    let cursor = declaration("cursor.operations.json");
    let unbound = with(
        &with(&base, "/items/query", json!({})),
        "/items/container",
        json!("order"),
    );
    let cases: Vec<(&str, Value, &str)> = vec![
        (
            "unknown list operation",
            with(&base, "/containers/operation_id", json!("missing")),
            "no operation `missing`",
        ),
        (
            "unknown lookup",
            with(&base, "/containers/lookup/operation_id", json!("missing")),
            "no operation `missing`",
        ),
        (
            "unknown items operation",
            with(&base, "/items/operation_id", json!("missing")),
            "no operation `missing`",
        ),
        (
            "container parameter absent",
            with(&base, "/items/container", json!("channel")),
            "binds `channel`",
        ),
        (
            "lookup parameter absent",
            with(&base, "/containers/lookup/parameter", json!("id")),
            "binds `id`",
        ),
        (
            "limit not a query parameter",
            with(&base, "/items/limit", json!("room")),
            "binds `room`",
        ),
        (
            "time parameter absent",
            with(&base, "/items/position/value/parameter", json!("since")),
            "binds `since`",
        ),
        (
            "cursor parameter absent",
            with(&base, "/containers/cursor/parameter", json!("page")),
            "binds `page`",
        ),
        (
            "constant not a parameter",
            with(&base, "/items/query", json!({"sort": "asc"})),
            "binds `sort`",
        ),
        (
            "constant on a bound parameter",
            with(&base, "/items/query", json!({"limit": "5"})),
            "`limit` of `listMessages` twice",
        ),
        (
            "resume parameter bound always",
            with(&base, "/items/container", json!("updated_since")),
            "`updated_since` of `listMessages` twice",
        ),
        (
            "required parameter left unbound",
            unbound,
            "leaves `room` of `listMessages` unbound",
        ),
        (
            "pointer without a slash",
            with(&base, "/items/id", json!("id")),
            "`items.id` is not a JSON pointer",
        ),
        (
            "empty representation",
            with(&base, "/items/body/representation", json!("")),
            "no representation",
        ),
        (
            "max_limit zero",
            with(&base, "/max_limit", json!(0)),
            "max_limit",
        ),
        (
            "max_limit above 100",
            with(&base, "/max_limit", json!(101)),
            "max_limit",
        ),
        (
            "empty profile",
            with(&base, "/profile", json!("")),
            "profile",
        ),
        (
            "stale status not a refusal",
            with(&cursor, "/items/position/value/stale", json!([500])),
            "stale statuses",
        ),
        (
            "listing continued both ways",
            adding(&base, "/containers/cursor", "last", json!("/id")),
            "exactly one of `next` and `last`",
        ),
        (
            "listing continued neither way",
            removing(&base, "/containers/cursor", "next"),
            "exactly one of `next` and `last`",
        ),
        (
            "last record key not a pointer",
            adding(
                &removing(&base, "/containers/cursor", "next"),
                "/containers/cursor",
                "last",
                json!("id"),
            ),
            "`containers.cursor.last` is not a JSON pointer",
        ),
        (
            "kind read and fixed",
            adding(&base, "/containers", "kind_word", json!("room")),
            "exactly one of `kind` and `kind_word`",
        ),
        (
            "kind neither read nor fixed",
            removing(&base, "/containers", "kind"),
            "exactly one of `kind` and `kind_word`",
        ),
        ("empty kind word", fixed_word(&base, json!("")), "kind_word"),
        (
            "kind word with a space",
            fixed_word(&base, json!("merge request")),
            "kind_word",
        ),
        (
            "kind word longer than 128 bytes",
            fixed_word(&base, json!("k".repeat(129))),
            "kind_word",
        ),
        (
            "kind word under a provider-word claim",
            with(
                &fixed_word(&base, json!("room")),
                "/capabilities/kind",
                json!("provider-word"),
            ),
            "`kind: provider-word`",
        ),
    ];
    for (case, feed, reason) in cases {
        match build(&feed, &[]) {
            Ok(_) => panic!("{case}: accepted"),
            Err(error) => assert!(error.message.contains(reason), "{case}: {}", error.message),
        }
    }
    // A selection may not take a family id for an operation of another meaning.
    let colliding: Selection =
        serde_json::from_value(json!({"id": ITEMS, "operation_id": "listRooms", "effect": "read"}))
            .unwrap();
    let refused = build(&base, &[colliding])
        .err()
        .expect("selection under a family id");
    assert!(
        refused.message.contains("an id of the feed family"),
        "{}",
        refused.message
    );
}

/// Each case changes one field of a valid offset, next-link or query declaration; each is
/// refused when the engine is built. A query template is checked whole before any request: a
/// slot it does not know, a slot where a quote or backslash would let a value leave its literal,
/// and text the provider would read as something other than the declared query.
#[test]
fn a_listing_shape_or_query_template_the_engine_cannot_hold_is_refused_before_any_request() {
    let time = declaration("time.operations.json");
    let query = declaration("time-query.operations.json");
    let next = declaration("time-next.operations.json");
    let first = "/items/position/value/query/first";
    let resume = "/items/position/value/query/resume";
    let cases: Vec<(&str, Value, &str)> = vec![
        (
            "offset beside a next pointer",
            adding(
                &time,
                "/containers/cursor",
                "offset",
                json!({"complete": {"pointer": "/isLast", "equals": true}}),
            ),
            "exactly one of `next` and `last`",
        ),
        (
            "url parameter without a next pointer",
            adding(
                &query,
                "/containers/cursor",
                "url_parameter",
                json!("startAt"),
            ),
            "`containers.cursor.url_parameter` reads a `next` link",
        ),
        (
            "empty url parameter",
            with(&next, "/containers/cursor/url_parameter", json!("")),
            "`containers.cursor.url_parameter` is not 1 to 128 visible ASCII characters",
        ),
        (
            "url parameter with a space",
            with(&next, "/containers/cursor/url_parameter", json!("st art")),
            "`containers.cursor.url_parameter` is not 1 to 128 visible ASCII characters",
        ),
        (
            "offset end flag not a pointer",
            with(
                &query,
                "/containers/cursor/offset/complete/pointer",
                json!("isLast"),
            ),
            "`containers.cursor.offset.complete.pointer` is not a JSON pointer",
        ),
        (
            "time read by a parameter and a query",
            adding(
                &query,
                "/items/position/value",
                "parameter",
                json!("pageToken"),
            ),
            "exactly one of `parameter` and `query`",
        ),
        (
            "time read by neither",
            removing(&query, "/items/position/value", "query"),
            "exactly one of `parameter` and `query`",
        ),
        (
            "query without minute precision",
            removing(&query, "/items/position/value", "precision"),
            "a query reads at `precision: minute`",
        ),
        (
            "query at instant precision",
            with(&query, "/items/position/value/precision", json!("instant")),
            "a query reads at `precision: minute`",
        ),
        (
            "container parameter beside a query",
            adding(&query, "/items", "container", json!("maxResults")),
            "`items.container` is absent exactly when a query selects the container",
        ),
        (
            "no container parameter and no query",
            removing(&time, "/items", "container"),
            "`items.container` is absent exactly when a query selects the container",
        ),
        (
            "unknown slot",
            with(&query, first, json!("room = {room} ORDER BY edited ASC")),
            "unknown slot `{room}`",
        ),
        (
            "first read with a time",
            with(
                &query,
                first,
                json!("room = {container} AND edited >= {since}"),
            ),
            "`items.position.query.first` names `{container}` once and `{since}` never",
        ),
        (
            "resume without a time",
            with(
                &query,
                resume,
                json!("room = {container} ORDER BY edited ASC"),
            ),
            "`items.position.query.resume` names `{container}` and `{since}` once each",
        ),
        (
            "container twice",
            with(
                &query,
                resume,
                json!("room = {container} OR room = {container} AND edited >= {since}"),
            ),
            "`items.position.query.resume` names `{container}` and `{since}` once each",
        ),
        (
            "slot inside double quotes",
            with(&query, first, json!("room = \"{container}\"")),
            "beside a quote or backslash",
        ),
        (
            "slot inside single quotes",
            with(&query, first, json!("room = '{container}'")),
            "beside a quote or backslash",
        ),
        (
            "slot after a backslash",
            with(&query, first, json!("room = \\{container}")),
            "beside a quote or backslash",
        ),
        (
            "slot inside a literal, apart from its quotes",
            with(&query, first, json!("text ~ \"a {container} b\"")),
            "puts a slot inside a literal",
        ),
        (
            "slot after an escaped quote inside a literal",
            with(&query, first, json!("text ~ \"a\\\" {container} b\"")),
            "puts a slot inside a literal",
        ),
        (
            "literal left open",
            with(&query, first, json!("room = {container} AND text ~ 'open")),
            "leaves a literal open",
        ),
        (
            "backslash outside a literal",
            with(&query, first, json!("room = {container} AND text ~ a\\ b")),
            "backslash outside a literal",
        ),
        (
            "stray closing brace",
            with(&query, first, json!("room = {container} }")),
            "stray brace",
        ),
        (
            "unclosed slot",
            with(&query, first, json!("room = {container")),
            "stray brace",
        ),
        (
            "control character",
            with(
                &query,
                first,
                json!("room = {container}\nORDER BY edited ASC"),
            ),
            "control character",
        ),
        (
            "template longer than 1024 bytes",
            with(
                &query,
                first,
                json!(format!("room = {{container}}{}", " ".repeat(1024))),
            ),
            "longer than 1024 bytes",
        ),
        (
            "query parameter absent",
            with(
                &query,
                "/items/position/value/query/parameter",
                json!("jql"),
            ),
            "binds `jql`",
        ),
        (
            "query parameter bound twice",
            with(
                &query,
                "/items/position/value/query/parameter",
                json!("maxResults"),
            ),
            "`maxResults` of `searchMessages` twice",
        ),
        (
            "page parameter absent",
            with(
                &query,
                "/items/position/value/page/parameter",
                json!("token"),
            ),
            "binds `token`",
        ),
        (
            "page continuation not a pointer",
            with(
                &query,
                "/items/position/value/page/next",
                json!("nextPageToken"),
            ),
            "`items.position.page.next` is not a JSON pointer",
        ),
        (
            "link base not a pointer",
            with(&next, "/items/url_base", json!("_links/base")),
            "`items.url_base` is not a JSON pointer",
        ),
        (
            "empty page url parameter",
            with(&next, "/items/position/value/page/url_parameter", json!("")),
            "`items.position.page.url_parameter` is not 1 to 128 visible ASCII characters",
        ),
    ];
    for (case, feed, reason) in cases {
        match build(&feed, &[]) {
            Ok(_) => panic!("{case}: accepted"),
            Err(error) => assert!(error.message.contains(reason), "{case}: {}", error.message),
        }
    }
    // Root-relative item links resolve under the origin of an https API base; a base with no
    // hostname origin leaves them `null` and never refuses a feed that loaded without it.
    let engine = || build(&query, &[]).unwrap();
    for base in [
        "https://feed.fixture.test/v1/",
        "https://[fd00::1]/v1",
        "https://feed.fixture.test./v1",
        "/v1",
        "http://feed.fixture.test/v1",
        "ftp://feed.fixture.test",
        "",
    ] {
        assert!(engine().with_link_origin(base).is_ok(), "{base}");
    }
}

fn with(feed: &Value, pointer: &str, value: Value) -> Value {
    let mut changed = feed.clone();
    *changed.pointer_mut(pointer).expect(pointer) = value;
    changed
}

fn adding(feed: &Value, object: &str, field: &str, value: Value) -> Value {
    let mut changed = feed.clone();
    changed
        .pointer_mut(object)
        .and_then(Value::as_object_mut)
        .expect(object)
        .insert(field.into(), value);
    changed
}

fn removing(feed: &Value, object: &str, field: &str) -> Value {
    let mut changed = feed.clone();
    changed
        .pointer_mut(object)
        .and_then(Value::as_object_mut)
        .expect(object)
        .remove(field)
        .expect(field);
    changed
}

/// `feed` with its container kind fixed to `word` and the claim that goes with it.
fn fixed_word(feed: &Value, word: Value) -> Value {
    let fixed = adding(
        &removing(feed, "/containers", "kind"),
        "/containers",
        "kind_word",
        word,
    );
    with(&fixed, "/capabilities/kind", json!("fixed-word"))
}

/// The rooms listing as a provider that lists after a key it is given answers it.
struct Rooms {
    pages: std::sync::Mutex<Vec<Value>>,
    queries: std::sync::Mutex<Vec<Vec<(String, String)>>>,
}

#[async_trait::async_trait]
impl connectors_sdk::AuthenticatedHttp for Rooms {
    async fn get(
        &self,
        path: &[&str],
        query: &[(&str, String)],
    ) -> connectors_core::Result<connectors_sdk::HttpResponse> {
        assert_eq!(path, ["rooms"]);
        let mut query: Vec<(String, String)> = query
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect();
        query.sort();
        self.queries.lock().unwrap().push(query);
        let page = self.pages.lock().unwrap().remove(0);
        Ok(connectors_sdk::HttpResponse {
            status: 200,
            headers: Default::default(),
            body: serde_json::to_vec(&page).unwrap(),
        })
    }
}

/// A room the lookup answers, then the item listing answered by `listing`.
struct Refusing {
    listing: fn() -> connectors_core::Result<connectors_sdk::HttpResponse>,
}

#[async_trait::async_trait]
impl connectors_sdk::AuthenticatedHttp for Refusing {
    async fn get(
        &self,
        path: &[&str],
        _query: &[(&str, String)],
    ) -> connectors_core::Result<connectors_sdk::HttpResponse> {
        match path {
            ["rooms", "room"] => Ok(connectors_sdk::HttpResponse {
                status: 200,
                headers: Default::default(),
                body: serde_json::to_vec(&json!({"room": {"id": "room", "access": "invite"}}))
                    .unwrap(),
            }),
            ["rooms", "room", _] => (self.listing)(),
            other => panic!("unexpected read {other:?}"),
        }
    }
}

/// Once the lookup answered the container, the provider's own 403, 404 or 410 on its item
/// listing is a container the connection cannot read: `not_found`, with the message an unknown
/// container gets. A quota refusal is not that, and neither is a refusal the host raised before
/// any answer, whose code and message are kept.
#[tokio::test]
async fn a_provider_refusal_of_a_visible_containers_items_is_not_found() {
    for name in ["time.operations.json", "cursor.operations.json"] {
        let engine = build(&declaration(name), &[]).unwrap();
        let read = |listing| {
            let engine = &engine;
            async move {
                engine
                    .read(
                        &Refusing { listing },
                        "feed-instance",
                        ITEMS,
                        json!({"container": "room", "limit": 10}),
                    )
                    .await
                    .unwrap_err()
            }
        };
        let forbidden = read(|| {
            Ok(connectors_sdk::HttpResponse {
                status: 403,
                headers: Default::default(),
                body: br#"{"message":"403 Forbidden"}"#.to_vec(),
            })
        })
        .await;
        assert_eq!(
            forbidden.code,
            connectors_core::ErrorCode::NotFound,
            "{name}"
        );
        assert_eq!(forbidden.message, "container not found", "{name}");
        let missing = read(|| {
            Ok(connectors_sdk::HttpResponse {
                status: 404,
                headers: Default::default(),
                body: br#"{"message":"404 Not found"}"#.to_vec(),
            })
        })
        .await;
        let gone = read(|| {
            Ok(connectors_sdk::HttpResponse {
                status: 410,
                headers: Default::default(),
                body: br#"{"message":"410 Gone"}"#.to_vec(),
            })
        })
        .await;
        for (status, error) in [(404, &missing), (410, &gone)] {
            assert_eq!(
                (&error.code, error.message.as_str()),
                (&connectors_core::ErrorCode::NotFound, "container not found"),
                "{name}: listing {status}"
            );
        }
        let busy = read(|| {
            Ok(connectors_sdk::HttpResponse {
                status: 429,
                headers: Default::default(),
                body: b"{}".to_vec(),
            })
        })
        .await;
        assert_eq!(busy.code, connectors_core::ErrorCode::RateLimited, "{name}");
        let host = read(|| {
            Err(connectors_core::Error::new(
                connectors_core::ErrorCode::Forbidden,
                "the host refused the read",
            ))
        })
        .await;
        assert_eq!(host.code, connectors_core::ErrorCode::Forbidden, "{name}");
        let host = read(|| {
            Err(connectors_core::Error::new(
                connectors_core::ErrorCode::NotFound,
                "the host found no such route",
            ))
        })
        .await;
        assert_eq!(
            (&host.code, host.message.as_str()),
            (
                &connectors_core::ErrorCode::NotFound,
                "the host found no such route"
            ),
            "{name}"
        );
    }
}

/// A room the lookup answers, or does not, and an item listing answered with `status`; every
/// item request's query is kept.
struct Admitting {
    known: bool,
    status: u16,
    listed: Value,
    queries: std::sync::Mutex<Vec<Vec<String>>>,
}

#[async_trait::async_trait]
impl connectors_sdk::AuthenticatedHttp for Admitting {
    async fn get(
        &self,
        path: &[&str],
        query: &[(&str, String)],
    ) -> connectors_core::Result<connectors_sdk::HttpResponse> {
        let (status, body) = match path {
            ["rooms", "room"] if self.known => {
                (200, json!({"room": {"id": "room", "access": "invite"}}))
            }
            ["rooms", "room"] => (404, json!({"error": "no such room"})),
            ["rooms", "room", _] => {
                let mut names: Vec<String> = query.iter().map(|(k, _)| k.to_string()).collect();
                names.sort();
                self.queries.lock().unwrap().push(names);
                (self.status, self.listed.clone())
            }
            other => panic!("unexpected read {other:?}"),
        };
        Ok(connectors_sdk::HttpResponse {
            status,
            headers: Default::default(),
            body: serde_json::to_vec(&body).unwrap(),
        })
    }
}

/// Admission of the container precedes whether a watermark is valid: with a watermark this read
/// cannot resume (another container's, another profile's, or none at all), an unknown room and
/// one whose items the provider refuses both answer `not_found` with one message, and only a
/// room whose listing answers is `stale_cursor`. The listing that decides it is a first read: it
/// carries no resume parameter, and none of its records is returned.
#[tokio::test]
async fn a_watermark_is_judged_only_after_the_container_is_admitted() {
    for (name, listed) in [
        ("time.operations.json", json!({"messages": []})),
        (
            "cursor.operations.json",
            json!({"changes": [], "cursor": "7", "has_more": false}),
        ),
    ] {
        let engine = build(&declaration(name), &[]).unwrap();
        let read = |known: bool, status: u16, watermark: Option<&str>| {
            let http = Admitting {
                known,
                status,
                listed: listed.clone(),
                queries: Default::default(),
            };
            let engine = &engine;
            let mut input = json!({"container": "room", "limit": 10});
            if let Some(watermark) = watermark {
                input["watermark"] = json!(watermark);
            }
            async move {
                let answer = engine.read(&http, "feed-instance", ITEMS, input).await;
                (answer, http.queries.into_inner().unwrap())
            }
        };
        let (issued, _) = read(true, 200, None).await;
        let issued = issued.unwrap()["next_watermark"]
            .as_str()
            .unwrap()
            .to_owned();
        let other = build(
            &with(&declaration(name), "/profile", json!("another-profile/1")),
            &[],
        )
        .unwrap()
        .read(
            &Admitting {
                known: true,
                status: 200,
                listed: listed.clone(),
                queries: Default::default(),
            },
            "feed-instance",
            ITEMS,
            json!({"container": "room", "limit": 10}),
        )
        .await
        .unwrap()["next_watermark"]
            .as_str()
            .unwrap()
            .to_owned();
        for watermark in [other.as_str(), "not-a-watermark", ""] {
            let (unknown, queries) = read(false, 200, Some(watermark)).await;
            let unknown = unknown.unwrap_err();
            assert_eq!(unknown.code, connectors_core::ErrorCode::NotFound, "{name}");
            assert!(
                queries.is_empty(),
                "{name}: an unknown room's items were read"
            );
            let (refused, _) = read(true, 403, Some(watermark)).await;
            let refused = refused.unwrap_err();
            assert_eq!(
                (refused.code, &refused.message),
                (unknown.code, &unknown.message),
                "{name}: {watermark:?}"
            );
            let (stale, queries) = read(true, 200, Some(watermark)).await;
            assert_eq!(
                stale.unwrap_err().code,
                connectors_core::ErrorCode::StaleCursor,
                "{name}: {watermark:?}"
            );
            assert_eq!(queries.len(), 1, "{name}");
            assert!(
                !queries[0]
                    .iter()
                    .any(|q| q == "updated_since" || q == "cursor"),
                "{name}: the deciding listing resumed from {watermark:?}: {queries:?}"
            );
        }
        // The watermark this read issued still resumes.
        let (resumed, _) = read(true, 200, Some(&issued)).await;
        assert!(resumed.is_ok(), "{name}: {resumed:?}");
    }
}

/// A time position's rooms and listings as the generic fixtures declare them: the room is known
/// or not, and every listing answers `status` with `listed`. Each listing's query is recorded.
struct Judging {
    known: bool,
    status: u16,
    listed: Value,
    listings: std::sync::Mutex<Vec<Vec<(String, String)>>>,
}

#[async_trait::async_trait]
impl connectors_sdk::AuthenticatedHttp for Judging {
    async fn get(
        &self,
        path: &[&str],
        query: &[(&str, String)],
    ) -> connectors_core::Result<connectors_sdk::HttpResponse> {
        let (status, body) = match path {
            ["rooms", "room"] if self.known => {
                (200, json!({"room": {"id": "room", "access": "invite"}}))
            }
            ["rooms", "room"] => (404, json!({"error": "no such room"})),
            ["rooms", "room", "messages"] | ["message-search"] | ["message-query"] => {
                self.listings.lock().unwrap().push(
                    query
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.clone()))
                        .collect(),
                );
                (self.status, self.listed.clone())
            }
            other => panic!("unexpected read {other:?}"),
        };
        Ok(connectors_sdk::HttpResponse {
            status,
            headers: Default::default(),
            body: serde_json::to_vec(&body).unwrap(),
        })
    }
}

/// A watermark in the engine's own format for `feed`'s profile, the instance and the room, kept
/// at `time` and holding `seen` pairs at it.
fn kept(feed: &Value, time: &str, seen: usize) -> String {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    let pairs: Vec<Value> = (0..seen).map(|n| json!([format!("m{n}"), "1"])).collect();
    let mut mark = json!({"format": "catalog-feed/1", "profile": feed["profile"],
                          "instance": "feed-instance", "container": "room", "time": time});
    if seen > 0 {
        mark["seen"] = json!(pairs);
    }
    URL_SAFE_NO_PAD.encode(serde_json::to_vec(&mark).unwrap())
}

/// The class of `a_watermark_is_judged_only_after_the_container_is_admitted`, for a watermark
/// in this binding's own format that this read still cannot resume: one whose `{since}` falls
/// outside the years 0000 to 9999, at minute precision with a parameter, with a query, and with
/// a query shifted by `utc_offsets`, and one holding as many pairs as one provider page, with no
/// `page` to read past them. Each is no longer resumable, so each is `stale_cursor`, as the
/// family answers such a watermark. Every such answer comes only after the lookup and a
/// first-read listing have answered: an unknown room and a room whose listing the provider
/// refuses answer `not_found` alike, whatever the watermark, and the listing that decides it
/// carries no resume filter. One pair fewer resumes.
#[tokio::test]
async fn a_watermark_this_binding_issued_but_cannot_resume_is_judged_only_after_admission() {
    use connectors_core::ErrorCode::StaleCursor;
    let time = declaration("time.operations.json");
    let minute = adding(&time, "/items/position/value", "precision", json!("minute"));
    let query = declaration("time-query.operations.json");
    let offsets = |least: i64, greatest: i64| {
        adding(
            &query,
            "/items/position/value/query",
            "utc_offsets",
            json!({"least": least, "greatest": greatest}),
        )
    };
    let next = declaration("time-next.operations.json");
    let (low, high) = ("0000-01-01T00:30:00+01:00", "9999-12-31T23:59:00-01:00");
    let cases = [
        (
            "instant, full seen set",
            time.clone(),
            "2026-10-06T09:00:00Z",
            100,
            StaleCursor,
        ),
        (
            "minute parameter, low year",
            minute.clone(),
            low,
            0,
            StaleCursor,
        ),
        (
            "minute parameter, high year",
            minute.clone(),
            high,
            0,
            StaleCursor,
        ),
        (
            "minute parameter, full seen set",
            minute.clone(),
            "2026-10-06T09:00:00Z",
            100,
            StaleCursor,
        ),
        ("query, low year", query.clone(), low, 0, StaleCursor),
        ("query, high year", query.clone(), high, 0, StaleCursor),
        (
            "query shifted -120..0, low year",
            offsets(-120, 0),
            "0000-01-01T01:00:00Z",
            0,
            StaleCursor,
        ),
        (
            "query shifted 120..120, high year",
            offsets(120, 120),
            "9999-12-31T23:00:00Z",
            0,
            StaleCursor,
        ),
        ("next link, low year", next.clone(), low, 0, StaleCursor),
        ("next link, high year", next.clone(), high, 0, StaleCursor),
    ];
    let mut wrong = Vec::new();
    for (name, feed, at, seen, verdict) in cases {
        let engine = build(&feed, &[]).unwrap_or_else(|e| panic!("{name}: {}", e.message));
        let records = feed["items"]["records"]
            .as_str()
            .unwrap()
            .trim_start_matches('/');
        let mut listed = json!({});
        listed[records] = json!([]);
        let read = |known: bool, status: u16, watermark: String| {
            let http = Judging {
                known,
                status,
                listed: listed.clone(),
                listings: Default::default(),
            };
            let engine = &engine;
            async move {
                let input = json!({"container": "room", "limit": 10, "watermark": watermark});
                let answer = engine.read(&http, "feed-instance", ITEMS, input).await;
                let answer = answer.map_err(|error| (error.code, error.message));
                (answer, http.listings.into_inner().unwrap())
            }
        };
        let watermark = kept(&feed, at, seen);
        let (unknown, listings) = read(false, 200, watermark.clone()).await;
        if unknown.as_ref().err().map(|e| &e.0) != Some(&connectors_core::ErrorCode::NotFound)
            || !listings.is_empty()
        {
            wrong.push(format!(
                "{name}: unknown room {unknown:?}, {} listing(s)",
                listings.len()
            ));
        }
        let (refused, _) = read(true, 403, watermark.clone()).await;
        if refused.as_ref().err() != unknown.as_ref().err() {
            wrong.push(format!(
                "{name}: refused listing {refused:?}, unknown room {unknown:?}"
            ));
        }
        let (judged, listings) = read(true, 200, watermark.clone()).await;
        let resumed_from = |listing: &Vec<(String, String)>| {
            listing
                .iter()
                .any(|(k, v)| k == "updated_since" || v.contains(">="))
        };
        if judged.as_ref().err().map(|e| &e.0) != Some(&verdict)
            || listings.len() != 1
            || listings.iter().any(resumed_from)
        {
            wrong.push(format!("{name}: {judged:?} after listings {listings:?}"));
        }
        if seen > 0 {
            let (resumed, _) = read(true, 200, kept(&feed, at, seen - 1)).await;
            if resumed.is_err() {
                wrong.push(format!(
                    "{name}: one pair fewer did not resume: {resumed:?}"
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// A listing declared with `cursor.last` continues after the key of a full page's last record,
/// a direct one included; a shorter page is the end; a full page whose last record has no key is
/// `unavailable`, never complete. A fixed `kind_word` is every listed container's `kind`.
#[tokio::test]
async fn a_last_key_listing_continues_after_a_full_page_and_a_fixed_word_is_every_kind() {
    let base = declaration("time.operations.json");
    let keyed = adding(
        &removing(&base, "/containers/cursor", "next"),
        "/containers/cursor",
        "last",
        json!("/id"),
    );
    let feed = fixed_word(&keyed, json!("room"));
    let engine = build(&feed, &[]).unwrap_or_else(|error| panic!("{}", error.message));
    let room = |id: &str, access: &str| json!({"id": id, "title": id, "type": "channel", "access": access});
    let rooms = Rooms {
        pages: std::sync::Mutex::new(vec![
            json!({"rooms": [room("a", "open"), room("b", "im")]}),
            json!({"rooms": [room("c", "invite")]}),
            json!({"rooms": [room("d", "open"), {"title": "keyless", "access": "open"}]}),
        ]),
        queries: Default::default(),
    };
    let first = engine
        .read(&rooms, "feed-instance", CONTAINERS, json!({"limit": 2}))
        .await
        .unwrap();
    assert_eq!(
        first["containers"],
        json!([{"id": "a", "name": "a", "kind": "room", "visibility": "public"}])
    );
    assert_eq!(first["complete"], json!(false));
    let second = engine
        .read(
            &rooms,
            "feed-instance",
            CONTAINERS,
            json!({"limit": 2, "cursor": first["next_cursor"]}),
        )
        .await
        .unwrap();
    assert_eq!(
        second["containers"],
        json!([{"id": "c", "name": "c", "kind": "room", "visibility": "private"}])
    );
    assert_eq!(second["complete"], json!(true));
    assert_eq!(second["next_cursor"], Value::Null);
    let keyless = engine
        .read(&rooms, "feed-instance", CONTAINERS, json!({"limit": 2}))
        .await
        .unwrap_err();
    assert_eq!(keyless.code, connectors_core::ErrorCode::Unavailable);
    let pair = |k: &str, v: &str| (k.to_owned(), v.to_owned());
    assert_eq!(
        *rooms.queries.lock().unwrap(),
        [
            vec![pair("limit", "2")],
            vec![pair("cursor", "b"), pair("limit", "2")],
            vec![pair("limit", "2")],
        ]
    );
}

/// A listing cursor carries the provider's cursor, so a provider cursor of 13,000 bytes passes
/// its 16 KiB ceiling: `unavailable`, with a message that names the provider's cursor, not items
/// sharing an instant, which a listing cursor never holds. One of 100 bytes does not pass it.
#[tokio::test]
async fn a_listing_cursor_past_the_ceiling_is_unavailable_for_the_providers_cursor() {
    let engine = build(&declaration("time.operations.json"), &[]).unwrap();
    let listing = |next: String| Rooms {
        pages: std::sync::Mutex::new(vec![json!({"rooms": [], "next_cursor": next})]),
        queries: Default::default(),
    };
    let short = engine
        .read(
            &listing("c".repeat(100)),
            "feed-instance",
            CONTAINERS,
            json!({"limit": 2}),
        )
        .await
        .unwrap();
    assert_eq!(short["complete"], json!(false), "control");
    let long = engine
        .read(
            &listing("c".repeat(13_000)),
            "feed-instance",
            CONTAINERS,
            json!({"limit": 2}),
        )
        .await
        .map_err(|error| (error.code, error.message));
    assert_eq!(
        long.map(|_| ()),
        Err((
            connectors_core::ErrorCode::Unavailable,
            "the cursor would exceed its ceiling; the provider's cursor is too long".to_owned()
        ))
    );
}

/// The local service reads the declaration from its `operations_file` and declares both
/// operations in the descriptor it prints, as reads with requirements.
#[test]
fn the_local_service_declares_the_feed_from_its_operations_file() {
    let scratch = tempfile::tempdir().unwrap();
    fs::set_permissions(scratch.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let bundles = scratch.path().join("bundles");
    bundle::write(&bundles, &bundle(), false).unwrap();
    let private = scratch.path().join("private");
    filesystem::directory(&private, true, true).unwrap();
    let path = private.join("catalog.json");
    let config = json!({
        "format": "connectors-catalog-local/2",
        "instance": "feed-instance",
        "provider": bundle().provider,
        "bundle_directory": bundles,
        "api_base": "https://feed.fixture.test/v1",
        "auth": {
            "profile": "fixture.token",
            "header": "authorization",
            "bearer": true,
            "label": "Fixture token",
            "identity": {"path": "rooms", "kind": "fixture.member", "subject_pointer": "/next_cursor"}
        },
        "operations_file": fixtures().join("time.operations.json").canonicalize().unwrap(),
    });
    fs::write(&path, serde_json::to_vec(&config).unwrap()).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
        .arg("--local-config")
        .arg(&path)
        .arg("--print-local-bootstrap")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bootstrap: Value = serde_json::from_slice(&output.stdout).unwrap();
    let descriptor: Value =
        serde_json::from_str(bootstrap["descriptor"].as_str().unwrap()).unwrap();
    let operations: Vec<(&str, &str, &str)> = descriptor["operations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| {
            (
                o["id"].as_str().unwrap(),
                o["contract"].as_str().unwrap(),
                o["profile"].as_str().unwrap(),
            )
        })
        .collect();
    let profile = declaration("time.operations.json")["profile"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        operations,
        [
            (CONTAINERS, FEED_CONTRACT, profile.as_str()),
            (ITEMS, FEED_CONTRACT, profile.as_str())
        ]
    );
    let required: Vec<&str> = bootstrap["requirements"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["operation"].as_str().unwrap())
        .collect();
    assert_eq!(required, [CONTAINERS, ITEMS]);
}

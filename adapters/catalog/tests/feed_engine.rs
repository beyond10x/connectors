//! A feed declaration in a selection file adds `feed.containers` and `feed.items` to the
//! engine, under the family's contract and the declaration's profile; a declaration the bundle
//! cannot carry is refused when the engine is built, before any request; and the local service
//! declares both operations from its `operations_file`. The suite run is
//! `crates/connectors-build/src/catalog_feed_conformance.rs`.
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
    for name in ["time.operations.json", "cursor.operations.json"] {
        let feed = declaration(name);
        let engine = build(&feed, &[]).unwrap();
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

fn with(feed: &Value, pointer: &str, value: Value) -> Value {
    let mut changed = feed.clone();
    *changed.pointer_mut(pointer).expect(pointer) = value;
    changed
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

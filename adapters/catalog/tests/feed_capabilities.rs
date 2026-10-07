//! A feed declaration states its profile's capabilities, and the engine refuses, when it is
//! built, a claim the rest of the declaration does not support: the suite holds a binding to
//! what it claims, so a claim the binding cannot honour is refused before any request, and one it
//! honours below the strongest provider is accepted. `contracts/catalog/v1alpha1/semantics.md`
//! section 3.3 states the rules; `crates/connectors-build/src/catalog_feed_conformance.rs` runs
//! the suite against what is claimed.
use connectors_catalog::{bundle::Bundle, ingest, inventory};
use connectors_catalog_provider::{Engine, feed::Declaration};
use serde_json::{Value, json};
use std::{fs, path::Path};

fn fixtures() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/feed")
}

fn declaration(name: &str) -> Value {
    let file: Value = serde_json::from_slice(&fs::read(fixtures().join(name)).unwrap()).unwrap();
    file["feed"].clone()
}

fn bundle() -> Bundle {
    let bytes = fs::read(fixtures().join("api.json")).unwrap();
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    Bundle {
        provider: "feed-fixture".into(),
        source: ingest("api.json", &bytes).unwrap(),
        inventory: inventory::extract(&document),
        auth_profile: "fixture.token".into(),
    }
}

fn build(feed: &Value) -> connectors_core::Result<Engine> {
    let feed: Declaration = serde_json::from_value(feed.clone()).expect("a declaration");
    Engine::with_feed(&bundle(), "/v1", &[], Some(&feed))
}

/// A JSON pointer into a declaration, and its new value, or `None` to remove it.
type Change = (String, Option<Value>);

/// `feed` with the value at each pointer replaced, or removed where the value is `None`.
fn with(feed: &Value, changes: &[(&str, Option<Value>)]) -> Value {
    let mut changed = feed.clone();
    for (pointer, value) in changes {
        match value {
            Some(value) => *changed.pointer_mut(pointer).expect(pointer) = value.clone(),
            None => {
                let (parent, key) = pointer.rsplit_once('/').unwrap();
                changed
                    .pointer_mut(parent)
                    .and_then(Value::as_object_mut)
                    .expect(parent)
                    .remove(key)
                    .expect(pointer);
            }
        }
    }
    changed
}

/// Both fixtures claim the strongest provider's capabilities, and their declarations carry each.
#[test]
fn the_fixture_declarations_claim_what_they_carry() {
    for name in ["time.operations.json", "cursor.operations.json"] {
        let feed = declaration(name);
        assert_eq!(
            feed["capabilities"],
            json!({"deletions": "observed", "kind": "provider-word", "revision": "opaque", "visibility": "mapped"}),
            "{name}"
        );
        build(&feed).unwrap_or_else(|error| panic!("{name}: {}", error.message));
    }
}

/// Each case claims one capability the declaration does not support; each is refused when the
/// engine is built, naming the claim.
#[test]
fn a_declaration_cannot_claim_a_capability_it_does_not_support() {
    let base = declaration("time.operations.json");
    let claim = |name: &str, value: &str| (format!("/capabilities/{name}"), Some(json!(value)));
    let cases: Vec<(&str, Vec<Change>, &str)> = vec![
        (
            "tombstones claimed without a deleted condition",
            vec![("/items/deleted".into(), None)],
            "`deletions: observed`",
        ),
        (
            "no deletions claimed while a deleted condition reports them",
            vec![claim("deletions", "not-observed")],
            "`deletions: not-observed`",
        ),
        (
            "a fixed kind word claimed where the declaration reads the provider's",
            vec![claim("kind", "fixed-word")],
            "`kind: fixed-word`",
        ),
        (
            "an update-time revision claimed for a distinct revision field",
            vec![claim("revision", "update-time")],
            "`revision: update-time`",
        ),
        (
            "an opaque revision claimed for the update time",
            vec![("/items/revision".into(), Some(json!("/edited")))],
            "`revision: opaque`",
        ),
        (
            "every container private claimed while one maps to public",
            vec![claim("visibility", "all-private")],
            "`visibility: all-private`",
        ),
        (
            "mapped visibility claimed with no visibility rule",
            vec![("/containers/visibility".into(), None)],
            "`visibility: mapped`",
        ),
        (
            "mapped visibility claimed with a rule that maps nothing to public",
            vec![(
                "/containers/visibility/map".into(),
                Some(json!({"invite": "private", "im": "direct"})),
            )],
            "`visibility: mapped`",
        ),
    ];
    for (case, changes, reason) in cases {
        let changes: Vec<(&str, Option<Value>)> = changes
            .iter()
            .map(|(pointer, value)| (pointer.as_str(), value.clone()))
            .collect();
        match build(&with(&base, &changes)) {
            Ok(_) => panic!("{case}: accepted"),
            Err(error) => assert!(error.message.contains(reason), "{case}: {}", error.message),
        }
    }
}

/// A declaration that carries less claims less, and is built.
#[test]
fn a_declaration_that_carries_less_claims_less() {
    let base = declaration("time.operations.json");
    let weaker = with(
        &base,
        &[
            ("/items/deleted", None),
            ("/capabilities/deletions", Some(json!("not-observed"))),
            ("/items/revision", Some(json!("/edited"))),
            ("/capabilities/revision", Some(json!("update-time"))),
            ("/containers/visibility/map", Some(json!({"im": "direct"}))),
            ("/capabilities/visibility", Some(json!("all-private"))),
        ],
    );
    build(&weaker).unwrap_or_else(|error| panic!("{}", error.message));
    let unmapped = with(
        &weaker,
        &[
            ("/containers/visibility", None),
            ("/items/revision", Some(json!("/version"))),
        ],
    );
    let unmapped = with(
        &unmapped,
        &[("/capabilities/revision", Some(json!("opaque")))],
    );
    build(&unmapped).unwrap_or_else(|error| panic!("{}", error.message));
}

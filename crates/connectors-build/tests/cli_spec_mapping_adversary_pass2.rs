//! Adversary pass 2 for `story:cli-spec-mapping-fixes` (wave 20261001b, correction
//! 6aa797a37 on a63a7f9dd, base b19027874).
//!
//! The correction declared `LocalOwnerHello`, `LocalOwnerBuildRequest` and the
//! `BuildDigest` newtype. These cases hold those declarations to the host source they
//! claim to describe (`crates/connectors-host/src/local/owner.rs` and
//! `owner/transport.rs`), field by field and comment by comment.

use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> String {
    fs::read_to_string(root().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn cli_types() -> Vec<serde_yaml_ng::Value> {
    let document: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&read("ess/domains/cli.yaml")).unwrap();
    document["types"].as_sequence().unwrap().clone()
}

fn named(name: &str) -> serde_yaml_ng::Value {
    cli_types()
        .into_iter()
        .find(|t| t["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("{name} is not declared"))
}

fn field_type(ty: &serde_yaml_ng::Value, name: &str) -> Option<String> {
    ty["fields"]
        .as_sequence()
        .unwrap()
        .iter()
        .find(|f| f["name"].as_str() == Some(name))
        .map(|f| f["type"].as_str().unwrap().to_owned())
}

/// True when `ty` is the ESS `Uuid` primitive or a cli newtype of it.
fn is_uuid(ty: &str) -> bool {
    ty == "Uuid"
        || cli_types()
            .iter()
            .any(|t| t["name"].as_str() == Some(ty) && t["of"].as_str() == Some("Uuid"))
}

const TRANSPORT: &str = "crates/connectors-host/src/local/owner/transport.rs";

/// The owner refuses a hello whose challenge is not a UUID and compares `authority`
/// with `Metadata::authority()`, a `uuid::Uuid` rendered with `to_string()`
/// (transport.rs `exchange`, :706). ESS has a `Uuid` primitive that this repository's
/// other domains use, and the pinned `ess` validates and generates the CLI with it on
/// these fields. `LocalOwnerHello` (introduced by 6aa797a37) types both as any `String`,
/// while the same correction narrowed `build` to its exact format.
#[test]
fn the_owner_hello_types_its_uuid_fields() {
    let transport = read(TRANSPORT);
    assert!(
        transport.contains("uuid::Uuid::parse_str(&challenge).is_err()"),
        "host anchor moved: the owner no longer refuses a non-UUID challenge"
    );
    assert!(
        transport.contains("let authority = metadata.authority()?.to_string();"),
        "host anchor moved: authority is no longer a rendered Uuid"
    );
    let hello = named("connectors.cli.LocalOwnerHello");
    for field in ["challenge", "authority"] {
        let declared = field_type(&hello, field).unwrap();
        assert!(
            is_uuid(&declared),
            "connectors.cli.LocalOwnerHello.{field} is `{declared}`, but the owner refuses \
             any value that is not a UUID"
        );
    }
}

/// The CLI refuses a greeting reply whose `host_incarnation` is not a UUID
/// (transport.rs `hello`: `uuid::Uuid::parse_str(&host_incarnation).is_ok()`), and the
/// reply echoes the UUID challenge and authority. `LocalOwnerGreeting` declares all three
/// as `String`; this was already so at the base.
#[test]
fn the_owner_greeting_types_its_uuid_fields() {
    let transport = read(TRANSPORT);
    assert!(
        transport.contains("uuid::Uuid::parse_str(&host_incarnation).is_ok()"),
        "host anchor moved: the CLI no longer checks host_incarnation"
    );
    let greeting = named("connectors.cli.LocalOwnerGreeting");
    for field in ["challenge", "host_incarnation", "authority"] {
        let declared = field_type(&greeting, field).unwrap();
        assert!(
            is_uuid(&declared),
            "connectors.cli.LocalOwnerGreeting.{field} is `{declared}`, but the host \
             produces and checks a UUID"
        );
    }
}

/// `Request` and `Reply` are `#[serde(tag = "kind")]`: every owner frame carries `kind`.
/// The correction models the tag as a field on `LocalOwnerBuildRequest` only, so the
/// spec now says the build request carries `kind` and the hello frames, which also do,
/// carry none. One convention has to hold for all three frames.
#[test]
fn the_owner_frame_tag_is_modelled_the_same_way_on_every_frame() {
    let owner = read("crates/connectors-host/src/local/owner.rs");
    assert!(
        owner.contains("#[serde(tag = \"kind\", rename_all = \"snake_case\", deny_unknown_fields)]\nenum Request {"),
        "host anchor moved: Request is no longer kind-tagged"
    );
    let frames = [
        "connectors.cli.LocalOwnerHello",
        "connectors.cli.LocalOwnerGreeting",
        "connectors.cli.LocalOwnerBuildRequest",
    ];
    let with_kind: Vec<&str> = frames
        .iter()
        .copied()
        .filter(|name| field_type(&named(name), "kind").is_some())
        .collect();
    assert!(
        with_kind.is_empty() || with_kind.len() == frames.len(),
        "only {with_kind:?} declare the `kind` tag; the other owner frames carry it on the \
         wire too"
    );
}

/// The `BuildDigest` comment says the digest is written "as owner.rs writes it
/// (hex::encode)". `owner.rs` contains no `hex::encode`; the digest is encoded by
/// `own_build` in `owner/transport.rs`.
#[test]
fn the_build_digest_comment_cites_the_file_that_encodes_it() {
    let spec = read("ess/domains/cli.yaml");
    let cite = regex::Regex::new(r"as (\S+\.rs) writes it \(hex::encode\)").unwrap();
    let cited = cite
        .captures(&spec)
        .map(|c| c[1].to_owned())
        .expect("BuildDigest comment no longer cites a writer");
    let candidates: Vec<PathBuf> = [
        format!("crates/connectors-host/src/local/{cited}"),
        format!("crates/connectors-host/src/{cited}"),
    ]
    .into_iter()
    .map(|p| root().join(p))
    .filter(|p| p.exists())
    .collect();
    assert!(!candidates.is_empty(), "cited file {cited} not found");
    assert!(
        candidates.iter().any(|p| fs::read_to_string(p)
            .unwrap()
            .contains("hex::encode(Sha256::digest")),
        "cli.yaml says {cited} writes the build digest with hex::encode; it does not \
         (own_build in local/owner/transport.rs does)"
    );
}

/// Control: the declared `BuildDigest` admits the lowercase 64-hex form the host writes
/// and nothing near it: not uppercase, not 63 or 65 characters, not empty.
#[test]
fn the_build_digest_admits_exactly_lowercase_64_hex() {
    let digest = named("connectors.cli.BuildDigest");
    assert_eq!(digest["of"].as_str(), Some("String"));
    let alphabet = digest["alphabet"].as_str().expect("no alphabet");
    let invariants: Vec<&str> = digest["invariants"]
        .as_sequence()
        .expect("no invariants")
        .iter()
        .map(|i| i.as_str().unwrap())
        .collect();
    let count = regex::Regex::new(r"^value\.count == (\d+)$").unwrap();
    let lengths: Vec<usize> = invariants
        .iter()
        .filter_map(|i| count.captures(i).map(|c| c[1].parse().unwrap()))
        .collect();
    assert_eq!(
        lengths.len(),
        invariants.len(),
        "unread invariant in {invariants:?}"
    );
    let admits = |value: &str| {
        value.chars().all(|c| alphabet.contains(c))
            && lengths.iter().all(|n| value.chars().count() == *n)
    };
    let real = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    assert!(admits(real));
    assert!(!admits(&real.to_uppercase()));
    assert!(!admits(&real[..63]));
    assert!(!admits(&format!("{real}0")));
    assert!(!admits(""));
    assert!(!admits(&format!("{}g", &real[..63])));
}

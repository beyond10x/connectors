//! Adversary cases for `story:cli-spec-mapping-fixes` (wave 20261001b, unit commit
//! a63a7f9dd on base b19027874).
//!
//! The unit's own checks (`ess specify validate`, `cli --check`, `metadata-entities
//! --check`) prove the specification is well formed and that the generated outputs are a
//! fixed point of it. None of them compares `ess/domains/cli.yaml` with the owner wire
//! types in `crates/connectors-host/src/local/owner.rs`, or a citation with the line it
//! cites. These cases do: they read the host's `Request`/`Reply` field lists from source
//! and the spec's struct fields from the parsed domain document.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> String {
    fs::read_to_string(root().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// Field names of `enum <enum_name> { <variant> { ... } }` in `owner.rs`, read from source.
fn host_variant_fields(enum_name: &str, variant: &str) -> BTreeSet<String> {
    let source = read("crates/connectors-host/src/local/owner.rs");
    let start = source
        .find(&format!("enum {enum_name} {{"))
        .unwrap_or_else(|| panic!("enum {enum_name} not found"));
    let body = &source[start..];
    let open = body
        .find(&format!("    {variant} {{"))
        .unwrap_or_else(|| panic!("{enum_name}::{variant} not found"));
    let body = &body[open..];
    let close = body.find("\n    },").expect("variant close");
    let field = regex::Regex::new(r"^\s+([a-z_][a-z0-9_]*):\s").unwrap();
    body[..close]
        .lines()
        .skip(1)
        .filter(|line| !line.trim_start().starts_with("//") && !line.trim_start().starts_with('#'))
        .filter_map(|line| field.captures(line).map(|c| c[1].to_owned()))
        .collect()
}

fn cli_types() -> Vec<serde_yaml_ng::Value> {
    let document: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&read("ess/domains/cli.yaml")).unwrap();
    document["types"].as_sequence().unwrap().clone()
}

fn struct_fields(ty: &serde_yaml_ng::Value) -> BTreeSet<String> {
    ty["fields"]
        .as_sequence()
        .into_iter()
        .flatten()
        .map(|f| f["name"].as_str().unwrap().to_owned())
        .collect()
}

/// The fields a frame carries besides its serde `kind` tag, which the host's variant
/// does not list as a field (decided in the pass-2 correction).
fn payload_fields(ty: &serde_yaml_ng::Value) -> BTreeSet<String> {
    let mut fields = struct_fields(ty);
    fields.remove("kind");
    fields
}

fn field_type(ty: &serde_yaml_ng::Value, name: &str) -> String {
    ty["fields"]
        .as_sequence()
        .unwrap()
        .iter()
        .find(|f| f["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("no field {name}"))["type"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn named(name: &str) -> serde_yaml_ng::Value {
    cli_types()
        .into_iter()
        .find(|t| t["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("{name} is not declared"))
}

/// Control: the reply greeting the owner sends is declared with exactly its fields.
#[test]
fn the_owner_greeting_reply_is_declared_with_the_hosts_fields() {
    let host = host_variant_fields("Reply", "Hello");
    assert_eq!(
        host,
        [
            "authority",
            "build",
            "challenge",
            "host_incarnation",
            "version"
        ]
        .map(String::from)
        .into(),
        "parser read the wrong Reply::Hello"
    );
    let greeting = named("connectors.cli.LocalOwnerGreeting");
    assert_eq!(payload_fields(&greeting), host);
    assert_eq!(
        field_type(&greeting, "kind"),
        "connectors.cli.LocalOwnerHelloKind",
        "the greeting declares its serde tag"
    );
    assert_eq!(
        field_type(&greeting, "build"),
        "Optional<connectors.cli.BuildDigest>",
        "the greeting carries the optional 64-hex build digest (decided in the pass-1 correction)"
    );
}

/// The greeting the CLI sends (`Request::Hello`) carries its own `build` and the
/// `configuration` path; acceptance cites that field. No spec type carries this shape.
#[test]
fn the_owner_greeting_request_the_cli_sends_is_declared() {
    let host = host_variant_fields("Request", "Hello");
    assert_eq!(
        host,
        [
            "authority",
            "build",
            "challenge",
            "configuration",
            "version"
        ]
        .map(String::from)
        .into(),
        "parser read the wrong Request::Hello"
    );
    let declared: Vec<String> = cli_types()
        .iter()
        .filter(|t| payload_fields(t) == host && struct_fields(t).contains("kind"))
        .map(|t| t["name"].as_str().unwrap().to_owned())
        .collect();
    assert!(
        !declared.is_empty(),
        "no connectors.cli struct declares the Request::Hello fields {host:?}"
    );
}

/// The owner writes `hex::encode(Sha256)` (64 lowercase hex) and the CLI's build probe
/// admits only a 64-character `build` (owner/transport.rs `probe_build`). ESS 0.45 accepts
/// a hex `alphabet:` newtype here (precedent: connectors.artifact_provenance.Sha256Hex),
/// and `generate cli` accepts it on these non-CLI types, yet both fields are bare `String`.
#[test]
fn the_build_digest_format_is_declared() {
    let types = cli_types();
    for (ty, wrapped) in [
        ("connectors.cli.LocalOwnerBuild", false),
        ("connectors.cli.LocalOwnerGreeting", true),
    ] {
        let declared = field_type(&named(ty), "build");
        let inner = if wrapped {
            declared
                .strip_prefix("Optional<")
                .and_then(|s| s.strip_suffix('>'))
                .unwrap_or(&declared)
                .to_owned()
        } else {
            declared.clone()
        };
        let constrained = types.iter().any(|t| {
            t["name"].as_str() == Some(inner.as_str())
                && t["alphabet"]
                    .as_str()
                    .is_some_and(|a| a.chars().all(|c| c.is_ascii_hexdigit()) && a.contains('f'))
        });
        assert!(
            constrained,
            "{ty}.build is `{declared}`: the 64-hex digest format is not declared"
        );
    }
}

/// Every `er.rs:<a>-<b>` citation in cli.yaml must land on the cursor transition it names.
#[test]
fn the_er_rs_citation_lands_on_the_cursor_expiry_path() {
    let spec = read("ess/domains/cli.yaml");
    let er = read("crates/connectors-host/src/local/metadata/er.rs");
    let lines: Vec<&str> = er.lines().collect();
    let cite = regex::Regex::new(r"er\.rs:(\d+)-(\d+)").unwrap();
    let mut seen = 0;
    for c in cite.captures_iter(&spec) {
        seen += 1;
        let (a, b): (usize, usize) = (c[1].parse().unwrap(), c[2].parse().unwrap());
        let cited = lines[a - 1..b].join("\n");
        assert!(
            cited.contains("ConnectionListCursor"),
            "cli.yaml cites er.rs:{a}-{b} for the cursor removal path, which reads:\n{cited}"
        );
    }
    assert!(seen > 0, "no er.rs citation found");
}

/// Control: the two semantics.md citations the unit corrected land on their rules.
#[test]
fn the_corrected_semantics_citations_land_on_their_rules() {
    let semantics = read("contracts/cli/v1alpha1/semantics.md");
    let lines: Vec<&str> = semantics.lines().collect();
    let spec = read("ess/domains/cli.yaml");
    assert!(spec.contains("semantics.md:351"));
    assert!(spec.contains("semantics.md:355"));
    assert!(
        lines[350].contains("500 maximum list items per page"),
        "{}",
        lines[350]
    );
    assert!(
        lines[354].contains("stale cursors refuse"),
        "{}",
        lines[354]
    );
}

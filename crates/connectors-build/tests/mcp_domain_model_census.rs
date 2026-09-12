//! Census cases for `story:mcp-domain-model`, written 2026-09-12 on branch
//! `unit/mcp-domain-model-20260912`.
//!
//! The story's acceptance turns on three claims, and the checks its brief names —
//! `ess specify validate --path adapters/mcp/spec/ess` and `connectors-build
//! ess-boundary` — establish only the first:
//!
//! 1. the root validates under the pinned toolchain;
//! 2. the three credential kinds `epic:mcp-contracts` keeps apart are three distinct
//!    declared types;
//! 3. every relation the model could not read from an `ess/1` document or the pinned
//!    specification is carried as an explicit `UNMAPPED:` marker rather than a chosen
//!    cardinality.
//!
//! Claims 2 and 3 have no mechanical check at all. Measured on this branch: injecting
//! `{name: binding, kind: references, target: connectors_mcp.state.McpServerBinding,
//! cardinality: one, via: binding_ref}` onto `McpOutboundSession` — census row 6, which
//! the story marks UNMAPPED — leaves `ess specify validate` printing
//! `connectors_mcp v1 — 3 file(s), valid` and exiting 0. Ten stories read this model, so
//! the guessed cardinality that the validator waves through is exactly the defect that
//! propagates. These cases close that gap for the whole census, not for one row.

use std::path::{Path, PathBuf};

/// What the story decided about one edge, and therefore what may appear in the model.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Verdict {
    /// Citable and stated in prose; a cross-root ESS relation is impossible here.
    Stated,
    /// Declined by a shared `ess/1` document; this model repeats the refusal.
    Absent,
    /// Unreadable from every legitimate source; carried as an `UNMAPPED:` marker.
    Unmapped,
    /// An open `decision-blocker:`; a cardinality here would answer it.
    Filed,
}

/// The whole relation census `story:mcp-domain-model` carries, in the story's order.
/// A row added to the story is added here; a row here with no marker in the model fails.
const CENSUS: &[(&str, &str, Verdict)] = &[
    (
        "McpAdvertisedCapability",
        "connectors.declarations.OperationDeclaration",
        Verdict::Stated,
    ),
    (
        "McpSession",
        "connectors.auth_bindings.Connection",
        Verdict::Absent,
    ),
    (
        "McpServerBinding",
        "connectors.declarations.ServiceConfiguration",
        Verdict::Unmapped,
    ),
    (
        "McpServerBinding",
        "connectors.auth_bindings.Connection",
        Verdict::Unmapped,
    ),
    (
        "McpServerBinding",
        "McpCapabilitySnapshot",
        Verdict::Unmapped,
    ),
    ("McpOutboundSession", "McpServerBinding", Verdict::Unmapped),
    ("McpServerBinding", "credential custody", Verdict::Unmapped),
    ("McpInboundSession", "McpCaller", Verdict::Unmapped),
    (
        "McpCaller",
        "connectors.auth_bindings.Connection",
        Verdict::Filed,
    ),
    ("McpServerBinding", "supervised OS process", Verdict::Filed),
];

/// The three kinds `epic:mcp-contracts` requires kept apart: "Caller credentials for
/// inbound access, credentials for an outbound MCP server and underlying provider
/// credentials are distinct."
const CREDENTIAL_KINDS: &[&str] = &[
    "connectors_mcp.state.McpInboundCallerCredential",
    "connectors_mcp.state.McpOutboundServerCredential",
    "connectors_mcp.state.UnderlyingProviderCredential",
];

/// Version strings the pinned revisions carry that are NOT the negotiable wire version.
/// `specification-sources.md` "## The version-string trap this pin makes checkable"
/// names all three; a model that read the obvious constant would carry one of them.
const TRAP_STRINGS: &[(&str, &str)] = &[
    (
        "DRAFT-2025-v3",
        "mcp-2025-11-25-schema.ts:14 `LATEST_PROTOCOL_VERSION`",
    ),
    (
        "2025-06-18",
        "mcp-2025-11-25-basic-transports.mdx:271 `MCP-Protocol-Version` example",
    ),
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/mcp/spec/ess")
}

fn read(relative: &str) -> String {
    let path = root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn document(relative: &str) -> serde_yaml_ng::Value {
    serde_yaml_ng::from_str(&read(relative)).unwrap_or_else(|e| panic!("parse {relative}: {e}"))
}

/// Every ESS relation the state domain actually declares, as `(entity, target)` with
/// the `connectors_mcp.state.` prefix stripped so it compares against the census.
fn declared_relations() -> Vec<(String, String)> {
    let short = |name: &str| name.trim_start_matches("connectors_mcp.state.").to_owned();
    let mut found = Vec::new();
    let state = document("domains/state.yaml");
    let Some(entities) = state.get("entities").and_then(|e| e.as_sequence()) else {
        return found;
    };
    for entity in entities {
        let name = entity
            .get("name")
            .and_then(|n| n.as_str())
            .expect("an entity without a name");
        let Some(relations) = entity.get("relations").and_then(|r| r.as_sequence()) else {
            continue;
        };
        for relation in relations {
            let target = relation
                .get("target")
                .and_then(|t| t.as_str())
                .expect("a relation without a target");
            found.push((short(name), short(target)));
        }
    }
    found
}

/// Every type and entity name the root declares, across both domains.
fn declared_names() -> Vec<String> {
    let mut names = Vec::new();
    for domain in ["domains/protocol.yaml", "domains/state.yaml"] {
        let value = document(domain);
        for key in ["types", "entities"] {
            let Some(items) = value.get(key).and_then(|i| i.as_sequence()) else {
                continue;
            };
            for item in items {
                names.push(
                    item.get("name")
                        .and_then(|n| n.as_str())
                        .expect("a declaration without a name")
                        .to_owned(),
                );
            }
        }
    }
    names
}

/// No census edge may be realised as a declared ESS relation unless the story states
/// it, and every census edge must be named in the model's own text.
///
/// This is the case the named checks cannot make: `ess specify validate` accepts a
/// guessed `cardinality: one` on an UNMAPPED row exactly as readily as a correct one.
#[test]
fn every_census_edge_is_marked_and_no_unreadable_one_is_declared_as_a_relation() {
    let text = read("domains/state.yaml");
    let declared = declared_relations();

    for (source, target, verdict) in CENSUS {
        let edge = format!("{source} → {target}");
        assert!(
            text.contains(&edge),
            "census edge `{edge}` is named nowhere in domains/state.yaml; every row of \
             story:mcp-domain-model's census must appear in the model that closes it"
        );

        let realised = declared
            .iter()
            .any(|(entity, to)| entity == source && to == target);
        match verdict {
            Verdict::Unmapped | Verdict::Filed | Verdict::Absent => assert!(
                !realised,
                "`{edge}` is declared as an ESS relation with a cardinality, but no \
                 source answers it — it is carried as a marker, not chosen. \
                 `ess specify validate` exits 0 on this; that is why this case exists"
            ),
            Verdict::Stated => {}
        }

        if matches!(verdict, Verdict::Unmapped) {
            assert!(
                text.contains("UNMAPPED:"),
                "no `UNMAPPED:` marker survives in domains/state.yaml, yet `{edge}` is \
                 unreadable from every legitimate source"
            );
        }
    }

    // The two filed questions are decisions, not modelling gaps: the model must name
    // the blocker that holds each, so a reader knows where the answer will come from.
    for blocker in [
        "decision-blocker:mcp-caller-connection-assignment",
        "decision-blocker:mcp-outbound-stdio-process-ownership",
    ] {
        assert!(
            text.contains(blocker),
            "`{blocker}` holds a census edge and is named nowhere in the model"
        );
    }

    // The one stated edge carries the source its cardinality was read from, not chosen.
    assert!(
        text.contains("ess/domains/declarations.yaml:119-123"),
        "the only citable census edge is stated without the `ess/1` block its \
         `cardinality: many` is read from"
    );
}

/// Three credential kinds, three declared types, never one polymorphic value.
#[test]
fn the_three_credential_kinds_are_three_distinct_declared_types() {
    let names = declared_names();

    for kind in CREDENTIAL_KINDS {
        assert!(
            names.iter().any(|n| n == kind),
            "`{kind}` is not declared; epic:mcp-contracts keeps three credential kinds \
             apart and collapsing any two is the modelling error it names explicitly"
        );
    }

    let credentials: Vec<&String> = names.iter().filter(|n| n.ends_with("Credential")).collect();
    assert_eq!(
        credentials.len(),
        CREDENTIAL_KINDS.len(),
        "the root declares {} credential types, not {}: {credentials:?}. Two collapsed \
         into one, or a fourth added, both change what a later reviewer can see",
        credentials.len(),
        CREDENTIAL_KINDS.len()
    );
}

/// The negotiable wire version strings, and none of the three traps the pin documents.
#[test]
fn the_protocol_revision_values_are_the_negotiated_strings_not_the_trap_constants() {
    let protocol = document("domains/protocol.yaml");
    let types = protocol
        .get("types")
        .and_then(|t| t.as_sequence())
        .expect("domains/protocol.yaml declares no types");
    let revision = types
        .iter()
        .find(|t| {
            t.get("name").and_then(|n| n.as_str())
                == Some("connectors_mcp.protocol.ProtocolRevision")
        })
        .expect("connectors_mcp.protocol.ProtocolRevision is not declared");
    let variants: Vec<&str> = revision
        .get("variants")
        .and_then(|v| v.as_sequence())
        .expect("ProtocolRevision declares no variants")
        .iter()
        .map(|v| v.as_str().expect("a non-string variant"))
        .collect();
    assert_eq!(
        variants,
        vec!["2026-07-28", "2025-11-25"],
        "the two negotiable strings are stated by the PRIMARY revision — \
         mcp-2026-07-28-basic-versioning.mdx:62, `\"supported\": [\"2026-07-28\", \
         \"2025-11-25\"]` — not by the revision each names"
    );

    for domain in ["domains/protocol.yaml", "domains/state.yaml"] {
        let text = read(domain);
        for (trap, origin) in TRAP_STRINGS {
            for line in text.lines() {
                let code = line.split('#').next().unwrap_or("");
                assert!(
                    !code.contains(trap),
                    "{domain} carries `{trap}` in a declaration. That string comes from \
                     {origin}; it is not the negotiable wire version"
                );
            }
        }
    }
}

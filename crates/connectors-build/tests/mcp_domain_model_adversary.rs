//! Adversary cases for `story:mcp-domain-model`, written 2026-09-12 against
//! `d5270ca` on branch `unit/mcp-domain-model-20260912`.
//!
//! `crates/connectors-build/tests/mcp_domain_model_census.rs` closes the gap that
//! `ess specify validate` leaves open for the *census*: it enumerates the ten rows and
//! the three credential kinds. These cases attack what that enumeration does not read.
//!
//! Each case compares two things this unit shipped in one commit against each other.
//! Nothing else in the repository compares them, so a disagreement between two of the
//! unit's own documents is green at every gate step:
//!
//! 1. the adapter owner document `adapters/mcp/design.md` against the model root that
//!    now exists beside it;
//! 2. the census's one *stated* cardinality against the field that realises it — the
//!    census case's `Verdict::Stated` arm asserts nothing at all;
//! 3. the revision a binding may *select* against the revisions a server may *report*,
//!    which the model derives from one cited sentence.

use std::path::{Path, PathBuf};

/// The repository these cases read. `MCP_ADVERSARY_ROOT` overrides it so that a
/// reviewer can point the same three cases at a copy carrying a candidate correction
/// and watch them go green, without editing the tree under review.
fn repo() -> PathBuf {
    match std::env::var_os("MCP_ADVERSARY_ROOT") {
        Some(root) => PathBuf::from(root),
        None => Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
    }
}

fn read(relative: &str) -> String {
    let path = repo().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn document(relative: &str) -> serde_yaml_ng::Value {
    serde_yaml_ng::from_str(&read(relative)).unwrap_or_else(|e| panic!("parse {relative}: {e}"))
}

/// The declared `(name, type)` of the first field of `<declaration>` whose name begins
/// with `prefix`, so that pluralising a carrier (`operation_ref` to `operation_refs`)
/// is read as the correction it is rather than as a missing field.
fn field_by_prefix(domain: &str, declaration: &str, prefix: &str) -> (String, String) {
    let value = document(domain);
    for section in ["types", "entities"] {
        let Some(items) = value.get(section).and_then(|i| i.as_sequence()) else {
            continue;
        };
        for item in items {
            if item.get("name").and_then(|n| n.as_str()) != Some(declaration) {
                continue;
            }
            let fields = item
                .get("fields")
                .and_then(|f| f.as_sequence())
                .unwrap_or_else(|| panic!("{declaration} declares no fields"));
            for entry in fields {
                let name = entry.get("name").and_then(|n| n.as_str()).unwrap_or("");
                if name.starts_with(prefix) {
                    return (
                        name.to_owned(),
                        entry
                            .get("type")
                            .and_then(|t| t.as_str())
                            .unwrap_or_else(|| panic!("{declaration}.{name} has no type"))
                            .trim()
                            .to_owned(),
                    );
                }
            }
            panic!("{declaration} declares no field starting with `{prefix}`");
        }
    }
    panic!("{domain} declares no `{declaration}`");
}

/// The declared field type of `<declaration>.<field>`, looked up across both the
/// `types:` and `entities:` sections of an MCP domain document.
fn field_type(domain: &str, declaration: &str, field: &str) -> String {
    let value = document(domain);
    for section in ["types", "entities"] {
        let Some(items) = value.get(section).and_then(|i| i.as_sequence()) else {
            continue;
        };
        for item in items {
            if item.get("name").and_then(|n| n.as_str()) != Some(declaration) {
                continue;
            }
            let fields = item
                .get("fields")
                .and_then(|f| f.as_sequence())
                .unwrap_or_else(|| panic!("{declaration} declares no fields"));
            for entry in fields {
                if entry.get("name").and_then(|n| n.as_str()) == Some(field) {
                    return entry
                        .get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or_else(|| panic!("{declaration}.{field} has no type"))
                        .trim()
                        .to_owned();
                }
            }
            panic!("{declaration} declares no field `{field}`");
        }
    }
    panic!("{domain} declares no `{declaration}`");
}

/// `Optional<T>` and `List<T>` unwrapped to `T`; anything else returned unchanged.
fn inner(declared: &str) -> &str {
    for wrapper in ["Optional<", "List<"] {
        if let Some(stripped) = declared
            .strip_prefix(wrapper)
            .and_then(|rest| rest.strip_suffix('>'))
        {
            return inner(stripped.trim());
        }
    }
    declared
}

/// Whether a declared type carries many values rather than one.
fn is_plural(declared: &str) -> bool {
    declared.starts_with("List<")
        || declared
            .strip_prefix("Optional<")
            .and_then(|rest| rest.strip_suffix('>'))
            .is_some_and(|rest| is_plural(rest.trim()))
}

const DESIGN: &str = "adapters/mcp/design.md";
const STATE: &str = "adapters/mcp/spec/ess/domains/state.yaml";

/// Sentences in the adapter owner document that are true only while this directory has
/// no authored model. `story:mcp-domain-model` gave it one and left every sentence in
/// place; `d5270ca` does not touch `adapters/mcp/design.md`.
const DENIALS: &[(&str, &str)] = &[
    (
        "No adapter service, package, ESS model",
        "the **Status:** line still says no ESS model exists here",
    ),
    (
        "`spec/ess/`, `generated/` and `src/` do not exist here",
        "`## Sources and remaining obligations` still says `spec/ess/` does not exist \
         and is not implied",
    ),
    (
        "No entity or relation is declared",
        "`## What is deliberately not decided here` still says this directory declares \
         no entity",
    ),
    (
        ", the authored native model,",
        "`Remaining obligations, all unstarted` still lists the authored native model \
         as unstarted",
    ),
];

/// The owner document for `adapters/mcp/` must not deny the model that now sits beside
/// it. `docs/design.md:279` gives `design.md` its role — "Adapter-owned design and
/// evidence notes" — and ten stories in `epic:mcp-contracts` read this directory's
/// owner document to learn what it already contains.
///
/// This is not a style point. A reader who believes `## Sources and remaining
/// obligations` will author the native model a second time, and a reader who believes
/// `## What is deliberately not decided here` will believe no entity has been declared
/// when four have.
#[test]
fn the_adapter_owner_document_does_not_deny_the_model_this_unit_added() {
    let system = repo().join("adapters/mcp/spec/ess/system.yaml");
    assert!(
        system.exists(),
        "{} does not exist; this case compares the owner document against a model that \
         is there",
        system.display()
    );
    let entities = document(STATE)
        .get("entities")
        .and_then(|e| e.as_sequence())
        .map(|e| e.len())
        .unwrap_or(0);
    assert!(entities > 0, "{STATE} declares no entities");

    let design = read(DESIGN);
    let mut standing = Vec::new();
    for (claim, why) in DENIALS {
        for (index, line) in design.lines().enumerate() {
            if line.contains(claim) {
                standing.push(format!("{DESIGN}:{}: {why}", index + 1));
            }
        }
    }
    assert!(
        standing.is_empty(),
        "`adapters/mcp/spec/ess` exists and declares {entities} entities, and the owner \
         document still denies it in {} place(s):\n  {}",
        standing.len(),
        standing.join("\n  ")
    );
}

/// The one census edge the model *states* rather than marks.
const STATED_EDGE: &str = "McpAdvertisedCapability → connectors.declarations.OperationDeclaration";

/// A stated cardinality must agree with the field that realises it.
///
/// The census case's `Verdict::Stated` arm is `{}` — it asserts nothing about the one
/// row the story lets the model answer. This repository binds the two unambiguously and
/// without exception: a `references` relation with `cardinality: many` carries a
/// `List<...>` via field (`ess/domains/artifact_provenance.yaml:85-89` via
/// `source_refs: List<String>`; `ess/domains/auth_bindings.yaml:107` via
/// `superseded_custody_version_refs: List<String>`) and one with `cardinality: one`
/// carries a scalar (`ess/domains/credentials.yaml:40-45` via `connection_ref: String`;
/// `ess/domains/auth_bindings.yaml:102-106`).
///
/// `domains/state.yaml` states this edge as MANY and realises it with a scalar
/// `operation_ref: String`, which is cardinality one. The cited source
/// (`ess/domains/declarations.yaml:119-123`) declares a different pair —
/// `AdapterSpecification owns many OperationDeclaration` — and says nothing about how
/// many operations one advertised capability projects.
///
/// Any of the three corrections passes this case: pluralise the carrier, restate the
/// edge as one, or carry it as a marker with no cardinality at all.
#[test]
fn the_one_stated_census_edge_agrees_with_the_field_that_realises_it() {
    let text = read(STATE);
    let mut stated: Vec<(usize, String)> = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let Some(at) = line.find(STATED_EDGE) else {
            continue;
        };
        let rest = &line[at + STATED_EDGE.len()..];
        let word = rest
            .split(|c: char| !c.is_ascii_alphanumeric())
            .map(|w| w.to_ascii_lowercase())
            .find(|w| w == "one" || w == "many");
        if let Some(word) = word {
            stated.push((index + 1, word));
        }
    }
    if stated.is_empty() {
        return; // carried as a marker with no cardinality: nothing to disagree with.
    }

    let (carrier, declared) = field_by_prefix(
        STATE,
        "connectors_mcp.state.McpAdvertisedCapability",
        "operation",
    );
    let realised = if is_plural(&declared) { "many" } else { "one" };
    for (line, word) in &stated {
        assert_eq!(
            word,
            realised,
            "{STATE}:{line} states `{STATED_EDGE}` as {} while the only field that \
             realises it, `McpAdvertisedCapability.{carrier}: {declared}`, is {}. \
             A scalar `via` carrier is `cardinality: one` everywhere else in this \
             repository. The cited block ess/domains/declarations.yaml:119-123 declares \
             `AdapterSpecification owns many OperationDeclaration`, a different pair, so \
             it does not answer this edge either way",
            word.to_uppercase(),
            realised.to_uppercase()
        );
    }
}

/// A binding must be able to record the selection it makes.
///
/// `McpCapabilitySnapshot.supported_versions` is `List<String>` on purpose, and the
/// model says why at `domains/state.yaml`, on `supported_versions`: "a server may name versions neither
/// pinned revision covers". `UnsupportedProtocolVersion.supported` is `List<String>`
/// for the same stated reason (`domains/protocol.yaml`, on `UnsupportedProtocolVersion.supported`). The selection is drawn
/// from exactly that list — `mcp-2026-07-28-basic-versioning.mdx:69-71`, quoted by
/// `McpServerBinding.selected_revision`'s own comment: "The client **SHOULD** select a
/// mutually supported version from the `supported` list".
///
/// So whatever carries a reported version must also carry a selected one. Typing the
/// selection to the two-variant `ProtocolRevision` enum closes the set that
/// `domains/protocol.yaml` declares open, on `ProtocolRevision`, in as many words — "UNMAPPED: the
/// protocol's version set is not closed by these two" — and it is a selection, which
/// `domains/protocol.yaml` says, in its header, this file does not make.
#[test]
fn a_binding_can_select_every_version_a_server_can_report_as_supported() {
    let reported = field_type(
        STATE,
        "connectors_mcp.state.McpCapabilitySnapshot",
        "supported_versions",
    );
    let selected = field_type(
        STATE,
        "connectors_mcp.state.McpServerBinding",
        "selected_revision",
    );
    assert_eq!(
        inner(&selected),
        inner(&reported),
        "a server may report `supported_versions: {reported}` — any string, because \
         domains/state.yaml, on supported_versions, says \"a server may name versions neither pinned \
         revision covers\" — but a binding records the one it picked from that same \
         list as `selected_revision: {selected}`, a closed two-variant enum. \
         `2025-06-18` and `2024-11-05` are reportable and unselectable, and \
         `http-sse-2024-11-05` is a declared TransportBinding variant whose revision is \
         one of them"
    );
}

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
    /// Held by the named open `decision-blocker:`; a cardinality here would answer it.
    Filed(&'static str),
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
        Verdict::Filed("decision-blocker:mcp-caller-connection-assignment"),
    ),
    (
        "McpServerBinding",
        "supervised OS process",
        Verdict::Filed("decision-blocker:mcp-outbound-stdio-process-ownership"),
    ),
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
/// documents **three**; this list carries **two** of them, and the omission is
/// deliberate.
///
/// The third is `2024-11-05`, from that revision's own lifecycle examples
/// (`mcp-2025-11-25-basic-lifecycle.mdx:61` and `:107`). It cannot be banned from a
/// declaration here, because `connectors_mcp.protocol.TransportBinding` legitimately
/// carries the variant `http-sse-2024-11-05` — the deprecated-not-removed HTTP+SSE
/// binding named at `mcp-2026-07-28-basic-transports-streamable-http.mdx:695`. A rule
/// that banned the substring would make a correct model unfixable, so `2024-11-05` is
/// guarded by the exact-variant assertion on `ProtocolRevision` below instead, which is
/// the only place the trap could actually do harm.
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

/// A declaration name without its namespace: `connectors_mcp.state.McpCaller` → `McpCaller`.
fn short(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or(name)
}

/// `McpServerBinding` → `["mcp", "server", "binding"]`; `credential custody` →
/// `["credential", "custody"]`. Census targets are a mix of declared type names and the
/// prose names of things this repository does not model, so both forms are reduced the
/// same way.
fn words(name: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for ch in short(name).chars() {
        if (ch.is_ascii_uppercase() || !ch.is_ascii_alphanumeric()) && !current.is_empty() {
            out.push(std::mem::take(&mut current));
        }
        if ch.is_ascii_alphanumeric() {
            current.push(ch.to_ascii_lowercase());
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// Every field name that would read as a carrier for `target`: any contiguous run of its
/// words, with and without the `_ref`/`_refs`/`_id`/`_ids` suffixes this repository uses
/// for flat reference carriers, and each of those pluralised. `McpServerBinding` yields
/// `binding`, `bindings`, `binding_ref`, `binding_refs`, `server_binding`,
/// `mcp_server_binding`, … so both the `binding_ref: String` the model names as the risk
/// and the plural form that escaped the first repair are in the set.
fn carrier_names(target: &str) -> Vec<String> {
    let parts = words(target);
    let mut out = Vec::new();
    for start in 0..parts.len() {
        for end in start + 1..=parts.len() {
            let stem = parts[start..end].join("_");
            for suffix in ["", "_ref", "_refs", "_id", "_ids"] {
                let name = format!("{stem}{suffix}");
                if !name.ends_with('s') {
                    out.push(format!("{name}s"));
                }
                if name.ends_with("sh") || name.ends_with('s') || name.ends_with('x') {
                    out.push(format!("{name}es"));
                }
                out.push(name);
            }
        }
    }
    out
}

/// Declared `relations:` entries of `domains/state.yaml`, as
/// `(entity, target, "<name>, cardinality: <c>")`, all names shortened.
fn declared_relations() -> Vec<(String, String, String)> {
    let mut found = Vec::new();
    let Some(entities) = document("domains/state.yaml")
        .get("entities")
        .and_then(|e| e.as_sequence())
        .cloned()
    else {
        return found;
    };
    for entity in &entities {
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
            found.push((
                short(name).to_owned(),
                short(target).to_owned(),
                format!(
                    "{}, cardinality: {}",
                    relation.get("name").and_then(|n| n.as_str()).unwrap_or("?"),
                    relation
                        .get("cardinality")
                        .and_then(|c| c.as_str())
                        .unwrap_or("?")
                ),
            ));
        }
    }
    found
}

/// The `(field name, field type)` pairs `<declaration>` declares in `domains/state.yaml`,
/// across **both** `types:` and `entities:`.
///
/// Values are read as well as entities because a carrier on the value side realises an
/// edge exactly as well: `binding_ref: String` on the `McpCapabilitySnapshot` *value*
/// binds a snapshot to one binding, produces no `relations:` entry, and validates and
/// compiles at exit 0. The first repair of this guard read `entities:` only and the
/// pass-2 adversary measured that hole.
///
/// `identity` is deliberately not read: it is the declaration's own key, not a carrier
/// to something else.
fn declared_fields(declaration_short: &str) -> Vec<(String, String)> {
    let value = document("domains/state.yaml");
    for section in ["types", "entities"] {
        let Some(items) = value.get(section).and_then(|i| i.as_sequence()) else {
            continue;
        };
        for item in items {
            let name = item.get("name").and_then(|n| n.as_str()).unwrap_or("");
            if short(name) != declaration_short {
                continue;
            }
            let Some(fields) = item.get("fields").and_then(|f| f.as_sequence()) else {
                return Vec::new();
            };
            return fields
                .iter()
                .map(|field| {
                    let text = |key| {
                        field
                            .get(key)
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_owned()
                    };
                    (text("name"), text("type"))
                })
                .collect();
        }
    }
    Vec::new()
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

/// How far from an edge its own verdict may sit, in lines. **One**, so that a marker may
/// lead a wrapped sentence and no further.
///
/// Two was one too many and it was measured: the census footer lists its rows two lines
/// apart, so at a window of two every row sat inside its neighbours' markers, a row
/// could lose its verdict at every site in the file, and this guard stayed green while
/// the mutant validated at exit 0 — the strike lands inside a YAML comment. The guard's
/// own words are that a marker in a heading above a list is not a marker for the edges
/// in it; at two lines the footer was exactly that list.
const MARKER_WINDOW: usize = 1;

/// The text a reader must find beside the edge itself for each verdict.
///
/// `UNMAPPED` is required for a `Filed` row **as well as** its blocker id, because the
/// acceptance of `story:mcp-domain-model` says "every relation the model could not read
/// … carried as an explicit `UNMAPPED:` comment", and a filed decision is a relation the
/// model could not read — that is why it is filed. An earlier version of this guard
/// wanted the blocker id *instead*, which made it a test of this implementation's own
/// two-vocabulary scheme rather than of the sentence the story is accepted against.
/// `story:mcp-profile-selection-matrix` greps for the word.
fn markers(verdict: Verdict) -> Vec<&'static str> {
    match verdict {
        Verdict::Unmapped => vec!["UNMAPPED"],
        Verdict::Filed(blocker) => vec!["UNMAPPED", blocker],
        Verdict::Absent => vec!["ess/domains/sessions.yaml:89-91"],
        Verdict::Stated => Vec::new(),
    }
}

/// Whether a census target names a type this root could embed, rather than the prose
/// name of something this repository does not model (`credential custody`).
fn is_type_name(target: &str) -> bool {
    let short = short(target);
    !short.contains(' ') && short.starts_with(|c: char| c.is_ascii_uppercase())
}

/// Every census edge is marked *where it is named*, and no unreadable edge is realised
/// — in either of the two forms a realisation takes.
///
/// This is the case the named checks cannot make: `ess specify validate` accepts a
/// guessed `cardinality: one` on an UNMAPPED row exactly as readily as a correct one.
///
/// Two holes the pass-1 adversary measured in the first version of this case are closed
/// here, and both were holes of the same kind — a guard that reads the whole file
/// instead of the thing under test:
///
/// 1. it walked only `entities[].relations[]`, so a flat `binding_ref: String` on
///    `McpOutboundSession` — the realisation `domains/state.yaml` itself names as the
///    risk — was invisible and validated at exit 0. Both forms are read now: a declared
///    relation, and a field named for or typed as the target.
/// 2. the `UNMAPPED:` assertion was file-global, so it held while any one marker
///    survived anywhere. Each edge now needs its own verdict within `MARKER_WINDOW`
///    lines of every line that names it.
#[test]
fn every_census_edge_is_marked_and_no_unreadable_one_is_realised() {
    let text = read("domains/state.yaml");
    let lines: Vec<&str> = text.lines().collect();
    let declared = declared_relations();

    for (source, target, verdict) in CENSUS {
        let edge = format!("{source} → {target}");
        let sites: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.contains(&edge))
            .map(|(index, _)| index)
            .collect();
        assert!(
            !sites.is_empty(),
            "census edge `{edge}` is named nowhere in domains/state.yaml; every row of \
             story:mcp-domain-model's census must appear in the model that closes it"
        );

        // — the verdict must be legible at every site, not merely present in the file —
        for wanted in markers(*verdict) {
            let unmarked: Vec<String> = sites
                .iter()
                .filter(|site| {
                    let low = site.saturating_sub(MARKER_WINDOW);
                    let high = (**site + MARKER_WINDOW).min(lines.len() - 1);
                    !lines[low..=high].iter().any(|line| line.contains(wanted))
                })
                .map(|site| format!("domains/state.yaml:{}", site + 1))
                .collect();
            assert!(
                unmarked.is_empty(),
                "`{edge}` is named at {} without `{wanted}` within {MARKER_WINDOW} \
                 line(s). A marker in a heading above a list is not a marker for the \
                 edges in it; a reader who lands on one edge must see its whole verdict",
                unmarked.join(", ")
            );
        }

        if matches!(verdict, Verdict::Stated) {
            continue;
        }

        // — realisation form 1: a declared ESS relation —
        if let Some((_, _, label)) = declared
            .iter()
            .find(|(entity, to, _)| entity == source && to == short(target))
        {
            panic!(
                "`{edge}` is realised by the declared relation `{label}`, but no source \
                 answers it — it is carried as a marker, not chosen. \
                 `ess specify validate` exits 0 on this; that is why this case exists"
            );
        }

        // — realisation form 2: a carrier named for, or typed as, the other end —
        //
        // Symmetric, and over `types:` as well as `entities:`. An edge is a claim about
        // two things; either of them may carry it, and a value carries it as well as a
        // record does. `binding_ref: String` on the `McpCapabilitySnapshot` value binds
        // a snapshot to one binding just as surely as the same field on
        // `McpOutboundSession` would, and both validate at exit 0.
        for (near, far) in [(source, target), (target, source)] {
            let names = carrier_names(far);
            for (field, declared_type) in declared_fields(short(near)) {
                assert!(
                    !names.contains(&field),
                    "`{edge}` is realised by the carrier \
                     `{}.{field}: {declared_type}`, which names the other end. A flat \
                     carrier is the same claim in a weaker syntax — a scalar says \
                     one-to-one and a `List<…>` says one-to-many, as loudly as \
                     `cardinality:` does — and it produces no `relations:` entry and \
                     validates at exit 0. This edge has no answered cardinality to \
                     carry, at either end",
                    short(near)
                );
                assert!(
                    !is_type_name(far) || !declared_type.contains(short(far)),
                    "`{edge}` is realised by the carrier \
                     `{}.{field}: {declared_type}`, whose type names the other end. \
                     Embedding one end in the other is the same claim as declaring the \
                     relation",
                    short(near)
                );
            }
        }
    }

    // The one stated edge carries the binding its cardinality was read from. Not
    // `ess/domains/declarations.yaml:119-123`: that block declares `AdapterSpecification
    // owns many OperationDeclaration`, a different pair, and does not answer this edge.
    // The cardinality is read off the carrier, under the scalar-via-is-one binding these
    // four sibling blocks keep.
    for citation in [
        "ess/domains/credentials.yaml:40-45",
        "ess/domains/auth_bindings.yaml:102-106",
        "ess/domains/artifact_provenance.yaml:85-89",
        "ess/domains/auth_bindings.yaml:107",
    ] {
        assert!(
            text.contains(citation),
            "the one stated census edge is stated without `{citation}`, one of the four \
             `kind: references` blocks that bind a scalar `via` carrier to \
             `cardinality: one` and a `List<…>` carrier to `cardinality: many`"
        );
    }

    // The `references` scope of that rule is load-bearing, and dropping it would condemn
    // two correct shared declarations: `ess/domains/declarations.yaml:119-123` and
    // `ess/domains/discovery_state.yaml:85-89` both carry `cardinality: many` through a
    // scalar `via`, and both are `kind: owns`, where `via` names the child's
    // back-reference to the owner's identity. The model must keep saying so.
    //
    // Bound to the rule's own sentence, not to the file. `kind: references` appears
    // elsewhere in this model as part of an unrelated citation, so asserting the bare
    // phrase would be satisfied by that mention — the same file-global weakness this
    // guard was corrected for once already, and a red run caught it here a second time.
    for scope in [
        "`kind: references` relation — and ONLY for those",
        "ess/domains/discovery_state.yaml:85-89",
    ] {
        assert!(
            text.contains(scope),
            "the cardinality rule is stated without `{scope}`. Unscoped, the rule reads \
             on `kind: owns` relations too, where a scalar `via` carries `many` — and it \
             would condemn two correct shared declarations, one of them the very block \
             the same paragraph withdraws"
        );
    }
}

/// `domains/protocol.yaml` says it makes no selection. That sentence is what tells the
/// next nine stories that an enum here is the specification's option space and not this
/// repository's support claim, and `story:mcp-profile-selection-matrix` is written
/// against it.
///
/// **This case guards the disclaimer, not the property.** Nothing in this repository
/// stops a selection being *added* — a new `SelectedTransport` enum would validate,
/// compile and pass every case here. `adapters/mcp/design.md` says so in the same words,
/// so that a green suite is not read as evidence of something no case establishes.
#[test]
fn the_protocol_domain_still_disclaims_making_a_selection() {
    let text = read("domains/protocol.yaml");
    assert!(
        text.contains("no selection is made")
            && text.contains("story:mcp-profile-selection-matrix"),
        "domains/protocol.yaml no longer disclaims making a selection, or no longer \
         names the story that owns one. Every enum in that file is the pinned \
         specification's option space; without the disclaimer a reader takes it for a \
         list of what this repository supports"
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

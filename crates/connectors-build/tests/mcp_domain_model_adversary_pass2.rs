//! Adversary pass 2 for `story:mcp-domain-model`, written 2026-09-12 against `ff8ec40`
//! — the correction round over `d5270ca` — on branch `unit/mcp-domain-model-20260912`.
//!
//! Pass 1 filed five findings and the correction answered all five, twice going wider
//! than the finding: the owner document's denial was swept as a class, and the census
//! guard grew a second realisation form, a two-line marker window and a per-blocker
//! binding. These cases attack the correction itself, on the principle that a repair
//! written by the author of the defect is checked by nobody else:
//!
//! 1. the class the correction swept — "a document denies what this root declares" — has
//!    a sibling the sweep did not run: a *declaration* that closes a set the pinned
//!    schema declares open. `Optional<ProtocolRevision>` became `Optional<String>` for
//!    exactly that reason; the field one line above it in the same struct did not;
//! 2. the repaired marker window is two lines wide and the census footer lists its rows
//!    two lines apart, so a row's verdict can be supplied by its neighbour's;
//! 3. the acceptance says every relation no source answers is carried as an explicit
//!    `UNMAPPED:` comment. Two of the ten are carried as `FILED:` instead, and the
//!    guard was written to the implementation rather than to that sentence.
//!
//! Each case reads the tree it is run in. `MCP_ADVERSARY_ROOT` points them at a copy
//! carrying a candidate correction, so a fix can be watched going green without editing
//! the tree under review — the same override pass 1's file uses.

use std::path::{Path, PathBuf};

const STATE: &str = "adapters/mcp/spec/ess/domains/state.yaml";
const PROTOCOL: &str = "adapters/mcp/spec/ess/domains/protocol.yaml";
const CENSUS_SOURCE: &str = "crates/connectors-build/tests/mcp_domain_model_census.rs";

/// The marker the census guard requires beside an edge no source answers.
const UNMAPPED: &str = "UNMAPPED";

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

/// The 1-based line of the first line containing `needle`, so a failure cites a place.
fn line_of(text: &str, needle: &str) -> usize {
    text.lines()
        .position(|line| line.contains(needle))
        .map_or(0, |index| index + 1)
}

/// A declaration name without its namespace.
fn short(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or(name)
}

/// `ServerCapabilityKind` and `server_capabilities` both reduce to word lists, so a
/// carrier named for a thing can be recognised whatever case convention wrote it.
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

/// Whether two words name the same thing across a plural: `capability` and
/// `capabilities` share a stem, `capability` and `versions` do not. Five characters,
/// because an English plural never changes the first five of a word this long.
fn shares_stem(left: &str, right: &str) -> bool {
    left.len() >= 5 && right.len() >= 5 && left[..5] == right[..5]
}

/// Every `- name: <declaration>` of `domain` paired with the comment block immediately
/// above it, which is where this root records what its sources say about the value.
fn commented_declarations(domain: &str) -> Vec<(String, String)> {
    let text = read(domain);
    let mut block = String::new();
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            block.push_str(trimmed);
            block.push('\n');
        } else if let Some(rest) = trimmed.strip_prefix("- name:") {
            out.push((rest.trim().to_owned(), std::mem::take(&mut block)));
        } else if trimmed.is_empty() {
            block.clear();
        }
    }
    out
}

/// Whether a declaration's own comment records that the upstream set is not closed.
/// `domains/protocol.yaml` says it twice in these words — of the revision strings
/// ("the protocol's version set is not closed by these two") and of the server
/// capabilities ("this is not a closed set: any server can define its own").
fn declares_an_open_set(block: &str) -> bool {
    let lower = block.to_lowercase();
    [
        "not a closed set",
        "not the closed set",
        "is not closed",
        "set is not closed",
    ]
    .iter()
    .any(|phrase| lower.contains(phrase))
}

/// `(declaration, carrier name, declared type)` for every field and identity of `domain`.
fn declared_fields(domain: &str) -> Vec<(String, String, String)> {
    let value = document(domain);
    let mut out = Vec::new();
    for section in ["types", "entities"] {
        let Some(items) = value.get(section).and_then(|i| i.as_sequence()) else {
            continue;
        };
        for item in items {
            let owner = item
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or_default()
                .to_owned();
            let mut carriers: Vec<&serde_yaml_ng::Value> = Vec::new();
            if let Some(identity) = item.get("identity") {
                carriers.push(identity);
            }
            if let Some(fields) = item.get("fields").and_then(|f| f.as_sequence()) {
                carriers.extend(fields.iter());
            }
            for carrier in carriers {
                let text = |key| {
                    carrier
                        .get(key)
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_owned()
                };
                out.push((owner.clone(), text("name"), text("type")));
            }
        }
    }
    out
}

/// The escape `domains/protocol.yaml` itself names, under `ServerCapabilityKind`, — "How an unknown advertised
/// key is carried — a String, a second variant, a refusal — is unread". A declaration
/// that types one carrier to a closed enum is not closing the set if a sibling carrier
/// on the same declaration can hold the key the enum has no variant for.
fn carries_the_unknown_key(
    fields: &[(String, String, String)],
    owner: &str,
    closed_carrier: &str,
    open_enum: &str,
) -> bool {
    let named = words(open_enum);
    fields.iter().any(|(declaration, field, declared)| {
        declaration == owner
            && field != closed_carrier
            && matches!(
                declared.trim_matches('"'),
                "String" | "List<String>" | "Optional<String>"
            )
            && words(field).iter().any(|word| {
                named
                    .iter()
                    .any(|wanted| wanted != "kind" && shares_stem(word, wanted))
            })
    })
}

/// A value the pinned schema says a server may invent cannot be recorded in a field
/// typed to a closed enum of this root, and this root knows it: `selected_revision` was
/// `Optional<ProtocolRevision>` at `d5270ca` and is `Optional<String>` at `ff8ec40`
/// precisely because "a server may name versions neither pinned revision covers".
///
/// The same sentence is written over the capability set, from the same schema, and the
/// carrier was not changed. `mcp-2026-07-28-schema.ts:789`: "Known capabilities are
/// defined here, in this schema, but this is not a closed set: any server can define
/// its own, additional capabilities." `domains/protocol.yaml` copies that in, under `ServerCapabilityKind`, as
/// the reason `ServerCapabilityKind` is not a closed set, and then
/// `domains/state.yaml` types the discovered set to it anyway — so a snapshot of a
/// `server/discover` response cannot record a capability the specification explicitly
/// permits the server to advertise, and the file says so in its own comment: "an
/// unknown advertised key has no carrier here."
///
/// The rule is read out of `domains/protocol.yaml`, not hard-coded, so it keeps
/// holding when a revision closes a set or a later story opens another one, and the
/// carrier that answers it may be either shape the model's own comment offers: the
/// enum replaced by a string, or a sibling string carrier for the unknown key.
#[test]
fn no_state_carrier_closes_a_set_the_pinned_schema_leaves_open() {
    let open: Vec<String> = commented_declarations(PROTOCOL)
        .into_iter()
        .filter(|(_, block)| declares_an_open_set(block))
        .map(|(name, _)| name)
        .collect();
    assert!(
        !open.is_empty(),
        "{PROTOCOL} records no open set at all; this case reads the openness out of that \
         file and has nothing to read"
    );

    let protocol_text = read(PROTOCOL);
    let state_text = read(STATE);
    let fields = declared_fields(STATE);

    let mut violations = Vec::new();
    for open_enum in &open {
        for (owner, carrier, declared) in &fields {
            if !declared.contains(open_enum.as_str())
                || carries_the_unknown_key(&fields, owner, carrier, open_enum)
            {
                continue;
            }
            violations.push(format!(
                "{STATE}:{} `{}.{carrier}: {declared}` — `{}` is declared open at \
                 {PROTOCOL}:{}",
                line_of(&state_text, &format!("name: {carrier}")),
                short(owner),
                short(open_enum),
                line_of(&protocol_text, &format!("name: {open_enum}"))
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "a carrier closes a set the pinned schema leaves open:\n  {}\n\n\
         This is the correction's own reasoning, unapplied. `selected_revision` was \
         retyped from `Optional<ProtocolRevision>` to `Optional<String>` in {} because \
         a closed type makes a value the server may legally send unrecordable; \
         `ProtocolRevision` is still marked open in {PROTOCOL} and now has no carrier, \
         which is what a swept class looks like. `ServerCapabilityKind` carries the \
         identical sentence from the identical schema and was left typed. Either shape \
         the model's own comment offers turns this green: the enum replaced by a string \
         carrier, or a sibling `List<String>` on the same declaration for the key the \
         enum has no variant for",
        violations.join("\n  "),
        "ff8ec40"
    );
}

/// The census rows whose verdict is `UNMAPPED:`, in the story's order.
const UNREADABLE_EDGES: &[&str] = &[
    "McpServerBinding → connectors.declarations.ServiceConfiguration",
    "McpServerBinding → connectors.auth_bindings.Connection",
    "McpServerBinding → McpCapabilitySnapshot",
    "McpOutboundSession → McpServerBinding",
    "McpServerBinding → credential custody",
    "McpInboundSession → McpCaller",
];

/// The window the census guard uses, read from its own source so that narrowing the
/// constant — the fix this case names — turns this case green rather than leaving it
/// red against a repaired guard. `None` when the constant is gone, which means the
/// mechanism was replaced rather than tuned and this case no longer describes it.
fn marker_window() -> Option<usize> {
    read(CENSUS_SOURCE).lines().find_map(|line| {
        line.trim()
            .strip_prefix("const MARKER_WINDOW: usize =")
            .and_then(|rest| rest.trim().trim_end_matches(';').trim().parse().ok())
    })
}

/// The repaired guard binds a verdict "within two lines of the edge itself" and says
/// what that is for: "A marker in a heading above a list of five edges is not a marker
/// for any of them" (`domains/state.yaml`, under the marker convention). The census footer lists its rows two
/// lines apart, so at a window of two every row is inside its neighbours' markers and
/// the binding it claims to make is not made.
///
/// Measured as the guard would see it: strike `UNMAPPED` from every line that names one
/// edge — the whole of that row's verdict, everywhere in the file — and ask whether any
/// of that edge's sites is then unmarked. The strike is inside a YAML comment, so the
/// mutant still validates under the pinned toolchain exactly as the unmutated file does.
///
/// A window of one is enough for this model: every `UNMAPPED:` sits on the line that
/// names its edge, and the two `FILED` rows carry their blocker id on the next line.
#[test]
fn a_census_rows_verdict_cannot_be_supplied_by_the_row_beside_it() {
    let Some(window) = marker_window() else {
        return;
    };
    let text = read(STATE);
    let lines: Vec<&str> = text.lines().collect();

    let mut borrowed = Vec::new();
    let mut escaped = Vec::new();
    for edge in UNREADABLE_EDGES {
        let sites: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.contains(edge))
            .map(|(index, _)| index)
            .collect();
        assert!(
            !sites.is_empty(),
            "census edge `{edge}` is named nowhere in {STATE}"
        );

        for &site in &sites {
            let low = site.saturating_sub(window);
            let high = (site + window).min(lines.len() - 1);
            let neighbours: Vec<String> = (low..=high)
                .filter(|index| *index != site && lines[*index].contains(UNMAPPED))
                .map(|index| format!("{}", index + 1))
                .collect();
            if !neighbours.is_empty() {
                borrowed.push(format!(
                    "{STATE}:{} (`{edge}`) is inside the marker on line(s) {}",
                    site + 1,
                    neighbours.join(", ")
                ));
            }
        }

        let mutated: Vec<String> = lines
            .iter()
            .enumerate()
            .map(|(index, line)| {
                if sites.contains(&index) {
                    line.replace("UNMAPPED: ", "").replace(UNMAPPED, "")
                } else {
                    (*line).to_owned()
                }
            })
            .collect();
        let caught = sites.iter().any(|&site| {
            let low = site.saturating_sub(window);
            let high = (site + window).min(mutated.len() - 1);
            !mutated[low..=high]
                .iter()
                .any(|line| line.contains(UNMAPPED))
        });
        if !caught {
            escaped.push(*edge);
        }
    }

    assert!(
        escaped.is_empty(),
        "with MARKER_WINDOW = {window}, a census row can lose its verdict at every site \
         in {STATE} and the census case stays green: {}\n\n\
         Every site of that row sits inside another row's marker:\n  {}\n\n\
         The guard's own words are that a marker above a list is not a marker for the \
         edges in it; at two lines the footer list is exactly that, one row apart. \
         MARKER_WINDOW = 1 catches this mutant and leaves the current model passing — \
         every `UNMAPPED:` is on the line that names its edge, and the two `FILED` rows \
         carry their blocker id on the next line",
        escaped.join(", "),
        borrowed.join("\n  ")
    );
}

/// The two census rows carried as decisions rather than as markers, with the blocker id
/// each carries instead.
const FILED_EDGES: &[(&str, &str)] = &[
    (
        "McpCaller → connectors.auth_bindings.Connection",
        "decision-blocker:mcp-caller-connection-assignment",
    ),
    (
        "McpServerBinding → supervised OS process",
        "decision-blocker:mcp-outbound-stdio-process-ownership",
    ),
];

/// The acceptance of `story:mcp-domain-model`, in its own words: "every relation the
/// model could not read from an existing `ess/1` document or the pinned specification
/// carried as an explicit `UNMAPPED:` comment rather than a chosen cardinality".
///
/// Eight of the ten census rows are relations no source answers. Six carry `UNMAPPED:`.
/// Two carry `FILED:` and a blocker id and no `UNMAPPED:` anywhere they are named, and
/// the census guard was written to that second vocabulary — `Verdict::Filed` wants the
/// blocker id — rather than to the sentence the story is accepted against, so nothing
/// in the repository compares the two.
///
/// The reader this costs is named and dated: `story:mcp-profile-selection-matrix` says
/// a row whose disposition needs an ownership relation "cites the `UNMAPPED:` marker
/// `story:mcp-domain-model` carries". Grepping that marker returns six of the eight
/// open questions, and the two it omits are the stdio process ownership and the caller
/// connection assignment — both of which that matrix has a row for.
#[test]
fn every_relation_no_source_answers_carries_the_marker_the_acceptance_names() {
    let text = read(STATE);
    let mut unmarked = Vec::new();
    for (edge, blocker) in FILED_EDGES {
        let sites: Vec<(usize, &str)> = text
            .lines()
            .enumerate()
            .filter(|(_, line)| line.contains(edge))
            .collect();
        assert!(
            !sites.is_empty(),
            "census edge `{edge}` is named nowhere in {STATE}"
        );
        if !sites.iter().any(|(_, line)| line.contains(UNMAPPED)) {
            unmarked.push(format!(
                "`{edge}` at {} carries only `{blocker}`",
                sites
                    .iter()
                    .map(|(index, _)| format!("{STATE}:{}", index + 1))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }

    assert!(
        unmarked.is_empty(),
        "a relation no source answers is carried without the `UNMAPPED:` comment the \
         acceptance names:\n  {}\n\n\
         A filed decision is a relation the model could not read — that is why it is \
         filed — so the acceptance covers it. The blocker id is the stronger half of \
         the verdict and belongs where it is; what is missing is the word the \
         acceptance requires and the next story greps for. `UNMAPPED: … FILED under \
         <blocker>` on the same line satisfies both and loses nothing",
        unmarked.join("\n  ")
    );
}

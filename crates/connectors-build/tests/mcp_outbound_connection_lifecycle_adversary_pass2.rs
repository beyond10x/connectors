//! Adversary pass 2 against `story:mcp-outbound-connection-lifecycle`, written
//! 2026-09-12 on branch `unit/mcp-outbound-connection-lifecycle-20260912` at `1688ba5`.
//!
//! Pass 1 found one root cause behind three MUST-level defects: the selection matrix
//! dispositions **two** revisions `supported` outbound, they disagree, and a passage that
//! named neither stated the primary one's rule as the transport's. The correction answered
//! that as a class — a preamble naming the pair, every numbered section saying what it
//! holds for each, every scenario naming the binding it is about, and a case that refuses
//! a section or a scenario that resolves neither.
//!
//! **The case reads the scenario's `given` and nothing else.** So a scenario whose `given`
//! admits both revisions and whose `when`/`then` states a mechanism the document itself
//! says exists under only one of them satisfies the new rule vacuously: it named a
//! revision, and then specified the other one's wire. Two of the twenty-two do exactly
//! that, and they are the two that decide the ordinary selection outcome and the ordinary
//! capability observation.
//!
//! Every case below asserts its premise — that the passage it drives still carries the
//! claim it is read for — before the assertion that rests on it, so a reworded document
//! reports drift here instead of going quietly green. Nothing in this file is hard-coded
//! that the contract can supply: the register rows, the revision names, the scenario a row
//! points at and the fields of the binding entity are all read out of the documents and
//! the model.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const STORY: &str = "story:mcp-outbound-connection-lifecycle";

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn client_dir() -> PathBuf {
    repository_root().join("adapters/mcp/contracts/client/v1alpha1")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn semantics() -> String {
    read(&client_dir().join("semantics.md"))
}

fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A register row, read out of the document's own table.
#[derive(Debug, Clone)]
struct Row {
    state: String,
    outcome: String,
    wire_code: String,
    actor: String,
    scenario: String,
    line: usize,
}

fn rows() -> Vec<Row> {
    let document = semantics();
    let link = regex::Regex::new(r"\(scenarios/([A-Za-z0-9._-]+\.yaml)\)").unwrap();
    let mut parsed = Vec::new();
    for (index, line) in document.lines().enumerate() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }
        let cells: Vec<String> = trimmed
            .trim_matches('|')
            .split('|')
            .map(|cell| cell.trim().to_string())
            .collect();
        let state = cells[0].trim_matches('`').to_string();
        if !state.starts_with("state:") || cells.len() != 5 {
            continue;
        }
        let Some(capture) = link.captures(&cells[4]) else {
            continue;
        };
        parsed.push(Row {
            state,
            outcome: cells[1].trim_matches('`').to_string(),
            wire_code: cells[2].trim_matches('`').to_string(),
            actor: cells[3].trim_matches('`').to_string(),
            scenario: capture.get(1).expect("scenario link").as_str().to_string(),
            line: index + 1,
        });
    }
    assert!(
        parsed.len() > 4,
        "semantics.md carries {} register rows this file can read, so it is reading the \
         wrong table",
        parsed.len()
    );
    parsed
}

fn row_for(outcome: &str) -> Row {
    let rows = rows();
    let matching: Vec<&Row> = rows.iter().filter(|row| row.outcome == outcome).collect();
    assert_eq!(
        matching.len(),
        1,
        "the register carries {} rows for `{outcome}`, so this case cannot say which one \
         it drives",
        matching.len()
    );
    matching[0].clone()
}

/// A `###` outcome section, its heading included, ending at the next heading of any level.
fn outcome_section(outcome: &str) -> String {
    let document = semantics();
    let heading = format!("### `{outcome}`");
    let start = document
        .find(&heading)
        .unwrap_or_else(|| panic!("semantics.md defines no `{heading}`"));
    let rest = &document[start + heading.len()..];
    let end = rest
        .find("\n## ")
        .into_iter()
        .chain(rest.find("\n### "))
        .min()
        .unwrap_or(rest.len());
    flat(&rest[..end])
}

/// A `##` section, its heading included, ending at the next `##`.
fn numbered_section(prefix: &str) -> String {
    let document = semantics();
    let heading = regex::Regex::new(&format!(r"(?m)^## {prefix}\..*$")).unwrap();
    let found = heading
        .find(&document)
        .unwrap_or_else(|| panic!("semantics.md carries no section `## {prefix}.`"));
    let rest = &document[found.start()..];
    let end = rest[found.len()..]
        .find("\n## ")
        .map(|offset| offset + found.len())
        .unwrap_or(rest.len());
    flat(&rest[..end])
}

/// The name the document's own preamble gives each revision it names.
fn revision_aliases() -> BTreeMap<String, String> {
    let document = flat(&semantics());
    let declaration =
        regex::Regex::new(r"`revision:([0-9A-Za-z.-]+)` \*\*(the [a-z ]+ revision)\*\*").unwrap();
    let aliases: BTreeMap<String, String> = declaration
        .captures_iter(&document)
        .map(|capture| (capture[1].to_string(), capture[2].to_string()))
        .collect();
    assert!(
        aliases.len() > 1,
        "semantics.md's preamble names {} revision(s), so no passage can state the wrong \
         one's rule and these cases have nothing to hold: {aliases:?}",
        aliases.len()
    );
    aliases
}

/// The revision this repository calls the interoperability one, as `(id, name)`.
fn interoperability_revision() -> (String, String) {
    let aliases = revision_aliases();
    let found = aliases
        .iter()
        .find(|(_, name)| name.contains("interoperability"))
        .unwrap_or_else(|| {
            panic!("semantics.md's preamble names no interoperability revision: {aliases:?}")
        });
    (found.0.clone(), found.1.clone())
}

fn scenario(name: &str) -> serde_yaml_ng::Value {
    let path = client_dir().join("scenarios").join(name);
    let text = read(&path);
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text)
        .unwrap_or_else(|e| panic!("{} is not YAML: {e}", path.display()));
    assert_eq!(
        value["story"].as_str(),
        Some(STORY),
        "{} is not owned by {STORY}",
        path.display()
    );
    value
}

/// One block of a scenario, as its bullets with each bullet's wrap collapsed.
fn bullets(value: &serde_yaml_ng::Value, key: &str) -> Vec<String> {
    match &value[key] {
        serde_yaml_ng::Value::Sequence(items) => items
            .iter()
            .filter_map(|item| item.as_str())
            .map(flat)
            .collect(),
        serde_yaml_ng::Value::String(text) => vec![flat(text)],
        _ => Vec::new(),
    }
}

fn every_scenario_owned_by_this_story() -> Vec<(String, serde_yaml_ng::Value)> {
    let directory = client_dir().join("scenarios");
    let mut owned = Vec::new();
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&directory)
        .unwrap_or_else(|e| panic!("read {}: {e}", directory.display()))
        .map(|entry| entry.expect("scenario directory entry").path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("yaml"))
        .collect();
    paths.sort();
    for path in paths {
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(&read(&path))
            .unwrap_or_else(|e| panic!("{} is not YAML: {e}", path.display()));
        if value["story"].as_str() == Some(STORY) {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .expect("scenario file name")
                .to_string();
            owned.push((name, value));
        }
    }
    assert!(
        owned.len() > 4,
        "{} scenario files carry `story: {STORY}`, so these cases are reading the wrong \
         directory",
        owned.len()
    );
    owned
}

/// The line a bullet's first words occupy in its own file, so a finding carries a
/// `file:line` a reader can open.
fn line_of(name: &str, needle: &str) -> usize {
    let path = client_dir().join("scenarios").join(name);
    let text = read(&path);
    let probe: String = needle
        .split_whitespace()
        .take(4)
        .collect::<Vec<_>>()
        .join(" ");
    for (index, line) in text.lines().enumerate() {
        if flat(line).contains(&probe) {
            return index + 1;
        }
    }
    0
}

// ── 1 ───────────────────────────────────────────────────────────────────────────────
//
// The scenario that decides the ordinary selection outcome admits a binding configured
// for the interoperability revision and then states the primary revision's framing for
// it: the revision is declared "in its body `_meta`", on "every request".
//
// Section 6 of the same document says that revision has no `_meta` mirror at all, and
// that `MCP-Protocol-Version` is required there only *after* the handshake — so the one
// request that opens an interoperability binding, `initialize`, carries neither of the
// two things this scenario says every request of it carries.
//
// The correction's own case cannot see this: it reads the `given` and asks only that a
// revision be named there.

#[test]
fn the_selection_scenario_states_a_framing_the_interoperability_revision_has_not() {
    let (id, name) = interoperability_revision();
    let row = row_for("mcp.outbound.server-selected");

    // Premise: this is the only scenario that decides a successful explicit selection, so
    // a binding configured for either revision is decided by this file and no other.
    let selection_rows: Vec<Row> = rows()
        .into_iter()
        .filter(|other| other.state == row.state && other.wire_code == "n/a")
        .collect();
    assert_eq!(
        selection_rows.len(),
        1,
        "`{}` now carries {} non-refusing rows, so this case can no longer say that one \
         scenario decides the selection of every binding: {:?}",
        row.state,
        selection_rows.len(),
        selection_rows
            .iter()
            .map(|other| other.outcome.as_str())
            .collect::<Vec<_>>()
    );

    // Premise: section 6 denies the interoperability revision a `_meta` mirror, and scopes
    // its one required header to after the handshake.
    let framing = numbered_section("6");
    assert!(
        framing.to_lowercase().contains(&name) || framing.contains(&id),
        "section 6 no longer says what it holds for `{id}`, which is the claim this case \
         drives the scenario against"
    );
    let denies_meta = regex::Regex::new(r"(?i)no `?_meta`? mirror").unwrap();
    assert!(
        denies_meta.is_match(&framing),
        "section 6 no longer denies the {name} a `_meta` mirror, so re-check the finding \
         this case reports rather than trusting its verdict"
    );
    assert!(
        framing.contains("after the handshake"),
        "section 6 no longer scopes the {name}'s required header to after the handshake, \
         so re-check the finding this case reports"
    );

    // Premise: the scenario admits a binding configured for the interoperability revision.
    let value = scenario(&row.scenario);
    let given = bullets(&value, "given").join(" ");
    assert!(
        given.to_lowercase().contains(&name) || given.contains(&id),
        "{} no longer admits a binding configured for `{id}`, which is the state this \
         finding is about: {given:?}",
        row.scenario
    );

    // The finding: its `then` requires of *that* binding a framing section 6 gives only
    // the other revision.
    let offending: Vec<String> = bullets(&value, "then")
        .into_iter()
        .filter(|bullet| bullet.contains("_meta"))
        .collect();
    assert!(
        offending.is_empty(),
        "semantics.md:{} files `{}` under the scenario {}, whose `given` admits a binding \
         configured for `{id}` ({name}) and whose `then` then states the other revision's \
         framing for it — section 6 says {name} has no `_meta` mirror and requires \
         `MCP-Protocol-Version` only after the handshake, so `initialize` carries neither \
         of the two things this bullet says every request carries:\n  {}:{} {}",
        row.line,
        row.outcome,
        row.scenario,
        row.scenario,
        line_of(&row.scenario, &offending[0]),
        offending
            .iter()
            .map(|bullet| format!("{bullet:?}"))
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

// ── 2 ───────────────────────────────────────────────────────────────────────────────
//
// The same shape, in the scenario that decides what happens to a capability key this
// repository does not consume. Its `given` admits both revisions in as many words —
// "an advertised key is recorded and projected into nothing on both" — and its `when`
// then sources the advertised set from the `server/discover` result, which section 1 says
// the interoperability revision does not have at all.
//
// So for half the bindings this scenario claims to cover, the only `when` it states never
// occurs, and the outcome the register says this file decides is decided by nothing.

#[test]
fn the_advertised_capability_scenario_reads_a_result_the_interoperability_revision_has_not() {
    let (id, name) = interoperability_revision();
    let row = row_for("mcp.outbound.advertised-capability-recorded-not-projected");

    // Premise: section 1 denies the interoperability revision a description request.
    let discovery = numbered_section("1");
    assert!(
        discovery.to_lowercase().contains(&name) || discovery.contains(&id),
        "section 1 no longer says what it holds for `{id}`, which is the claim this case \
         drives the scenario against"
    );
    let denies_discovery =
        regex::Regex::new(r"(?i)(no description request at all|has no description request)")
            .unwrap();
    assert!(
        denies_discovery.is_match(&discovery),
        "section 1 no longer denies the {name} a description request, so re-check the \
         finding this case reports rather than trusting its verdict: {discovery:?}"
    );

    // Premise: the scenario admits a binding configured for the interoperability revision.
    let value = scenario(&row.scenario);
    let given = bullets(&value, "given").join(" ");
    assert!(
        given.to_lowercase().contains(&name) || given.contains(&id),
        "{} no longer admits a binding configured for `{id}`: {given:?}",
        row.scenario
    );

    // The finding: the only `when` it states is a request that revision does not define.
    let discover =
        regex::Regex::new(r"(?i)(server/discover|discover result|discovery result)").unwrap();
    let offending: Vec<String> = bullets(&value, "when")
        .into_iter()
        .filter(|bullet| discover.is_match(bullet))
        .collect();
    assert!(
        offending.is_empty(),
        "semantics.md:{} files `{}` under the scenario {}, whose `given` admits a binding \
         configured for `{id}` ({name}) and says the outcome holds \"on both\" — and whose \
         only `when` sources the recorded set from a request section 1 says {name} \
         does not have, citing the other revision's archive for it. On that half of the \
         bindings this file claims, the `when` never occurs and the outcome is decided by \
         nothing:\n  {}:{} {}",
        row.line,
        row.outcome,
        row.scenario,
        row.scenario,
        line_of(&row.scenario, &offending[0]),
        offending
            .iter()
            .map(|bullet| format!("{bullet:?}"))
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

// ── 3 ───────────────────────────────────────────────────────────────────────────────
//
// The correction answered "no sentence writes a binding field" with a case over
// `semantics.md`. The contract is `semantics.md` **and its scenarios** — the story's
// acceptance says so in as many words — and the scenarios are not read by it. Nor is the
// active voice: its detector matches `is recorded` and not `records`.
//
// Both halves of that gap are occupied by one bullet, which says of a field the document
// itself refuses to say it: section 4 declines to state whether what a probe settles
// outlives the process, and its own scenario says the probe records it.

#[test]
fn no_sentence_of_this_contract_writes_a_field_of_the_binding_entity() {
    let model = read(&repository_root().join("adapters/mcp/spec/ess/domains/state.yaml"));
    let start = model
        .find("- name: connectors_mcp.state.McpServerBinding")
        .expect("the MCP model declares no `McpServerBinding`");
    let rest = &model[start..];
    let end = rest
        .find("\n    lifecycle:")
        .expect("`McpServerBinding` declares no lifecycle, so its field block has no end");
    let fields: Vec<String> = regex::Regex::new(r"\{name: ([a-z_]+), type:")
        .unwrap()
        .captures_iter(&rest[..end])
        .map(|capture| capture[1].to_string())
        .collect();
    assert!(
        fields.iter().any(|field| field.contains('_')),
        "no field of `McpServerBinding` carries an underscore, so the normalisation this \
         case rests on is unnecessary and the case should be re-read: {fields:?}"
    );

    // The unit's own detector, copied verbatim from
    // `mcp_outbound_connection_lifecycle.rs`: passive voice only.
    let unit_detector = regex::Regex::new(
        r"(?i)\b(?:is|are|was|were|gets?|becomes?)\s+(?:then\s+)?(?:recorded|set|written|stored|saved|persisted|updated|assigned|remembered)\b",
    )
    .unwrap();
    // The same verbs, in the voice the contract actually uses as well.
    let writes = regex::Regex::new(
        r"(?i)\b(?:(?:is|are|was|were|gets?|becomes?)\s+(?:then\s+)?(?:recorded|set|written|stored|saved|persisted|updated|assigned|remembered)|records?|sets?|writes?|stores?|saves?|persists?|updates?|assigns?|remembers?)\b",
    )
    .unwrap();

    // Premise: the unit's case is green on the file it reads, so anything below is ground
    // that case does not cover rather than a second wording of it.
    let document = semantics();
    let uncovered: Vec<String> = document
        .split("\n\n")
        .filter(|paragraph| fields.iter().any(|field| paragraph.contains(field)))
        .filter(|paragraph| unit_detector.is_match(paragraph))
        .map(flat)
        .collect();
    assert!(
        uncovered.is_empty(),
        "the unit's own case is already red on semantics.md, so this case is not measuring \
         the gap it claims: {uncovered:?}"
    );

    let mut broken = Vec::new();
    for (name, value) in every_scenario_owned_by_this_story() {
        // `never:` states a prohibition, so a writing verb there is the opposite of a
        // write and is not read.
        for key in ["when", "then"] {
            for bullet in bullets(&value, key) {
                let named: Vec<&String> = fields
                    .iter()
                    .filter(|field| {
                        let separators = field.replace('_', "[_ ]");
                        regex::Regex::new(&format!(r"(?i)\b{separators}\b"))
                            .unwrap()
                            .is_match(&bullet)
                    })
                    .collect();
                if named.is_empty() {
                    continue;
                }
                let Some(verb) = writes.find(&bullet) else {
                    continue;
                };
                let seen_by_the_unit = if unit_detector.is_match(&bullet) {
                    "and the unit's own detector sees it"
                } else {
                    "and the unit's own detector is passive-voice only, so it does not"
                };
                broken.push(format!(
                    "{name}:{} `{key}` says `{}` of {named:?}, which are fields of the \
                     `McpServerBinding` entity — {seen_by_the_unit}: {bullet:?}",
                    line_of(&name, &bullet),
                    verb.as_str().trim(),
                ));
            }
        }
    }
    assert!(
        broken.is_empty(),
        "{} scenario bullets write a field of the binding entity, in a file the unit's \
         case does not read; this contract is `semantics.md` **and its scenarios**, and \
         the document itself declines to state what these say:\n  {}",
        broken.len(),
        broken.join("\n  ")
    );
}

// ── 4 ───────────────────────────────────────────────────────────────────────────────
//
// `mcp.outbound.answer-never-arrived` was added to close pass 1's third finding: a
// single-object answer lost in flight reached no row. It was filed under
// `state:transport-unreachable` — beside `mcp.outbound.transport-unavailable`, whose own
// section says "no request was ever framed" — and its own section says the opposite twice:
// the request "was framed and sent", and the endpoint "was reachable enough to take the
// request".
//
// So the state a reader routes on now both requires and forbids a framed request, and its
// name denies one of the two outcomes it carries. Ten stories read this register.

#[test]
fn a_state_does_not_carry_two_outcomes_that_deny_each_other_s_precondition() {
    let lost = row_for("mcp.outbound.answer-never-arrived");
    let unreachable = row_for("mcp.outbound.transport-unavailable");

    // Premise: the two sections state opposite preconditions, in the document's own words.
    let lost_section = outcome_section(&lost.outcome);
    let unreachable_section = outcome_section(&unreachable.outcome);
    assert!(
        lost_section.contains("was framed and sent"),
        "`{}` no longer states that the request was framed and sent, so re-check the \
         finding this case reports: {lost_section:?}",
        lost.outcome
    );
    assert!(
        lost_section.contains("reachable enough to take the request"),
        "`{}` no longer states that the endpoint was reachable, so re-check the finding \
         this case reports: {lost_section:?}",
        lost.outcome
    );
    assert!(
        unreachable_section.contains("no request was ever framed"),
        "`{}` no longer states that no request was ever framed, so re-check the finding \
         this case reports: {unreachable_section:?}",
        unreachable.outcome
    );

    assert_ne!(
        lost.state, unreachable.state,
        "semantics.md:{} files `{}` under `{}`, the same state semantics.md:{} files `{}` \
         under. That one state's two outcomes deny each other's precondition in the \
         document's own words — one says the request \"was framed and sent\" and the \
         endpoint \"was reachable enough to take the request\", the other says \"no \
         request was ever framed\" — and the state's own name asserts the transport could \
         not be reached, which its newer row explicitly denies",
        lost.line, lost.outcome, lost.state, unreachable.line, unreachable.outcome,
    );
}

// ── 5 ───────────────────────────────────────────────────────────────────────────────
//
// The fourth column was the correction's answer to the second class: whose act a refusal
// reports is a fact a register can carry, and once it does, a code that contradicts it is
// mechanical. The unit's case holds the column against the *code*. Nothing holds it
// against the *prose*, and the unit said it could not bound that class.
//
// One row cannot be filled at all under the preamble's own definitions.
// `mcp.outbound.transport-unavailable` reports the act of `peer` — "something the server
// did" — and enumerates four reasons, of which one is a name that never resolved to a
// server and one is a bound within which nothing answered, which is the preamble's own
// definition of `unobserved`: "where nothing was seen at all".

#[test]
fn a_refusal_does_not_name_the_peer_for_a_reason_in_which_no_peer_was_seen() {
    let document = flat(&semantics());
    // Premise: the preamble still defines the two actors this case reasons with.
    assert!(
        document.contains("`peer` for something the server did"),
        "the preamble no longer defines `peer` as something the server did, so re-check \
         the finding this case reports"
    );
    assert!(
        document.contains("`unobserved` where nothing was seen at all"),
        "the preamble no longer defines `unobserved` as nothing seen at all, so re-check \
         the finding this case reports"
    );

    let row = row_for("mcp.outbound.transport-unavailable");
    let section = outcome_section(&row.outcome);
    // Premise: the section still enumerates the reasons this case reads.
    let unseen = regex::Regex::new(
        r"(?i)(the name did not resolve|nothing answered within the connect bound)",
    )
    .unwrap();
    let found: Vec<String> = unseen
        .find_iter(&section)
        .map(|hit| hit.as_str().to_string())
        .collect();
    assert!(
        found.len() > 1,
        "`{}` no longer enumerates both a name that did not resolve and a bound nothing \
         answered within, so re-check the finding this case reports: {section:?}",
        row.outcome
    );

    assert_ne!(
        row.actor, "peer",
        "semantics.md:{} reports `{}` as the act of `peer` — \"something the server did\" \
         — and its own section enumerates {found:?}. Neither is an act of a server: the \
         first never resolved a name to one, and the second is the preamble's own \
         definition of `unobserved`, \"where nothing was seen at all\". The register's \
         sibling row makes the inversion plain: `mcp.outbound.answer-never-arrived`, where \
         the endpoint demonstrably acted by accepting the request, is `unobserved`, and \
         this row, where nothing from the endpoint was seen at all, is `peer`. One actor \
         per row cannot express this enumeration",
        row.line, row.outcome,
    );
}

//! Cases for `story:mcp-inbound-local-binding`, written 2026-09-12 on branch
//! `unit/mcp-inbound-local-binding-20260912`.
//!
//! The story's acceptance is a correspondence, not a prose quality: the inbound binding
//! document must carry **six** protocol behaviours, each with a named observable
//! outcome, the state transition it turns on in the `connectors.sessions.Session`
//! vocabulary, and a scenario file named after it — six behaviours, six outcomes, six
//! scenario files, one to one. Nothing in the repository checks a correspondence like
//! that: `ess verify conform author` compiles a scenario against the shared model and
//! says nothing about whether a document names it, and a document is prose to every
//! other gate step. These cases are what fails when the two drift apart.
//!
//! Two of them are derived rather than listed, because a hand-maintained list is the
//! defect the derived form removes:
//!
//! * the legal transition names, their `from` states and their `to` state come out of
//!   `ess/domains/sessions.yaml`, so a document naming `permit_data (Ready → Closing)`
//!   fails even though both halves exist;
//!   and the command/outcome pair that performs a transition comes from the same file's
//!   `moves:` bindings, so a scenario that names a behaviour but never performs its
//!   transition fails;
//! * the selected inbound transport comes out of
//!   `adapters/mcp/contracts/protocol/v1alpha1/selection.md`, so this document cannot
//!   specify a transport the matrix deferred or refused — which is the one thing the
//!   story says this document may not do ("Your document specifies what the selected
//!   things do; it does not select").

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The six behaviours `story:mcp-inbound-local-binding`'s acceptance enumerates, in its
/// own words and its own order. A seventh row in the document is not a failure; a
/// missing one of these is.
const BEHAVIOURS: &[&str] = &[
    "message framing",
    "streaming",
    "progress",
    "cancellation",
    "connection and session loss",
    "version and capability mismatch",
];

/// The story that owns this document and its scenario files. Two sibling stories —
/// `story:mcp-inbound-capability-projection` and `story:mcp-inbound-mutation-replay` —
/// add their own files to the same directory under their own names, so every rule below
/// that quantifies over the directory is scoped to the files that name this owner.
const OWNER: &str = "story:mcp-inbound-local-binding";

/// Cloud, identity and network dependencies the acceptance forbids this binding from
/// resting on: "the whole binding is described without reference to any cloud control
/// plane, identity service or network service this repository does not already have".
/// Naming one is allowed — refusing it is half the document's job — so the rule is that
/// every occurrence sits on a line that refuses, defers or excludes it.
const OUTSIDE_DEPENDENCIES: &[&str] = &[
    "oauth",
    "authorization server",
    "identity service",
    "identity provider",
    "control plane",
    "token endpoint",
    "federation",
    "cloud",
    "https://",
];

/// Words that make a line a refusal rather than a dependency.
const REFUSAL_WORDS: &[&str] = &[
    "not ", "no ", "none", "never", "refus", "without", "outside", "deferred", "held", "cannot",
    "stops",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(relative: &str) -> String {
    let path = root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// The inbound binding document this story writes.
fn binding() -> String {
    read("adapters/mcp/contracts/server/v1alpha1/semantics.md")
}

fn scenario_directory() -> PathBuf {
    root().join("adapters/mcp/contracts/server/v1alpha1/scenarios")
}

/// The `## <number>. <title>` section whose title contains `title`, without its heading.
fn section(document: &str, title: &str) -> String {
    let mut collecting = false;
    let mut found = String::new();
    for line in document.lines() {
        if line.starts_with("## ") {
            if collecting {
                break;
            }
            collecting = line.to_lowercase().contains(&title.to_lowercase());
            continue;
        }
        if collecting {
            found.push_str(line);
            found.push('\n');
        }
    }
    assert!(
        !found.trim().is_empty(),
        "the inbound binding document has no `## …{title}…` section"
    );
    found
}

/// Markdown table rows of `text`, as cell vectors, without the header and separator.
fn table_rows(text: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }
        let cells: Vec<String> = trimmed
            .trim_matches('|')
            .split('|')
            .map(|cell| cell.trim().to_string())
            .collect();
        if cells
            .iter()
            .all(|cell| !cell.is_empty() && cell.chars().all(|c| c == '-' || c == ':'))
        {
            continue;
        }
        rows.push(cells);
    }
    rows
}

/// A cell's text with backticks, emphasis and link syntax removed, lowercased.
fn key_of(cell: &str) -> String {
    cell.trim()
        .trim_matches('*')
        .trim_matches('`')
        .trim()
        .to_lowercase()
}

/// The six behaviour rows of the document's enumeration, keyed by behaviour.
/// Columns: behaviour, observable outcome, session transition, scenario.
fn behaviour_rows() -> BTreeMap<String, Vec<String>> {
    let document = binding();
    let enumeration = section(&document, "the six behaviours");
    let mut rows = BTreeMap::new();
    for cells in table_rows(&enumeration) {
        let key = key_of(&cells[0]);
        if !BEHAVIOURS.contains(&key.as_str()) {
            continue;
        }
        assert_eq!(
            cells.len(),
            4,
            "the behaviour row for `{key}` has {} cells, not the four the enumeration \
             declares (behaviour, observable outcome, session transition, scenario)",
            cells.len()
        );
        rows.insert(key, cells);
    }
    rows
}

/// The refusal rows of `## … does not offer`, keyed by the same six behaviours.
fn refusal_rows() -> BTreeMap<String, Vec<String>> {
    let document = binding();
    let refusals = section(&document, "does not offer");
    let mut rows = BTreeMap::new();
    for cells in table_rows(&refusals) {
        let key = key_of(&cells[0]);
        if !BEHAVIOURS.contains(&key.as_str()) {
            continue;
        }
        rows.insert(key, cells);
    }
    rows
}

/// `` `permit_data` (`Ready` → `Ready`) `` → `("permit_data", "Ready", "Ready")`.
fn parse_transition(cell: &str) -> (String, String, String) {
    let pattern =
        regex::Regex::new(r"`([a-z_]+)`\s*\(\s*`([A-Za-z]+)`\s*(?:→|->)\s*`([A-Za-z]+)`\s*\)")
            .unwrap();
    let capture = pattern.captures(cell).unwrap_or_else(|| {
        panic!(
            "transition cell {cell:?} is not of the form `<transition>` (`<From>` → `<To>`), \
             which is what binds it to the `connectors.sessions.Session` lifecycle"
        )
    });
    (
        capture[1].to_string(),
        capture[2].to_string(),
        capture[3].to_string(),
    )
}

/// The `connectors.sessions.Session` lifecycle as `ess/domains/sessions.yaml` declares
/// it: transition name → (from states, to state). Read, never listed.
fn declared_transitions() -> BTreeMap<String, (BTreeSet<String>, String)> {
    let document: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&read("ess/domains/sessions.yaml")).expect("parse sessions.yaml");
    let entities = document["entities"]
        .as_sequence()
        .expect("sessions.yaml declares entities");
    let session = entities
        .iter()
        .find(|entity| entity["name"].as_str() == Some("connectors.sessions.Session"))
        .expect("sessions.yaml declares connectors.sessions.Session");
    let mut found = BTreeMap::new();
    for transition in session["lifecycle"]["transitions"]
        .as_sequence()
        .expect("the Session lifecycle declares transitions")
    {
        let name = transition["name"].as_str().expect("a named transition");
        let to = transition["to"].as_str().expect("a transition target");
        let from: BTreeSet<String> = transition["from"]
            .as_sequence()
            .expect("a transition source list")
            .iter()
            .map(|state| state.as_str().expect("a state name").to_string())
            .collect();
        found.insert(name.to_string(), (from, to.to_string()));
    }
    found
}

/// Transition name → the `(command, outcome)` pairs that perform it, read off the
/// `moves:` bindings of `ess/domains/sessions.yaml`.
fn movers() -> BTreeMap<String, BTreeSet<(String, String)>> {
    let document: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&read("ess/domains/sessions.yaml")).expect("parse sessions.yaml");
    let mut found: BTreeMap<String, BTreeSet<(String, String)>> = BTreeMap::new();
    for command in document["commands"]
        .as_sequence()
        .expect("sessions.yaml declares commands")
    {
        let name = command["name"].as_str().expect("a named command");
        let Some(outcomes) = command["outcomes"].as_sequence() else {
            continue;
        };
        for outcome in outcomes {
            let Some(moves) = outcome["moves"].as_str() else {
                continue;
            };
            let Some(transition) = moves.strip_prefix("connectors.sessions.Session.") else {
                continue;
            };
            let outcome_name = outcome["name"].as_str().expect("a named outcome");
            found
                .entry(transition.to_string())
                .or_default()
                .insert((name.to_string(), outcome_name.to_string()));
        }
    }
    found
}

/// Every scenario file in the directory, as `(file stem, whole text)`.
fn scenario_files() -> Vec<(String, String)> {
    let directory = scenario_directory();
    let mut entries = std::fs::read_dir(&directory)
        .unwrap_or_else(|e| panic!("read {}: {e}", directory.display()))
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| {
            path.is_file()
                && matches!(
                    path.extension().and_then(|ext| ext.to_str()),
                    Some("yaml" | "yml")
                )
        })
        .collect::<Vec<_>>();
    entries.sort();
    entries
        .into_iter()
        .map(|path| {
            let stem = path
                .file_stem()
                .expect("a scenario file name")
                .to_string_lossy()
                .to_string();
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            (stem, text)
        })
        .collect()
}

/// The scenario files this story owns, by file stem. A file that names another owner is
/// a sibling story's and is not this document's to account for.
fn owned_scenarios() -> BTreeMap<String, String> {
    scenario_files()
        .into_iter()
        .filter(|(_, text)| text.contains(OWNER))
        .collect()
}

/// A `# <Field>: <value>` marker from a scenario file's own header comment.
fn marker(text: &str, field: &str) -> String {
    let prefix = format!("# {field}:");
    text.lines()
        .find(|line| line.starts_with(&prefix))
        .map(|line| line[prefix.len()..].trim().to_string())
        .unwrap_or_else(|| panic!("a scenario of this story carries no `{prefix}` marker"))
}

/// Every `<archived file>:<line>` citation in `text`, as written.
fn archive_citations(text: &str) -> Vec<String> {
    let pattern = regex::Regex::new(r"`(mcp-[0-9A-Za-z._+-]+\.(?:mdx|ts)):(\d+)(?:-(\d+))?`")
        .expect("a valid citation pattern");
    pattern
        .captures_iter(text)
        .map(|capture| capture[0].to_string())
        .collect()
}

/// Inbound transport dispositions of the selection matrix, as `key → disposition`.
/// A `both` row counts as inbound, which is what the matrix's own coverage rule says.
fn inbound_transport_dispositions() -> BTreeMap<String, String> {
    let matrix = read("adapters/mcp/contracts/protocol/v1alpha1/selection.md");
    let mut found = BTreeMap::new();
    for cells in table_rows(&matrix) {
        if cells.len() != 5 {
            continue;
        }
        let key = cells[0].trim_matches('`').to_string();
        if !key.starts_with("transport:") {
            continue;
        }
        let direction = key_of(&cells[1]);
        if direction != "inbound" && direction != "both" {
            continue;
        }
        found.insert(key, key_of(&cells[3]));
    }
    assert!(
        !found.is_empty(),
        "the selection matrix has no inbound transport rows; the derivation below is empty"
    );
    found
}

// ── The correspondence the acceptance names ────────────────────────────────────────

#[test]
fn every_behaviour_the_acceptance_names_has_an_outcome_a_transition_and_a_scenario() {
    let rows = behaviour_rows();
    let missing: Vec<&str> = BEHAVIOURS
        .iter()
        .filter(|behaviour| !rows.contains_key(**behaviour))
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "the enumeration is missing {} of the six behaviours the acceptance names: {}",
        missing.len(),
        missing.join(", ")
    );

    let scenario_link = regex::Regex::new(r"\(([^)]+\.ya?ml)\)").expect("a valid link pattern");
    let mut outcomes = BTreeSet::new();
    for behaviour in BEHAVIOURS {
        let row = &rows[*behaviour];
        let outcome = row[1].trim_matches('`').to_string();
        assert!(
            !outcome.is_empty() && outcome.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
            "`{behaviour}` names the observable outcome {:?}, which is not a scenario-shaped \
             name; the outcome is what the scenario file is named after",
            row[1]
        );
        assert!(
            outcomes.insert(outcome.clone()),
            "`{outcome}` is the observable outcome of two behaviours; the acceptance requires \
             six outcomes and six scenario files corresponding one to one"
        );
        parse_transition(&row[2]);

        let link = scenario_link.captures(&row[3]).unwrap_or_else(|| {
            panic!("`{behaviour}` links no scenario file: {:?}", row[3]);
        })[1]
            .to_string();
        assert_eq!(
            link,
            format!("scenarios/{outcome}.yaml"),
            "`{behaviour}` names the outcome `{outcome}` but links `{link}`; the scenario file \
             is named after the outcome"
        );
        let path = root()
            .join("adapters/mcp/contracts/server/v1alpha1")
            .join(&link);
        assert!(
            path.is_file(),
            "`{behaviour}` links {} , which does not exist",
            path.display()
        );
    }
}

#[test]
fn no_scenario_this_story_owns_names_an_outcome_the_document_does_not() {
    let rows = behaviour_rows();
    let declared: BTreeMap<String, String> = rows
        .iter()
        .map(|(behaviour, cells)| {
            (
                cells[1].trim_matches('`').to_string(),
                behaviour.to_string(),
            )
        })
        .collect();
    let owned = owned_scenarios();
    assert_eq!(
        owned.len(),
        BEHAVIOURS.len(),
        "this story owns {} scenario files and the acceptance requires {}: {:?}",
        owned.len(),
        BEHAVIOURS.len(),
        owned.keys().collect::<Vec<_>>()
    );
    for (stem, text) in &owned {
        let behaviour = declared.get(stem).unwrap_or_else(|| {
            panic!(
                "scenarios/{stem}.yaml names an outcome the document's enumeration does not \
                 carry; every scenario of this story is named after one of its six outcomes"
            )
        });
        assert_eq!(
            marker(text, "Outcome"),
            *stem,
            "scenarios/{stem}.yaml declares an `# Outcome:` marker that is not its own name"
        );
        assert_eq!(
            key_of(&marker(text, "Behaviour")),
            *behaviour,
            "scenarios/{stem}.yaml declares a behaviour the document files under `{behaviour}`"
        );
        let scenario_name = text
            .lines()
            .find_map(|line| line.strip_prefix("scenario:"))
            .unwrap_or_else(|| panic!("scenarios/{stem}.yaml declares no `scenario:` name"))
            .trim();
        assert_eq!(
            scenario_name, stem,
            "scenarios/{stem}.yaml compiles under a name that is not its file's"
        );
    }
}

#[test]
fn every_named_transition_is_one_the_sessions_lifecycle_declares() {
    let declared = declared_transitions();
    for (behaviour, cells) in behaviour_rows() {
        let (name, from, to) = parse_transition(&cells[2]);
        let (legal_from, legal_to) = declared.get(&name).unwrap_or_else(|| {
            panic!(
                "`{behaviour}` turns on `{name}`, which `connectors.sessions.Session` does not \
                 declare; its transitions are {:?}",
                declared.keys().collect::<Vec<_>>()
            )
        });
        assert!(
            legal_from.contains(&from),
            "`{behaviour}` names `{name}` out of `{from}`, and the lifecycle declares it only \
             out of {legal_from:?}"
        );
        assert_eq!(
            *legal_to, to,
            "`{behaviour}` names `{name}` into `{to}`, and the lifecycle declares it into \
             `{legal_to}`"
        );
    }
}

#[test]
fn each_scenario_performs_the_transition_its_behaviour_names() {
    let movers = movers();
    let owned = owned_scenarios();
    for (behaviour, cells) in behaviour_rows() {
        let outcome = cells[1].trim_matches('`').to_string();
        let (transition, _, _) = parse_transition(&cells[2]);
        let text = owned
            .get(&outcome)
            .unwrap_or_else(|| panic!("scenarios/{outcome}.yaml is not owned by {OWNER}"));
        let pairs = movers.get(&transition).unwrap_or_else(|| {
            panic!("no command outcome in `ess/domains/sessions.yaml` moves `{transition}`")
        });
        let trace: serde_yaml_ng::Value = serde_yaml_ng::from_str(text)
            .unwrap_or_else(|e| panic!("parse scenarios/{outcome}.yaml: {e}"));
        let acts = trace["timeline"]
            .as_sequence()
            .unwrap_or_else(|| panic!("scenarios/{outcome}.yaml declares no timeline"));
        let performed = acts.iter().any(|act| {
            let command = act["command"].as_str().unwrap_or_default();
            let act_outcome = act["outcome"].as_str().unwrap_or_default();
            pairs.contains(&(command.to_string(), act_outcome.to_string()))
        });
        assert!(
            performed,
            "scenarios/{outcome}.yaml is the trace for `{behaviour}`, whose transition is \
             `{transition}`, and it performs none of the command outcomes that move it: {pairs:?}"
        );
        assert_eq!(
            marker(text, "Transition"),
            format!("connectors.sessions.Session.{}", cells[2].replace('`', "")),
            "scenarios/{outcome}.yaml declares a `# Transition:` marker that is not the one the \
             document's enumeration names for `{behaviour}`"
        );
    }
}

#[test]
fn every_behaviour_states_what_the_selected_transport_does_not_offer() {
    let refusals = refusal_rows();
    for behaviour in BEHAVIOURS {
        let row = refusals.get(*behaviour).unwrap_or_else(|| {
            panic!(
                "`{behaviour}` is absent from the enumeration of what inbound stdio does not \
                 offer; the acceptance requires a behaviour the selected transport does not \
                 offer to appear there refused with its reason rather than absent"
            )
        });
        assert_eq!(
            row.len(),
            4,
            "the refusal row for `{behaviour}` has {} cells, not the four that enumeration \
             declares (behaviour, what is not offered, what a caller observes, source)",
            row.len()
        );
        for (index, part) in [
            "what inbound stdio does not offer",
            "what a caller observes instead",
        ]
        .iter()
        .enumerate()
        {
            assert!(
                row[index + 1].len() > 30,
                "the refusal row for `{behaviour}` states {part} in {:?}, which is too short to \
                 be a reason a reader can act on",
                row[index + 1]
            );
        }
        assert!(
            !archive_citations(&row[3]).is_empty(),
            "the refusal row for `{behaviour}` cites no archived specification line: {:?}",
            row[3]
        );
    }
}

// ── The two boundaries the story says this document may not cross ───────────────────

#[test]
fn the_document_specifies_the_transport_the_matrix_selected_and_selects_nothing() {
    let document = binding();
    let dispositions = inbound_transport_dispositions();
    let supported: Vec<&String> = dispositions
        .iter()
        .filter(|(_, disposition)| *disposition == "supported")
        .map(|(key, _)| key)
        .collect();
    assert_eq!(
        supported.len(),
        1,
        "the matrix disposes {} inbound transports as supported, and this document specifies \
         one binding: {supported:?}",
        supported.len()
    );
    assert!(
        document.contains(supported[0]),
        "the document never names `{}`, the inbound transport the matrix selected",
        supported[0]
    );
    for (key, disposition) in &dispositions {
        if disposition == "supported" {
            continue;
        }
        for (number, line) in document.lines().enumerate() {
            if !line.contains(key.as_str()) {
                continue;
            }
            let lowered = line.to_lowercase();
            assert!(
                REFUSAL_WORDS.iter().any(|word| lowered.contains(word)),
                "semantics.md line {} names `{key}`, which the matrix disposes `{disposition}`, \
                 without refusing or holding it: {line}",
                number + 1
            );
        }
    }
}

#[test]
fn the_single_principal_boundary_is_stated_and_no_scenario_crosses_it() {
    let document = binding();
    assert!(
        document.contains("decision-blocker:mcp-caller-connection-assignment"),
        "the document never names the blocker that owns the caller-to-connection assignment, \
         which is where it stops the moment there is more than one principal"
    );
    assert!(
        document.contains("UNMAPPED: `McpInboundSession → McpCaller`")
            || document.contains("UNMAPPED: McpInboundSession → McpCaller"),
        "the document does not carry the `UNMAPPED:` marker the domain model holds for \
         `McpInboundSession → McpCaller`"
    );
    let coordinates = ["instance_ref", "connection_ref", "authority_ref"];
    let mut seen: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for (stem, text) in owned_scenarios() {
        for coordinate in coordinates {
            for line in text.lines() {
                let trimmed = line.trim();
                if let Some(value) = trimmed.strip_prefix(&format!("{coordinate}:")) {
                    seen.entry(coordinate)
                        .or_default()
                        .insert(format!("{}(in {stem})", value.trim()));
                }
            }
        }
    }
    for coordinate in coordinates {
        let values = seen.get(coordinate).unwrap_or_else(|| {
            panic!("no scenario of this story states a `{coordinate}` binding coordinate")
        });
        let distinct: BTreeSet<&str> = values
            .iter()
            .map(|value| value.split("(in ").next().unwrap_or(value))
            .collect();
        assert_eq!(
            distinct.len(),
            1,
            "the scenarios name {} distinct `{coordinate}` values, {distinct:?}; the local \
             placement admits exactly one principal, and a second one makes the assignment \
             `decision-blocker:mcp-caller-connection-assignment` rather than this document's",
            distinct.len()
        );
    }
}

#[test]
fn the_binding_names_no_cloud_identity_or_network_dependency_it_does_not_refuse() {
    let document = binding();
    let mut offending = Vec::new();
    for (number, line) in document.lines().enumerate() {
        let lowered = line.to_lowercase();
        for dependency in OUTSIDE_DEPENDENCIES {
            if !lowered.contains(dependency) {
                continue;
            }
            if REFUSAL_WORDS.iter().any(|word| lowered.contains(word)) {
                continue;
            }
            offending.push(format!("semantics.md:{}: {line}", number + 1));
        }
    }
    assert!(
        offending.is_empty(),
        "the acceptance requires the whole binding to be described without reference to a cloud \
         control plane, identity service or network service this repository does not already \
         have; these lines name one without refusing it:\n{}",
        offending.join("\n")
    );
}

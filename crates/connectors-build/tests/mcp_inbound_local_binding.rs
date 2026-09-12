//! Cases for `story:mcp-inbound-local-binding`, written 2026-09-12 on branch
//! `unit/mcp-inbound-local-binding-20260912`.
//!
//! The story's acceptance is a correspondence, not a prose quality: the inbound binding
//! document must carry **six** protocol behaviours, each with a named observable
//! outcome, the state transition it turns on in the `connectors.sessions.Session`
//! vocabulary, and a scenario file named after it. Six behaviours, and one row per route
//! where a behaviour has two, each row complete and each with its own trace (see
//! `behaviour_of`). Nothing in the repository checks a correspondence like
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

/// The behaviour a table cell is filed under: one of the acceptance's six, allowing a
/// route qualifier after it. A behaviour whose prose describes two routes — a per-request
/// refusal and a connection-level denial — carries one row per route, and each row is
/// complete in its own right. Answering adversary pass 1, finding 5: one row per
/// behaviour collapsed two routes into one and left a reader implementing from the table
/// denying readiness where the same document's prose forbids it.
fn behaviour_of(cell: &str) -> Option<&'static str> {
    let key = key_of(cell);
    BEHAVIOURS
        .iter()
        .find(|behaviour| key == **behaviour || key.starts_with(&format!("{behaviour} (")))
        .copied()
}

/// The behaviour rows of the document's enumeration, keyed by behaviour, in table order.
/// Columns: behaviour, observable outcome, session transition, scenario.
fn behaviour_rows() -> BTreeMap<String, Vec<Vec<String>>> {
    let document = binding();
    let enumeration = section(&document, "the six behaviours");
    let mut rows: BTreeMap<String, Vec<Vec<String>>> = BTreeMap::new();
    for (index, cells) in table_rows(&enumeration).into_iter().enumerate() {
        // The header row is the one row that names no behaviour. Every other row must:
        // skipping a row this function does not recognise is how a typo'd behaviour name
        // becomes an unenumerated route.
        if index == 0 {
            continue;
        }
        let key = behaviour_of(&cells[0]).unwrap_or_else(|| {
            panic!(
                "the enumeration carries a row for {:?}, which is none of the behaviours the \
                 acceptance names: {BEHAVIOURS:?}",
                cells[0]
            )
        });
        assert_eq!(
            cells.len(),
            4,
            "the behaviour row for `{key}` has {} cells, not the four the enumeration \
             declares (behaviour, observable outcome, session transition, scenario)",
            cells.len()
        );
        rows.entry(key.to_string()).or_default().push(cells);
    }
    rows
}

/// Every `### 3.x <behaviour> — …` section of the enumeration, as
/// `(behaviour, section text)`. The prose of a behaviour is where a route the table does
/// not carry can hide.
fn behaviour_sections(document: &str) -> Vec<(String, String)> {
    let mut sections: Vec<(String, String)> = Vec::new();
    let mut current: Option<String> = None;
    for line in document.lines() {
        if let Some(heading) = line.strip_prefix("### ") {
            let title = heading
                .split('—')
                .next()
                .unwrap_or_default()
                .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ' ');
            current = behaviour_of(title).map(|behaviour| {
                sections.push((behaviour.to_string(), String::new()));
                behaviour.to_string()
            });
            continue;
        }
        if line.starts_with("## ") {
            current = None;
            continue;
        }
        if current.is_some()
            && let Some(last) = sections.last_mut()
        {
            last.1.push_str(line);
            last.1.push('\n');
        }
    }
    assert_eq!(
        sections.len(),
        BEHAVIOURS.len(),
        "the document carries {} `### <behaviour>` sections and the acceptance names {}: {:?}",
        sections.len(),
        BEHAVIOURS.len(),
        sections
            .iter()
            .map(|(behaviour, _)| behaviour)
            .collect::<Vec<_>>()
    );
    sections
}

/// The refusal rows of `## … does not offer`, keyed by the same six behaviours.
fn refusal_rows() -> BTreeMap<String, Vec<String>> {
    let document = binding();
    let refusals = section(&document, "does not offer");
    let mut rows = BTreeMap::new();
    for (index, cells) in table_rows(&refusals).into_iter().enumerate() {
        if index == 0 {
            continue;
        }
        let key = behaviour_of(&cells[0]).unwrap_or_else(|| {
            panic!(
                "the enumeration of what inbound stdio does not offer carries a row for {:?}, \
                 which is none of the behaviours the acceptance names: {BEHAVIOURS:?}",
                cells[0]
            )
        });
        rows.insert(key.to_string(), cells);
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

/// The scenario files this story owns, by file stem. Ownership is decided by the
/// structured `# Owner:` marker each file carries, never by searching for the story id
/// anywhere in the text: this document tells two sibling stories to add files to the same
/// directory, and the natural header comment of such a file cites this story — which a
/// substring test would silently adopt, and the count below would then fail in the
/// sibling's tree for the sibling's reason. Answering adversary pass 1, finding 6.
fn owned_scenarios() -> BTreeMap<String, String> {
    scenario_files()
        .into_iter()
        .filter(|(stem, text)| {
            let marked = marker_value(text, "Owner");
            // A file that cites this story but carries no marker is the adoption hazard
            // the marker replaced, read from the other side: it would silently drop out
            // of every rule below instead of silently joining them.
            assert!(
                marked.is_some() || !text.contains(OWNER),
                "scenarios/{stem}.yaml names {OWNER} but carries no `# Owner:` marker, so no \
                 rule in this file can tell whether it is this story's deliverable or a \
                 sibling's file citing it"
            );
            marked.is_some_and(|owner| owner.split_whitespace().next() == Some(OWNER))
        })
        .collect()
}

/// A `# <Field>: <value>` marker from a scenario file's own header comment, if present.
fn marker_value(text: &str, field: &str) -> Option<String> {
    let prefix = format!("# {field}:");
    text.lines()
        .find(|line| line.starts_with(&prefix))
        .map(|line| line[prefix.len()..].trim().to_string())
}

/// The same marker, required.
fn marker(text: &str, field: &str) -> String {
    marker_value(text, field)
        .unwrap_or_else(|| panic!("a scenario of this story carries no `# {field}:` marker"))
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
        for row in &rows[*behaviour] {
            let outcome = row[1].trim_matches('`').to_string();
            assert!(
                !outcome.is_empty() && outcome.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
                "`{behaviour}` names the observable outcome {:?}, which is not a scenario-shaped \
                 name; the outcome is what the scenario file is named after",
                row[1]
            );
            assert!(
                outcomes.insert(outcome.clone()),
                "`{outcome}` is the observable outcome of two rows; every route of every \
                 behaviour names its own outcome and its own scenario file"
            );
            parse_transition(&row[2]);

            let link = scenario_link.captures(&row[3]).unwrap_or_else(|| {
                panic!("`{behaviour}` links no scenario file: {:?}", row[3]);
            })[1]
                .to_string();
            assert_eq!(
                link,
                format!("scenarios/{outcome}.yaml"),
                "`{behaviour}` names the outcome `{outcome}` but links `{link}`; the scenario \
                 file is named after the outcome"
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
}

#[test]
fn no_scenario_this_story_owns_names_an_outcome_the_document_does_not() {
    let rows = behaviour_rows();
    let declared: BTreeMap<String, String> = rows
        .iter()
        .flat_map(|(behaviour, rows)| {
            rows.iter().map(move |cells| {
                (
                    cells[1].trim_matches('`').to_string(),
                    behaviour.to_string(),
                )
            })
        })
        .collect();
    let owned = owned_scenarios();
    assert_eq!(
        owned.len(),
        declared.len(),
        "this story owns {} scenario files and its enumeration carries {} rows, each of which \
         names one: {:?}. What that number has to be is not settled here — it is settled by \
         the acceptance, in `the_directory_ships_what_the_story_acceptance_states`",
        owned.len(),
        declared.len(),
        owned.keys().collect::<Vec<_>>()
    );
    for (stem, text) in &owned {
        let behaviour = declared.get(stem).unwrap_or_else(|| {
            panic!(
                "scenarios/{stem}.yaml names an outcome the document's enumeration does not \
                 carry; every scenario of this story is named after an outcome one of its rows names"
            )
        });
        assert_eq!(
            marker(text, "Outcome"),
            *stem,
            "scenarios/{stem}.yaml declares an `# Outcome:` marker that is not its own name"
        );
        let declared_behaviour = marker(text, "Behaviour");
        assert_eq!(
            behaviour_of(&declared_behaviour),
            Some(behaviour.as_str()),
            "scenarios/{stem}.yaml declares the behaviour {declared_behaviour:?}, and the \
             document files its outcome under `{behaviour}`"
        );
        assert_eq!(
            key_of(&declared_behaviour),
            key_of(
                &rows[behaviour.as_str()]
                    .iter()
                    .find(|cells| cells[1].trim_matches('`') == stem.as_str())
                    .expect("the row that names this outcome")[0]
            ),
            "scenarios/{stem}.yaml declares a route the enumeration does not name in the row \
             that links it; a behaviour with two routes carries one row per route and each \
             trace says which route it is"
        );
        let trace: serde_yaml_ng::Value = serde_yaml_ng::from_str(text)
            .unwrap_or_else(|e| panic!("parse scenarios/{stem}.yaml: {e}"));
        let scenario_name = trace["scenario"]
            .as_str()
            .unwrap_or_else(|| panic!("scenarios/{stem}.yaml declares no `scenario:` name"));
        assert_eq!(
            scenario_name, stem,
            "scenarios/{stem}.yaml compiles under a name that is not its file's"
        );
    }
}

#[test]
fn every_named_transition_is_one_the_sessions_lifecycle_declares() {
    let declared = declared_transitions();
    for (behaviour, cells) in behaviour_rows().into_iter().flat_map(|(behaviour, rows)| {
        rows.into_iter()
            .map(move |cells| (behaviour.clone(), cells))
    }) {
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
    for (behaviour, cells) in behaviour_rows().into_iter().flat_map(|(behaviour, rows)| {
        rows.into_iter()
            .map(move |cells| (behaviour.clone(), cells))
    }) {
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
    // Read out of the parsed trace, not off line prefixes: a flow-style mapping
    // (`binding: {instance_ref: a, …}`) writes the same fact on one line and evades a
    // prefix scan, and a coordinate the model adds later would be covered by neither.
    // The field set itself comes from `connectors.sessions.Binding`, so a new coordinate
    // joins this rule the moment the model declares it.
    let coordinates = declared_binding_fields();
    let mut seen: BTreeMap<String, BTreeMap<String, BTreeSet<String>>> = BTreeMap::new();
    for (stem, text) in owned_scenarios() {
        let trace: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text)
            .unwrap_or_else(|e| panic!("parse scenarios/{stem}.yaml: {e}"));
        for value in bindings_in(&trace) {
            let mapping = value.as_mapping().unwrap_or_else(|| {
                panic!(
                    "scenarios/{stem}.yaml states a binding that is not a \
                                           mapping of its coordinates"
                )
            });
            let stated: BTreeSet<String> = mapping
                .keys()
                .filter_map(|key| key.as_str().map(str::to_string))
                .collect();
            assert_eq!(
                stated, coordinates,
                "scenarios/{stem}.yaml states a `connectors.sessions.Binding` whose coordinates \
                 are not the ones the model declares; an unstated one is a principal coordinate \
                 this rule would not see"
            );
            for coordinate in &coordinates {
                let stated = mapping[coordinate.as_str()]
                    .as_str()
                    .unwrap_or_else(|| {
                        panic!("scenarios/{stem}.yaml states a non-string `{coordinate}`")
                    })
                    .to_string();
                seen.entry(coordinate.clone())
                    .or_default()
                    .entry(stated)
                    .or_default()
                    .insert(stem.clone());
            }
        }
    }
    for coordinate in &coordinates {
        let values = seen.get(coordinate).unwrap_or_else(|| {
            panic!("no scenario of this story states a `{coordinate}` binding coordinate")
        });
        assert_eq!(
            values.len(),
            1,
            "the scenarios name {} distinct `{coordinate}` values, {values:?}; the local \
             placement admits exactly one principal, and a second one makes the assignment \
             `decision-blocker:mcp-caller-connection-assignment` rather than this document's",
            values.len()
        );
    }
}

/// The coordinates `connectors.sessions.Binding` declares, read off the model.
fn declared_binding_fields() -> BTreeSet<String> {
    let document: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&read("ess/domains/sessions.yaml")).expect("parse sessions.yaml");
    let values = document["types"]
        .as_sequence()
        .expect("sessions.yaml declares types");
    let binding = values
        .iter()
        .find(|value| value["name"].as_str() == Some("connectors.sessions.Binding"))
        .expect("sessions.yaml declares connectors.sessions.Binding");
    binding["fields"]
        .as_sequence()
        .expect("connectors.sessions.Binding declares fields")
        .iter()
        .map(|field| {
            field["name"]
                .as_str()
                .expect("a named binding field")
                .to_string()
        })
        .collect()
}

/// Every `binding:` value anywhere in a parsed trace, at any depth.
fn bindings_in(value: &serde_yaml_ng::Value) -> Vec<serde_yaml_ng::Value> {
    let mut found = Vec::new();
    match value {
        serde_yaml_ng::Value::Mapping(mapping) => {
            for (key, child) in mapping {
                if key.as_str() == Some("binding") {
                    found.push(child.clone());
                }
                found.extend(bindings_in(child));
            }
        }
        serde_yaml_ng::Value::Sequence(items) => {
            for item in items {
                found.extend(bindings_in(item));
            }
        }
        _ => {}
    }
    found
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

// ── The timing obligations the traces inherit with the vocabulary ───────────────────
//
// Added 2026-09-12 answering adversary pass 1. The class the findings are instances of:
// these traces were authored against the session vocabulary's *lifecycle* and never
// against its *timing obligations*. The cases above read `ess/domains/sessions.yaml` for
// which transitions exist and which command outcomes perform them; §4.1 of the normative
// owner that same model names was read by nothing, and ESS cannot catch the difference
// because it "compiles obligations but does NOT execute a sequential trace". Every bound
// below is read out of the two files that state it, never written here: a ceiling this
// file carried itself would be a ceiling a correction could move by editing this file.

/// The file `ess/domains/sessions.yaml` names as the normative owner of its timing rules,
/// read out of its own comment rather than pinned here.
fn sessions_contract() -> String {
    let model = read("ess/domains/sessions.yaml");
    let path = regex::Regex::new(r"#\s*Normative owner:\s*([A-Za-z0-9_./-]+\.md)")
        .expect("a valid pattern")
        .captures(&model)
        .unwrap_or_else(|| {
            panic!(
                "ess/domains/sessions.yaml no longer names the normative owner of the timing \
                 obligations these traces inherit; the cases below cannot be derived without it"
            )
        })[1]
        .to_string();
    read(&path)
}

/// Milliseconds captured by `pattern` in the sessions contract's §4.1.
fn contract_ceiling_ms(pattern: &str, what: &str) -> i64 {
    let contract = sessions_contract();
    regex::Regex::new(pattern)
        .expect("a valid pattern")
        .captures(&contract)
        .unwrap_or_else(|| {
            panic!("the sessions contract §4.1 no longer states the ceiling on {what}")
        })[1]
        .replace(',', "")
        .parse::<i64>()
        .expect("a whole number of milliseconds")
}

/// The maximum lifetime of a live data lease. Stated by the contract and repeated by the
/// model, and the two are required to agree: a fix that edits one source into agreement
/// with a trace is caught by the other.
fn lease_ceiling_ms() -> i64 {
    let in_contract = contract_ceiling_ms(
        r"effective deadline of a data lease is at most \*\*([0-9,]+)\s*ms",
        "the effective deadline of a data lease",
    );
    let model = read("ess/domains/sessions.yaml");
    let in_model = regex::Regex::new(r"lease lifetime <=\s*([0-9,]+)\s*ms")
        .expect("a valid pattern")
        .captures(&model)
        .expect("ess/domains/sessions.yaml states the live data-lease ceiling")[1]
        .replace(',', "")
        .parse::<i64>()
        .expect("a whole number of milliseconds");
    assert_eq!(
        in_model, in_contract,
        "ess/domains/sessions.yaml bounds a live data lease at {in_model} ms and its own \
         normative owner bounds it at {in_contract} ms; a trace cannot be held to a ceiling \
         the two sources disagree about"
    );
    in_contract
}

fn cutoff_ceiling_ms() -> i64 {
    contract_ceiling_ms(
        r"\*\*([0-9,]+)\s*ms maximum from authoritative revocation to data cutoff\*\*",
        "revocation-to-cutoff",
    )
}

fn teardown_ceiling_ms() -> i64 {
    contract_ceiling_ms(
        r"\*\*([0-9,]+)\s*ms maximum from an accepted terminal fact to completion of local teardown",
        "terminal-to-teardown",
    )
}

/// Days since 1970-01-01 for a proleptic Gregorian date (Howard Hinnant's `days_from_civil`).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let shifted = (month + 9) % 12;
    let day_of_year = (153 * shifted + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// A scenario timestamp as whole seconds since the epoch. The traces write
/// `YYYY-MM-DDTHH:MM:SSZ` and say in their own headers that whole-second acts express
/// causal order, so a sub-second form is rejected rather than silently truncated.
fn instant(text: &str) -> i64 {
    let bytes = text.as_bytes();
    assert!(
        text.len() == 20
            && bytes[4] == b'-'
            && bytes[7] == b'-'
            && bytes[10] == b'T'
            && bytes[13] == b':'
            && bytes[16] == b':'
            && bytes[19] == b'Z',
        "a scenario timestamp is not of the form `YYYY-MM-DDTHH:MM:SSZ`: {text:?}"
    );
    let field = |from: usize, to: usize| {
        text[from..to]
            .parse::<i64>()
            .unwrap_or_else(|_| panic!("a scenario timestamp is not numeric: {text:?}"))
    };
    days_from_civil(field(0, 4), field(5, 7), field(8, 10)) * 86_400
        + field(11, 13) * 3_600
        + field(14, 16) * 60
        + field(17, 19)
}

/// One act of a trace's timeline, in the terms the timing rules are written in.
struct Act {
    index: usize,
    at: i64,
    command: String,
    outcome: String,
    input: serde_yaml_ng::Value,
}

fn timeline(stem: &str, text: &str) -> Vec<Act> {
    let trace: serde_yaml_ng::Value = serde_yaml_ng::from_str(text)
        .unwrap_or_else(|e| panic!("parse scenarios/{stem}.yaml: {e}"));
    trace["timeline"]
        .as_sequence()
        .unwrap_or_else(|| panic!("scenarios/{stem}.yaml declares no timeline"))
        .iter()
        .enumerate()
        .map(|(index, act)| Act {
            index,
            at: instant(
                act["at"]
                    .as_str()
                    .unwrap_or_else(|| panic!("scenarios/{stem}.yaml act {index} has no `at:`")),
            ),
            // Neither defaults to the empty string: an act with no command or no outcome
            // matches no rule below, so a silent default reads as a clean act to every
            // timing case in this file.
            command: act["command"]
                .as_str()
                .unwrap_or_else(|| {
                    panic!("scenarios/{stem}.yaml act {index} declares no `command:`")
                })
                .to_string(),
            outcome: act["outcome"]
                .as_str()
                .unwrap_or_else(|| {
                    panic!("scenarios/{stem}.yaml act {index} declares no `outcome:`")
                })
                .to_string(),
            input: act["input"].clone(),
        })
        .collect()
}

/// `(issued_at, effective_expiry, sequence)` of an act's lease input, if it carries one.
fn lease_of(input: &serde_yaml_ng::Value) -> Option<(i64, i64, i64)> {
    let lease = input.get("lease")?;
    Some((
        instant(lease.get("issued_at")?.as_str()?),
        instant(lease.get("effective_expiry")?.as_str()?),
        lease.get("sequence")?.as_i64()?,
    ))
}

/// The 1-based line of the first line containing `needle`, so a failure opens at the act
/// it is about.
fn line_of(text: &str, needle: &str) -> usize {
    text.lines()
        .position(|line| line.contains(needle))
        .map(|index| index + 1)
        .unwrap_or(0)
}

#[test]
fn no_trace_issues_a_lease_longer_than_the_contract_ceiling() {
    let ceiling = lease_ceiling_ms();
    let mut offending = Vec::new();
    for (stem, text) in owned_scenarios() {
        for act in timeline(&stem, &text) {
            let Some((issued, expiry, _)) = lease_of(&act.input) else {
                continue;
            };
            let lifetime = (expiry - issued) * 1_000;
            let expiry_text = act.input["lease"]["effective_expiry"]
                .as_str()
                .unwrap_or_default();
            let at_text = act.input["lease"]["issued_at"].as_str().unwrap_or_default();
            if lifetime > ceiling {
                offending.push(format!(
                    "scenarios/{stem}.yaml:{} — act {} `{}` issues a lease of {lifetime} ms, \
                     and the ceiling is {ceiling} ms",
                    line_of(&text, &format!("effective_expiry: '{expiry_text}'")),
                    act.index,
                    act.command,
                ));
            }
            if issued != act.at {
                offending.push(format!(
                    "scenarios/{stem}.yaml:{} — act {} `{}` issues a lease dated {at_text}, and \
                     the act that issues it is the authoritative issuance the ceiling runs from",
                    line_of(&text, &format!("issued_at: '{at_text}'")),
                    act.index,
                    act.command,
                ));
            }
            // A lease that expires at or before its own issuance admits nothing and would
            // pass the ceiling above, being at most as long as it allows.
            if expiry <= issued {
                offending.push(format!(
                    "scenarios/{stem}.yaml:{} — act {} `{}` issues a lease whose effective \
                     expiry is not after its issuance, so it admits no data path at all",
                    line_of(&text, &format!("effective_expiry: '{expiry_text}'")),
                    act.index,
                    act.command,
                ));
            }
        }
    }
    assert!(
        offending.is_empty(),
        "a trace issues a data lease the sessions contract §4.1 does not permit — it caps the \
         effective deadline at {ceiling} ms after authoritative issuance, including delivery \
         delay, clock uncertainty, scheduling delay and already-buffered output, and requires a \
         binding that cannot enforce that to refuse its own session admission:\n{}",
        offending.join("\n")
    );
}

#[test]
fn no_trace_admits_data_or_renews_after_its_live_lease_deadline() {
    let ceiling = lease_ceiling_ms() / 1_000;
    let mut offending = Vec::new();
    for (stem, text) in owned_scenarios() {
        let mut deadline: Option<i64> = None;
        let mut sequence: Option<i64> = None;
        let mut terminal: Option<(usize, String)> = None;
        for act in timeline(&stem, &text) {
            let admitted = matches!(act.outcome.as_str(), "permitted" | "renewed");
            let at_text = format!(
                "at: '{}'",
                serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&text)
                    .ok()
                    .and_then(|trace| trace["timeline"][act.index]["at"]
                        .as_str()
                        .map(String::from))
                    .unwrap_or_default()
            );
            // `>=`, not `>`. The effective deadline is defined *including* delivery delay
            // and already-buffered output, so it is the moment by which admitted output
            // has already left, and the expiry is itself a terminal fact that atomically
            // enters `closing`. An admission placed on it delivers past it by
            // construction. This document says the same thing twice — the supervisor
            // renews *before* the effective deadline — and this is the guard that holds
            // the traces to it.
            if admitted
                && let Some(live) = deadline
                && act.at >= live
            {
                offending.push(format!(
                    "scenarios/{stem}.yaml:{} — act {} `{}` is admitted ({}) {} ms after the \
                     last moment the lease that admits it can legally be live, and the last \
                     admissible moment is strictly before the effective expiry, not on it",
                    line_of(&text, &at_text),
                    act.index,
                    act.command,
                    act.outcome,
                    (act.at - live) * 1_000,
                ));
            }
            // The hole pass 2 named: every bound below is reached through `deadline`, so a
            // trace that never issued a lease at all used to pass every timing case here.
            // §4.1 makes a live data lease mandatory for every admitted data path.
            if admitted && deadline.is_none() {
                offending.push(format!(
                    "scenarios/{stem}.yaml:{} — act {} `{}` is admitted ({}) with no data lease \
                     issued anywhere before it, and §4.1 makes a live lease mandatory for every \
                     admitted data path",
                    line_of(&text, &at_text),
                    act.index,
                    act.command,
                    act.outcome,
                ));
            }
            if admitted && let Some((index, fact)) = &terminal {
                offending.push(format!(
                    "scenarios/{stem}.yaml:{} — act {} `{}` is admitted ({}) after act {index} \
                     `{fact}` recorded a terminal, and §4.1 refuses data admission and renewals \
                     from the moment the host accepts one",
                    line_of(&text, &at_text),
                    act.index,
                    act.command,
                    act.outcome,
                ));
            }
            if let Some((issued, expiry, next)) = lease_of(&act.input)
                && matches!(act.outcome.as_str(), "ready" | "renewed")
            {
                if let Some(previous) = sequence
                    && next <= previous
                {
                    offending.push(format!(
                        "scenarios/{stem}.yaml:{} — act {} `{}` renews at sequence {next}, which \
                         does not advance on {previous}; §4.1 rejects replayed authority",
                        line_of(&text, &at_text),
                        act.index,
                        act.command,
                    ));
                }
                sequence = Some(next);
                deadline = Some(std::cmp::min(expiry, issued + ceiling));
            }
            if matches!(
                act.outcome.as_str(),
                "closing" | "authority-terminated" | "lost"
            ) && terminal.is_none()
            {
                terminal = Some((act.index, act.command.clone()));
            }
        }
    }
    assert!(
        offending.is_empty(),
        "a trace admits data, or renews, past the deadline of the live data lease that admits \
         it. §4.1 makes a live lease mandatory for every admitted data path, caps it at \
         {} ms from issuance and allows no grace past the effective expiry, so a request that \
         outlives one lease has to be renewed by the supervisor before that deadline or end \
         `lease_expired`. Shortening a trace hides this; renewing it is what the contract \
         describes:\n{}",
        lease_ceiling_ms(),
        offending.join("\n")
    );
}

#[test]
fn every_close_deadline_a_trace_records_is_bounded_by_the_contract_and_the_live_lease() {
    let lease_ceiling = lease_ceiling_ms() / 1_000;
    let cutoff_ceiling = cutoff_ceiling_ms() / 1_000;
    let teardown_ceiling = teardown_ceiling_ms() / 1_000;
    let mut offending = Vec::new();
    for (stem, text) in owned_scenarios() {
        let mut deadline: Option<i64> = None;
        let mut terminal_at: Option<i64> = None;
        let mut teardown_due: Option<i64> = None;
        for act in timeline(&stem, &text) {
            if act.outcome == "closing" {
                let cutoff = act.input["cutoff_due_at"].as_str().unwrap_or_else(|| {
                    panic!("scenarios/{stem}.yaml act {} has no cutoff", act.index)
                });
                let teardown = act.input["teardown_due_at"].as_str().unwrap_or_else(|| {
                    panic!("scenarios/{stem}.yaml act {} has no teardown", act.index)
                });
                let bound = match deadline {
                    Some(live) => std::cmp::min(live, act.at + cutoff_ceiling),
                    None => act.at + cutoff_ceiling,
                };
                if instant(cutoff) > bound {
                    offending.push(format!(
                        "scenarios/{stem}.yaml:{} — act {} records a data cutoff {} ms later \
                         than the deadline that dominates it ({} ms after the terminal, and the \
                         live lease's own deadline dominates a later one)",
                        line_of(&text, &format!("cutoff_due_at: '{cutoff}'")),
                        act.index,
                        (instant(cutoff) - bound) * 1_000,
                        cutoff_ceiling * 1_000,
                    ));
                }
                if instant(teardown) > act.at + teardown_ceiling {
                    offending.push(format!(
                        "scenarios/{stem}.yaml:{} — act {} records teardown {} ms after the \
                         terminal fact, and §4.1 requires it within {} ms",
                        line_of(&text, &format!("teardown_due_at: '{teardown}'")),
                        act.index,
                        (instant(teardown) - act.at) * 1_000,
                        teardown_ceiling * 1_000,
                    ));
                }
                if instant(teardown) < instant(cutoff) {
                    offending.push(format!(
                        "scenarios/{stem}.yaml:{} — act {} records teardown before its own data \
                         cutoff",
                        line_of(&text, &format!("teardown_due_at: '{teardown}'")),
                        act.index,
                    ));
                }
                teardown_due = Some(instant(teardown));
            }
            if matches!(
                act.outcome.as_str(),
                "closing" | "authority-terminated" | "lost"
            ) && terminal_at.is_none()
            {
                terminal_at = Some(act.at);
            }
            if act.command.ends_with("FinishTeardown") {
                let t0 = terminal_at.unwrap_or_else(|| {
                    panic!(
                        "scenarios/{stem}.yaml finishes teardown at act {} with no terminal fact \
                         before it",
                        act.index
                    )
                });
                if act.at > t0 + teardown_ceiling {
                    offending.push(format!(
                        "scenarios/{stem}.yaml — act {} finishes teardown {} ms after the \
                         terminal fact, and §4.1 requires it within {} ms",
                        act.index,
                        (act.at - t0) * 1_000,
                        teardown_ceiling * 1_000,
                    ));
                }
                if let Some(due) = teardown_due
                    && act.at > due
                {
                    offending.push(format!(
                        "scenarios/{stem}.yaml — act {} finishes teardown {} ms after the \
                         deadline the close itself recorded",
                        act.index,
                        (act.at - due) * 1_000,
                    ));
                }
            }
            if let Some((issued, expiry, _)) = lease_of(&act.input)
                && matches!(act.outcome.as_str(), "ready" | "renewed")
            {
                deadline = Some(std::cmp::min(expiry, issued + lease_ceiling));
            }
        }
    }
    assert!(
        offending.is_empty(),
        "a trace records a close deadline the sessions contract §4.1 and its model do not \
         permit; the model says those deadlines are bounded by §4.1 and that earlier \
         expiry/drain deadlines dominate, and §4.1 forbids any additional grace:\n{}",
        offending.join("\n")
    );
}

/// The sentences of a document, flattened so a sentence survives the author's hard wrap.
/// Sentence ends are `. ` — the citations in this document are `file.md:12` and `§4.1`,
/// neither of which carries a period followed by a space.
fn sentences(document: &str) -> Vec<String> {
    flattened(document)
        .split(". ")
        .map(|sentence| sentence.trim().to_string())
        .filter(|sentence| !sentence.is_empty())
        .collect()
}

#[test]
fn the_document_states_the_lease_obligation_its_traces_inherit() {
    let document = binding();
    let sentences = sentences(&document);
    let spelled = |ms: i64| format!("{},{:03} ms", ms / 1_000, ms % 1_000);
    let lease = spelled(lease_ceiling_ms());
    let teardown = spelled(teardown_ceiling_ms());

    // Each of these is a sentence the document has to carry, not a literal it has to
    // contain somewhere. Pass 2, finding 5: the lease ceiling and the cutoff ceiling are
    // the same number two lines apart, so `contains("2,000 ms")` went on passing with §2's
    // lease sentence deleted. A statement is a subject and a bound in one sentence.
    let required: [(&str, Vec<String>); 5] = [
        (
            "the ceiling on a live data lease, with what it bounds",
            vec!["data lease".into(), lease.clone(), "issuance".into()],
        ),
        (
            "the section of the sessions contract that owns these obligations",
            vec![
                "contracts/sessions/v1alpha1/semantics.md".into(),
                "§4.1".into(),
            ],
        ),
        (
            "the renewal that keeps a request alive past one lease, and that it is the \
             supervisor's",
            vec!["RenewDataLease".into(), "before".into()],
        ),
        (
            "what happens when a lease is not renewed in time",
            vec!["lease_expired".into()],
        ),
        (
            "the ceiling on local teardown after a terminal fact",
            vec!["teardown".into(), teardown.clone()],
        ),
    ];
    let mut missing = Vec::new();
    for (what, needles) in &required {
        if !sentences
            .iter()
            .any(|sentence| needles.iter().all(|needle| sentence.contains(needle)))
        {
            missing.push(format!("{what} — no sentence carries all of {needles:?}"));
        }
    }
    assert!(
        missing.is_empty(),
        "the document does not state, in its own sentences, the timing obligations its traces \
         are held to; a reader implementing from the document alone would not be told them, and \
         every one of these is a bound the cases above enforce on the traces:\n{}",
        missing.join("\n")
    );
}

#[test]
fn every_transition_a_behaviour_section_asserts_is_bound_to_one_of_its_traces() {
    let document = binding();
    let declared = declared_transitions();
    let rows = behaviour_rows();
    let negation = regex::Regex::new(r"(?i)\b(not|never|no|cannot|instead of|rather than)\b")
        .expect("a valid pattern");
    let mut offending = Vec::new();
    for (behaviour, section_text) in behaviour_sections(&document) {
        let named: Vec<(String, String)> = section_text
            .lines()
            .flat_map(|line| {
                declared
                    .keys()
                    .filter(|name| line.contains(&format!("`{name}`")))
                    .map(move |name| (name.clone(), line.to_string()))
            })
            .collect();
        let bound: BTreeSet<String> = rows
            .get(&behaviour)
            .map(|rows| {
                rows.iter()
                    .map(|cells| parse_transition(&cells[2]).0)
                    .collect()
            })
            .unwrap_or_default();
        for (name, line) in named {
            // Row membership, not row *or* trace. A transition a trace happens to perform
            // is not a transition the table tells a reader to implement, and the table is
            // what a reader implements from: answering adversary pass 2, finding 3, where
            // `renew` was asserted in prose, performed by one trace, and named by no row.
            if bound.contains(&name) {
                continue;
            }
            let before = line.split(&format!("`{name}`")).next().unwrap_or_default();
            let tail: String = before
                .chars()
                .rev()
                .take(40)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            if negation.is_match(&tail) {
                continue;
            }
            offending.push(format!(
                "`{behaviour}` asserts `{name}` in its prose and no row of the enumeration \
                 names it, so the table sends a reader down a route without it: {}",
                line.trim()
            ));
        }
    }
    // Every row's transition must also be performed by that row's own trace, which is
    // `each_scenario_performs_the_transition_its_behaviour_names`; the two together are
    // what make a row an obligation rather than a label.
    assert!(
        offending.is_empty(),
        "a behaviour section states that the binding performs a transition that the \
         enumeration binds to no trace, so a reader implementing from the table takes a route \
         the prose describes and has nothing that fails when the implementation takes the \
         other one:\n{}",
        offending.join("\n")
    );
}

// ── The deliverable against the statement that governs it ───────────────────────────
//
// Added 2026-09-12 answering adversary pass 2, finding 2. The scenario count used to be
// pinned at six here and, when the routes were split, became a comparison between the
// directory and the row count of the document the same unit wrote — both halves this
// unit's, with nothing comparing either to the acceptance. What the deliverable has to
// contain is the story's to say, so it is read from the story.

/// The `## Acceptance` section of the story that owns this document, without its heading.
/// Read-only: the planning store is the CLI's to write.
fn acceptance_statement() -> String {
    let story = read(".engineering/planning/story/mcp-inbound-local-binding.md");
    let mut collecting = false;
    let mut found = String::new();
    for line in story.lines() {
        if line.starts_with("## ") {
            if collecting {
                break;
            }
            collecting = line.trim() == "## Acceptance";
            continue;
        }
        if collecting {
            found.push_str(line);
            found.push('\n');
        }
    }
    assert!(
        !found.trim().is_empty(),
        "{OWNER} has no `## Acceptance` section, and the deliverable below is measured against it"
    );
    found
}

/// Any document as one line, so a sentence survives whatever column the author wrapped at.
fn flattened(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn number_word(word: &str) -> Option<usize> {
    Some(match word {
        "one" => 1,
        "two" => 2,
        "three" => 3,
        "four" => 4,
        "five" => 5,
        "six" => 6,
        "seven" => 7,
        "eight" => 8,
        "nine" => 9,
        "ten" => 10,
        "eleven" => 11,
        "twelve" => 12,
        _ => return None,
    })
}

/// What an acceptance statement says the scenario directory has to contain.
#[derive(Debug, PartialEq, Eq)]
enum Deliverable {
    /// "… six scenario files …" — a fixed count, whatever the document's table does.
    Files(usize),
    /// "… one scenario per observable route …" — as many as the enumeration has routes,
    /// which the row-to-file correspondence above then pins exactly.
    OnePerRoute,
}

/// Read the rule out of an acceptance statement. A literal count wins where one is
/// stated, because a statement that counts files is measured by counting files.
fn deliverable_rule(acceptance: &str) -> Deliverable {
    let flat = flattened(acceptance);
    let counted = regex::Regex::new(r"([a-z]+) scenario files")
        .expect("a valid pattern")
        .captures_iter(&flat)
        .find_map(|capture| number_word(&capture[1]));
    let per_route =
        regex::Regex::new(r"(?i)one scenario (file )?per (observable )?(route|outcome)")
            .expect("a valid pattern")
            .is_match(&flat);
    match (counted, per_route) {
        (Some(files), false) => Deliverable::Files(files),
        (None, true) => Deliverable::OnePerRoute,
        // Preferring one of the two here would resolve the contradiction silently, and a
        // statement amended in one sentence and left standing in another is how this
        // unit's deliverable and its acceptance came apart in the first place.
        (Some(files), true) => panic!(
            "{OWNER}'s acceptance states both a fixed count of {files} scenario files and a rule \
             of one scenario per observable route; they cannot both govern the directory, and \
             this case will not pick one:\n{flat}"
        ),
        (None, false) => panic!(
            "{OWNER}'s acceptance states neither a number of scenario files nor a rule for how \
             many there are, so nothing outside this unit says what the directory has to \
             contain:\n{flat}"
        ),
    }
}

/// How many behaviours the acceptance enumerates, so the list this file carries is
/// checked against the statement rather than trusted.
fn stated_behaviour_count(acceptance: &str) -> usize {
    let flat = flattened(acceptance);
    regex::Regex::new(r"those ([a-z]+) behaviours|([a-z]+) behaviours")
        .expect("a valid pattern")
        .captures_iter(&flat)
        .find_map(|capture| {
            capture
                .get(1)
                .or_else(|| capture.get(2))
                .and_then(|word| number_word(word.as_str()))
        })
        .unwrap_or_else(|| {
            panic!(
                "{OWNER}'s acceptance no longer states how many behaviours it enumerates, and \
                 the list of behaviour names in this file is checked against that number"
            )
        })
}

#[test]
fn the_directory_ships_what_the_story_acceptance_states() {
    let acceptance = acceptance_statement();
    let stated = stated_behaviour_count(&acceptance);
    assert_eq!(
        BEHAVIOURS.len(),
        stated,
        "{OWNER}'s acceptance enumerates {stated} behaviours and this file carries {}: {:?}",
        BEHAVIOURS.len(),
        BEHAVIOURS
    );

    let owned = owned_scenarios();
    let rows = behaviour_rows();
    let route_count: usize = rows.values().map(Vec::len).sum();
    match deliverable_rule(&acceptance) {
        Deliverable::Files(expected) => assert_eq!(
            owned.len(),
            expected,
            "{OWNER}'s acceptance says this story ships {expected} scenario files and the \
             directory carries {}: {:?}. Either the deliverable or the statement is wrong, and \
             the statement is the planning store's to amend, not this unit's",
            owned.len(),
            owned.keys().collect::<Vec<_>>()
        ),
        Deliverable::OnePerRoute => {
            assert_eq!(
                owned.len(),
                route_count,
                "{OWNER}'s acceptance says one scenario per observable route, the enumeration \
                 carries {route_count} routes and the directory carries {} files: {:?}",
                owned.len(),
                owned.keys().collect::<Vec<_>>()
            );
            for behaviour in BEHAVIOURS {
                assert!(
                    rows.get(*behaviour).is_some_and(|rows| !rows.is_empty()),
                    "`{behaviour}` is one of the behaviours the acceptance enumerates and the \
                     document gives it no route at all"
                );
            }
        }
    }
}

#[test]
fn the_acceptance_rule_is_read_from_the_statement_in_either_form_it_can_take() {
    assert_eq!(
        deliverable_rule(
            "such that every one of those six behaviours has a named observable outcome … so the\n\
             six behaviours and their six scenario files correspond one to one."
        ),
        Deliverable::Files(6),
        "a statement that counts scenario files has to be measured by counting them"
    );
    assert_eq!(
        deliverable_rule(
            "such that every one of those six behaviours has a named observable outcome, and one\n\
             scenario per observable route under `…/scenarios/` named after it."
        ),
        Deliverable::OnePerRoute,
        "a statement that states a rule per route has to be measured by the routes"
    );
    assert_eq!(
        stated_behaviour_count("… every one of those six behaviours has a named outcome …"),
        6
    );
}

#[test]
#[should_panic(expected = "cannot both govern")]
fn an_acceptance_that_states_both_a_count_and_a_rule_is_refused_rather_than_resolved() {
    deliverable_rule(
        "… every one of those six behaviours has a named observable outcome, and one scenario \
         per observable route named after it … Those two plus the six scenario files above are \
         the inbound half of the epic's acceptance criterion 5.",
    );
}

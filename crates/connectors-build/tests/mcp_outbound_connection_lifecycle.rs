//! The coverage property for `story:mcp-outbound-connection-lifecycle`, written
//! 2026-09-12 on branch `unit/mcp-outbound-connection-lifecycle-20260912`.
//!
//! The story's acceptance is countable in two directions at once: every lifecycle state
//! it names has **a named observable outcome and a scenario file**, and **none of them is
//! described as "the client retries"**. So this file derives the required states from the
//! story's own text rather than from the document under test — a list hard-coded here
//! would be the document written twice, and a state the acceptance names but nobody
//! noticed would be absent from both.
//!
//! `adapters/mcp/contracts/client/v1alpha1/semantics.md` carries the machine-readable
//! surface: a register table whose rows are `state | outcome | wire code | scenario`.
//! Every row must resolve to a scenario file under `scenarios/`, every scenario file this
//! story owns must resolve back to a row, and every refusal must name a variant of
//! `connectors.service_wire.ErrorCode` **read out of the shared model**, not spelled here.
//!
//! Citations are checked the way `mcp_profile_selection_matrix.rs` checks the matrix's:
//! a `<archived file>:<line>` citation must name a file the pin's manifest records and a
//! line that file actually has. Archives are read with `gzip -dc`, the command the
//! pinned-source record's own reproduction snippet uses.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The scenario files this story owns. A sibling story writing into the same directory
/// (`story:mcp-outbound-auth-lifecycle`, `story:mcp-outbound-invocation-results`) carries
/// its own id and is skipped by every case below rather than failed.
const STORY: &str = "story:mcp-outbound-connection-lifecycle";
const SCENARIO_TYPE: &str = "mcp-outbound-lifecycle/1";

/// The states the acceptance names, each bound to a phrase of the story that names it.
/// The probe is checked against the story's own text, so a renamed or dropped state fails
/// here rather than drifting silently out of the register.
const REQUIRED_STATES: [(&str, &str); 12] = [
    ("state:discovery", "how a server is discovered"),
    ("state:explicit-selection", "explicitly selected"),
    (
        "state:initialize-negotiation",
        "what `initialize` negotiates",
    ),
    (
        "state:version-mismatch",
        "what a protocol version or capability mismatch refuses",
    ),
    ("state:capability-mismatch", "capability mismatch refuses"),
    ("state:framing", "how framing"),
    ("state:streaming", "streaming, progress"),
    ("state:progress", "progress, cancellation"),
    ("state:cancellation", "cancellation, session loss"),
    ("state:session-loss", "session loss and shutdown behave"),
    ("state:shutdown", "shutdown behave"),
    // Not from the acceptance sentence but from the same story, which requires the
    // distinction in as many words: an outcome mapping onto neither `unavailable` nor
    // `session_lost`, or onto both, "is a defect this document must resolve".
    (
        "state:transport-unreachable",
        "Loss is a distinct outcome from failure",
    ),
];

/// The two refusals the story requires to stay apart, and the shared-model variant each
/// must name.
const LOSS_ROW: (&str, &str) = ("state:session-loss", "session_lost");
const FAILURE_ROW: (&str, &str) = ("state:transport-unreachable", "unavailable");

/// Words whose appearance anywhere outside the document's own quarantine section would
/// make some state's answer "send it again". The acceptance forbids exactly this.
///
/// Matched on word boundaries, not as substrings: `resent` is inside `presented`, and a
/// substring rule would refuse an ordinary English word while proving nothing.
const SENDING_IT_AGAIN: &str = r"(?i)\b(retry|retries|retried|retrying|re-?sends?|re-?sent)\b";

const QUARANTINE_HEADING: &str = "## Why no outcome above sends the request a second time";
const HELD_HEADING: &str = "## Outbound stdio is held, not specified here";

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn client_dir() -> PathBuf {
    repository_root().join("adapters/mcp/contracts/client/v1alpha1")
}

fn evidence_dir() -> PathBuf {
    repository_root().join("adapters/mcp/contracts/protocol/v1alpha1/evidence/20260912")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn semantics() -> String {
    read(&client_dir().join("semantics.md"))
}

fn story() -> String {
    read(
        &repository_root().join(".engineering/planning/story/mcp-outbound-connection-lifecycle.md"),
    )
}

/// The story with every run of whitespace collapsed, because its body is hard-wrapped and
/// a probe phrase straddles the wrap.
fn story_text() -> String {
    story().split_whitespace().collect::<Vec<_>>().join(" ")
}

fn manifest_files() -> BTreeSet<String> {
    let path = evidence_dir().join("specification-source-hashes.json");
    let value: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display())),
    )
    .expect("specification-source-hashes.json is not JSON");
    value
        .as_array()
        .expect("specification-source-hashes.json is not a JSON array")
        .iter()
        .map(|entry| {
            entry["file"]
                .as_str()
                .expect("manifest entry has no string `file`")
                .to_string()
        })
        .collect()
}

/// Line count of one archived source, from its uncompressed bytes.
fn archived_lines(file: &str) -> usize {
    let archive = evidence_dir().join("vendor").join(format!("{file}.gz"));
    let output = Command::new("gzip")
        .arg("-dc")
        .arg(&archive)
        .output()
        .unwrap_or_else(|e| panic!("run gzip -dc {}: {e}", archive.display()));
    assert!(
        output.status.success(),
        "gzip -dc {} exited {}",
        archive.display(),
        output.status
    );
    String::from_utf8(output.stdout)
        .expect("archived source is not UTF-8")
        .lines()
        .count()
}

// ── The register, as rows ───────────────────────────────────────────────────────────

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
        if !state.starts_with("state:") {
            continue;
        }
        assert_eq!(
            cells.len(),
            5,
            "semantics.md:{} is a register row with {} cells, not the five the register \
             declares (state, outcome, wire code, whose act, scenario): {trimmed}",
            index + 1,
            cells.len()
        );
        let scenario = link
            .captures(&cells[4])
            .unwrap_or_else(|| {
                panic!(
                    "semantics.md:{} names no `scenarios/<file>.yaml` link: {trimmed}",
                    index + 1
                )
            })
            .get(1)
            .expect("captured scenario file")
            .as_str()
            .to_string();
        parsed.push(Row {
            state,
            outcome: cells[1].trim_matches('`').to_string(),
            wire_code: cells[2].trim_matches('`').to_string(),
            actor: cells[3].trim_matches('`').to_string(),
            scenario,
            line: index + 1,
        });
    }
    assert!(
        !parsed.is_empty(),
        "semantics.md carries no register row this check can read"
    );
    parsed
}

fn scenario_paths() -> Vec<PathBuf> {
    let directory = client_dir().join("scenarios");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&directory)
        .unwrap_or_else(|e| panic!("read {}: {e}", directory.display()))
        .map(|entry| entry.expect("scenario directory entry").path())
        .filter(|path| {
            path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("yaml")
        })
        .collect();
    files.sort();
    files
}

/// Every scenario file in the shared directory that carries **this** story's id.
fn owned_scenarios() -> Vec<(PathBuf, serde_yaml_ng::Value)> {
    let mut owned = Vec::new();
    for path in scenario_paths() {
        let text = read(&path);
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text)
            .unwrap_or_else(|e| panic!("{} is not YAML: {e}", path.display()));
        let story = value["story"].as_str().unwrap_or_else(|| {
            panic!(
                "{} declares no `story:` owner, so no story can be held to it",
                path.display()
            )
        });
        if story == STORY {
            owned.push((path, value));
        }
    }
    owned
}

fn field(value: &serde_yaml_ng::Value, key: &str, path: &Path) -> String {
    value[key]
        .as_str()
        .unwrap_or_else(|| panic!("{} has no string `{key}`", path.display()))
        .to_string()
}

/// Every `` `mcp-…:line` `` citation in a text, as `(file, line)`.
fn citations(text: &str) -> Vec<(String, usize)> {
    let citation =
        regex::Regex::new(r"(mcp-[0-9A-Za-z._+-]+\.(?:mdx|ts)):(\d+)(?:-(\d+))?").unwrap();
    let mut found = Vec::new();
    for capture in citation.captures_iter(text) {
        let file = capture[1].to_string();
        found.push((file.clone(), capture[2].parse().unwrap()));
        if let Some(end) = capture.get(3) {
            found.push((file, end.as_str().parse().unwrap()));
        }
    }
    found
}

/// A `##` section including its own heading line, which is part of the section for every
/// rule below: a heading that names what the section quarantines would otherwise be
/// refused by the rule the section exists to satisfy.
fn section(document: &str, heading: &str) -> String {
    let start = document
        .find(heading)
        .unwrap_or_else(|| panic!("semantics.md carries no section `{heading}`"));
    let rest = &document[start..];
    let end = rest[heading.len()..]
        .find("\n## ")
        .map(|offset| offset + heading.len())
        .unwrap_or(rest.len());
    rest[..end].to_string()
}

// ── Cases ───────────────────────────────────────────────────────────────────────────

/// The acceptance, stated as a count: every state the story names has a named observable
/// outcome and a scenario file.
///
/// The state list is not this file's opinion. Each entry carries a phrase of the story
/// that names it, and the phrase is asserted against the story's own text first — so a
/// state that stops being required, or is renamed upstream, fails here rather than
/// quietly passing on a list nobody re-read.
#[test]
fn every_state_the_story_names_has_a_named_outcome_and_a_scenario_file() {
    let text = story_text();
    assert!(
        text.contains("states has a named observable outcome and a scenario file"),
        "the story no longer states the property this case checks"
    );
    let mut unnamed = Vec::new();
    for (state, probe) in REQUIRED_STATES {
        if !text.contains(probe) {
            unnamed.push(format!(
                "{state} probes {probe:?}, which the story does not say"
            ));
        }
    }
    assert!(
        unnamed.is_empty(),
        "this case's state list has drifted from the story it claims to read:\n  {}",
        unnamed.join("\n  ")
    );

    let rows = rows();
    let document = semantics();
    let scenarios_dir = client_dir().join("scenarios");
    let mut broken = Vec::new();
    for (state, _) in REQUIRED_STATES {
        let covering: Vec<&Row> = rows.iter().filter(|row| row.state == state).collect();
        if covering.is_empty() {
            broken.push(format!("{state} has no row in the register"));
            continue;
        }
        for row in covering {
            let heading = format!("### `{}`", row.outcome);
            match document.find(&heading) {
                None => broken.push(format!(
                    "semantics.md:{} names outcome `{}`, which no `{heading}` section defines",
                    row.line, row.outcome
                )),
                Some(start) => {
                    let rest = &document[start + heading.len()..];
                    let end = rest.find("\n### ").unwrap_or(rest.len());
                    let words = rest[..end].split_whitespace().count();
                    if words < 40 {
                        broken.push(format!(
                            "semantics.md:{} outcome `{}` is defined in {words} words, which \
                             states no observable outcome",
                            row.line, row.outcome
                        ));
                    }
                }
            }
            let path = scenarios_dir.join(&row.scenario);
            if !path.is_file() {
                broken.push(format!(
                    "semantics.md:{} `{}` names scenario `{}`, which does not exist",
                    row.line, row.state, row.scenario
                ));
            }
        }
    }
    assert!(
        broken.is_empty(),
        "{} of the story's states are not carried by the register:\n  {}",
        broken.len(),
        broken.join("\n  ")
    );

    let outcomes: BTreeSet<&str> = rows.iter().map(|row| row.outcome.as_str()).collect();
    assert_eq!(
        outcomes.len(),
        rows.len(),
        "two register rows share an outcome name, so an outcome does not identify a state"
    );
    let files: BTreeSet<&str> = rows.iter().map(|row| row.scenario.as_str()).collect();
    assert_eq!(
        files.len(),
        rows.len(),
        "two register rows share a scenario file, so a scenario does not decide one outcome"
    );
}

/// The other direction: a scenario file this story owns names an outcome the document
/// declares. An orphan scenario is a behaviour specified only in a trace.
#[test]
fn every_scenario_this_story_owns_names_a_row_of_the_register() {
    let rows = rows();
    let by_file: BTreeMap<&str, &Row> = rows
        .iter()
        .map(|row| (row.scenario.as_str(), row))
        .collect();
    let owned = owned_scenarios();
    assert!(
        !owned.is_empty(),
        "no scenario file in {} carries `story: {STORY}`",
        client_dir().join("scenarios").display()
    );

    let mut broken = Vec::new();
    for (path, value) in &owned {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("scenario file name");
        let Some(row) = by_file.get(name) else {
            broken.push(format!(
                "{name} is owned by this story and named by no register row"
            ));
            continue;
        };
        for (key, expected) in [
            ("type", SCENARIO_TYPE),
            ("state", row.state.as_str()),
            ("outcome", row.outcome.as_str()),
            ("wire_code", row.wire_code.as_str()),
            (
                "document",
                "adapters/mcp/contracts/client/v1alpha1/semantics.md",
            ),
        ] {
            let actual = field(value, key, path);
            if actual != expected {
                broken.push(format!(
                    "{name} declares {key}: {actual:?}, the register row says {expected:?}"
                ));
            }
        }
        for key in ["summary", "given", "when", "then", "never"] {
            let present = match &value[key] {
                serde_yaml_ng::Value::String(text) => !text.trim().is_empty(),
                serde_yaml_ng::Value::Sequence(items) => !items.is_empty(),
                _ => false,
            };
            if !present {
                broken.push(format!("{name} carries no `{key}`"));
            }
        }
    }
    assert!(
        broken.is_empty(),
        "{} scenario files disagree with the register:\n  {}",
        broken.len(),
        broken.join("\n  ")
    );
    assert_eq!(
        owned.len(),
        rows.len(),
        "the register has {} rows and this story owns {} scenario files",
        rows.len(),
        owned.len()
    );
}

/// "None of them is described as 'the client retries'."
///
/// The phrase itself must appear nowhere, and no word that means *send it again* may
/// appear in a scenario at all or in the document outside the one section that exists to
/// quote the upstream permission and decline it.
#[test]
fn no_state_is_answered_by_sending_the_request_again() {
    let document = semantics();
    assert!(
        !document.to_lowercase().contains("the client retries"),
        "semantics.md describes a state as \"the client retries\""
    );
    let quarantine = section(&document, QUARANTINE_HEADING);
    let again = regex::Regex::new(SENDING_IT_AGAIN).unwrap();

    let mut broken = Vec::new();
    for (index, line) in document.lines().enumerate() {
        for found in again.find_iter(line) {
            if quarantine.contains(line) {
                continue;
            }
            broken.push(format!(
                "semantics.md:{} uses `{}` outside `{QUARANTINE_HEADING}`: {}",
                index + 1,
                found.as_str(),
                line.trim()
            ));
        }
    }
    for (path, _) in owned_scenarios() {
        let text = read(&path);
        assert!(
            !text.to_lowercase().contains("the client retries"),
            "{} describes its outcome as \"the client retries\"",
            path.display()
        );
        for found in again.find_iter(&text) {
            broken.push(format!(
                "{} uses `{}`; a scenario carries an outcome, and no outcome is to send it \
                 again",
                path.display(),
                found.as_str()
            ));
        }
    }
    assert!(
        broken.is_empty(),
        "{} places answer a state by sending the request again:\n  {}",
        broken.len(),
        broken.join("\n  ")
    );
}

/// Loss is a distinct outcome from failure, and every refusal names a code the shared
/// model actually declares.
///
/// The variant list is read out of `ess/domains/service_wire.yaml` rather than spelled
/// here, so a register naming a plausible code that no model declares fails, and so does
/// one that silently follows a renamed variant.
#[test]
fn every_refusal_names_a_declared_wire_code_and_loss_is_not_failure() {
    let model = read(&repository_root().join("ess/domains/service_wire.yaml"));
    let declaration = model
        .lines()
        .skip_while(|line| !line.contains("connectors.service_wire.ErrorCode"))
        .find(|line| line.trim_start().starts_with("variants:"))
        .expect("ess/domains/service_wire.yaml declares no ErrorCode variants");
    let variants: BTreeSet<&str> = declaration
        .trim()
        .trim_start_matches("variants:")
        .trim()
        .trim_matches(|c| c == '[' || c == ']')
        .split(',')
        .map(str::trim)
        .filter(|variant| !variant.is_empty())
        .collect();
    assert!(
        variants.contains("unavailable") && variants.contains("session_lost"),
        "the shared model no longer separates `unavailable` from `session_lost`: {variants:?}"
    );

    let rows = rows();
    let mut broken = Vec::new();
    for row in &rows {
        if row.wire_code != "n/a" && !variants.contains(row.wire_code.as_str()) {
            broken.push(format!(
                "semantics.md:{} `{}` names wire code `{}`, which `connectors.service_wire.\
                 ErrorCode` does not declare",
                row.line, row.outcome, row.wire_code
            ));
        }
    }
    for (state, expected) in [LOSS_ROW, FAILURE_ROW] {
        let codes: BTreeSet<&str> = rows
            .iter()
            .filter(|row| row.state == state)
            .map(|row| row.wire_code.as_str())
            .collect();
        if !codes.contains(expected) {
            broken.push(format!(
                "{state} carries {codes:?}, not the `{expected}` the story requires"
            ));
        }
    }
    let loss: BTreeSet<&str> = rows
        .iter()
        .filter(|row| row.state == LOSS_ROW.0)
        .map(|row| row.wire_code.as_str())
        .collect();
    let failure: BTreeSet<&str> = rows
        .iter()
        .filter(|row| row.state == FAILURE_ROW.0)
        .map(|row| row.wire_code.as_str())
        .collect();
    if !loss.is_disjoint(&failure) {
        broken.push(format!(
            "session loss carries {loss:?} and transport failure {failure:?}; the story \
             requires an outcome that maps onto neither both nor the same"
        ));
    }
    assert!(
        broken.is_empty(),
        "{} refusals disagree with the shared error vocabulary:\n  {}",
        broken.len(),
        broken.join("\n  ")
    );
}

/// Every archived-source citation in the document and in every owned scenario resolves:
/// the file is one the pin's manifest records, and the line is one that file has.
#[test]
fn every_citation_resolves_into_an_archived_file_of_the_pin() {
    let archived = manifest_files();
    let mut lengths: BTreeMap<String, usize> = BTreeMap::new();
    let mut sources = vec![("semantics.md".to_string(), semantics())];
    for (path, _) in owned_scenarios() {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("scenario file name")
            .to_string();
        sources.push((name, read(&path)));
    }

    let mut broken = Vec::new();
    let mut checked = 0;
    for (name, text) in &sources {
        for (file, line) in citations(text) {
            checked += 1;
            if !archived.contains(&file) {
                broken.push(format!(
                    "{name} cites `{file}:{line}`, which the pin's manifest does not record"
                ));
                continue;
            }
            let length = *lengths
                .entry(file.clone())
                .or_insert_with(|| archived_lines(&file));
            if line == 0 || line > length {
                broken.push(format!(
                    "{name} cites `{file}:{line}`, and that file has {length} lines"
                ));
            }
        }
    }
    assert!(
        broken.is_empty(),
        "{} citations do not resolve:\n  {}",
        broken.len(),
        broken.join("\n  ")
    );
    assert!(
        checked >= REQUIRED_STATES.len(),
        "the document and its scenarios carry {checked} archived citations for {} states, \
         which cannot be a cited specification of each",
        REQUIRED_STATES.len()
    );
}

/// The document does not close a set the model leaves open.
///
/// `connectors_mcp.state.McpServerBinding.selected_revision` is a `String` and
/// `McpCapabilitySnapshot.supported_versions` a `List<String>` precisely so that a version
/// a server may report stays representable. Both carriers are read out of the model here,
/// so a document that narrowed either — or a later edit that narrowed the model under a
/// document that relies on it — fails.
#[test]
fn the_document_does_not_close_the_revision_set_the_model_leaves_open() {
    let model = read(&repository_root().join("adapters/mcp/spec/ess/domains/state.yaml"));
    let carrier = |field: &str| -> String {
        let pattern = regex::Regex::new(&format!(
            r"\{{name: {field}, type: \x22?([A-Za-z<>_.]+)\x22?\}}"
        ))
        .unwrap();
        pattern
            .captures(&model)
            .unwrap_or_else(|| panic!("the MCP model declares no carrier `{field}`"))
            .get(1)
            .expect("captured carrier type")
            .as_str()
            .to_string()
    };
    assert_eq!(
        carrier("selected_revision"),
        "Optional<String>",
        "the model's selected revision is no longer an open string"
    );
    assert_eq!(
        carrier("supported_versions"),
        "List<String>",
        "the model's reported version list is no longer an open string list"
    );

    let document = semantics();
    let mut broken = Vec::new();
    for reportable in ["2025-06-18", "2024-11-05"] {
        if !model.contains(reportable) {
            broken.push(format!(
                "the model no longer names `{reportable}` as a reportable version, so this \
                 case's pair has drifted from it"
            ));
        }
        if !document.contains(reportable) {
            broken.push(format!(
                "semantics.md names no outcome for a server reporting `{reportable}`, a \
                 version the model keeps representable and the matrix refuses"
            ));
        }
    }
    assert!(
        broken.is_empty(),
        "{} statements close a set the model leaves open:\n  {}",
        broken.len(),
        broken.join("\n  ")
    );
}

/// Outbound stdio is held by an open decision-blocker, so this document specifies none of
/// it — and nothing else is held by that blocker either.
///
/// The first pass of this case exempted every line that named the blocker, which let a
/// line naming it say anything about any transport: the document called the remainder of
/// a **four**-family space "the other outbound transport family" and attributed it here,
/// while the matrix refuses two of those families under no blocker at all. So the
/// exemption now requires the line to name stdio in its own words rather than inside the
/// blocker's id, and a collective reference to "the other" transport is refused outright,
/// because the family a disposition is stated for has to be the one that was named.
#[test]
fn outbound_stdio_is_held_by_its_blocker_and_nothing_else_is() {
    let document = semantics();
    let blocker = "decision-blocker:mcp-outbound-stdio-process-ownership";
    assert!(
        document.contains(blocker),
        "semantics.md specifies an outbound transport without naming the blocker that \
         holds the other one"
    );
    let families = outbound_transport_families();
    assert!(
        families.len() > 2,
        "the selection matrix names {} outbound transport families, so \"the other\" one \
         would be an unambiguous reference: {families:?}",
        families.len()
    );

    let held = section(&document, HELD_HEADING);
    // The held section, in line numbers, so a paragraph can be placed inside or outside
    // it without matching wrapped text against wrapped text.
    let held_from = document
        .lines()
        .position(|line| line.starts_with(HELD_HEADING))
        .expect("semantics.md carries no held section")
        + 1;
    let held_to = held_from + held.lines().count();
    let collective = regex::Regex::new(r"(?i)the other (outbound |inbound )?transport").unwrap();
    let mut broken = Vec::new();
    // Read by paragraph, not by line: the claim a disposition belongs to is a sentence,
    // and this document is hard-wrapped, so the transport a paragraph names and the
    // blocker it attributes it to land on different lines about half the time.
    for (line, text) in paragraphs(&document) {
        let held_here = (held_from..held_to).contains(&line);
        // The blocker's own id contains the word, so a paragraph has only named the
        // transport if it names it outside that id.
        let outside_the_id = text.replace(blocker, "").to_lowercase();
        if collective.is_match(&text) {
            broken.push(format!(
                "semantics.md:{line} calls a {}-family transport space \"the other\" one: \
                 {text:?}",
                families.len()
            ));
        }
        if text.contains(blocker) && !outside_the_id.contains("stdio") {
            broken.push(format!(
                "semantics.md:{line} attributes `{blocker}` to a transport it does not \
                 name, and that blocker holds outbound stdio alone: {text:?}"
            ));
        }
        if outside_the_id.contains("stdio") && !held_here && !text.contains(blocker) {
            broken.push(format!(
                "semantics.md:{line} states something about stdio outside \
                 `{HELD_HEADING}` and without naming the blocker: {text:?}"
            ));
        }
    }
    assert!(
        broken.is_empty(),
        "{} lines answer a held question, or hold a question that was not asked:\n  {}",
        broken.len(),
        broken.join("\n  ")
    );
    assert!(
        held.contains("`transport:stdio`") && held.contains("deferred"),
        "the held section does not name the selection matrix's own disposition of \
         outbound stdio"
    );
}

/// A `file:line` citation into a document this repository can edit drifts, and this epic
/// has spent findings on exactly that. Line numbers are cited only into the archived
/// bytes, whose lines a digest pins.
#[test]
fn no_citation_into_an_editable_document_carries_a_line_number() {
    let pattern = regex::Regex::new(r"[A-Za-z0-9._/-]+\.(?:md|yaml|yml|rs|json|toml):\d+").unwrap();
    let mut sources = vec![("semantics.md".to_string(), semantics())];
    for (path, _) in owned_scenarios() {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("scenario file name")
            .to_string();
        sources.push((name, read(&path)));
    }
    let mut broken = Vec::new();
    for (name, text) in &sources {
        for found in pattern.find_iter(text) {
            broken.push(format!(
                "{name} cites `{}` by line; only the digest-pinned archives may be cited \
                 that way",
                found.as_str()
            ));
        }
    }
    assert!(
        broken.is_empty(),
        "{} citations will drift when their target is edited:\n  {}",
        broken.len(),
        broken.join("\n  ")
    );
}

// ── Round 1: the classes behind the adversary's findings ────────────────────────────
//
// Each case below answers a *class* an adversary pass found one instance of. The
// instance is named in the case's own documentation; the rule is what the case asserts.

fn selection_matrix() -> String {
    read(&repository_root().join("adapters/mcp/contracts/protocol/v1alpha1/selection.md"))
}

fn mcp_model() -> String {
    read(&repository_root().join("adapters/mcp/spec/ess/domains/state.yaml"))
}

/// Every run of whitespace collapsed, because these documents are hard-wrapped and a
/// phrase straddles the wrap.
fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The text with every archived-source citation removed.
///
/// An archived file is named `mcp-<revision>-…`, so a passage that cites one contains a
/// revision string without having said anything about which revision it holds for. Every
/// rule below that asks a passage to name a revision asks it of the prose.
fn without_citations(text: &str) -> String {
    regex::Regex::new(r"mcp-[0-9A-Za-z._+-]+\.(?:mdx|ts)(?::\d+(?:-\d+)?)?")
        .unwrap()
        .replace_all(text, " ")
        .to_string()
}

/// Paragraphs of a document as `(first line number, text with the wrap collapsed)`.
fn paragraphs(document: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    let mut start = 0usize;
    let mut buffer: Vec<&str> = Vec::new();
    for (index, line) in document.lines().enumerate() {
        if line.trim().is_empty() {
            if !buffer.is_empty() {
                found.push((start + 1, flat(&buffer.join(" "))));
                buffer.clear();
            }
            continue;
        }
        if buffer.is_empty() {
            start = index;
        }
        buffer.push(line);
    }
    if !buffer.is_empty() {
        found.push((start + 1, flat(&buffer.join(" "))));
    }
    found
}

/// The revisions the selection matrix dispositions `supported` in the outbound direction,
/// read out of the matrix. Two of them disagreeing is the whole reason for the rule below.
fn supported_outbound_revisions() -> BTreeSet<String> {
    let row = regex::Regex::new(
        r"\|\s*`revision:([0-9A-Za-z.-]+)`\s*\|\s*(?:outbound|both)\s*\|[^|]*\|\s*supported\s*\|",
    )
    .unwrap();
    row.captures_iter(&selection_matrix())
        .map(|capture| capture[1].to_string())
        .collect()
}

/// The outbound transport families the matrix names, read the same way.
fn outbound_transport_families() -> BTreeSet<String> {
    let row =
        regex::Regex::new(r"\|\s*`(transport:[a-z-]+)`\s*\|\s*(?:outbound|both)\s*\|").unwrap();
    row.captures_iter(&selection_matrix())
        .map(|capture| capture[1].to_string())
        .collect()
}

/// The name the document gives each revision, declared once in its own preamble. Reading
/// the aliases out of the document rather than writing them here means a section may
/// resolve the question in the document's own words, and a renamed revision fails.
fn revision_aliases() -> BTreeMap<String, String> {
    let document = flat(&semantics());
    let declaration =
        regex::Regex::new(r"`revision:([0-9A-Za-z.-]+)` \*\*(the [a-z ]+ revision)\*\*").unwrap();
    declaration
        .captures_iter(&document)
        .map(|capture| (capture[1].to_string(), capture[2].to_string()))
        .collect()
}

/// The fields `connectors_mcp.state.McpServerBinding` declares, read out of the model.
fn binding_fields() -> Vec<String> {
    let model = mcp_model();
    let start = model
        .find("- name: connectors_mcp.state.McpServerBinding")
        .expect("the MCP model declares no `McpServerBinding`");
    let rest = &model[start..];
    let end = rest
        .find("\n    lifecycle:")
        .expect("`McpServerBinding` declares no lifecycle, so its field block has no end");
    let field = regex::Regex::new(r"\{name: ([a-z_]+), type:").unwrap();
    field
        .captures_iter(&rest[..end])
        .map(|capture| capture[1].to_string())
        .collect()
}

/// The bullets of one scenario field, flattened. A field may be a string or a block
/// sequence, and a bullet is the unit a claim is made in.
fn scenario_bullets(value: &serde_yaml_ng::Value, key: &str) -> Vec<String> {
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

/// The revisions whose archives a text cites, read off the archived file names: an
/// archive is `mcp-<revision>-…`, so a citation is a claim about that revision.
fn revisions_cited(text: &str) -> BTreeSet<String> {
    regex::Regex::new(r"mcp-(\d{4}-\d{2}-\d{2})-")
        .unwrap()
        .captures_iter(text)
        .map(|capture| capture[1].to_string())
        .collect()
}

/// One revision's archived bytes, concatenated: every file of the pin's manifest whose
/// name carries that revision.
fn archives_of(revision: &str) -> String {
    let mut joined = String::new();
    for file in manifest_files() {
        if !file.contains(revision) {
            continue;
        }
        let archive = evidence_dir().join("vendor").join(format!("{file}.gz"));
        let output = Command::new("gzip")
            .arg("-dc")
            .arg(&archive)
            .output()
            .unwrap_or_else(|e| panic!("run gzip -dc {}: {e}", archive.display()));
        assert!(
            output.status.success(),
            "gzip -dc {} exited {}",
            archive.display(),
            output.status
        );
        joined.push_str(&String::from_utf8_lossy(&output.stdout));
    }
    assert!(
        !joined.is_empty(),
        "no archived file of the pin carries `{revision}`, so a term cannot be attributed \
         to it"
    );
    joined
}

/// The code spans the scenarios use that occur in exactly one supported revision's
/// archives, as `span -> revision`.
///
/// Nothing here is a list of protocol terms: the spans come from the scenarios and the
/// attribution from the archived bytes, so a term that stops being exclusive — or a
/// scenario that introduces a new one — is classified on the next run rather than on the
/// next time somebody remembers this file exists.
fn revision_exclusive_spans(supported: &BTreeSet<String>) -> BTreeMap<String, String> {
    let span = regex::Regex::new(r"`([^`]+)`").unwrap();
    let mut spans: BTreeSet<String> = BTreeSet::new();
    for (_, value) in owned_scenarios() {
        for key in ["summary", "given", "when", "then", "never"] {
            for bullet in scenario_bullets(&value, key) {
                for found in span.captures_iter(&bullet) {
                    let text = found[1].trim().to_string();
                    // A citation, a planning id or a file name is not a protocol term.
                    if text.len() < 3
                        || text.contains(".mdx")
                        || text.contains(".ts")
                        || text.contains(".yaml")
                        || text.contains(':') && !text.contains('/')
                    {
                        continue;
                    }
                    spans.insert(text);
                }
            }
        }
    }
    let corpus: BTreeMap<&String, String> = supported
        .iter()
        .map(|revision| (revision, archives_of(revision)))
        .collect();
    let mut exclusive = BTreeMap::new();
    for text in spans {
        let carried: Vec<&String> = corpus
            .iter()
            .filter(|(_, archives)| archives.contains(&text))
            .map(|(revision, _)| *revision)
            .collect();
        if carried.len() == 1 {
            exclusive.insert(text, carried[0].clone());
        }
    }
    exclusive
}

/// The revisions one bullet rests on: those whose archives it cites, and those whose
/// archives alone carry a term it uses.
fn revisions_a_bullet_rests_on(
    bullet: &str,
    supported: &BTreeSet<String>,
    exclusive: &BTreeMap<String, String>,
) -> BTreeSet<String> {
    let mut rests_on: BTreeSet<String> = revisions_cited(bullet)
        .into_iter()
        .filter(|revision| supported.contains(revision))
        .collect();
    for (text, revision) in exclusive {
        if bullet.contains(&format!("`{text}`")) {
            rests_on.insert(revision.clone());
        }
    }
    rests_on
}

/// **The class behind three of the adversary's findings.** The matrix dispositions two
/// revisions `supported` outbound; where they disagree, a passage that names neither
/// states one of them as the transport's rule. Sections 3, 10 and 11 said which revision
/// they held for; 6 to 9 did not, and all three findings landed in 6 to 9.
///
/// So: every numbered section resolves the question for every supported revision, and so
/// does every scenario's `given` — a scenario is read as a case about a binding, and a
/// binding has a configured revision. The pair is read from the matrix and the names from
/// the document's own preamble; neither is written here.
#[test]
fn every_section_and_scenario_resolves_which_revision_it_holds_for() {
    let supported = supported_outbound_revisions();
    assert!(
        supported.len() > 1,
        "the selection matrix dispositions {} revision(s) supported outbound, so no \
         passage can state the wrong one's rule and this case has nothing to hold: \
         {supported:?}",
        supported.len()
    );
    let aliases = revision_aliases();
    let mut undeclared: Vec<&String> = supported
        .iter()
        .filter(|revision| !aliases.contains_key(*revision))
        .collect();
    undeclared.sort();
    assert!(
        undeclared.is_empty(),
        "semantics.md gives no name to {undeclared:?}, which the matrix dispositions \
         supported outbound; it names {aliases:?}"
    );

    let names_the_revision = |text: &str, revision: &str| -> bool {
        let prose = without_citations(&flat(text)).to_lowercase();
        prose.contains(revision) || prose.contains(&aliases[revision].to_lowercase())
    };

    let document = semantics();
    let mut unresolved = Vec::new();
    let numbered = regex::Regex::new(r"(?m)^## \d+\. .*$").unwrap();
    let headings: Vec<String> = numbered
        .find_iter(&document)
        .map(|found| found.as_str().to_string())
        .collect();
    assert!(
        headings.len() > 1,
        "semantics.md carries {} numbered sections, so this case is reading the wrong \
         document",
        headings.len()
    );
    for heading in &headings {
        let body = section(&document, heading);
        for revision in &supported {
            if !names_the_revision(&body, revision) {
                unresolved.push(format!(
                    "`{heading}` says nothing about `{revision}`, which the matrix \
                     dispositions supported outbound, so its rule reads as the \
                     transport's for a binding it may be false for"
                ));
            }
        }
    }
    // A scenario's `given` names the binding it is about — and, where it admits both
    // revisions, every bullet of it that states one revision's wire has to say so. Pass 1
    // asked only the first question, which a scenario widened to admit both revisions and
    // left specifying one answers vacuously: that is this round's root cause, and it is
    // the same class the sections are held to, applied to the file that decides them.
    let exclusive = revision_exclusive_spans(&supported);
    for (path, value) in owned_scenarios() {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("scenario file name")
            .to_string();
        let given = scenario_bullets(&value, "given").join(" ");
        let admitted: Vec<&String> = supported
            .iter()
            .filter(|revision| names_the_revision(&given, revision))
            .collect();
        if admitted.is_empty() {
            unresolved.push(format!(
                "{name} names no revision in its `given`, so it reads as a case about \
                 every binding this repository selects: {:?}",
                flat(&given)
            ));
            continue;
        }
        if admitted.len() < 2 {
            continue;
        }
        for key in ["summary", "given", "when", "then", "never"] {
            for bullet in scenario_bullets(&value, key) {
                let evidence = revisions_a_bullet_rests_on(&bullet, &supported, &exclusive);
                if evidence.len() != 1 {
                    continue;
                }
                let revision = evidence.iter().next().expect("one revision");
                if names_the_revision(&bullet, revision) {
                    continue;
                }
                unresolved.push(format!(
                    "{name} `{key}` rests on `{revision}` alone — by the archive it cites \
                     or by a term only that revision's archives carry — while its `given` \
                     admits both, so it reads as true for a binding it is false for: \
                     {bullet:?}"
                ));
            }
        }
    }
    assert!(
        unresolved.is_empty(),
        "{} passages state a rule without saying which of {supported:?} it holds for:\n  {}",
        unresolved.len(),
        unresolved.join("\n  ")
    );
}

/// A text's sentences, which is the unit a claim is made in. A paragraph naming a field
/// in one sentence and using a writing verb about something else in another says nothing
/// about the field, and reading whole paragraphs made that a finding three times.
fn sentences(text: &str) -> Vec<String> {
    flat(text)
        .split(". ")
        .map(|sentence| sentence.trim().to_string())
        .filter(|sentence| !sentence.is_empty())
        .collect()
}

/// The writing verb a sentence uses, in either voice, or `None`.
///
/// `record`, `set` and `record`'s plural are nouns as often as verbs in this contract —
/// "a durable record", "that set" — so a match introduced by a determiner is not read as
/// a verb.
fn writing_verb(sentence: &str) -> Option<String> {
    let writes = regex::Regex::new(
        r"(?i)\b(a|an|the|that|this|its|their|no|one|any|every|own|durable|permanent|second)?\s*\b((?:is|are|was|were|gets?|becomes?)\s+(?:then\s+)?(?:recorded|set|written|stored|saved|persisted|updated|assigned|remembered)|records?|sets?|writes?|stores?|saves?|persists?|updates?|assigns?|remembers?)\b",
    )
    .unwrap();
    writes.captures_iter(sentence).find_map(|capture| {
        if capture.get(1).is_some_and(|word| !word.as_str().is_empty()) {
            return None;
        }
        Some(capture[2].trim().to_string())
    })
}

/// **The class behind the durable-record finding.** `determined_era` was declared
/// byte-identical before and after an observation in one paragraph and recorded from a
/// wire probe in another — and it is a field of the `McpServerBinding` **entity**, which
/// the model says a client may persist.
///
/// The instance was one field and one verb. The rule is the story's own: this document
/// specifies what a live connection does, and no sentence of it writes a field of the
/// binding entity. Every field is read out of the model, so a field added later is
/// covered without this file being edited.
#[test]
fn no_sentence_writes_a_field_of_the_binding_entity() {
    let fields = binding_fields();
    assert!(
        fields.len() > 1,
        "the model declares {} field(s) on `McpServerBinding`, so this case is reading \
         the wrong entity: {fields:?}",
        fields.len()
    );
    // Three narrownesses a second pass found in this case's first shape: it read only
    // `semantics.md`, while the story's acceptance is that document **and its scenarios**;
    // it matched the passive voice only, while the contract writes "a probe records the
    // determined era" in the active; and it matched the model's spelling only, while prose
    // says "the determined era" with a space.
    let named_in = |text: &str| -> Vec<String> {
        fields
            .iter()
            .filter(|field| {
                let spelt = field.replace('_', "[_ ]");
                regex::Regex::new(&format!(r"(?i)\b{spelt}\b"))
                    .unwrap()
                    .is_match(text)
            })
            .cloned()
            .collect()
    };

    let mut broken = Vec::new();
    let document = semantics();
    for (line, text) in paragraphs(&document) {
        for sentence in sentences(&text) {
            let named = named_in(&sentence);
            if named.is_empty() {
                continue;
            }
            if let Some(verb) = writing_verb(&sentence) {
                broken.push(format!(
                    "semantics.md:{line} says `{verb}` of {named:?}, which are fields of \
                     the `McpServerBinding` entity; this contract specifies a live \
                     connection and implies no durable record of one: {sentence:?}"
                ));
            }
        }
    }
    for (path, value) in owned_scenarios() {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("scenario file name")
            .to_string();
        // `never:` states a prohibition, so a writing verb there is the opposite of a
        // write and is not read.
        for key in ["summary", "given", "when", "then"] {
            for bullet in scenario_bullets(&value, key) {
                for sentence in sentences(&bullet) {
                    let named = named_in(&sentence);
                    if named.is_empty() {
                        continue;
                    }
                    if let Some(verb) = writing_verb(&sentence) {
                        broken.push(format!(
                            "{name} `{key}` says `{verb}` of {named:?}, which are fields \
                             of the `McpServerBinding` entity: {sentence:?}"
                        ));
                    }
                }
            }
        }
    }
    assert!(
        broken.is_empty(),
        "{} passages of this contract write a field of the binding entity:\n  {}",
        broken.len(),
        broken.join("\n  ")
    );
}

/// The `###` section that defines one outcome, heading included.
fn outcome_section(document: &str, outcome: &str) -> String {
    let heading = format!("### `{outcome}`");
    let start = document
        .find(&heading)
        .unwrap_or_else(|| panic!("semantics.md defines no outcome `{outcome}`"));
    let rest = &document[start + heading.len()..];
    let end = rest
        .find("\n### ")
        .into_iter()
        .chain(rest.find("\n## "))
        .min()
        .unwrap_or(rest.len());
    format!("{heading}{}", &rest[..end])
}

/// The variants `connectors.service_wire.ErrorCode` declares, read out of the shared
/// model.
fn wire_code_variants() -> Vec<&'static str> {
    let model = read(&repository_root().join("ess/domains/service_wire.yaml"));
    let declaration = model
        .lines()
        .skip_while(|line| !line.contains("connectors.service_wire.ErrorCode"))
        .find(|line| line.trim_start().starts_with("variants:"))
        .expect("ess/domains/service_wire.yaml declares no ErrorCode variants")
        .to_string();
    declaration
        .trim()
        .trim_start_matches("variants:")
        .trim()
        .trim_matches(|c| c == '[' || c == ']')
        .split(',')
        .map(|variant| Box::leak(variant.trim().to_string().into_boxed_str()) as &'static str)
        .filter(|variant| !variant.is_empty())
        .collect()
}

/// **The class behind the wrong wire code.** `-32020` was filed under `upstream_protocol`
/// — a code that blames the peer — for a condition only this client or an intermediary
/// can produce. Which code is right for a given condition is a judgement no test can
/// make; *whose act a refusal reports* is a fact the register can carry, and once it does,
/// the code that contradicts it is mechanical.
///
/// So the register carries a fourth column, and this case holds the mapping: an
/// observation names no actor, a refusal names one, `upstream_protocol` is the peer's,
/// `internal` is this client's, and `outcome_unknown` is nobody's because nothing was
/// observed.
#[test]
fn every_refusal_names_whose_act_it_reports() {
    const ACTORS: [&str; 5] = ["n/a", "peer", "client", "operator", "unobserved"];
    /// A code that names a side may only be reported as that side's act.
    const CODE_NAMES_THE_ACT_OF: [(&str, &str); 3] = [
        ("upstream_protocol", "peer"),
        ("internal", "client"),
        ("outcome_unknown", "unobserved"),
    ];
    /// The other direction, for the one act that has exactly one code: a refusal
    /// reporting this client's own act is `internal`.
    ///
    /// `unobserved` was bound to `outcome_unknown` here in the first correction, and a
    /// second pass showed the binding was wrong rather than strong: an endpoint that never
    /// answered is a refusal in which nothing was seen, and its code is `unavailable`, not
    /// uncertainty about an effect. What replaces the binding is the rule below, which
    /// holds every refusal's column against its own prose — the thing that was unchecked
    /// when the column disagreed with the section it labels.
    const ACT_IS_CARRIED_BY: [(&str, &str); 1] = [("client", "internal")];
    /// How each act is written where a section declares it.
    const DECLARED_AS: [(&str, &str); 4] = [
        ("peer", "The act reported is the peer's"),
        ("client", "The act reported is this client's"),
        ("operator", "The act reported is the operator's"),
        ("unobserved", "The act reported is nobody's"),
    ];

    let variants = wire_code_variants();
    let another_outcome = regex::Regex::new(r"`mcp\.outbound\.[a-z-]+`").unwrap();
    let document = semantics();
    let rows = rows();
    let mut broken = Vec::new();
    for row in &rows {
        if !ACTORS.contains(&row.actor.as_str()) {
            broken.push(format!(
                "semantics.md:{} `{}` reports the act of `{}`, which is not one of \
                 {ACTORS:?}",
                row.line, row.outcome, row.actor
            ));
            continue;
        }
        match (row.wire_code.as_str(), row.actor.as_str()) {
            ("n/a", "n/a") => {}
            ("n/a", actor) => broken.push(format!(
                "semantics.md:{} `{}` is an observation, not a refusal, and reports the \
                 act of `{actor}`",
                row.line, row.outcome
            )),
            (_, "n/a") => broken.push(format!(
                "semantics.md:{} `{}` refuses with `{}` and names whose act it reports as \
                 nobody's",
                row.line, row.outcome, row.wire_code
            )),
            _ => {}
        }
        for (code, actor) in CODE_NAMES_THE_ACT_OF {
            if row.wire_code == code && row.actor != actor {
                broken.push(format!(
                    "semantics.md:{} `{}` carries `{code}`, which reports the act of \
                     `{actor}`, and names `{}`",
                    row.line, row.outcome, row.actor
                ));
            }
        }
        for (actor, code) in ACT_IS_CARRIED_BY {
            if row.actor == actor && row.wire_code != code {
                broken.push(format!(
                    "semantics.md:{} `{}` reports the act of `{actor}`, whose code is \
                     `{code}`, and carries `{}`",
                    row.line, row.outcome, row.wire_code
                ));
            }
        }

        // The column against the prose it labels. A refusal declares its act in the
        // section that defines it, in one of four sentences, and the sentence is the one
        // the column names; an observation declares none, because it reports no act.
        let section = outcome_section(&document, &row.outcome);
        let flat_section = flat(&section);
        let declared: Vec<&str> = DECLARED_AS
            .iter()
            .filter(|(_, sentence)| flat_section.contains(sentence))
            .map(|(actor, _)| *actor)
            .collect();
        if row.wire_code == "n/a" {
            if !declared.is_empty() {
                broken.push(format!(
                    "semantics.md:{} `{}` is an observation and its section declares the \
                     act of {declared:?}",
                    row.line, row.outcome
                ));
            }
        } else if declared != [row.actor.as_str()] {
            broken.push(format!(
                "semantics.md:{} `{}` reports the act of `{}` and its own section declares \
                 {declared:?}; a refusal says whose act it reports in the words the \
                 register uses, or the column is a label nobody checked",
                row.line, row.outcome, row.actor
            ));
        }

        // An observation that names a refusal's code is describing another row's outcome,
        // and has to say which row. This is where a register row carrying `n/a` while its
        // prose produces an error stops being invisible.
        if row.wire_code == "n/a" {
            for sentence in flat_section.split(". ") {
                let named: Vec<&&str> = variants
                    .iter()
                    .filter(|variant| {
                        sentence.contains(&format!("`{variant}`")) && **variant != "n/a"
                    })
                    .collect();
                if named.is_empty() {
                    continue;
                }
                let cross_reference = another_outcome
                    .find_iter(sentence)
                    .any(|found| found.as_str() != format!("`{}`", row.outcome));
                if !cross_reference {
                    broken.push(format!(
                        "semantics.md:{} `{}` carries no code, and a sentence of its \
                         section names {named:?} without naming the outcome that carries \
                         it: {sentence:?}",
                        row.line, row.outcome
                    ));
                }
            }
        }
    }
    assert!(
        broken.is_empty(),
        "{} register rows disagree about whose act they report:\n  {}",
        broken.len(),
        broken.join("\n  ")
    );
}

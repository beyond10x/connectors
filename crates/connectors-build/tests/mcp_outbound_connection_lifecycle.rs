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
            4,
            "semantics.md:{} is a register row with {} cells, not the four the register \
             declares (state, outcome, wire code, scenario): {trimmed}",
            index + 1,
            cells.len()
        );
        let scenario = link
            .captures(&cells[3])
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
/// it. Every mention of the word sits in the section that holds it, or on a line that
/// names the blocker.
#[test]
fn outbound_stdio_is_held_by_its_blocker_and_specified_nowhere() {
    let document = semantics();
    let blocker = "decision-blocker:mcp-outbound-stdio-process-ownership";
    assert!(
        document.contains(blocker),
        "semantics.md specifies an outbound transport without naming the blocker that \
         holds the other one"
    );
    let held = section(&document, HELD_HEADING);
    let mut broken = Vec::new();
    for (index, line) in document.lines().enumerate() {
        if !line.to_lowercase().contains("stdio") {
            continue;
        }
        if held.contains(line) || line.contains(blocker) {
            continue;
        }
        broken.push(format!(
            "semantics.md:{} states something about stdio outside `{HELD_HEADING}` and \
             without naming the blocker: {}",
            index + 1,
            line.trim()
        ));
    }
    assert!(
        broken.is_empty(),
        "{} lines answer a held question:\n  {}",
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

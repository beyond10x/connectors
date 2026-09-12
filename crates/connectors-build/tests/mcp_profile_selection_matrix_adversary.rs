//! Adversary pass over `story:mcp-profile-selection-matrix`, written 2026-09-12 against
//! `4ad2ee5` on branch `unit/mcp-profile-selection-matrix-20260912`.
//!
//! `mcp_profile_selection_matrix.rs` derives the feature list from the archived bytes and
//! compares it against `adapters/mcp/contracts/protocol/v1alpha1/selection.md`. Both
//! halves — the derivation rule in the document's "How this list was derived" and the
//! derivation in that file — were written by the same unit, so they agree with each other
//! by construction and nothing compares either against the story's acceptance statement
//! or against the records the matrix cites as its authority. These three cases do that,
//! each from an authority the unit did not write:
//!
//! 1. the archived bytes, read for a version-declaring position the unit's four positions
//!    do not cover — the protocol version a peer is told to *assume* for a request;
//! 2. the planning store, read for whether the blocker a `deferred` row rests on says
//!    anywhere in its own record that it reaches that feature;
//! 3. the archived schemas again, read per **side** rather than as a flat union, because
//!    "named somewhere in the document" is not "named in the row that carries it".

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

const PRIMARY: &str = "2026-07-28";
const INTEROP: &str = "2025-11-25";

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn adapter_dir() -> PathBuf {
    repository_root().join("adapters/mcp/contracts/protocol/v1alpha1")
}

fn evidence_dir() -> PathBuf {
    adapter_dir().join("evidence/20260912")
}

fn matrix() -> String {
    let path = adapter_dir().join("selection.md");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn manifest_files() -> Vec<String> {
    let path = evidence_dir().join("specification-source-hashes.json");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).expect("specification-source-hashes.json is not JSON");
    value
        .as_array()
        .expect("specification-source-hashes.json is not a JSON array")
        .iter()
        .map(|entry| {
            entry["file"]
                .as_str()
                .unwrap_or_else(|| panic!("manifest entry is missing a string `file`: {entry}"))
                .to_string()
        })
        .collect()
}

/// Uncompressed bytes of one archived source, as text — the same `gzip -dc` the pinned
/// record's own reproduction snippet uses.
fn archived(file: &str) -> String {
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
    String::from_utf8(output.stdout).expect("archived source is not UTF-8")
}

// ── The matrix, as rows ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct Row {
    key: String,
    direction: String,
    disposition: String,
    reason: String,
    line: usize,
}

fn rows() -> Vec<Row> {
    let document = matrix();
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
        if cells.len() != 5 {
            continue;
        }
        let key = cells[0].trim_matches('`').to_string();
        if !(key.starts_with("revision:")
            || key.starts_with("transport:")
            || key.starts_with("capability:"))
        {
            continue;
        }
        parsed.push(Row {
            key,
            direction: cells[1].clone(),
            disposition: cells[3].clone(),
            reason: cells[4].clone(),
            line: index + 1,
        });
    }
    assert!(
        !parsed.is_empty(),
        "selection.md carries no matrix row this check can read"
    );
    parsed
}

/// The file's lines joined by single spaces, with the 1-based source line of every
/// joined position kept, so a match in the joined text can be cited at its real line.
///
/// A sentence in an `.mdx` source is wrapped across lines, so a phrase the specification
/// states normatively does not appear on any one line of the archived bytes.
fn flattened(text: &str) -> (String, Vec<(usize, usize)>) {
    let mut flat = String::with_capacity(text.len());
    let mut starts = Vec::new();
    for (index, line) in text.lines().enumerate() {
        starts.push((flat.len(), index + 1));
        flat.push_str(line.trim());
        flat.push(' ');
    }
    (flat, starts)
}

fn line_of(starts: &[(usize, usize)], offset: usize) -> usize {
    starts
        .iter()
        .rev()
        .find(|(start, _)| *start <= offset)
        .map(|(_, line)| *line)
        .unwrap_or(0)
}

// ── Case 1 ──────────────────────────────────────────────────────────────────────────

/// Every protocol version the pinned archives tell a peer to **assume** for a request is
/// dispositioned by the matrix.
///
/// The story's acceptance is that "every protocol revision … named by the pinned
/// specification appears exactly once … so that a capability absent from the matrix is a
/// defect in the matrix rather than a silent approximation". The document narrows that to
/// four version-declaring positions of its own choosing, and
/// `mcp_profile_selection_matrix.rs` implements those same four, so the narrowing is
/// never tested against the acceptance it narrows.
///
/// This case reads a fifth position, and it is the most normative of the lot: the version
/// a peer is required to treat a request as carrying when the request does not say. Both
/// pinned revisions state it, in the same words, about the same string.
#[test]
fn every_protocol_version_the_archives_tell_a_peer_to_assume_is_dispositioned() {
    let assumed =
        regex::Regex::new(r"(?:assume|as) protocol version `?(\d{4}-\d{2}-\d{2})`?").unwrap();

    let mut declared: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for file in manifest_files() {
        let text = archived(&file);
        let (flat, starts) = flattened(&text);
        for capture in assumed.captures_iter(&flat) {
            let whole = capture.get(0).expect("capture 0 always exists");
            let line = line_of(&starts, whole.start());
            declared
                .entry(capture[1].to_string())
                .or_default()
                .push(format!("{file}:{line} {:?}", whole.as_str()));
        }
    }
    assert!(
        !declared.is_empty(),
        "no archived file states an assumed protocol version; this derivation is broken \
         and the case proves nothing"
    );

    let dispositioned: BTreeSet<String> = rows()
        .into_iter()
        .filter(|row| row.key.starts_with("revision:"))
        .map(|row| row.key.trim_start_matches("revision:").to_string())
        .collect();

    let missing: Vec<String> = declared
        .iter()
        .filter(|(version, _)| !dispositioned.contains(*version))
        .map(|(version, sites)| format!("{version} — stated at {}", sites.join("; ")))
        .collect();

    assert!(
        missing.is_empty(),
        "the pinned archives tell a peer to assume {} protocol version(s) the matrix \
         dispositions nowhere, while it does disposition `2025-06-18`, which the same \
         archives state only as a header example:\n  {}\nthe matrix's revision keys are \
         {dispositioned:?}",
        missing.len(),
        missing.join("\n  ")
    );
}

// ── Case 2 ──────────────────────────────────────────────────────────────────────────

/// A `deferred` row's blocker names, in its own record, the feature the row defers.
///
/// The matrix claims exactly this standard for itself: "Two rows are deferred against
/// open blockers and two beside them are not, and the difference is cited rather than
/// inferred", and "the reason is in each blocker's own text rather than inferred here".
/// `mcp_profile_selection_matrix.rs` checks only that a row naming a blocker defers — it
/// never opens the blocker, so a row can defer a feature the blocker says nothing about
/// and stay green.
///
/// The whole blocker record is searched, not just its "What this stops" section, so a
/// failure here means the blocker never mentions the deferred feature anywhere at all.
#[test]
fn every_deferred_row_rests_on_a_blocker_whose_own_record_names_that_feature() {
    let blocker = regex::Regex::new(r"decision-blocker:([a-z0-9][a-z0-9-]*)").unwrap();
    let planning = repository_root().join(".engineering/planning/decision-blocker");

    let mut unreached = Vec::new();
    let mut checked = 0usize;
    for row in rows() {
        if row.disposition != "deferred" {
            continue;
        }
        for capture in blocker.captures_iter(&row.reason) {
            let id = capture[1].to_string();
            let record = planning.join(format!("{id}.md"));
            let body = std::fs::read_to_string(&record)
                .unwrap_or_else(|e| panic!("read {}: {e}", record.display()));
            let normalised: String = body
                .chars()
                .map(|character| {
                    if character.is_ascii_alphanumeric() {
                        character.to_ascii_lowercase()
                    } else {
                        ' '
                    }
                })
                .collect::<String>()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            let feature = row
                .key
                .split_once(':')
                .map(|(_, rest)| rest)
                .unwrap_or(&row.key)
                .replace(['-', '/', '_'], " ")
                .to_ascii_lowercase();
            checked += 1;
            if !normalised.contains(&feature) {
                unreached.push(format!(
                    "selection.md:{} `{}` ({}) is deferred against \
                     `decision-blocker:{id}`, but that record never names {feature:?} — \
                     the string appears 0 times in {}",
                    row.line,
                    row.key,
                    row.direction,
                    record.display()
                ));
            }
        }
    }
    assert!(
        checked > 0,
        "no deferred row names a decision-blocker; this case proves nothing"
    );
    assert!(
        unreached.is_empty(),
        "{} of {checked} blocker-backed deferrals rest on a blocker that does not reach \
         the deferred feature:\n  {}",
        unreached.len(),
        unreached.join("\n  ")
    );
}

// ── Case 3 ──────────────────────────────────────────────────────────────────────────

fn strip_comments(source: &str) -> String {
    let characters: Vec<char> = source.chars().collect();
    let mut kept = String::with_capacity(source.len());
    let mut at = 0usize;
    while at < characters.len() {
        if characters[at] == '/' && at + 1 < characters.len() && characters[at + 1] == '*' {
            at += 2;
            while at + 1 < characters.len() && !(characters[at] == '*' && characters[at + 1] == '/')
            {
                at += 1;
            }
            at = (at + 2).min(characters.len());
        } else if characters[at] == '/' && at + 1 < characters.len() && characters[at + 1] == '/' {
            while at < characters.len() && characters[at] != '\n' {
                at += 1;
            }
        } else {
            kept.push(characters[at]);
            at += 1;
        }
    }
    kept
}

fn closing_brace(characters: &[char], after_open: usize) -> usize {
    let mut depth = 1usize;
    let mut at = after_open;
    while at < characters.len() {
        match characters[at] {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return at;
                }
            }
            _ => {}
        }
        at += 1;
    }
    characters.len()
}

fn interface_body(source: &str, name: &str) -> String {
    let needle = format!("export interface {name} {{");
    let start = source
        .find(&needle)
        .unwrap_or_else(|| panic!("`{name}` is not declared in this schema"))
        + needle.len();
    let characters: Vec<char> = source[start..].chars().collect();
    let end = closing_brace(&characters, 0);
    characters[..end].iter().collect()
}

/// Dotted paths of the settings nested inside an optional field, and nothing else: a
/// top-level field has no dot and is a capability with a row of its own.
fn nested_settings(body: &str, prefix: &str, found: &mut BTreeSet<String>) {
    let characters: Vec<char> = body.chars().collect();
    let mut at = 0usize;
    let mut token = String::new();
    while at < characters.len() {
        let character = characters[at];
        if character.is_ascii_alphanumeric() || character == '_' || character == '$' {
            token.push(character);
            at += 1;
            continue;
        }
        if character == '?' && at + 1 < characters.len() && characters[at + 1] == ':' {
            if token.is_empty() {
                at += 2;
                continue;
            }
            let path = if prefix.is_empty() {
                token.clone()
            } else {
                format!("{prefix}.{token}")
            };
            if path.contains('.') {
                found.insert(path.clone());
            }
            token.clear();
            at += 2;
            while at < characters.len() && characters[at].is_whitespace() {
                at += 1;
            }
            if at < characters.len() && characters[at] == '{' {
                let end = closing_brace(&characters, at + 1);
                let inner: String = characters[at + 1..end].iter().collect();
                nested_settings(&inner, &path, found);
                at = end + 1;
            }
            continue;
        }
        if character == '{' {
            at = closing_brace(&characters, at + 1) + 1;
            token.clear();
            continue;
        }
        token.clear();
        at += 1;
    }
}

/// `side -> nested setting paths that side's schemas declare`, over both pinned revisions.
fn nested_by_side() -> BTreeMap<&'static str, BTreeSet<String>> {
    let mut by_side: BTreeMap<&'static str, BTreeSet<String>> = BTreeMap::new();
    for (side, interface) in [
        ("server", "ServerCapabilities"),
        ("client", "ClientCapabilities"),
    ] {
        let entry = by_side.entry(side).or_default();
        for revision in [PRIMARY, INTEROP] {
            let schema = strip_comments(&archived(&format!("mcp-{revision}-schema.ts")));
            nested_settings(&interface_body(&schema, interface), "", entry);
        }
    }
    by_side
}

/// The `## <heading>` section of the matrix, up to the next `## ` heading.
fn section(document: &str, heading: &str) -> String {
    let mut collecting = false;
    let mut collected = String::new();
    for line in document.lines() {
        if line.starts_with("## ") {
            if collecting {
                break;
            }
            collecting = line.trim() == heading;
            continue;
        }
        if collecting {
            collected.push_str(line);
            collected.push('\n');
        }
    }
    assert!(
        !collected.trim().is_empty(),
        "selection.md has no `{heading}` section"
    );
    collected
}

/// A nested setting is named in the section for the side whose schema declares it.
///
/// `mcp_profile_selection_matrix.rs` unions the two sides into one flat set of dotted
/// paths and asks only that each appears somewhere in the whole document, so a setting
/// declared on both sides is satisfied for both by one mention on either. The matrix's
/// own promise is stronger and is the one the story rests on: the nested settings are
/// "named in the rows that carry them", and "each is a setting of a capability that is
/// already dispositioned". A setting named only in the other side's paragraph is named
/// beside a row that does not carry it.
#[test]
fn every_nested_setting_is_named_in_the_section_of_the_side_whose_schema_declares_it() {
    let document = matrix();
    let sections = BTreeMap::from([
        ("server", section(&document, "## Server capabilities")),
        ("client", section(&document, "## Client capabilities")),
    ]);

    let by_side = nested_by_side();
    let derived: usize = by_side.values().map(BTreeSet::len).sum();
    assert!(
        derived >= 18,
        "only {derived} nested settings were derived from the pinned schemas; the schema \
         parse is broken and this case proves nothing"
    );

    let mut unnamed = Vec::new();
    for (side, paths) in &by_side {
        let text = &sections[side];
        for path in paths {
            if !text.contains(&format!("`{path}`")) {
                unnamed.push(format!(
                    "`{path}` is declared by a {side} capability of a pinned schema and is \
                     named nowhere in the matrix's `## {} capabilities` section",
                    if *side == "server" {
                        "Server"
                    } else {
                        "Client"
                    }
                ));
            }
        }
    }

    assert!(
        unnamed.is_empty(),
        "{} of the {derived} side-qualified nested settings are named outside the section \
         that dispositions them:\n  {}",
        unnamed.len(),
        unnamed.join("\n  ")
    );
}

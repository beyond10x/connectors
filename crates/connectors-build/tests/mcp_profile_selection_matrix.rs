//! The coverage-matrix property for `story:mcp-profile-selection-matrix`, written
//! 2026-09-12 on branch `unit/mcp-profile-selection-matrix-20260912`.
//!
//! The story's acceptance is countable: every protocol revision, every transport and
//! every optional capability *named by the pinned specification* appears exactly once in
//! `adapters/mcp/contracts/protocol/v1alpha1/selection.md`, with one of three
//! dispositions, a reason and a source line into the pinned revision — "so that a
//! capability absent from the matrix is a defect in the matrix rather than a silent
//! approximation."
//!
//! A check that hard-coded the feature list would restate the matrix rather than test
//! it: the list and the matrix would be one document written twice, and a feature
//! upstream names but nobody here noticed would be absent from both. So every feature
//! this file compares against is **derived from the archived bytes** under
//! `adapters/mcp/contracts/protocol/v1alpha1/evidence/20260912/vendor/`, whose contents
//! are fixed by the digests `specification-sources.md` records. The derivation rules are
//! stated in the matrix document itself, under "How this list was derived", and are
//! implemented here.
//!
//! Archives are read with `gzip -dc`, the command the pinned-source record's own
//! reproduction snippet uses.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

const PRIMARY: &str = "2026-07-28";
const INTEROP: &str = "2025-11-25";
const DISPOSITIONS: [&str; 3] = ["supported", "explicitly refused", "deferred"];

fn adapter_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/mcp/contracts/protocol/v1alpha1")
}

fn evidence_dir() -> PathBuf {
    adapter_dir().join("evidence/20260912")
}

fn matrix() -> String {
    let path = adapter_dir().join("selection.md");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn manifest_entries() -> Vec<serde_json::Value> {
    let path = evidence_dir().join("specification-source-hashes.json");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).expect("specification-source-hashes.json is not JSON");
    value
        .as_array()
        .expect("specification-source-hashes.json is not a JSON array")
        .clone()
}

fn manifest_field(entry: &serde_json::Value, key: &str) -> String {
    entry[key]
        .as_str()
        .unwrap_or_else(|| panic!("manifest entry is missing a string `{key}`: {entry}"))
        .to_string()
}

/// Uncompressed bytes of one archived source, as text.
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

// ── Derivation 1: protocol revisions ────────────────────────────────────────────────
//
// A revision is a protocol version string the archives state in a version-declaring
// position. Four such positions exist across the 54 archived files, and they do not
// agree with one another — that disagreement is the trap `specification-sources.md`
// records, and a derivation that read only one of them would state the wrong version.

fn derived_revisions() -> BTreeSet<String> {
    let mut found = BTreeSet::new();

    // (a) The negotiable strings, as the PRIMARY revision states them: the `supported`
    //     list of its `UnsupportedProtocolVersionError` example.
    let versioning = archived("mcp-2026-07-28-basic-versioning.mdx");
    let start = versioning
        .find("\"supported\": [")
        .expect("the primary revision's versioning document has no `supported` list")
        + "\"supported\": [".len();
    let tail = &versioning[start..];
    let end = tail.find(']').expect("unterminated `supported` list");
    for (index, quoted) in tail[..end].split('"').enumerate() {
        if index % 2 == 1 {
            found.insert(quoted.to_string());
        }
    }
    assert!(
        found.len() >= 2,
        "the `supported` list parsed to {found:?}, which cannot be the negotiable set"
    );

    // (b) Every version string any archived file declares as a schema constant, as the
    //     `protocolVersion` of a message, or as an `MCP-Protocol-Version` header value.
    let constant = regex::Regex::new(r#"LATEST_PROTOCOL_VERSION\s*=\s*"([^"]+)""#).unwrap();
    let body = regex::Regex::new(r#""protocolVersion"\s*:\s*"([^"]+)""#).unwrap();
    let header =
        regex::Regex::new(r"MCP-Protocol-Version:[ \t]+([0-9A-Za-z][0-9A-Za-z._-]*)").unwrap();
    for entry in manifest_entries() {
        let text = archived(&manifest_field(&entry, "file"));
        for pattern in [&constant, &body, &header] {
            for capture in pattern.captures_iter(&text) {
                found.insert(capture[1].to_string());
            }
        }
    }

    found
}

// ── Derivation 2: transports ────────────────────────────────────────────────────────
//
// The interoperability revision's transports document is organised one top-level
// section per transport family, so its `##` headings are the families that revision
// names. The fourth family is not a section anywhere: the primary revision carries
// HTTP+SSE as a subsection of Backward Compatibility and registers it in
// `deprecated.mdx`, so the registry's own rows supply it — every row whose feature
// links into a `/basic/transports` page is a transport family.

fn key_of(name: &str) -> String {
    let mut key = String::new();
    let mut pending_dash = false;
    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            key.push(character.to_ascii_lowercase());
            pending_dash = false;
        } else if !key.is_empty() && !pending_dash {
            key.push('-');
            pending_dash = true;
        }
    }
    key.trim_end_matches('-').to_string()
}

fn derived_transports() -> BTreeSet<String> {
    let mut found = BTreeSet::new();

    for line in archived("mcp-2025-11-25-basic-transports.mdx").lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            found.insert(key_of(heading.trim()));
        }
    }

    let deprecated = archived("mcp-2026-07-28-deprecated.mdx");
    let registry = deprecated
        .split("## Deprecated")
        .nth(1)
        .expect("the primary revision's deprecation registry has no `## Deprecated` section");
    let registry = registry.split("## Removed").next().unwrap_or(registry);
    for line in registry.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }
        let feature = trimmed
            .trim_matches('|')
            .split('|')
            .next()
            .unwrap_or_default()
            .trim();
        let (Some(open), Some(close)) = (feature.find('['), feature.find("](")) else {
            continue;
        };
        let target_end = match feature[close..].find(')') {
            Some(at) => close + at,
            None => continue,
        };
        if feature[close + 2..target_end].contains("/basic/transports") {
            found.insert(key_of(&feature[open + 1..close]));
        }
    }

    found
}

// ── Derivation 3: capabilities ──────────────────────────────────────────────────────
//
// The capability sets are the fields of `ServerCapabilities` and `ClientCapabilities`
// in each pinned revision's own normative `schema.ts`, unioned across the two. Top-level
// fields are the capabilities a peer advertises and each takes a matrix row; their
// nested fields are per-capability settings of an advertised capability and are required
// to be named by the matrix rather than given rows of their own.

fn strip_comments(source: &str) -> String {
    let characters: Vec<char> = source.chars().collect();
    let mut kept = String::with_capacity(source.len());
    let mut at = 0;
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

fn interface_body(source: &str, name: &str) -> String {
    let needle = format!("export interface {name} {{");
    let start = source
        .find(&needle)
        .unwrap_or_else(|| panic!("`{name}` is not declared in this schema"))
        + needle.len();
    let characters: Vec<char> = source[start..].chars().collect();
    let mut depth = 1usize;
    for (at, character) in characters.iter().enumerate() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return characters[..at].iter().collect();
                }
            }
            _ => {}
        }
    }
    panic!("`{name}` has no closing brace");
}

fn declared_fields(body: &str, prefix: &str, found: &mut BTreeSet<String>) {
    let characters: Vec<char> = body.chars().collect();
    let mut at = 0;
    let mut depth = 0usize;
    let mut token = String::new();
    while at < characters.len() {
        let character = characters[at];
        if depth == 0 && (character.is_ascii_alphanumeric() || character == '_' || character == '$')
        {
            token.push(character);
            at += 1;
            continue;
        }
        if depth == 0
            && character == '?'
            && at + 1 < characters.len()
            && characters[at + 1] == ':'
            && !token.is_empty()
        {
            let path = if prefix.is_empty() {
                token.clone()
            } else {
                format!("{prefix}.{token}")
            };
            found.insert(path.clone());
            token.clear();
            at += 2;
            while at < characters.len() && characters[at].is_whitespace() {
                at += 1;
            }
            if at < characters.len() && characters[at] == '{' {
                let inner_start = at + 1;
                let mut inner_depth = 1usize;
                let mut cursor = inner_start;
                while cursor < characters.len() {
                    match characters[cursor] {
                        '{' => inner_depth += 1,
                        '}' => {
                            inner_depth -= 1;
                            if inner_depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    cursor += 1;
                }
                let inner: String = characters[inner_start..cursor].iter().collect();
                declared_fields(&inner, &path, found);
                at = cursor + 1;
            }
            continue;
        }
        if character == '{' {
            depth += 1;
        } else if character == '}' {
            depth = depth.saturating_sub(1);
        }
        token.clear();
        at += 1;
    }
}

/// `(row keys, nested settings paths)`.
fn derived_capabilities() -> (BTreeSet<String>, BTreeSet<String>) {
    let mut keys = BTreeSet::new();
    let mut nested = BTreeSet::new();
    for side in ["server", "client"] {
        for revision in [PRIMARY, INTEROP] {
            let schema = strip_comments(&archived(&format!("mcp-{revision}-schema.ts")));
            let interface = match side {
                "server" => "ServerCapabilities",
                _ => "ClientCapabilities",
            };
            let mut fields = BTreeSet::new();
            declared_fields(&interface_body(&schema, interface), "", &mut fields);
            for path in fields {
                if path.contains('.') {
                    nested.insert(path);
                } else {
                    keys.insert(format!("capability:{side}/{path}"));
                }
            }
        }
    }
    (keys, nested)
}

fn derived_keys() -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for revision in derived_revisions() {
        keys.insert(format!("revision:{revision}"));
    }
    for transport in derived_transports() {
        keys.insert(format!("transport:{transport}"));
    }
    keys.extend(derived_capabilities().0);
    keys
}

// ── The matrix, as rows ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct Row {
    key: String,
    direction: String,
    defined_by: String,
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
        let key = cells[0].trim_matches('`').to_string();
        if !(key.starts_with("revision:")
            || key.starts_with("transport:")
            || key.starts_with("capability:"))
        {
            continue;
        }
        assert_eq!(
            cells.len(),
            5,
            "selection.md:{} is a matrix row with {} cells, not the five the matrix \
             declares (key, direction, defined by, disposition, reason): {trimmed}",
            index + 1,
            cells.len()
        );
        parsed.push(Row {
            key,
            direction: cells[1].clone(),
            defined_by: cells[2].clone(),
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

fn rows_by_key() -> BTreeMap<String, Vec<Row>> {
    let mut grouped: BTreeMap<String, Vec<Row>> = BTreeMap::new();
    for row in rows() {
        grouped.entry(row.key.clone()).or_default().push(row);
    }
    grouped
}

/// Every `` `mcp-…:line` `` citation in a cell, as `(file, line)`.
fn citations(cell: &str) -> Vec<(String, usize)> {
    let citation =
        regex::Regex::new(r"`(mcp-[0-9A-Za-z._+-]+\.(?:mdx|ts)):(\d+)(?:-(\d+))?`").unwrap();
    let mut found = Vec::new();
    for capture in citation.captures_iter(cell) {
        let file = capture[1].to_string();
        found.push((file.clone(), capture[2].parse().unwrap()));
        if let Some(end) = capture.get(3) {
            found.push((file, end.as_str().parse().unwrap()));
        }
    }
    found
}

// ── Cases ───────────────────────────────────────────────────────────────────────────

/// The story's acceptance, stated as a count.
///
/// "Exactly once" is enforced per direction, because `epic:mcp-contracts` says outbound
/// and inbound "are separate directions with separate trust boundaries" and two of the
/// matrix's dispositions differ between them. A feature is covered when its rows give
/// each direction exactly one disposition: one `both` row, or one `outbound` row and one
/// `inbound` row. A missing feature, a duplicated row, a half-covered feature and a row
/// naming something the pinned specification does not all fail here.
#[test]
fn every_feature_the_pinned_specification_names_is_dispositioned_once_per_direction() {
    let derived = derived_keys();
    let grouped = rows_by_key();

    let missing: Vec<&String> = derived
        .iter()
        .filter(|key| !grouped.contains_key(*key))
        .collect();
    assert!(
        missing.is_empty(),
        "the pinned archives name {} features the matrix does not disposition: {:?}",
        missing.len(),
        missing
    );

    let invented: Vec<&String> = grouped
        .keys()
        .filter(|key| !derived.contains(*key))
        .collect();
    assert!(
        invented.is_empty(),
        "the matrix dispositions {} keys no archived byte names: {:?}",
        invented.len(),
        invented
    );

    let mut broken = Vec::new();
    for (key, key_rows) in &grouped {
        let directions: Vec<&str> = key_rows.iter().map(|row| row.direction.as_str()).collect();
        let covered = match directions.as_slice() {
            ["both"] => true,
            [first, second] => {
                let pair: BTreeSet<&str> = [*first, *second].into_iter().collect();
                pair == BTreeSet::from(["outbound", "inbound"])
            }
            _ => false,
        };
        if !covered {
            broken.push(format!(
                "{key} at line(s) {:?} covers {directions:?}, not one `both` row or one \
                 `outbound` and one `inbound` row",
                key_rows.iter().map(|row| row.line).collect::<Vec<_>>()
            ));
        }
    }
    assert!(
        broken.is_empty(),
        "{} features are not dispositioned exactly once per direction:\n  {}",
        broken.len(),
        broken.join("\n  ")
    );

    assert_eq!(
        grouped.len(),
        derived.len(),
        "the matrix and the archives disagree on how many features exist"
    );
}

/// Each row carries one of the three dispositions the acceptance names, a reason, and a
/// source line that resolves into an archived file of the pinned revisions.
///
/// A citation is checked against the manifest and against the archived file's own
/// length, so a row citing a file this repository never pinned, or a line past the end of
/// one it did, fails rather than reading as evidence.
#[test]
fn each_row_carries_a_named_disposition_a_reason_and_a_resolvable_source_line() {
    let archived_files: BTreeSet<String> = manifest_entries()
        .iter()
        .map(|entry| manifest_field(entry, "file"))
        .collect();
    let mut lengths: BTreeMap<String, usize> = BTreeMap::new();

    let mut broken = Vec::new();
    for row in rows() {
        if !DISPOSITIONS.contains(&row.disposition.as_str()) {
            broken.push(format!(
                "selection.md:{} `{}` is dispositioned `{}`, which is not one of {:?}",
                row.line, row.key, row.disposition, DISPOSITIONS
            ));
        }
        if row.reason.split_whitespace().count() < 8 {
            broken.push(format!(
                "selection.md:{} `{}` carries no reason, only {:?}",
                row.line, row.key, row.reason
            ));
        }
        let cited = citations(&row.defined_by);
        if cited.is_empty() {
            broken.push(format!(
                "selection.md:{} `{}` cites no source line into a pinned revision: {:?}",
                row.line, row.key, row.defined_by
            ));
        }
        for (file, line) in cited {
            if !archived_files.contains(&file) {
                broken.push(format!(
                    "selection.md:{} `{}` cites `{file}`, which the pin's manifest does \
                     not archive",
                    row.line, row.key
                ));
                continue;
            }
            let length = *lengths
                .entry(file.clone())
                .or_insert_with(|| archived(&file).lines().count());
            if line == 0 || line > length {
                broken.push(format!(
                    "selection.md:{} `{}` cites `{file}:{line}`, but that archived file \
                     has {length} lines",
                    row.line, row.key
                ));
            }
        }
    }
    assert!(
        broken.is_empty(),
        "{} matrix rows are not evidence-bearing:\n  {}",
        broken.len(),
        broken.join("\n  ")
    );
}

/// Nested capability settings are not dropped on the way to a row.
///
/// A top-level capability takes a row; `resources.subscribe` and its fifteen siblings are
/// settings of an advertised capability rather than separately advertisable capabilities,
/// so the matrix names them inside the rows that carry them. This case holds that
/// distinction to the same standard as the rows: every nested field either pinned schema
/// declares must appear in the document as an inline-code path, which a longer path
/// containing it as a substring does not satisfy.
#[test]
fn every_nested_capability_setting_the_pinned_schemas_declare_is_named_by_the_matrix() {
    let document = matrix();
    let (_, nested) = derived_capabilities();
    assert!(
        !nested.is_empty(),
        "no nested capability setting was derived; the schema parse is broken"
    );
    let unnamed: Vec<&String> = nested
        .iter()
        .filter(|path| !document.contains(&format!("`{path}`")))
        .collect();
    assert!(
        unnamed.is_empty(),
        "{} of the {} nested capability settings the pinned schemas declare are named \
         nowhere in the matrix: {:?}",
        unnamed.len(),
        nested.len(),
        unnamed
    );
}

/// A row whose reason rests on an open decision-blocker is `deferred`.
///
/// The brief for this unit states the rule and the failure it guards: such a row "is not
/// `refused`, and it is certainly not `supported`". Both blockers are open, so a
/// selection resting on either is a selection nobody has made.
#[test]
fn no_row_resting_on_an_open_decision_blocker_claims_support_or_refusal() {
    let mispositioned: Vec<String> = rows()
        .into_iter()
        .filter(|row| row.reason.contains("decision-blocker:"))
        .filter(|row| row.disposition != "deferred")
        .map(|row| {
            format!(
                "selection.md:{} `{}` ({}) rests on an open blocker and is dispositioned `{}`",
                row.line, row.key, row.direction, row.disposition
            )
        })
        .collect();
    assert!(
        mispositioned.is_empty(),
        "{} rows name an open decision-blocker without deferring:\n  {}",
        mispositioned.len(),
        mispositioned.join("\n  ")
    );
}

/// No revision is supported whose bytes this repository has not pinned.
///
/// The derived revision list is deliberately wider than the pin: three of the strings it
/// yields come from a schema constant left at a draft value, from initialization examples
/// and from a header example, which is the trap `specification-sources.md` records. A row
/// that read one of those as this repository's selection would be the exact defect that
/// record was written to prevent, and `specification-sources.md` "## Limits" holds that
/// without archived bytes a statement about a revision is a hypothesis.
#[test]
fn no_revision_outside_the_pin_is_dispositioned_supported() {
    let pinned: BTreeSet<String> = manifest_entries()
        .iter()
        .map(|entry| manifest_field(entry, "revision"))
        .collect();
    let overreaching: Vec<String> = rows()
        .into_iter()
        .filter(|row| row.key.starts_with("revision:"))
        .filter(|row| !pinned.contains(row.key.trim_start_matches("revision:")))
        .filter(|row| row.disposition == "supported")
        .map(|row| {
            format!(
                "selection.md:{} `{}` is supported, but no bytes of it are archived; \
                 pinned revisions are {pinned:?}",
                row.line, row.key
            )
        })
        .collect();
    assert!(
        overreaching.is_empty(),
        "{} revision rows claim support without a pin:\n  {}",
        overreaching.len(),
        overreaching.join("\n  ")
    );
}

/// The document says, in its own text, that a disposition is a selection rather than an
/// implementation claim.
///
/// This is a presence check and is worth exactly what a presence check is worth: it stops
/// the statement being deleted quietly, the way `mcp_domain_model_census.rs` holds the
/// no-selection disclaimer in `domains/protocol.yaml`. It cannot stop the document
/// claiming implementation elsewhere.
#[test]
fn the_matrix_states_that_a_supported_row_is_a_selection_not_implemented_support() {
    let document = matrix();
    let normalised = document.split_whitespace().collect::<Vec<_>>().join(" ");
    for required in [
        "Do not equate a protocol capability listing with implemented support.",
        "No row of this matrix is evidence that any MCP server, including a future Connectors one, implements anything.",
    ] {
        assert!(
            normalised.contains(required),
            "selection.md does not carry the sentence {required:?}"
        );
    }
}

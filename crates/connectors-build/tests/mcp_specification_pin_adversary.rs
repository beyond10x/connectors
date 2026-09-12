//! Adversarial cases for `story:mcp-specification-pin`, written 2026-09-12 against
//! commit 4192883 on branch `unit/mcp-specification-pin-20260912`.
//!
//! The unit shipped a pinned-source record and the bytes it describes in one commit.
//! Nothing in this repository compares the two, so these cases drive the record —
//! `adapters/mcp/contracts/protocol/v1alpha1/evidence/20260912/specification-sources.md`
//! and its manifest — against the archived bytes it pins. Every statement asserted here
//! is a statement the record makes about a file whose bytes are in the same directory.
//!
//! Archives are read with `gzip -dc`, the same command the record's own reproduction
//! snippet uses; the platform is Linux x86_64 per `initiative:complete-local-connectors`.

use std::path::{Path, PathBuf};
use std::process::Command;

const REVISION_PRIMARY: &str = "2026-07-28";
const REVISION_INTEROP: &str = "2025-11-25";
const URL_PREFIX: &str =
    "https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/";

fn evidence_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../adapters/mcp/contracts/protocol/v1alpha1/evidence/20260912")
}

fn record() -> String {
    let path = evidence_dir().join("specification-sources.md");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn manifest() -> serde_json::Value {
    let path = evidence_dir().join("specification-source-hashes.json");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_slice(&bytes).expect("specification-source-hashes.json is not JSON")
}

fn entries() -> Vec<serde_json::Value> {
    manifest()
        .as_array()
        .expect("specification-source-hashes.json is not a JSON array")
        .clone()
}

fn field<'a>(entry: &'a serde_json::Value, key: &str) -> &'a str {
    entry[key]
        .as_str()
        .unwrap_or_else(|| panic!("manifest entry is missing a string `{key}`: {entry}"))
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

/// Line numbers the record cites for one archived file, as `name` (a, b, c).
fn cited_lines(row: &str, file: &str) -> Vec<usize> {
    let needle = format!("`{file}` (");
    let start = match row.find(&needle) {
        Some(at) => at + needle.len(),
        None => return Vec::new(),
    };
    let end = start + row[start..].find(')').expect("unterminated citation list");
    row[start..end]
        .split(',')
        .filter_map(|n| n.trim().parse::<usize>().ok())
        .collect()
}

/// The record tells eleven downstream stories where the interoperability revision names
/// its own version string. Every archived `.mdx` of that revision that carries a
/// `**Protocol Revision**: 2025-11-25` banner is a self-identifying declaration and not a
/// path, so the record's description has to agree with the archives.
///
/// As first written this case asserted the record's *false* claim — that the string
/// appears only in documentation paths — guarded by the claim still being present. Both
/// halves cannot hold at once for any record: the 18 banners are upstream bytes that a
/// correction cannot remove, so the body could only pass if the archives changed, and
/// withdrawing the claim trips the guard instead. The case now asserts what the archives
/// show and that the record says it, which is red for the original record and green for a
/// corrected one.
///
/// `specification-sources.md`, the version-string section.
#[test]
fn interop_revision_names_its_own_version_string_outside_a_documentation_path() {
    let record = record();
    let normalised = record.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        !normalised.contains("appears only in documentation paths"),
        "the record still claims the string appears only in documentation paths; the \
         archived documents below say otherwise"
    );

    let mut declarations = Vec::new();
    for entry in entries() {
        if field(&entry, "revision") != REVISION_INTEROP {
            continue;
        }
        let file = field(&entry, "file").to_string();
        if !file.ends_with(".mdx") {
            continue;
        }
        for (index, line) in archived(&file).lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.contains("Protocol Revision") && trimmed.contains(REVISION_INTEROP) {
                declarations.push(format!("{file}:{}: {trimmed}", index + 1));
            }
        }
    }

    let declaring: std::collections::BTreeSet<&str> = declarations
        .iter()
        .filter_map(|line| line.split(':').next())
        .collect();
    assert!(
        !declaring.is_empty(),
        "no archived {REVISION_INTEROP} document declares its own version string; the \
         manifest or the archives changed and this case is describing neither"
    );
    // The count comes from the archives, so removing archived documents from the manifest
    // cannot quietly satisfy this case; it moves the number the record has to state.
    assert!(
        normalised.contains("Protocol Revision")
            && normalised.contains(&format!("{} of the", declaring.len())),
        "the record does not state that {} archived documents of the {REVISION_INTEROP} \
         revision declare the version string in a Protocol Revision banner:\n{}",
        declaring.len(),
        declarations.join("\n")
    );
}

/// RED. The record's citation table has one row for the interoperability seam. That
/// revision negotiates its version in `#### Version Negotiation` of its lifecycle
/// document; the row cites three other sections of the same file and not that one, so a
/// later contract statement about the seam has no cited line for the rule it needs.
///
/// `specification-sources.md:81`.
#[test]
fn version_negotiation_row_cites_the_interop_revisions_negotiation_section() {
    let lifecycle = format!("mcp-{REVISION_INTEROP}-basic-lifecycle.mdx");
    let source = archived(&lifecycle);
    let negotiation = source
        .lines()
        .position(|line| {
            line.trim_start_matches('#').trim() == "Version Negotiation" && line.starts_with('#')
        })
        .map(|index| index + 1)
        .expect("the archived interoperability lifecycle has no Version Negotiation heading");

    let record = record();
    let row = record
        .lines()
        .find(|line| {
            line.contains(&format!(
                "Version negotiation and the {REVISION_INTEROP} seam"
            ))
        })
        .expect("the record has no version-negotiation row");
    let cited = cited_lines(row, &lifecycle);

    assert!(
        cited.contains(&negotiation),
        "specification-sources.md:81 names version negotiation for the {REVISION_INTEROP} seam \
         but cites {lifecycle} lines {cited:?}; that revision specifies version negotiation at \
         line {negotiation} ({:?}), which the record cites nowhere",
        source.lines().nth(negotiation - 1).unwrap_or_default()
    );
}

/// GREEN guard. The record's reproduction snippet lives only in a fenced block; no code
/// in this repository runs it. This executes it, so an altered archive or an altered
/// recorded digest becomes a red suite rather than an unread document.
#[test]
fn every_recorded_digest_and_byte_length_rederives_from_the_archived_bytes() {
    let entries = entries();
    assert_eq!(
        entries.len(),
        54,
        "the pin no longer holds 54 archived sources"
    );

    let mut wrong = Vec::new();
    for entry in &entries {
        let file = field(entry, "file");
        let archive = evidence_dir().join(field(entry, "archive"));
        assert!(archive.is_file(), "recorded archive is missing: {file}");
        let bytes = archived(file);
        let recorded = entry["bytes"]
            .as_u64()
            .expect("recorded bytes is not a number");
        if bytes.len() as u64 != recorded {
            wrong.push(format!(
                "{file}: recorded {recorded} bytes, archive holds {}",
                bytes.len()
            ));
        }
        let digest = Command::new("sha256sum")
            .arg(&archive)
            .output()
            .expect("run sha256sum");
        assert!(digest.status.success(), "sha256sum failed for {file}");
        // sha256sum reads the compressed file; compare the uncompressed stream instead.
        let stream = Command::new("bash")
            .arg("-c")
            .arg(format!(
                "gzip -dc {} | sha256sum | cut -d' ' -f1",
                archive.display()
            ))
            .output()
            .expect("run gzip | sha256sum");
        let got = String::from_utf8_lossy(&stream.stdout).trim().to_string();
        if got != field(entry, "sha256") {
            wrong.push(format!(
                "{file}: recorded sha256 {}, archive hashes to {got}",
                field(entry, "sha256")
            ));
        }
    }

    let archived_files = std::fs::read_dir(evidence_dir().join("vendor"))
        .expect("read vendor/")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().ends_with(".gz"))
        .count();
    assert_eq!(
        archived_files,
        entries.len(),
        "vendor/ holds {archived_files} archives for {} manifest entries",
        entries.len()
    );
    assert!(
        wrong.is_empty(),
        "recorded digests drifted:\n{}",
        wrong.join("\n")
    );
}

/// GREEN guard. The reproduction snippet compares digests, lengths and archive names and
/// reads none of `url`, `path` or `commit`. Those three carry the whole provenance claim,
/// so an entry that names another revision's path and commit passes the documented
/// command unchanged. This asserts the three agree with the revision the entry claims.
#[test]
fn manifest_provenance_fields_agree_with_the_revision_each_entry_claims() {
    let mut broken = Vec::new();
    for entry in entries() {
        let revision = field(&entry, "revision").to_string();
        let file = field(&entry, "file").to_string();
        let path = field(&entry, "path").to_string();
        let commit = field(&entry, "commit").to_string();
        let url = field(&entry, "url").to_string();

        let under_revision = path.starts_with(&format!("docs/specification/{revision}/"))
            || path.starts_with(&format!("schema/{revision}/"));
        if !under_revision {
            broken.push(format!("{file}: revision {revision} but path {path}"));
        }
        if url != format!("{URL_PREFIX}{commit}/{path}") {
            broken.push(format!(
                "{file}: url does not derive from commit and path: {url}"
            ));
        }
        if field(&entry, "archive") != format!("vendor/{file}.gz") {
            broken.push(format!(
                "{file}: archive name does not follow the file name"
            ));
        }
        match revision.as_str() {
            REVISION_PRIMARY | REVISION_INTEROP => {}
            other => broken.push(format!("{file}: unpinned revision {other}")),
        }
    }

    // One commit per revision, and the record names which.
    let mut commits: Vec<(String, String)> = entries()
        .iter()
        .map(|e| {
            (
                field(e, "revision").to_string(),
                field(e, "commit").to_string(),
            )
        })
        .collect();
    commits.sort();
    commits.dedup();
    assert_eq!(
        commits.len(),
        2,
        "expected one commit per pinned revision, found {commits:?}"
    );
    let record = record();
    for (revision, commit) in &commits {
        assert!(
            record.contains(commit.as_str()),
            "the record does not name the commit {commit} that the manifest pins for {revision}"
        );
    }

    assert!(
        broken.is_empty(),
        "provenance fields disagree:\n{}",
        broken.join("\n")
    );
}

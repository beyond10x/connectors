//! Adversary pass 2 over `story:mcp-profile-selection-matrix`, written 2026-09-12 against
//! `cc7b121` on branch `unit/mcp-profile-selection-matrix-20260912`.
//!
//! Pass 1 attacked the revision enumeration and the `deferred` definition, and the
//! correction at `43980ac` made the revision scan exhaustive over version-shaped tokens
//! and enforced both halves of `deferred`. Two things that correction did not make
//! mechanical are what these two cases read, each against an authority the unit did not
//! write:
//!
//! 1. **The set of optional extensions the pinned revisions identify.** Both `extensions`
//!    rows of the matrix are `deferred` on a stated enumeration — "the only one either
//!    pinned revision names for this model is the tasks extension", "the only named
//!    instance is the tasks extension". `mcp_profile_selection_matrix.rs` derives
//!    capabilities from the two capability *interfaces* and never reads an extension
//!    identifier, so nothing compares that enumeration against the archives. The
//!    archives name two, by the same sentence form, fifteen lines apart, in the range
//!    the client row cites as its own source.
//!
//! 2. **The refusal a caller observes, across two rows that describe one wire input.**
//!    The matrix says nothing holds the rule that an `explicitly refused` row states an
//!    observable refusal, and leaves it to review. It is not only the quality of a
//!    refusal cell that is unheld: nothing compares a refusal against a `supported` row
//!    of the same document. `revision:2025-03-26` inbound refuses every request that
//!    omits the `MCP-Protocol-Version` header; `revision:2025-11-25` inbound supports a
//!    legacy `initialize`, which by that revision's own pinned transport document is
//!    exactly such a request.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

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

/// Uncompressed bytes of one archived source, as text — the `gzip -dc` the pinned record's
/// own reproduction snippet uses.
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

/// The file's lines joined by single spaces, with the 1-based source line of every joined
/// position kept, so a match in the joined text can be cited at its real line. A sentence
/// in an `.mdx` source is wrapped, so a phrase the specification states normatively
/// appears on no single line of the archived bytes.
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
    let mut line = starts.first().map(|(_, line)| *line).unwrap_or(1);
    for (start, at) in starts {
        if *start <= offset {
            line = *at;
        } else {
            break;
        }
    }
    line
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

// ── Case 1: the extensions the pinned revisions identify ────────────────────────────

/// One optional extension the pinned bytes name by identifier.
#[derive(Debug, Clone)]
struct NamedExtension {
    /// The display name the specification links, e.g. `MCP Apps`.
    name: String,
    /// The advertised key, e.g. `io.modelcontextprotocol/ui`.
    identifier: String,
    /// `<archived file>:<line>` of the sentence that identifies it.
    at: String,
}

/// Every extension the pinned revisions name *by identifier*, derived structurally.
///
/// The rule is a property of where the token sits, in the shape the matrix uses for its
/// own three derivations: the specification introduces an extension with the sentence
/// form ``[<Name> extension](<link>) identified as `<identifier>` ``, and
/// `mcp-2026-07-28-basic-versioning.mdx` "## Extension Negotiation" is where it does so.
/// Nothing here is a denylist or an allowlist of strings.
fn extensions_the_archives_identify() -> Vec<NamedExtension> {
    let sentence =
        regex::Regex::new(r"\[([^\]]+) extension\]\([^)]*\) identified as `([^`]+)`").unwrap();
    let mut found = Vec::new();
    let mut seen = BTreeSet::new();
    for file in manifest_files() {
        let (flat, starts) = flattened(&archived(&file));
        for capture in sentence.captures_iter(&flat) {
            let whole = capture.get(0).expect("capture 0 always exists");
            let identifier = capture[2].to_string();
            if !seen.insert(identifier.clone()) {
                continue;
            }
            found.push(NamedExtension {
                name: capture[1].to_string(),
                identifier,
                at: format!("{file}:{}", line_of(&starts, whole.start())),
            });
        }
    }
    found
}

/// The matrix defers both `extensions` rows on an enumeration of the extensions the
/// pinned revisions name, and states that enumeration has one member. It has two.
///
/// This is the matrix's own claim, made countable. It is not a demand that every
/// extension take a row: `extensions` is a map of identifiers, and the matrix's stated
/// rule that a map's keys are not separately advertisable capabilities is defensible.
/// What is not defensible is a `deferred` whose stated ground is that exactly one
/// instance is named, written beside a citation of the other one's lines.
///
/// The oracle survives its own fix. Deleting the word "only" does not satisfy it —
/// the second identifier is still named nowhere in the document. Naming it, and saying
/// what this repository does with it, does.
#[test]
fn every_extension_the_pinned_revisions_identify_is_named_by_the_matrix() {
    let identified = extensions_the_archives_identify();
    assert!(
        identified.len() >= 2,
        "the archives yielded {identified:?}, so the scan for identified extensions is \
         broken and this case proves nothing"
    );

    let document = matrix();
    let lowered = document.to_lowercase();
    let unnamed: Vec<&NamedExtension> = identified
        .iter()
        .filter(|extension| {
            let by_identifier = document.contains(&extension.identifier);
            let by_name = lowered.contains(&format!("{} extension", extension.name.to_lowercase()));
            !(by_identifier || by_name)
        })
        .collect();

    let uniqueness_claims: Vec<String> = rows()
        .into_iter()
        .filter(|row| row.key.ends_with("/extensions"))
        .map(|row| {
            format!(
                "selection.md:{} `{}` ({}) is `{}` because {:?}",
                row.line,
                row.key,
                row.direction,
                row.disposition,
                row.reason.chars().take(210).collect::<String>()
            )
        })
        .collect();

    assert!(
        unnamed.is_empty(),
        "the pinned revisions identify {} optional extensions and the matrix names \
         {} of them; {} is named nowhere in selection.md, neither by identifier nor by \
         name:\n  identified by the archives:\n    {}\n  named by no line of the \
         matrix:\n    {}\n  while the rows that rest on that enumeration say:\n    {}",
        identified.len(),
        identified.len() - unnamed.len(),
        unnamed.len(),
        identified
            .iter()
            .map(|extension| format!(
                "{} `{}` at {}",
                extension.name, extension.identifier, extension.at
            ))
            .collect::<Vec<_>>()
            .join("\n    "),
        unnamed
            .iter()
            .map(|extension| format!(
                "{} `{}` at {}",
                extension.name, extension.identifier, extension.at
            ))
            .collect::<Vec<_>>()
            .join("\n    "),
        uniqueness_claims.join("\n    "),
    );
}

// ── Case 2: one wire input, two dispositions ────────────────────────────────────────

/// The revision the primary pin calls the last `legacy` one — the newest revision that
/// establishes a session with an `initialize` handshake — read from its own terminology.
fn newest_legacy_revision() -> (String, String) {
    let file = "mcp-2026-07-28-basic-versioning.mdx";
    let (flat, starts) = flattened(&archived(file));
    let bullet = regex::Regex::new(
        r"\*\*Legacy\*\*: protocol versions that establish a session with an `initialize` handshake \(`(\d{4}-\d{2}-\d{2})` and earlier\)",
    )
    .unwrap();
    let capture = bullet.captures(&flat).unwrap_or_else(|| {
        panic!("{file} no longer states which revisions are legacy; this case proves nothing")
    });
    let whole = capture.get(0).expect("capture 0 always exists");
    (
        capture[1].to_string(),
        format!("{file}:{}", line_of(&starts, whole.start())),
    )
}

/// Where the legacy revision's own transport document puts the `MCP-Protocol-Version`
/// header: on requests *subsequent* to initialization, not on the `initialize` itself.
fn legacy_header_rule(revision: &str) -> (String, String) {
    let file = format!("mcp-{revision}-basic-transports.mdx");
    let (flat, starts) = flattened(&archived(&file));
    let rule = regex::Regex::new(
        r"client \*\*MUST\*\* include the `MCP-Protocol-Version:[^`]*` HTTP header on ([a-z ]+) requests",
    )
    .unwrap();
    let capture = rule.captures(&flat).unwrap_or_else(|| {
        panic!("{file} no longer states when the header is required; this case proves nothing")
    });
    let whole = capture.get(0).expect("capture 0 always exists");
    (
        capture[1].trim().to_string(),
        format!("{file}:{}", line_of(&starts, whole.start())),
    )
}

/// A refusal of every header-less request and a support for the legacy `initialize` are
/// two dispositions of one wire input.
///
/// The matrix's covering property is that each feature gets one disposition per
/// direction. Over keys it holds. Over the bytes on the wire it does not: a POST to the
/// inbound MCP endpoint carrying `{"method":"initialize","params":{"protocolVersion":
/// "<legacy>"}}` and no `MCP-Protocol-Version` header is refused `400 Bad Request` /
/// `HeaderMismatch -32020` by one row and answered by another. The primary revision's own
/// sentence scopes the reject-or-assume choice to clients "implementing protocol versions
/// earlier than `2025-06-18`", which the supported legacy revision is not.
///
/// The oracle survives its own fix: it passes when the refusing cell names the exception
/// — the `initialize`, or the legacy revision it does not reach — and it passes if the
/// legacy revision stops being supported inbound. It does not pass on a reworded refusal
/// that still refuses everything.
#[test]
fn the_inbound_refusal_of_a_header_less_request_excepts_the_legacy_initialize_it_supports() {
    let (legacy, legacy_at) = newest_legacy_revision();
    let (when_required, header_at) = legacy_header_rule(&legacy);
    assert!(
        when_required.contains("subsequent"),
        "{header_at} requires the header on {when_required:?} requests, not on subsequent \
         ones, so the legacy `initialize` may carry it after all and this case proves \
         nothing"
    );

    let rows = rows();
    let refusing: Vec<&Row> = rows
        .iter()
        .filter(|row| row.direction == "inbound" || row.direction == "both")
        .filter(|row| {
            row.disposition == "explicitly refused"
                && row
                    .reason
                    .contains("missing the MCP-Protocol-Version header")
        })
        .collect();
    let supported_legacy: Vec<&Row> = rows
        .iter()
        .filter(|row| row.key == format!("revision:{legacy}"))
        .filter(|row| row.direction == "inbound" || row.direction == "both")
        .filter(|row| row.disposition == "supported")
        .collect();

    // Liveness, held against something no fix to this defect removes: the legacy revision
    // is still a key of this matrix. The assertion below is an implication and passes
    // either way once the conflict is gone — by excepting the `initialize` in the refusal,
    // by dropping the refusal, or by withdrawing inbound support for the legacy revision.
    assert!(
        rows.iter()
            .any(|row| row.key == format!("revision:{legacy}")),
        "the matrix dispositions no `revision:{legacy}` row at all, so this case has \
         nothing to compare and proves nothing"
    );

    let excepted = |row: &Row| row.reason.contains("initialize") || row.reason.contains(&legacy);
    let unexcepted: Vec<&&Row> = refusing
        .iter()
        .filter(|_| !supported_legacy.is_empty())
        .filter(|row| !excepted(row))
        .collect();

    assert!(
        unexcepted.is_empty(),
        "{} inbound refusal(s) of a request that omits the `MCP-Protocol-Version` header \
         reach the legacy `initialize` the same matrix supports, and except it \
         nowhere:\n  refuses, naming neither the `initialize` nor `{legacy}`:\n    {}\n  \
         supports, over a transport this matrix also selects:\n    {}\n  the pinned bytes \
         that make these one wire input:\n    {legacy_at} — `{legacy}` and earlier are \
         the legacy revisions, the ones that establish a session with an `initialize` \
         handshake\n    {header_at} — that revision requires the \
         `MCP-Protocol-Version` header on {when_required:?} requests, so its `initialize` \
         carries none",
        unexcepted.len(),
        unexcepted
            .iter()
            .map(|row| format!(
                "selection.md:{} `{}` ({}) — {:?}",
                row.line,
                row.key,
                row.direction,
                row.reason
                    .split_once("takes the alternative")
                    .map(|(_, tail)| tail.trim().to_string())
                    .unwrap_or_else(|| row.reason.clone())
            ))
            .collect::<Vec<_>>()
            .join("\n    "),
        supported_legacy
            .iter()
            .map(|row| format!(
                "selection.md:{} `{}` ({}) — {:?}",
                row.line,
                row.key,
                row.direction,
                row.reason.chars().take(150).collect::<String>()
            ))
            .collect::<Vec<_>>()
            .join("\n    "),
    );
}

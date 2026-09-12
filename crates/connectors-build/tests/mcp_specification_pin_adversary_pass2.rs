//! Adversarial cases, pass 2, for `story:mcp-specification-pin`, written 2026-09-12
//! against commit `1e15b49` (correction round 1) of branch
//! `unit/mcp-specification-pin-20260912`.
//!
//! Pass 1 drove the record against the bytes it pins. The correction round answered it
//! by rewriting the version-string paragraph, adding a transport-family count to
//! `adapters/mcp/design.md`, and adding a documented `jq` provenance program to the
//! record. These cases drive those *corrected* documents against the same archived
//! bytes, and drive the record's own provenance program against manifests it should
//! refuse.
//!
//! Nothing here restates a rule the record states. The provenance program under test is
//! extracted from the fenced block of `specification-sources.md`, so strengthening the
//! record strengthens these cases with it, and the counts are derived from the archives
//! rather than written down here.
//!
//! Mutated manifests are built under `CARGO_TARGET_TMPDIR`. No file under version
//! control is written by any case in this file.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const REVISION_PRIMARY: &str = "2026-07-28";
const REVISION_INTEROP: &str = "2025-11-25";
const URL_PREFIX: &str =
    "https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/";

fn adapter_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/mcp")
}

fn evidence_dir() -> PathBuf {
    adapter_dir().join("contracts/protocol/v1alpha1/evidence/20260912")
}

fn read_text(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn record() -> String {
    read_text(&evidence_dir().join("specification-sources.md"))
}

fn design() -> String {
    read_text(&adapter_dir().join("design.md"))
}

fn manifest() -> serde_json::Value {
    let path = evidence_dir().join("specification-source-hashes.json");
    serde_json::from_slice(
        &std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display())),
    )
    .expect("specification-source-hashes.json is not JSON")
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

/// Uncompressed bytes of one archived source, as text. Same command the record's own
/// reproduction snippet uses.
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

/// Archived `.mdx` files of one revision, in manifest order.
fn documents(revision: &str) -> Vec<String> {
    entries()
        .iter()
        .filter(|e| field(e, "revision") == revision)
        .map(|e| field(e, "file").to_string())
        .filter(|f| f.ends_with(".mdx"))
        .collect()
}

/// Archived `.mdx` files of the interoperability revision that open with a
/// `**Protocol Revision**: 2025-11-25` banner. Derived from the bytes, never written
/// down, so the set moves when the archives move.
fn banner_documents() -> BTreeSet<String> {
    let mut carrying = BTreeSet::new();
    for file in documents(REVISION_INTEROP) {
        let has_banner = archived(&file).lines().any(|line| {
            let trimmed = line.trim();
            trimmed.contains("Protocol Revision") && trimmed.contains(REVISION_INTEROP)
        });
        if has_banner {
            carrying.insert(file);
        }
    }
    carrying
}

/// The provenance program the record documents, lifted out of its fenced block. Running
/// the extracted text rather than a restatement of it is what makes these cases drive
/// the document: a record that strengthens the program strengthens the case.
fn documented_provenance_program() -> String {
    let record = record();
    let open = "jq -e -r '";
    let start = record
        .find(open)
        .expect("the record documents no `jq -e -r '...'` provenance program")
        + open.len();
    let rest = &record[start..];
    let end = rest
        .find("' specification-source-hashes.json")
        .expect("the documented jq provenance program is unterminated");
    rest[..end].to_string()
}

/// Run the record's documented provenance program over an arbitrary manifest value.
fn provenance_verdict(tag: &str, manifest: &serde_json::Value) -> Output {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("mcp-pin-pass2-{tag}"));
    std::fs::create_dir_all(&dir).expect("create case scratch directory");
    let program = dir.join("provenance.jq");
    std::fs::write(&program, documented_provenance_program()).expect("write jq program");
    let subject = dir.join("specification-source-hashes.json");
    std::fs::write(
        &subject,
        serde_json::to_vec_pretty(manifest).expect("serialise manifest"),
    )
    .expect("write manifest");
    Command::new("jq")
        .args(["-e", "-r", "-f"])
        .arg(&program)
        .arg(&subject)
        .output()
        .unwrap_or_else(|e| {
            panic!(
                "run jq: {e}. The record documents its provenance check as a jq program, \
                 so a machine that cannot run jq cannot verify this pin at all."
            )
        })
}

fn describe(output: &Output) -> String {
    format!(
        "exit={} stdout={:?} stderr={:?}",
        output.status,
        String::from_utf8_lossy(&output.stdout).trim(),
        String::from_utf8_lossy(&output.stderr).trim()
    )
}

/// RED. `adapters/mcp/design.md` bounds the transport option space for the later
/// transport-selection story: it says the pinned revisions define *three* transport
/// families and names them. The pinned revisions specify a fourth binding by name —
/// HTTP+SSE from `2024-11-05` — with its own section in the primary revision, its own
/// fallback procedure in both revisions, and a row in the primary revision's deprecated
/// registry that keeps it part of the specification rather than removed from it.
///
/// The count is the sentence a selection matrix reads, and it was already wrong once
/// (pass 1 found it reading "two"). `adapters/mcp/design.md:36-42`.
#[test]
fn design_document_accounts_for_every_transport_binding_the_pinned_revisions_specify() {
    let mut headings = Vec::new();
    let mut interop_mentions = Vec::new();
    for revision in [REVISION_PRIMARY, REVISION_INTEROP] {
        for file in documents(revision) {
            for (index, line) in archived(&file).lines().enumerate() {
                let trimmed = line.trim();
                if !trimmed.contains("HTTP+SSE") {
                    continue;
                }
                if trimmed.starts_with('#') {
                    headings.push(format!("{file}:{}: {trimmed}", index + 1));
                } else if revision == REVISION_INTEROP {
                    interop_mentions.push(format!("{file}:{}: {trimmed}", index + 1));
                }
            }
        }
    }
    // Guards: the binding is specified by the primary revision under its own heading and
    // is named by the interoperability revision too. If the archives ever stop saying
    // that, this case is describing neither document and must say so rather than fail.
    assert!(
        !headings.is_empty(),
        "no archived document gives the HTTP+SSE binding its own heading; the archives \
         changed and this case is describing neither document"
    );
    assert!(
        !interop_mentions.is_empty(),
        "the {REVISION_INTEROP} revision no longer names the HTTP+SSE binding; the \
         archives changed and this case is describing neither document"
    );

    // Guard: it is a live binding of the pinned revision, not a historical note — the
    // deprecated registry lists it under `## Deprecated`, above `## Removed`.
    let registry_file = format!("mcp-{REVISION_PRIMARY}-deprecated.mdx");
    let registry = archived(&registry_file);
    let registry_lines: Vec<&str> = registry.lines().collect();
    let deprecated_at = registry_lines
        .iter()
        .position(|line| line.trim() == "## Deprecated")
        .expect("the deprecated registry has no `## Deprecated` section");
    let removed_at = registry_lines
        .iter()
        .position(|line| line.trim() == "## Removed")
        .expect("the deprecated registry has no `## Removed` section");
    let row_at = registry_lines
        .iter()
        .position(|line| line.contains("HTTP+SSE transport"))
        .expect("the deprecated registry does not list the HTTP+SSE transport");
    assert!(
        deprecated_at < row_at && row_at < removed_at,
        "{registry_file}:{} no longer lists the HTTP+SSE transport as Deprecated; the \
         archives changed and this case is describing neither document",
        row_at + 1
    );

    let design = design();
    assert!(
        design.contains("HTTP+SSE") || design.contains("2024-11-05"),
        "adapters/mcp/design.md states the pinned revisions define three transport \
         families and names stdio, Streamable HTTP and custom transports. It never names \
         the fourth binding those revisions specify:\n{}\n{}\n{registry_file}:{} lists it \
         as Deprecated and not Removed, so it is still part of the pinned specification, \
         and the interoperability revision — pinned for exactly this seam — tells a \
         client to fall back to it.",
        headings.join("\n"),
        interop_mentions.join("\n"),
        row_at + 1
    );
}

/// RED. The correction added a documented provenance program and the record says it
/// pins "one commit per revision". It counts the revision-and-commit pairs and never
/// binds a revision to *its* commit, so exchanging the two commits between the two
/// revisions — and rederiving every url from the swapped commit, as the program
/// requires — leaves exactly two pairs and passes. Every url then names a tree that is
/// not the one the archived bytes came from, and the record says the network is no
/// longer consulted, so nothing downstream can notice.
///
/// `specification-sources.md:60-73`, the `# 2. provenance` block.
#[test]
fn documented_provenance_check_refuses_a_wholesale_revision_to_commit_swap() {
    // Guard: the extracted program accepts the manifest as committed. A red below is
    // then the mutation and not a broken extraction.
    let clean = provenance_verdict("clean-swap-guard", &manifest());
    assert!(
        clean.status.success(),
        "the provenance program extracted from the record refuses the manifest as \
         committed, so this case cannot distinguish a mutation: {}",
        describe(&clean)
    );

    let commits: BTreeSet<String> = entries()
        .iter()
        .map(|e| field(e, "commit").to_string())
        .collect();
    let commits: Vec<String> = commits.into_iter().collect();
    assert_eq!(
        commits.len(),
        2,
        "the manifest no longer pins exactly two commits: {commits:?}"
    );

    let mut swapped = manifest();
    for entry in swapped
        .as_array_mut()
        .expect("manifest is not a JSON array")
        .iter_mut()
    {
        let commit = field(entry, "commit").to_string();
        let path = field(entry, "path").to_string();
        let exchanged = if commit == commits[0] {
            commits[1].clone()
        } else {
            commits[0].clone()
        };
        entry["url"] = serde_json::Value::from(format!("{URL_PREFIX}{exchanged}/{path}"));
        entry["commit"] = serde_json::Value::from(exchanged);
    }
    assert_ne!(
        swapped,
        manifest(),
        "the swap changed nothing; this case is not testing what it claims"
    );

    let verdict = provenance_verdict("commit-swap", &swapped);
    assert!(
        !verdict.status.success(),
        "the provenance program documented at specification-sources.md:60-73 accepts a \
         manifest in which the {REVISION_PRIMARY} entries claim the {REVISION_INTEROP} \
         commit and the {REVISION_INTEROP} entries claim the {REVISION_PRIMARY} commit. \
         It counts revision-and-commit pairs and never binds a revision to its own \
         commit, so every recorded url now names the wrong tree: {}",
        describe(&verdict)
    );
}

/// RED. The same program checks that `archive` derives from `file` and that `path` lies
/// under the revision the entry claims. It never checks that `file` derives from `path`,
/// which is the naming rule the record states in its own words at
/// `specification-sources.md:104-109`. So an entry repointed at a *different file of its
/// own revision* — url rederived — passes the documented provenance check and the
/// documented content check alike, and the recorded url stops being the source url of
/// the archived bytes the acceptance requires it to be.
#[test]
fn documented_provenance_check_refuses_a_path_repointed_within_its_own_revision() {
    let clean = provenance_verdict("clean-repoint-guard", &manifest());
    assert!(
        clean.status.success(),
        "the provenance program extracted from the record refuses the manifest as \
         committed, so this case cannot distinguish a mutation: {}",
        describe(&clean)
    );

    let same_revision: Vec<serde_json::Value> = entries()
        .into_iter()
        .filter(|e| field(e, "revision") == REVISION_PRIMARY)
        .collect();
    assert!(
        same_revision.len() >= 2,
        "the manifest holds fewer than two {REVISION_PRIMARY} entries"
    );
    let victim = field(&same_revision[0], "file").to_string();
    let donor_path = field(&same_revision[1], "path").to_string();
    assert_ne!(
        field(&same_revision[0], "path"),
        donor_path,
        "the two entries chosen name the same path; this case is not testing what it \
         claims"
    );

    let mut repointed = manifest();
    for entry in repointed
        .as_array_mut()
        .expect("manifest is not a JSON array")
        .iter_mut()
    {
        if field(entry, "file") != victim {
            continue;
        }
        let commit = field(entry, "commit").to_string();
        entry["url"] = serde_json::Value::from(format!("{URL_PREFIX}{commit}/{donor_path}"));
        entry["path"] = serde_json::Value::from(donor_path.clone());
    }
    assert_ne!(
        repointed,
        manifest(),
        "the repoint changed nothing; this case is not testing what it claims"
    );

    let verdict = provenance_verdict("path-repoint", &repointed);
    assert!(
        !verdict.status.success(),
        "the provenance program documented at specification-sources.md:60-73 accepts a \
         manifest whose entry for {victim} records the path and url of {donor_path}. The \
         record states at specification-sources.md:104-109 that the archive name is the \
         upstream path below its revision directory, flattened; nothing checks that rule, \
         so the recorded url no longer identifies the archived bytes: {}",
        describe(&verdict)
    );
}

/// RED. The corrected version-string paragraph states that 18 of the 22 archived
/// interoperability documents carry the banner, names the four that do not, and then
/// says "two more name it in running text" — naming two documents that are themselves
/// among the 18. 18 + 2 + 4 is 24 against a set of 22, so a reader who sums the
/// paragraph's own figures over-counts the documents that declare the string.
///
/// This fires only while the record describes those documents as *more*; rewording the
/// clause, or naming documents that genuinely are not among the banner set, makes it
/// green. `specification-sources.md:135-146`.
#[test]
fn record_does_not_count_a_banner_document_twice_when_naming_running_text_mentions() {
    let normalised = record().split_whitespace().collect::<Vec<_>>().join(" ");
    let marker = "more name it in running text";
    let Some(at) = normalised.find(marker) else {
        // The record no longer describes any document as an additional one; there is
        // nothing here to over-count.
        return;
    };
    let clause = &normalised[at + marker.len()..];
    // Bound the clause at the sentence that closes the set, so later paragraphs naming
    // other archived documents cannot be read as part of this claim.
    let clause = clause.split("without the banner").next().unwrap_or(clause);
    let named: BTreeSet<String> = clause
        .split('`')
        .skip(1)
        .step_by(2)
        .filter(|token| {
            token.starts_with(&format!("mcp-{REVISION_INTEROP}-")) && token.ends_with(".mdx")
        })
        .map(|token| token.to_string())
        .collect();
    assert!(
        !named.is_empty(),
        "the record describes documents as additional to the banner set but names none \
         of them; this case is describing neither the record nor the archives"
    );

    let banner = banner_documents();
    let total = documents(REVISION_INTEROP).len();
    let double_counted: Vec<&String> = named.iter().filter(|file| banner.contains(*file)).collect();
    assert!(
        double_counted.is_empty(),
        "specification-sources.md calls {:?} additional to the {} archived \
         {REVISION_INTEROP} documents that carry the banner, but each of them carries \
         the banner itself. Adding the {} the record names as carrying it, the {} it \
         names as additional and the four it names as carrying none gives {} documents \
         out of {}.",
        double_counted,
        banner.len(),
        banner.len(),
        named.len(),
        banner.len() + named.len() + 4,
        total
    );
}

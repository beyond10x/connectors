//! Enrol committed generated output with ESS in a checkout that has no ownership record.
//!
//! ESS writes only into output it owns, recorded in a git-ignored `.ess-output/` ledger beside it. A
//! fresh checkout has the committed bytes and no ledger, so regenerating refuses with "unowned output
//! destination". The generator adopts the existing bytes when they equal what the pinned ESS
//! generates from the working-tree source or from the committed (`HEAD`) source. Bytes that equal
//! neither are stale against the source: unmodified committed ones are removed so ESS regenerates
//! them as its own output (git restores them); uncommitted or edited bytes are never overwritten.
use super::Result;
use std::{
    collections::BTreeMap,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

const LEDGER: &str = ".ess-output";

/// One generated destination: where it lives, who owns it in ESS, and the committed inputs it is
/// generated from.
pub struct Family<'a> {
    pub destination: &'a str,
    pub owner: &'a str,
    pub sources: &'a [&'a str],
}

/// Writes the output the pinned ESS generates from the sources under the first path into the second.
pub type Generate<'a> = &'a dyn Fn(&Path, &Path) -> Result<()>;

pub fn adopt_existing(
    root: &Path,
    ess: &Path,
    family: &Family<'_>,
    generate: Generate<'_>,
) -> Result<()> {
    let destination = root.join(family.destination);
    if !destination.is_dir() || destination.join(LEDGER).exists() {
        return Ok(());
    }
    let existing = artifacts(&destination)?;
    let base = root.join(".local/tmp");
    std::fs::create_dir_all(&base)?;
    let temp = tempfile::Builder::new()
        .prefix("ownership-")
        .tempdir_in(base)?;
    let mut differing = Vec::new();
    let mut generates = false;
    for attempt in 0..2 {
        let (label, source, reference) = if attempt == 0 {
            (
                "working tree",
                root.to_path_buf(),
                temp.path().join("working-reference"),
            )
        } else {
            let head = temp.path().join("head-source");
            if committed_sources(root, family.sources, &head).is_err() {
                break;
            }
            ("committed HEAD", head, temp.path().join("head-reference"))
        };
        if generate(&source, &reference).is_err() {
            differing.push(format!("{label}: generation failed"));
            continue;
        }
        let generated = artifacts(&reference)?;
        if generated == existing {
            return adopt(ess, root, family, &reference);
        }
        if attempt == 0 {
            generates = true;
        }
        differing.push(format!(
            "{label}: {}",
            differing_files(&generated, &existing).join(", ")
        ));
    }
    // The bytes are stale against the source (the usual reason to regenerate): they equal no
    // generation the pinned ESS can reproduce. Unmodified committed bytes are still git's to
    // restore, so they are replaced; anything uncommitted or edited is refused below.
    if generates && committed_unmodified(root, family.destination) {
        std::fs::remove_dir_all(&destination)?;
        return Ok(());
    }
    Err(format!(
        "unowned output destination: {dest} has no ESS ownership record and its bytes differ from \
         what ESS generates ({detail}), and they are not unmodified committed bytes; inspect the \
         differing files and keep hand-written bytes. To enrol bytes you know are generated, run \
         the same ESS generation into a new empty directory and `\"$CONNECTORS_ESS\" generate \
         output adopt --ownership-root {dest} --from <that directory> --owner {owner}`",
        dest = family.destination,
        detail = differing.join("; "),
        owner = family.owner,
    )
    .into())
}

/// Every file under `destination` is tracked by git and none differs from `HEAD`.
fn committed_unmodified(root: &Path, destination: &str) -> bool {
    let git = |args: &[&str]| {
        Command::new("git")
            .current_dir(root)
            .args(args)
            .arg("--")
            .arg(destination)
            .output()
            .ok()
            .filter(|output| output.status.success())
    };
    let tracked = git(&["ls-files"]).is_some_and(|output| !output.stdout.is_empty());
    let clean = git(&["status", "--porcelain", "--ignored=no"])
        .is_some_and(|output| output.stdout.is_empty());
    tracked && clean
}

fn adopt(ess: &Path, root: &Path, family: &Family<'_>, reference: &Path) -> Result<()> {
    let status = Command::new(ess)
        .current_dir(root)
        .args(["generate", "output", "adopt", "--ownership-root"])
        .arg(family.destination)
        .arg("--from")
        .arg(reference)
        .args(["--owner", family.owner])
        .status()?;
    if !status.success() {
        return Err(format!("ESS refused to adopt {}: {status}", family.destination).into());
    }
    Ok(())
}

fn committed_sources(root: &Path, sources: &[&str], into: &Path) -> Result<()> {
    std::fs::create_dir_all(into)?;
    let archive = Command::new("git")
        .current_dir(root)
        .args(["archive", "HEAD", "--"])
        .args(sources)
        .output()?;
    if !archive.status.success() {
        return Err("git archive HEAD failed".into());
    }
    let mut tar = Command::new("tar")
        .arg("-x")
        .arg("-C")
        .arg(into)
        .stdin(Stdio::piped())
        .spawn()?;
    tar.stdin
        .take()
        .ok_or("tar has no stdin")?
        .write_all(&archive.stdout)?;
    if !tar.wait()?.success() {
        return Err("tar failed to extract the committed sources".into());
    }
    Ok(())
}

fn artifacts(path: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut files = connectors_spec::v2::tree(path)?;
    files.retain(|name, _| !name.starts_with(".ess-output/"));
    Ok(files)
}

fn differing_files(
    generated: &BTreeMap<String, Vec<u8>>,
    existing: &BTreeMap<String, Vec<u8>>,
) -> Vec<String> {
    let mut names: Vec<String> = generated
        .keys()
        .chain(existing.keys())
        .filter(|name| generated.get(*name) != existing.get(*name))
        .cloned()
        .collect();
    names.sort();
    names.dedup();
    names
}

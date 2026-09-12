//! Re-derive every recorded upstream source digest from its archived bytes.
//!
//! A `*source-hashes.json` manifest beside a `vendor/` directory claims the exact
//! uncompressed SHA-256 and byte length of retained upstream sources. Nothing in this
//! repository re-derived them, so a drifted record and a faithful one looked the same.
//! This check reads the archives, not the record.
use super::Result;
use serde::Deserialize;
use std::{
    collections::BTreeSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[derive(Deserialize)]
struct Entry {
    archive: String,
    sha256: String,
    bytes: u64,
}

// Review records hash repository sources and keep no archive. Only a manifest with its
// own `vendor/` directory claims retained bytes, and only those bytes can be re-derived.
fn manifests(path: &Path, output: &mut Vec<PathBuf>) -> Result<()> {
    let mut entries = fs::read_dir(path)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort();
    for entry in entries {
        let name = entry
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_owned();
        if fs::symlink_metadata(&entry)?.is_dir() {
            // Skipping is decided per child, never for the root: a root whose own name
            // starts with a dot would otherwise silently check nothing and exit green.
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            manifests(&entry, output)?;
        } else if name.ends_with("source-hashes.json") && path.join("vendor").is_dir() {
            output.push(entry);
        }
    }
    Ok(())
}

fn decompress(archive: &Path) -> Result<Vec<u8>> {
    let output = Command::new("gzip").arg("-dc").arg(archive).output()?;
    if !output.status.success() {
        return Err(format!(
            "{}: gzip refused the archive: {}",
            archive.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }
    Ok(output.stdout)
}

fn digest(bytes: &[u8]) -> Result<String> {
    let mut child = Command::new("sha256sum")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("sha256sum accepted no input")?
        .write_all(bytes)?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err("sha256sum failed".into());
    }
    Ok(String::from_utf8(output.stdout)?
        .split_whitespace()
        .next()
        .ok_or("sha256sum printed no digest")?
        .to_owned())
}

/// Returns the number of archived sources re-derived, so that a caller — and the test
/// below — can tell a check that ran from a walk that selected nothing.
pub fn run(root: &Path) -> Result<usize> {
    let mut paths = Vec::new();
    manifests(root, &mut paths)?;
    let mut checked = 0usize;
    for manifest in &paths {
        let directory = manifest.parent().ok_or("manifest has no directory")?;
        let entries: Vec<Entry> = serde_json::from_slice(&fs::read(manifest)?)?;
        if entries.is_empty() {
            return Err(format!("{}: records no source", manifest.display()).into());
        }
        let mut named = BTreeSet::new();
        for entry in &entries {
            let relative = Path::new(&entry.archive);
            if relative.is_absolute() || entry.archive.contains("..") {
                return Err(format!(
                    "{}: archive {} leaves its evidence directory",
                    manifest.display(),
                    entry.archive
                )
                .into());
            }
            let archive = directory.join(relative);
            let bytes = decompress(&archive)?;
            if bytes.len() as u64 != entry.bytes {
                return Err(format!(
                    "{}: {} decompresses to {} bytes, the record claims {}",
                    manifest.display(),
                    entry.archive,
                    bytes.len(),
                    entry.bytes
                )
                .into());
            }
            let observed = digest(&bytes)?;
            if observed != entry.sha256 {
                return Err(format!(
                    "{}: {} decompresses to {}, the record claims {}",
                    manifest.display(),
                    entry.archive,
                    observed,
                    entry.sha256
                )
                .into());
            }
            if !named.insert(archive) {
                return Err(
                    format!("{}: records {} twice", manifest.display(), entry.archive).into(),
                );
            }
            checked += 1;
        }
        // Bytes no record names are unattributed: the manifest is the whole inventory.
        for archive in fs::read_dir(directory.join("vendor"))? {
            let archive = archive?.path();
            if !named.contains(&archive) {
                return Err(format!(
                    "{}: {} is archived but no record names it",
                    manifest.display(),
                    archive.display()
                )
                .into());
            }
        }
    }
    println!(
        "gate: {checked} archived upstream source(s) across {} manifest(s) match their recorded \
         uncompressed digests and lengths; exit=0",
        paths.len()
    );
    Ok(checked)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn archive(directory: &Path, name: &str, contents: &[u8]) -> (String, u64) {
        let source = directory.join(name);
        fs::write(&source, contents).unwrap();
        let archived = Command::new("gzip")
            .arg("-nc")
            .arg(&source)
            .output()
            .unwrap();
        assert!(archived.status.success());
        fs::create_dir_all(directory.join("vendor")).unwrap();
        fs::write(
            directory.join("vendor").join(format!("{name}.gz")),
            archived.stdout,
        )
        .unwrap();
        fs::remove_file(&source).unwrap();
        (digest(contents).unwrap(), contents.len() as u64)
    }

    fn manifest(directory: &Path, body: &str) {
        fs::write(directory.join("provider-source-hashes.json"), body).unwrap();
    }

    #[test]
    fn recorded_digests_are_rederived_from_the_archived_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let evidence = temp.path().join("adapters/example/evidence/20260101");
        fs::create_dir_all(&evidence).unwrap();
        let (sha, bytes) = archive(&evidence, "upstream.go", b"package v1\n");
        manifest(
            &evidence,
            &format!(
                r#"[{{"file":"upstream.go","url":"https://example.invalid/upstream.go",
                     "sha256":"{sha}","bytes":{bytes},"archive":"vendor/upstream.go.gz"}}]"#
            ),
        );
        // The count, not the exit, is the evidence the walk selected this manifest.
        assert_eq!(run(temp.path()).unwrap(), 1);

        // A record nobody re-derives cannot distinguish these four states from the one above.
        manifest(
            &evidence,
            &format!(
                r#"[{{"sha256":"{}","bytes":{bytes},"archive":"vendor/upstream.go.gz"}}]"#,
                "0".repeat(64)
            ),
        );
        assert!(run(temp.path()).is_err());
        manifest(
            &evidence,
            &format!(r#"[{{"sha256":"{sha}","bytes":1,"archive":"vendor/upstream.go.gz"}}]"#),
        );
        assert!(run(temp.path()).is_err());
        manifest(
            &evidence,
            &format!(r#"[{{"sha256":"{sha}","bytes":{bytes},"archive":"vendor/absent.go.gz"}}]"#),
        );
        assert!(run(temp.path()).is_err());
        manifest(
            &evidence,
            &format!(r#"[{{"sha256":"{sha}","bytes":{bytes},"archive":"vendor/upstream.go.gz"}}]"#),
        );
        archive(&evidence, "unrecorded.go", b"package v2\n");
        assert!(run(temp.path()).is_err());
    }

    #[test]
    fn a_manifest_without_retained_archives_is_not_this_check() {
        let temp = tempfile::tempdir().unwrap();
        let reviews = temp.path().join("docs/evidence/example/reviews");
        fs::create_dir_all(&reviews).unwrap();
        fs::write(
            reviews.join("a-initial-source-hashes.json"),
            r#"[{"source":"contracts/example/semantics.md","sha256":"0","bytes":1}]"#,
        )
        .unwrap();
        assert_eq!(run(temp.path()).unwrap(), 0);
    }
}

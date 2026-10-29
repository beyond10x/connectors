//! The cache entry and the lock (`connectors.definitions_install.CacheEntry` and
//! `Lock`).
//!
//! An entry is `<cache root>/<index sha256>/`: `index.json` and `files/<path>`. It is
//! written under a temporary name in the cache root, made read-only (files 0444,
//! directories 0555) and renamed into place whole, so an entry that exists is
//! complete. The lock is written to a temporary file beside it and renamed over the
//! previous one, owner-only.
use super::{
    Refusal,
    index::{Index, sha256},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

/// `definitions.lock`: one installed release for one connectors configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lock {
    pub format: String,
    pub source: String,
    pub tag: String,
    pub commit: String,
    pub index_sha256: String,
}

/// A name no other writer in this or another process picks.
fn temporary(prefix: &str) -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    format!(
        ".{prefix}.{}.{nanos}.{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
}

#[cfg(not(unix))]
fn set_mode(path: &Path, mode: u32) -> std::io::Result<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_readonly(mode & 0o200 == 0);
    fs::set_permissions(path, permissions)
}

fn private_directory(path: &Path) -> std::io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
    builder.create(path)
}

/// Removes the tree at `path`, making its directories writable first; the
/// outcome is not reported, as it only undoes this install's own writes.
pub(crate) fn remove(path: &Path) {
    fn writable(path: &Path) {
        let Ok(metadata) = fs::symlink_metadata(path) else {
            return;
        };
        if metadata.is_dir() {
            let _ = set_mode(path, 0o700);
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    writable(&entry.path());
                }
            }
        }
    }
    writable(path);
    let _ = fs::remove_dir_all(path);
}

/// Stores the verified release under `root` as the entry `index_sha256`, or
/// verifies the entry already there. Returns the entry and whether this call
/// created it.
pub(crate) fn cache(
    root: &Path,
    index_sha256: &str,
    index_bytes: &[u8],
    index: &Index,
    files: &BTreeMap<String, Vec<u8>>,
) -> Result<(PathBuf, bool), Refusal> {
    let entry = root.join(index_sha256);
    if fs::symlink_metadata(&entry).is_ok() {
        verify_entry(&entry, index_bytes, index)?;
        return Ok((entry, false));
    }
    private_directory(root).map_err(|_| Refusal::WriteFailed)?;
    let staged = root.join(temporary(index_sha256));
    let written = write_entry(&staged, index_bytes, files);
    if written.is_err() {
        remove(&staged);
        return Err(Refusal::WriteFailed);
    }
    match fs::rename(&staged, &entry) {
        Ok(()) => Ok((entry, true)),
        Err(_) => {
            remove(&staged);
            // Another install of the same release renamed its entry first.
            if fs::symlink_metadata(&entry).is_ok() {
                verify_entry(&entry, index_bytes, index)?;
                Ok((entry, false))
            } else {
                Err(Refusal::WriteFailed)
            }
        }
    }
}

fn write_entry(
    staged: &Path,
    index_bytes: &[u8],
    files: &BTreeMap<String, Vec<u8>>,
) -> std::io::Result<()> {
    fs::create_dir(staged)?;
    write_file(&staged.join("index.json"), index_bytes)?;
    let base = staged.join("files");
    fs::create_dir(&base)?;
    for (path, bytes) in files {
        let file = base.join(path);
        if let Some(parent) = file.parent() {
            fs::create_dir_all(parent)?;
        }
        write_file(&file, bytes)?;
    }
    seal(staged)
}

/// Makes `directory` and every directory beneath it read-only, each after
/// everything in it.
fn seal(directory: &Path) -> std::io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            seal(&entry.path())?;
        }
    }
    set_mode(directory, 0o555)
}

fn write_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    set_mode(path, 0o444)
}

/// An existing entry holds exactly this index's bytes and its files.
fn verify_entry(entry: &Path, index_bytes: &[u8], index: &Index) -> Result<(), Refusal> {
    if fs::read(entry.join("index.json")).ok().as_deref() != Some(index_bytes) {
        return Err(Refusal::DigestMismatch("index.json".to_owned()));
    }
    for file in &index.files {
        let held = fs::read(entry.join("files").join(&file.path)).ok();
        if !held
            .is_some_and(|bytes| bytes.len() as u64 == file.size && sha256(&bytes) == file.sha256)
        {
            return Err(Refusal::DigestMismatch(file.path.clone()));
        }
    }
    Ok(())
}

/// Writes `lock` at `path`, owner-only, through a temporary file renamed over it.
pub(crate) fn write_lock(path: &Path, lock: &Lock) -> Result<(), Refusal> {
    let directory = path.parent().ok_or(Refusal::WriteFailed)?;
    let staged = directory.join(temporary("definitions.lock"));
    let mut text = serde_json::to_string(lock).map_err(|_| Refusal::WriteFailed)?;
    text.push('\n');
    let written = (|| -> std::io::Result<()> {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        let mut file = options.open(&staged)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        drop(file);
        fs::rename(&staged, path)
    })();
    if written.is_err() {
        let _ = fs::remove_file(&staged);
        return Err(Refusal::WriteFailed);
    }
    Ok(())
}

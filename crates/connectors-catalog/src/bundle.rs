//! Per-provider bundles and the local index that finds them. A directory and a
//! file are the whole mechanism: nothing here contacts a catalog service, and a
//! bundle whose bytes no longer match what the index recorded is refused rather
//! than read as current.

use crate::{SourceRecord, inventory::Inventory};
use connectors_core::{Error, ErrorCode, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// The file that names every bundle in a directory.
pub const INDEX_FILE: &str = "index.json";
/// The largest bundle this loader reads back.
pub const BUNDLE_LIMIT: usize = 256 * 1024 * 1024;

/// What a provider's bundle carries. The auth profile is referenced by id; the
/// bundle holds no credential and no endpoint authority of its own.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    pub provider: String,
    pub source: SourceRecord,
    pub inventory: Inventory,
    pub auth_profile: String,
}

/// One row of the index: enough to find the bundle and to refuse a wrong one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub provider: String,
    pub file_name: String,
    /// SHA-256 of the bundle file's exact bytes.
    pub bundle_sha256: String,
    /// The source digest the bundle was built from, carried so a reader can tell
    /// which pinned document a loaded inventory describes without opening it.
    pub source_sha256: String,
    pub operations: usize,
    pub unsupported: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Index {
    pub entries: Vec<Entry>,
}

impl Index {
    pub fn providers(&self) -> Vec<&str> {
        self.entries.iter().map(|e| e.provider.as_str()).collect()
    }
    pub fn find(&self, provider: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.provider == provider)
    }
}

fn index_path(directory: &Path) -> PathBuf {
    directory.join(INDEX_FILE)
}

fn refuse(code: ErrorCode, message: &str) -> Error {
    Error::new(code, message)
}

/// Read the index, or an empty one when the directory holds none yet.
pub fn read_index(directory: &Path) -> Result<Index> {
    let path = index_path(directory);
    if !path.exists() {
        return Ok(Index::default());
    }
    let bytes =
        std::fs::read(&path).map_err(|_| refuse(ErrorCode::NotFound, "index cannot be read"))?;
    connectors_core::read_json(&bytes).map_err(|_| {
        refuse(
            ErrorCode::InvalidInput,
            "index is not a document this build parses",
        )
    })
}

fn put_index(directory: &Path, bytes: &[u8]) -> Result<()> {
    // Written in place, not renamed over: an index somebody made read-only is a
    // refusal this writer is meant to raise, and a rename would replace it
    // without ever asking the file's mode.
    std::fs::write(index_path(directory), bytes)
        .map_err(|_| refuse(ErrorCode::Unavailable, "index cannot be written"))
}

/// Put the index back exactly as it was found. Best effort by construction: it
/// runs on a path where a write has already failed, and the error it answers is
/// the one the caller returns, so a failure here must not replace it.
fn restore_index(directory: &Path, previous: Option<&[u8]>) {
    match previous {
        Some(bytes) => {
            let _ = std::fs::write(index_path(directory), bytes);
        }
        None => {
            let _ = std::fs::remove_file(index_path(directory));
        }
    }
}

/// A bundle written under a working name, put in place by a rename.
///
/// Dropping one without committing removes the working file, so a refusal raised
/// between the bundle write and the index write leaves the directory holding
/// exactly what it held before — which is the guarantee a caller of `write` is
/// given, and which a plain write to the final name cannot keep.
struct Staged {
    working: PathBuf,
    placed: PathBuf,
}

impl Staged {
    fn write(directory: &Path, file_name: &str, bytes: &[u8]) -> Result<Self> {
        // No bundle can be called this: a bundle file ends `.bundle.json` and the
        // index is `index.json`, so nothing a reader looks for can be shadowed.
        // The process id and the counter keep two writers in one directory — in
        // one process or two — off each other's working file.
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let working = directory.join(format!(
            "{file_name}.writing-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&working, bytes)
            .map_err(|_| refuse(ErrorCode::Unavailable, "bundle cannot be written"))?;
        Ok(Self {
            working,
            placed: directory.join(file_name),
        })
    }

    /// Put the bundle at the name the index records. A rename inside one
    /// directory is the one step here that needs no space and no new name, so it
    /// is the last one taken.
    fn commit(self) -> Result<()> {
        std::fs::rename(&self.working, &self.placed)
            .map_err(|_| refuse(ErrorCode::Unavailable, "bundle cannot be written"))
        // The guard still drops, and removing the working name is then a no-op:
        // the rename moved it, and no other writer can have taken that name.
    }
}

impl Drop for Staged {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.working);
    }
}

/// Write a bundle and record it. `replace` is explicit: without it, a provider
/// the index already carries is refused, so a second writer cannot quietly
/// overwrite an inventory somebody is loading.
///
/// The bundle and the index are one change, not two. Everything that can fail
/// without touching the directory — serialising both documents, the size check,
/// the duplicate check — fails first; then the bundle goes to a working name, the
/// index is written, and only then is the bundle renamed into place. A refusal at
/// any point leaves the directory exactly as it was found, including the index.
/// The window a crash can still cut is the single rename, and what it leaves is
/// an index naming a bundle file that is not there yet — which `load` refuses by
/// name and a replacing write repairs.
pub fn write(directory: &Path, bundle: &Bundle, replace: bool) -> Result<Entry> {
    if !connectors_core::valid_id(&bundle.provider) {
        return Err(refuse(
            ErrorCode::InvalidInput,
            "invalid provider identifier",
        ));
    }
    let bytes = serde_json::to_vec(bundle).map_err(|_| Error::internal())?;
    if bytes.len() > BUNDLE_LIMIT {
        return Err(refuse(
            ErrorCode::Capacity,
            "bundle exceeds the permitted size",
        ));
    }
    std::fs::create_dir_all(directory)
        .map_err(|_| refuse(ErrorCode::Unavailable, "bundle directory cannot be created"))?;
    let mut index = read_index(directory)?;
    if index.find(&bundle.provider).is_some() && !replace {
        return Err(refuse(
            ErrorCode::InvalidInput,
            "provider is already indexed; ask for replacement",
        ));
    }
    let file_name = format!("{}.bundle.json", bundle.provider);
    let (operations, unsupported) = bundle.inventory.coverage();
    let entry = Entry {
        provider: bundle.provider.clone(),
        file_name: file_name.clone(),
        bundle_sha256: hex::encode(Sha256::digest(&bytes)),
        source_sha256: bundle.source.source_sha256.clone(),
        operations,
        unsupported,
    };
    index.entries.retain(|e| e.provider != entry.provider);
    index.entries.push(entry.clone());
    // A stable order, so two directories built by the same writes read the same.
    index.entries.sort_by(|a, b| a.provider.cmp(&b.provider));
    let index_bytes = serde_json::to_vec_pretty(&index).map_err(|_| Error::internal())?;

    let staged = Staged::write(directory, &file_name, &bytes)?;
    // The exact bytes the index holds now, so a later failure can put them back.
    // `None` is the directory that had no index at all, and restoring that means
    // removing the one this write is about to create.
    let previous = std::fs::read(index_path(directory)).ok();
    put_index(directory, &index_bytes)
        .inspect_err(|_| restore_index(directory, previous.as_deref()))?;
    staged.commit().inspect_err(|_| {
        // The index now names a bundle that could not be put in place. Undoing the
        // index is what leaves the directory as it was found.
        restore_index(directory, previous.as_deref());
    })?;
    Ok(entry)
}

/// Load a provider's bundle, checking it against what the index recorded.
pub fn load(directory: &Path, provider: &str) -> Result<Bundle> {
    let index = read_index(directory)?;
    let entry = index
        .find(provider)
        .ok_or_else(|| refuse(ErrorCode::NotFound, "index carries no such provider"))?;
    let path = directory.join(&entry.file_name);
    let bytes = std::fs::read(&path)
        .map_err(|_| refuse(ErrorCode::NotFound, "indexed bundle file is absent"))?;
    if bytes.len() > BUNDLE_LIMIT {
        return Err(refuse(
            ErrorCode::Capacity,
            "bundle exceeds the permitted size",
        ));
    }
    if hex::encode(Sha256::digest(&bytes)) != entry.bundle_sha256 {
        return Err(refuse(
            ErrorCode::InvalidInput,
            "bundle does not match the digest the index recorded",
        ));
    }
    let bundle: Bundle = connectors_core::read_json(&bytes).map_err(|_| {
        refuse(
            ErrorCode::InvalidInput,
            "bundle is not a document this build parses",
        )
    })?;
    if bundle.provider != entry.provider {
        return Err(refuse(
            ErrorCode::InvalidInput,
            "bundle names a different provider than the index",
        ));
    }
    Ok(bundle)
}

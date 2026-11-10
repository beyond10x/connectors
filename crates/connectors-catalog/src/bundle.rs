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
use std::time::{Duration, Instant};

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

/// Open a file in the bundle directory once, without following a link, and
/// read it from that handle. A link is refused as `InvalidInput`, a file that is
/// not regular is too, and a file that is missing is `NotFound`; nothing is
/// resolved twice, so a link swapped in after the check cannot be followed.
fn read_regular(path: &Path, what: &str) -> Result<Vec<u8>> {
    use std::io::Read;
    let not_regular = || {
        refuse(
            ErrorCode::InvalidInput,
            &format!("{what} is not a regular file"),
        )
    };
    let unreadable = || refuse(ErrorCode::NotFound, &format!("{what} cannot be read"));
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    #[cfg(not(unix))]
    if std::fs::symlink_metadata(path).is_ok_and(|m| !m.is_file()) {
        return Err(not_regular());
    }
    let mut file = options.open(path).map_err(|error| {
        #[cfg(unix)]
        if error.raw_os_error() == Some(libc::ELOOP) {
            return not_regular();
        }
        if error.kind() == std::io::ErrorKind::NotFound {
            return refuse(ErrorCode::NotFound, &format!("{what} is absent"));
        }
        unreadable()
    })?;
    if !file.metadata().is_ok_and(|m| m.is_file()) {
        return Err(not_regular());
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(|_| unreadable())?;
    Ok(bytes)
}

/// Read the index, or an empty one when the directory holds none yet.
pub fn read_index(directory: &Path) -> Result<Index> {
    let path = index_path(directory);
    if std::fs::symlink_metadata(&path).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound) {
        return Ok(Index::default());
    }
    let bytes = read_regular(&path, "index")?;
    let index: Index = connectors_core::read_json(&bytes).map_err(|_| {
        refuse(
            ErrorCode::InvalidInput,
            "index is not a document this build parses",
        )
    })?;
    check_rows(&index)?;
    Ok(index)
}

/// The one file name a provider's bundle has. `write` names files with it and
/// `read_index` refuses a row that names any other.
fn bundle_file_name(provider: &str) -> String {
    format!("{provider}.bundle.json")
}

/// Refuse an index `write` could not have produced: a row whose provider is not
/// an identifier, a row whose file is not `<provider>.bundle.json`, or two rows
/// for one provider. `load` joins the row's file name onto the directory and
/// `Index::find` takes the first match, so either shape would let a hand-edited
/// index name a file outside the directory or pick between two answers.
fn check_rows(index: &Index) -> Result<()> {
    let mut seen = std::collections::BTreeSet::new();
    for entry in &index.entries {
        if !connectors_core::valid_id(&entry.provider) {
            return Err(refuse(
                ErrorCode::InvalidInput,
                "index names an invalid provider identifier",
            ));
        }
        if entry.file_name != bundle_file_name(&entry.provider) {
            return Err(refuse(
                ErrorCode::InvalidInput,
                "index names a bundle file other than <provider>.bundle.json",
            ));
        }
        if !seen.insert(entry.provider.as_str()) {
            return Err(refuse(
                ErrorCode::InvalidInput,
                "index names one provider more than once",
            ));
        }
    }
    Ok(())
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

/// The name that holds one directory's index while a writer reads it, changes it
/// and writes it back. A bundle file ends `.bundle.json` and the index is
/// `index.json`, so this name shadows nothing a reader looks for.
const LOCK_FILE: &str = ".index.writing";
/// How long a writer waits for another to finish before refusing. The held
/// section is a read, a sort and a write of one small file.
const LOCK_WAIT: Duration = Duration::from_secs(10);
/// A lock older than this is treated as abandoned by a writer that died. It is
/// far longer than the held section, so a live writer is never displaced.
const LOCK_STALE: Duration = Duration::from_secs(120);

/// Exclusive hold over one directory's index.
///
/// Without it, `write` reads the index, appends its row and writes the whole file
/// back with nothing in between, so two writers each read the same index and the
/// second erases the first's row while both are told they succeeded. Measured
/// before this existed: eight concurrent writers, eight bundle files on disk, two
/// rows in the index.
///
/// `create_new` is the exclusion: on a local filesystem exactly one caller
/// creates the name. Dropping the guard removes it, including on the error paths,
/// because the guard outlives every `?` in the section it covers.
struct IndexLock {
    path: PathBuf,
}

impl IndexLock {
    fn acquire(directory: &Path) -> Result<Self> {
        let path = directory.join(LOCK_FILE);
        let deadline = Instant::now() + LOCK_WAIT;
        loop {
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(_) => return Ok(Self { path }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    if std::fs::metadata(&path)
                        .and_then(|meta| meta.modified())
                        .ok()
                        .and_then(|modified| modified.elapsed().ok())
                        .is_some_and(|age| age > LOCK_STALE)
                    {
                        // The holder is gone. Removing the name is safe to race:
                        // whoever wins the next `create_new` holds the lock.
                        let _ = std::fs::remove_file(&path);
                        continue;
                    }
                    if Instant::now() >= deadline {
                        return Err(refuse(
                            ErrorCode::Unavailable,
                            "another writer holds the bundle index",
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(2));
                }
                Err(_) => {
                    return Err(refuse(
                        ErrorCode::Unavailable,
                        "bundle index cannot be locked",
                    ));
                }
            }
        }
    }
}

impl Drop for IndexLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
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
    // Held from before the index is read until after it is written back, which
    // is the whole read-modify-write. The duplicate-provider check reads the
    // index too, so it is inside the hold and cannot decide on a stale copy.
    let _lock = IndexLock::acquire(directory)?;
    let mut index = read_index(directory)?;
    if index.find(&bundle.provider).is_some() && !replace {
        return Err(refuse(
            ErrorCode::InvalidInput,
            "provider is already indexed; ask for replacement",
        ));
    }
    let file_name = bundle_file_name(&bundle.provider);
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
    let bytes = read_regular(&path, "indexed bundle file")?;
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

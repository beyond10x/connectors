//! Where this Integration keeps its own bookkeeping.
//!
//! Not GitLab data — the list of Connections that exist and the credential transactions that are
//! half-committed. A hosted placement runs several replicas of one Connector, so that list has to
//! be shared and Postgres holds it. A personal placement is one process on one machine, where an
//! owner-only file is both sufficient and correct.
//!
//! Only the hosted half existed, so `GitlabBackend` could not be constructed anywhere without a
//! database — which read as "GitLab needs Postgres" and is why a workstation could not reach GitLab
//! at all. `integration-slack` has carried both halves since it shipped; this is the same split.

use std::fs::{self, File, OpenOptions};
use std::io::{Read as _, Write as _};
use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};

use hosted_state::PostgresState;

use crate::backend::GitlabError;

/// The bookkeeping store behind one placement.
pub(crate) enum GitlabState {
    /// Shared across replicas of one hosted Connector.
    Hosted(PostgresState),
    /// Owner-only files beside one personal placement's socket.
    Local { root: PathBuf },
}

impl GitlabState {
    pub(crate) fn read(&self, key: &str, bound: usize) -> Result<Option<Vec<u8>>, GitlabError> {
        match self {
            Self::Hosted(state) => state
                .read(key, bound)
                .map_err(|_| GitlabError::new("connection-state")),
            Self::Local { root } => {
                let path = Self::path(root, key);
                let Some(mut file) = open_owner_read(&path)? else {
                    return Ok(None);
                };
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes)
                    .map_err(|_| GitlabError::new("connection-state"))?;
                if bytes.len() > bound {
                    return Err(GitlabError::new("connection-state"));
                }
                Ok(Some(bytes))
            }
        }
    }

    pub(crate) fn replace(&self, key: &str, body: &[u8], bound: usize) -> Result<(), GitlabError> {
        if body.len() > bound {
            return Err(GitlabError::new("connection-state"));
        }
        match self {
            Self::Hosted(state) => state
                .replace(key, body, bound)
                .map_err(|_| GitlabError::new("connection-state")),
            // Written to a temporary and renamed, so a crash mid-write leaves the previous list
            // intact rather than a truncated one: a half-written connection list is a placement
            // that comes back having silently lost a Connection.
            Self::Local { root } => {
                let path = Self::path(root, key);
                let parent = path
                    .parent()
                    .ok_or_else(|| GitlabError::new("connection-state"))?;
                ensure_owner_directory(parent)?;
                let temporary = parent.join(format!(".{}.tmp", file_name(key)));
                let mut file = OpenOptions::new()
                    .create(true)
                    .truncate(true)
                    .write(true)
                    .mode(0o600)
                    .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
                    .open(&temporary)
                    .map_err(|_| GitlabError::new("connection-state"))?;
                file.write_all(body)
                    .and_then(|()| file.sync_all())
                    .map_err(|_| GitlabError::new("connection-state"))?;
                fs::rename(&temporary, &path).map_err(|_| GitlabError::new("connection-state"))?;
                File::open(parent)
                    .and_then(|directory| directory.sync_all())
                    .map_err(|_| GitlabError::new("connection-state"))
            }
        }
    }

    pub(crate) fn append(&self, key: &str, line: &[u8], bound: usize) -> Result<(), GitlabError> {
        match self {
            Self::Hosted(state) => state
                .append(key, line, bound)
                .map(|_| ())
                .map_err(|_| GitlabError::new("audit-store")),
            Self::Local { root } => {
                let path = Self::path(root, key);
                let parent = path
                    .parent()
                    .ok_or_else(|| GitlabError::new("audit-store"))?;
                ensure_owner_directory(parent)?;
                let mut file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .mode(0o600)
                    .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
                    .open(&path)
                    .map_err(|_| GitlabError::new("audit-store"))?;
                if file.metadata().map(|data| data.len()).unwrap_or_default()
                    > bound as u64
                {
                    return Err(GitlabError::new("audit-store"));
                }
                file.write_all(line)
                    .and_then(|()| file.sync_all())
                    .map_err(|_| GitlabError::new("audit-store"))
            }
        }
    }

    fn path(root: &Path, key: &str) -> PathBuf {
        root.join(file_name(key))
    }
}

/// One state key as one file name. Keys are compile-time constants in this crate, so this only has
/// to keep a `.`-separated key from becoming a directory path.
fn file_name(key: &str) -> String {
    format!("gitlab-{}.json", key.replace(['/', '.'], "-"))
}

fn ensure_owner_directory(path: &Path) -> Result<(), GitlabError> {
    fs::create_dir_all(path).map_err(|_| GitlabError::new("connection-state"))?;
    let metadata = fs::symlink_metadata(path).map_err(|_| GitlabError::new("connection-state"))?;
    if !metadata.is_dir()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err(GitlabError::new("connection-state"));
    }
    Ok(())
}

/// Opens one state file, refusing anything another account could have written.
fn open_owner_read(path: &Path) -> Result<Option<File>, GitlabError> {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return Ok(None);
    };
    if !metadata.file_type().is_file()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err(GitlabError::new("connection-state"));
    }
    OpenOptions::new()
        .read(true)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(path)
        .map(Some)
        .map_err(|_| GitlabError::new("connection-state"))
}

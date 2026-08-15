//! Same-handle, bounded trust checks for deployment configuration files.

use std::fs::OpenOptions;
use std::io::Read as _;
use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _};
use std::path::Path;

#[derive(Debug, Clone, Copy)]
pub(crate) enum TrustedOwner {
    CurrentUser,
    CurrentUserOrRoot,
}

#[derive(Debug)]
pub(crate) enum TrustedConfigReadError {
    Io(std::io::Error),
    Unsafe,
}

/// Open once without following a final symlink, inspect that handle, and read at most `limit + 1`
/// bytes from the same inode. Hosted configuration may be installed by root for a non-root
/// runtime; personal configuration remains bound to the current owner.
pub(crate) fn read_trusted_config(
    path: &Path,
    limit: u64,
    trusted_owner: TrustedOwner,
) -> Result<String, TrustedConfigReadError> {
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(path)
        .map_err(TrustedConfigReadError::Io)?;
    let metadata = file.metadata().map_err(TrustedConfigReadError::Io)?;
    let effective_uid = rustix::process::geteuid().as_raw();
    let trusted_uid = metadata.uid() == effective_uid
        || matches!(trusted_owner, TrustedOwner::CurrentUserOrRoot) && metadata.uid() == 0;
    if !metadata.file_type().is_file()
        || !trusted_uid
        || metadata.permissions().mode() & 0o022 != 0
        || metadata.len() > limit
    {
        return Err(TrustedConfigReadError::Unsafe);
    }
    let mut bytes = Vec::with_capacity(metadata.len().min(limit) as usize);
    (&mut file)
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(TrustedConfigReadError::Io)?;
    if bytes.len() as u64 > limit {
        return Err(TrustedConfigReadError::Unsafe);
    }
    String::from_utf8(bytes).map_err(|_| TrustedConfigReadError::Unsafe)
}

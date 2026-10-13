//! The adapter executable a unit test configures: this test binary, at a path
//! the configured-executable admission accepts, with its SHA-256.
//!
//! On Unix that is the binary where Cargo built it. On Windows the admission
//! (`config::Executable::source`, contracts/cli/v1alpha1/semantics.md,
//! "Windows platform guarantees") refuses an executable whose directory or any
//! ancestor lets another trustee replace its entries, and a hosted runner's
//! build directory (`D:\a\…\target`) inherits such a grant from its volume
//! root. The guarantee is not relaxed for tests: the binary is copied once per
//! process into a fresh owner-only directory in the system temporary
//! directory, under the user's profile, and that copy is configured.
use sha2::{Digest, Sha256};
use std::{path::PathBuf, sync::OnceLock};

pub(crate) fn path_and_sha256() -> (PathBuf, String) {
    static EXECUTABLE: OnceLock<(PathBuf, String)> = OnceLock::new();
    EXECUTABLE
        .get_or_init(|| {
            let path = admitted();
            let hash = hex::encode(Sha256::digest(std::fs::read(&path).unwrap()));
            (path, hash)
        })
        .clone()
}

#[cfg(unix)]
fn admitted() -> PathBuf {
    std::env::current_exe().unwrap()
}

/// Kept for the life of the test process: every child started from it is
/// started from its own captured copy, never from this file.
#[cfg(windows)]
fn admitted() -> PathBuf {
    use crate::local::filesystem::windows::owner_only;
    let built = std::env::current_exe().unwrap();
    let directory = tempfile::Builder::new()
        .prefix("connectors-test-adapter-")
        .tempdir()
        .unwrap()
        .keep();
    owner_only(&directory);
    let path = directory.join(built.file_name().unwrap());
    std::fs::copy(&built, &path).unwrap();
    owner_only(&path);
    path
}

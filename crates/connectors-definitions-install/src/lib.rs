//! Installs a connectors-definitions release (story:definitions-install, ADR D5 of
//! provider-definitions-and-runtimes; `contracts/cli/v1alpha1/semantics.md` §3,
//! "Installing a definitions release").
//!
//! [`install`] fetches a release by tag from a [`Source`], an `https` repository or a
//! local directory, checks its `index.json` and every file of its asset against that
//! index, and only then writes: a read-only cache entry keyed by the SHA-256 of the
//! index's bytes, then a lock naming source, tag, commit and that SHA-256. A refusal
//! leaves the lock and every cache entry as they were.
//!
//! The only things it takes from the build are [`INDEX_SCHEMAS`] and [`FEATURES`]:
//! the index format versions this build reads and the definition features it can
//! lower. It depends on no connectors crate and embeds no catalog, so a release this
//! binary was never built with installs without a rebuild (SM-D209). Installing
//! grants nothing: no connection, no operation admission, no OAuth exchange and no
//! write authority.

mod archive;
mod fetch;
mod index;
mod store;

pub use fetch::Source;
pub use index::Index;
pub use store::Lock;

use std::path::{Path, PathBuf};

/// The `index.json` format versions this build reads.
pub const INDEX_SCHEMAS: &[&str] = &["urn:beyond10x:connectors-definitions-index:1"];
/// The optional definition members install admits, sorted: the four connectors-definitions
/// 0.3.0 and later declare. A release whose `features` names another is refused as
/// [`Refusal::UnmetFeature`]. The definition lowerer (`connectors-catalog`) does not
/// read these members yet; until it does, an installed release is stored and locked
/// but no instance resolves through it.
pub const FEATURES: &[&str] = &[
    "embedded_text",
    "query_fixed",
    "query_folds",
    "rate_limit_headers",
];
/// The lock file's format.
pub const LOCK_FORMAT: &str = "connectors-definitions-lock/1";
/// The lock file's name, beside the configuration file.
pub const LOCK_FILE: &str = "definitions.lock";
/// The cache's directory under the state directory.
pub const CACHE_DIRECTORY: &str = "installed-definitions";
/// The most bytes of `index.json` read.
pub const INDEX_LIMIT: u64 = 4 * 1024 * 1024;
/// The most bytes of release asset read.
pub const ASSET_LIMIT: u64 = 64 * 1024 * 1024;
/// The most files an index may name.
pub const FILES_LIMIT: usize = 16_384;
/// The most bytes all files an index names may hold together.
pub const UNPACKED_LIMIT: u64 = 512 * 1024 * 1024;
/// The most redirects an HTTPS request follows.
pub const REDIRECT_LIMIT: usize = 5;

/// Why an install was refused. Nothing was written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The source is neither an `https` URL without credentials, query or fragment
    /// nor a path.
    InvalidSource,
    /// The tag is not 1 to 128 bytes of `[A-Za-z0-9._-]`, or is `.` or `..`.
    InvalidTag,
    /// The source does not have the directory or a file of the release.
    NotFound,
    /// The source did not answer, answered another status, redirected where no
    /// redirect is followed, or a file exceeded its bound.
    Unavailable,
    /// `index.json` names a format version this build does not read.
    UnknownFormatVersion,
    /// The release requires this feature, which this build does not support.
    UnmetFeature(String),
    /// The index or the asset is not what the index format states.
    ReleaseInvalid(String),
    /// The bytes of this file, relative to the release, or of the asset, named by
    /// its name, differ from the index.
    DigestMismatch(String),
    /// The cache entry or the lock could not be written; nothing was renamed into
    /// place.
    WriteFailed,
}

impl Refusal {
    /// The `Failure.code` the CLI reports this refusal as.
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidSource | Self::InvalidTag => "invalid_input",
            Self::NotFound => "not_found",
            Self::Unavailable | Self::WriteFailed => "unavailable",
            Self::UnknownFormatVersion => "unknown_format_version",
            Self::UnmetFeature(_) => "unmet_feature",
            Self::ReleaseInvalid(_) => "release_invalid",
            Self::DigestMismatch(_) => "digest_mismatch",
        }
    }
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnmetFeature(feature) => write!(f, "{}: {feature}", self.code()),
            Self::ReleaseInvalid(reason) => write!(f, "{}: {reason}", self.code()),
            Self::DigestMismatch(file) => write!(f, "{}: {file}", self.code()),
            _ => f.write_str(self.code()),
        }
    }
}

impl std::error::Error for Refusal {}

/// What [`install`] recorded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Installed {
    /// The lock it wrote.
    pub lock: Lock,
    /// The release's features, sorted.
    pub features: Vec<String>,
    /// The number of files the index names.
    pub files: usize,
    /// The lock file.
    pub lock_path: PathBuf,
    /// The cache entry, `<cache root>/<index sha256>`.
    pub cache_path: PathBuf,
    /// Whether this call wrote the cache entry; `false` when an entry for the same
    /// index was already there and its files verified.
    pub created: bool,
}

/// Whether `tag` is a tag [`install`] admits: 1 to 128 bytes of `[A-Za-z0-9._-]`,
/// not `.` or `..`, so it is one path segment and one URL path segment.
pub fn valid_tag(tag: &str) -> bool {
    (1..=128).contains(&tag.len())
        && tag != "."
        && tag != ".."
        && tag
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// Fetches the release `tag` from `source`, verifies it, stores it under
/// `cache_root` and writes the lock at `lock_path`, in that order. Every check
/// precedes the first write; a refusal leaves the lock and every cache entry as
/// they were.
pub fn install(
    source: &Source,
    tag: &str,
    cache_root: &Path,
    lock_path: &Path,
) -> Result<Installed, Refusal> {
    if !valid_tag(tag) {
        return Err(Refusal::InvalidTag);
    }
    let fetcher = fetch::Fetcher::new(source)?;
    install_with(fetcher, source.recorded(), tag, cache_root, lock_path)
}

/// [`install`] through `fetcher`, recording `source` in the lock.
pub(crate) fn install_with(
    mut fetcher: fetch::Fetcher,
    source: String,
    tag: &str,
    cache_root: &Path,
    lock_path: &Path,
) -> Result<Installed, Refusal> {
    if !valid_tag(tag) {
        return Err(Refusal::InvalidTag);
    }
    let index_bytes = fetcher.read(tag, "index.json", INDEX_LIMIT)?;
    let index = Index::read(&index_bytes, tag)?;
    let asset = fetcher.read(tag, &index.asset.name, ASSET_LIMIT)?;
    let files = archive::verify(&index, &asset)?;
    let index_sha256 = index::sha256(&index_bytes);
    let lock = Lock {
        format: LOCK_FORMAT.to_owned(),
        source,
        tag: index.tag.clone(),
        commit: index.commit.clone(),
        index_sha256: index_sha256.clone(),
    };
    let root_existed = std::fs::symlink_metadata(cache_root).is_ok();
    let (cache_path, created) =
        match store::cache(cache_root, &index_sha256, &index_bytes, &index, &files) {
            Ok(stored) => stored,
            Err(refusal) => {
                if !root_existed {
                    let _ = std::fs::remove_dir(cache_root);
                }
                return Err(refusal);
            }
        };
    if let Err(refusal) = store::write_lock(lock_path, &lock) {
        if created {
            store::remove(&cache_path);
        }
        if !root_existed {
            // Only an empty directory is removed: one this call created.
            let _ = std::fs::remove_dir(cache_root);
        }
        return Err(refusal);
    }
    Ok(Installed {
        lock,
        features: index.features.clone(),
        files: index.files.len(),
        lock_path: lock_path.to_owned(),
        cache_path,
        created,
    })
}

#[cfg(test)]
mod adversary_tests;
#[cfg(test)]
mod tests;

//! Per-provider bundles and the local index that finds them. A directory and a
//! file are the whole mechanism: nothing here contacts a catalog service, and a
//! bundle whose bytes no longer match what the index recorded is refused rather
//! than read as current.

use crate::{SourceRecord, inventory::Inventory};
use connectors_core::{Error, ErrorCode, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

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

fn write_index(directory: &Path, index: &Index) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(index).map_err(|_| Error::internal())?;
    std::fs::write(index_path(directory), bytes)
        .map_err(|_| refuse(ErrorCode::Unavailable, "index cannot be written"))
}

/// Write a bundle and record it. `replace` is explicit: without it, a provider
/// the index already carries is refused, so a second writer cannot quietly
/// overwrite an inventory somebody is loading.
pub fn write(directory: &Path, bundle: &Bundle, replace: bool) -> Result<Entry> {
    if !connectors_core::valid_id(&bundle.provider) {
        return Err(refuse(
            ErrorCode::InvalidInput,
            "invalid provider identifier",
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
    let bytes = serde_json::to_vec(bundle).map_err(|_| Error::internal())?;
    if bytes.len() > BUNDLE_LIMIT {
        return Err(refuse(
            ErrorCode::Capacity,
            "bundle exceeds the permitted size",
        ));
    }
    std::fs::write(directory.join(&file_name), &bytes)
        .map_err(|_| refuse(ErrorCode::Unavailable, "bundle cannot be written"))?;
    let (operations, unsupported) = bundle.inventory.coverage();
    let entry = Entry {
        provider: bundle.provider.clone(),
        file_name,
        bundle_sha256: hex::encode(Sha256::digest(&bytes)),
        source_sha256: bundle.source.source_sha256.clone(),
        operations,
        unsupported,
    };
    index.entries.retain(|e| e.provider != entry.provider);
    index.entries.push(entry.clone());
    // A stable order, so two directories built by the same writes read the same.
    index.entries.sort_by(|a, b| a.provider.cmp(&b.provider));
    write_index(directory, &index)?;
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

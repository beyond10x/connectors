//! `index.json` as the connectors-definitions repository publishes it beside each tag
//! (`spec/domains/release.yaml` there; `connectors.definitions_install.Index` here).
use super::{FEATURES, FILES_LIMIT, INDEX_SCHEMAS, Refusal, UNPACKED_LIMIT};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

/// A release index this build reads.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Index {
    pub schema: String,
    pub tag: String,
    pub commit: String,
    pub features: Vec<String>,
    pub asset: Asset,
    pub files: Vec<IndexFile>,
}

/// The release tarball.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub name: String,
    pub sha256: String,
}

/// One regular file of the release, its path relative to the release.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexFile {
    pub path: String,
    pub sha256: String,
    pub size: u64,
}

/// The lower-case hexadecimal SHA-256 of `bytes`.
pub fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn hex_of(text: &str, lengths: &[usize]) -> bool {
    lengths.contains(&text.len())
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Whether `path` is relative, with no empty, `.` or `..` segment, no backslash and
/// no NUL, so it names one file beneath any root on every platform.
pub(crate) fn relative_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 4096
        && !path.contains(['\\', '\0'])
        && path
            .split('/')
            .all(|s| !s.is_empty() && s != "." && s != "..")
}

fn invalid(reason: &str) -> Refusal {
    Refusal::ReleaseInvalid(reason.to_owned())
}

impl Index {
    /// Reads `bytes` as the index of the release `tag`: its format version first
    /// ([`Refusal::UnknownFormatVersion`]), then the rest of the format
    /// ([`Refusal::ReleaseInvalid`]), then its features ([`Refusal::UnmetFeature`]).
    pub fn read(bytes: &[u8], tag: &str) -> Result<Self, Refusal> {
        let value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| invalid("index.json is not JSON"))?;
        let object = value
            .as_object()
            .ok_or_else(|| invalid("index.json is not an object"))?;
        match object.get("schema").and_then(serde_json::Value::as_str) {
            Some(schema) if INDEX_SCHEMAS.contains(&schema) => {}
            _ => return Err(Refusal::UnknownFormatVersion),
        }
        let index: Self = serde_json::from_slice(bytes)
            .map_err(|_| invalid("index.json does not have the index format's members"))?;
        if index.tag != tag {
            return Err(invalid("index.json names another tag"));
        }
        if !hex_of(&index.commit, &[40, 64]) {
            return Err(invalid("commit is not a full commit id"));
        }
        if !hex_of(&index.asset.sha256, &[64]) {
            return Err(invalid("asset.sha256 is not a SHA-256"));
        }
        let name = &index.asset.name;
        if !relative_path(name) || name.contains('/') || name.len() > 255 {
            return Err(invalid("asset.name is not one path segment"));
        }
        if index.files.len() > FILES_LIMIT {
            return Err(invalid("index.json names too many files"));
        }
        let mut total: u64 = 0;
        let mut paths = BTreeSet::new();
        let mut previous: Option<&str> = None;
        for file in &index.files {
            if !relative_path(&file.path) {
                return Err(invalid("a file path is not a relative path"));
            }
            if !hex_of(&file.sha256, &[64]) {
                return Err(invalid("a file sha256 is not a SHA-256"));
            }
            if previous.is_some_and(|previous| previous.as_bytes() >= file.path.as_bytes()) {
                return Err(invalid("files are not sorted by path, once each"));
            }
            previous = Some(&file.path);
            total = total
                .checked_add(file.size)
                .filter(|total| *total <= UNPACKED_LIMIT)
                .ok_or_else(|| invalid("the files exceed the unpacked bound"))?;
            paths.insert(file.path.as_str());
        }
        // A path that is also another path's directory cannot be both.
        for file in &index.files {
            let mut prefix = file.path.as_str();
            while let Some((parent, _)) = prefix.rsplit_once('/') {
                if paths.contains(parent) {
                    return Err(invalid("a file path is another file's directory"));
                }
                prefix = parent;
            }
        }
        // A feature is reported by name, so it is a bounded identifier.
        if !index.features.iter().all(|feature| {
            (1..=64).contains(&feature.len())
                && feature
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        }) {
            return Err(invalid("a feature is not 1 to 64 bytes of [a-z0-9_]"));
        }
        if let Some(unmet) = index
            .features
            .iter()
            .find(|feature| !FEATURES.contains(&feature.as_str()))
        {
            return Err(Refusal::UnmetFeature(unmet.clone()));
        }
        Ok(index)
    }
}

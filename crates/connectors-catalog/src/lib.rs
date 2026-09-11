//! Catalog ingest: a pinned OpenAPI source document, recorded with the provenance
//! every later artifact cites. Nothing here reaches the network, generates code or
//! touches a runtime; a document this crate cannot represent is refused by name
//! rather than accepted in part.

pub mod bundle;
pub mod inventory;
pub mod template;

use connectors_core::{Error, ErrorCode, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

/// The largest source document this ingest reads. A larger one is refused rather
/// than truncated, because a partial specification silently under-reports coverage.
pub const SOURCE_LIMIT: usize = 64 * 1024 * 1024;

/// Why a document was refused. Each case is distinct so a report can name it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The file could not be read at all.
    Unreadable,
    /// Larger than `SOURCE_LIMIT`.
    TooLarge,
    /// Not JSON, or JSON this build refuses (duplicate keys, trailing content).
    Malformed,
    /// Parsed, but the root is not a JSON object.
    NotADocument,
    /// No `openapi` field, or one that is not a string.
    VersionAbsent,
    /// An `openapi` value outside 3.0 and 3.1 — Swagger 2.0 included.
    VersionUnsupported,
}

impl Refusal {
    pub fn reason(self) -> &'static str {
        match self {
            Self::Unreadable => "source document cannot be read",
            Self::TooLarge => "source document exceeds the ingest limit",
            Self::Malformed => "source document is not a document this build parses",
            Self::NotADocument => "source root is not an object",
            Self::VersionAbsent => "source declares no openapi version",
            Self::VersionUnsupported => "source declares an openapi version outside 3.0 and 3.1",
        }
    }
}

impl From<Refusal> for Error {
    fn from(value: Refusal) -> Self {
        let code = match value {
            Refusal::VersionUnsupported => ErrorCode::Unsupported,
            Refusal::Unreadable => ErrorCode::NotFound,
            Refusal::TooLarge => ErrorCode::Capacity,
            _ => ErrorCode::InvalidInput,
        };
        Error::new(code, value.reason())
    }
}

/// The two document families this ingest represents. A minor beyond the declared
/// pair is still that family: 3.0.3 and 3.1.1 are `V30` and `V31`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Dialect {
    #[serde(rename = "3.0")]
    V30,
    #[serde(rename = "3.1")]
    V31,
}

/// What was ingested, and what it came from. `source_sha256` is over the exact
/// file bytes, not over a parsed or re-serialised form, so it can be recomputed
/// with `sha256sum` on the original file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRecord {
    pub file_name: String,
    pub source_sha256: String,
    pub source_bytes: usize,
    pub dialect: Dialect,
    /// The document's own `openapi` string, kept exactly as written.
    pub openapi: String,
    /// `info.version`, when the document carries one.
    pub info_version: Option<String>,
    /// `info.license.name`, when the document carries one.
    pub license: Option<String>,
}

fn dialect(openapi: &str) -> std::result::Result<Dialect, Refusal> {
    let mut parts = openapi.split('.');
    let major = parts.next().unwrap_or_default();
    let minor = parts.next().unwrap_or_default();
    match (major, minor) {
        ("3", "0") => Ok(Dialect::V30),
        ("3", "1") => Ok(Dialect::V31),
        _ => Err(Refusal::VersionUnsupported),
    }
}

fn text(value: &serde_json::Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(key)?;
    }
    current.as_str().map(str::to_owned)
}

/// Ingest exactly these bytes, under this file name.
pub fn ingest(file_name: &str, bytes: &[u8]) -> std::result::Result<SourceRecord, Refusal> {
    if bytes.len() > SOURCE_LIMIT {
        return Err(Refusal::TooLarge);
    }
    let document: serde_json::Value =
        connectors_core::read_json(bytes).map_err(|_| Refusal::Malformed)?;
    if !document.is_object() {
        return Err(Refusal::NotADocument);
    }
    let openapi = match document.get("openapi") {
        Some(serde_json::Value::String(value)) => value.clone(),
        _ => return Err(Refusal::VersionAbsent),
    };
    Ok(SourceRecord {
        file_name: file_name.to_owned(),
        source_sha256: hex::encode(Sha256::digest(bytes)),
        source_bytes: bytes.len(),
        dialect: dialect(&openapi)?,
        openapi,
        info_version: text(&document, &["info", "version"]),
        license: text(&document, &["info", "license", "name"]),
    })
}

/// Read and ingest a local file. The name recorded is the file's own name, not
/// the path it was read through, so a record does not carry a home directory.
pub fn ingest_file(path: &Path) -> Result<SourceRecord> {
    let bytes = std::fs::read(path).map_err(|_| Error::from(Refusal::Unreadable))?;
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| Error::from(Refusal::Unreadable))?;
    ingest(name, &bytes).map_err(Error::from)
}

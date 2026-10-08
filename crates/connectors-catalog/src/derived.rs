//! The documents a catalog source may be projected from, told apart by their own
//! content: a JSON object with a top-level `swagger` key is Swagger 2.0
//! ([`crate::swagger`]); anything else is read as Google Discovery
//! ([`crate::discovery`]), which refuses what it is not. The choice reads the
//! document, never its file name, so a pinned source cannot be projected by the
//! wrong projector because of what it is called.

use crate::{Derivation, SOURCE_LIMIT, discovery, swagger};

/// A projected source, from whichever format it was projected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Projection {
    Discovery(discovery::Projection),
    Swagger(swagger::Projection),
}

/// The refusing projector's own refusal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    Discovery(discovery::Refusal),
    Swagger(swagger::Refusal),
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Discovery(refusal) => refusal.fmt(f),
            Self::Swagger(refusal) => refusal.fmt(f),
        }
    }
}

impl std::error::Error for Refusal {}

impl From<Refusal> for connectors_core::Error {
    fn from(value: Refusal) -> Self {
        match value {
            Refusal::Discovery(refusal) => refusal.into(),
            Refusal::Swagger(refusal) => refusal.into(),
        }
    }
}

fn is_swagger(bytes: &[u8]) -> bool {
    bytes.len() <= SOURCE_LIMIT
        && connectors_core::read_json::<serde_json::Value>(bytes)
            .is_ok_and(|document| document.get("swagger").is_some())
}

/// Project `bytes` with the projector its content names.
pub fn project(bytes: &[u8]) -> Result<Projection, Refusal> {
    if is_swagger(bytes) {
        swagger::project(bytes)
            .map(Projection::Swagger)
            .map_err(Refusal::Swagger)
    } else {
        discovery::project(bytes)
            .map(Projection::Discovery)
            .map_err(Refusal::Discovery)
    }
}

impl Projection {
    /// The projected OpenAPI document's bytes.
    pub fn openapi(&self) -> &[u8] {
        match self {
            Self::Discovery(projection) => &projection.openapi,
            Self::Swagger(projection) => &projection.openapi,
        }
    }

    /// The projection record, canonical like the document.
    pub fn record_bytes(&self) -> Vec<u8> {
        match self {
            Self::Discovery(projection) => projection.record_bytes(),
            Self::Swagger(projection) => projection.record_bytes(),
        }
    }

    /// The input format a derivation names.
    pub fn format(&self) -> &'static str {
        match self {
            Self::Discovery(_) => discovery::FORMAT,
            Self::Swagger(_) => swagger::FORMAT,
        }
    }

    /// The derivation a source projected from the file `from_file` records.
    pub fn derivation(&self, from_file: &str) -> Derivation {
        match self {
            Self::Discovery(projection) => Derivation {
                from_file: from_file.to_owned(),
                from_sha256: projection.record.source_sha256.clone(),
                from_bytes: projection.record.source_bytes,
                format: discovery::FORMAT.to_owned(),
                discovery_revision: Some(projection.record.discovery_revision.clone()),
                projector: projection.record.projector.clone(),
            },
            Self::Swagger(projection) => Derivation {
                from_file: from_file.to_owned(),
                from_sha256: projection.record.source_sha256.clone(),
                from_bytes: projection.record.source_bytes,
                format: swagger::FORMAT.to_owned(),
                discovery_revision: None,
                projector: projection.record.projector.clone(),
            },
        }
    }
}

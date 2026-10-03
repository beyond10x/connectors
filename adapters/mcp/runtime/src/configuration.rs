//! Validated native deployment input. Opening files and admitting the configured
//! owner belong to application composition, never to this protocol library.
use configuration_types::{ConnectorsMcpLocalServerConfiguration as Document, EssPresence};
use std::{
    collections::BTreeSet,
    path::{Component, Path},
};

pub const FILE_LIMIT: usize = 1024 * 1024;
pub const FORMAT: &str = "connectors-mcp-local/1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Size,
    Json,
    Shape,
    Format,
    Limits,
    Exposure,
    ApprovalPath,
}

/// The structural carrier comes from ESS. Validation adds deployment bounds;
/// neither successful parsing nor an exposure grants host/provider authority.
pub struct Configuration {
    document: Document,
}
impl Configuration {
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > FILE_LIMIT {
            return Err(Error::Size);
        }
        let value = connectors_core::json::decode(bytes, 16).map_err(|_| Error::Json)?;
        let limits = value
            .get("limits")
            .and_then(serde_json::Value::as_object)
            .ok_or(Error::Limits)?;
        // Check actual JSON number nodes before generated Number deserialization:
        // objects that resemble Serde's private number token are still objects.
        for (name, low, high) in [
            ("frame_octets", 256, 1_048_576),
            ("response_octets", 1024, 33_554_432),
            ("concurrent_requests", 1, 16),
            ("request_milliseconds", 1, 120_000),
        ] {
            let number = limits
                .get(name)
                .and_then(serde_json::Value::as_u64)
                .ok_or(Error::Limits)?;
            if !(low..=high).contains(&number) {
                return Err(Error::Limits);
            }
        }
        let document: Document = serde_json::from_value(value).map_err(|_| Error::Shape)?;
        if document.format != FORMAT {
            return Err(Error::Format);
        }
        if document.exposures.len() > 1000 {
            return Err(Error::Exposure);
        }
        let mut identities = BTreeSet::new();
        for exposure in &document.exposures {
            if [
                &exposure.adapter_alias,
                &exposure.operation_ref,
                &exposure.connection_ref,
            ]
            .iter()
            .any(|id| !connectors_core::valid_id(id))
                || exposure.families.is_empty()
                || !identities.insert((&exposure.adapter_alias, &exposure.operation_ref))
            {
                return Err(Error::Exposure);
            }
            let mut families = BTreeSet::new();
            for family in &exposure.families {
                // The generated enum owns wire spellings; do not mirror it.
                let name = serde_json::to_string(family).map_err(|_| Error::Shape)?;
                if !families.insert(name) {
                    return Err(Error::Exposure);
                }
            }
            if let EssPresence::Present(path) = &exposure.approval_file
                && (path.len() > 4096
                    || path.chars().any(char::is_control)
                    || !Path::new(path).is_absolute()
                    || Path::new(path).file_name().is_none()
                    || Path::new(path)
                        .components()
                        .any(|c| !matches!(c, Component::RootDir | Component::Normal(_))))
            {
                return Err(Error::ApprovalPath);
            }
        }
        Ok(Self { document })
    }
    pub fn document(&self) -> &Document {
        &self.document
    }
}

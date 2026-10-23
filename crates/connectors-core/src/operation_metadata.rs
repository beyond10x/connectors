//! Explicit public metadata values. Decoding grants no execution authority.
pub use operation_types::ConnectorsServiceWireOperationMetadata as Document;
use operation_types::EssPresence;
pub use operation_types::{
    ConnectorsServiceWireAdapterOperationCuration as AdapterCuration,
    ConnectorsServiceWireOperationCuration as CurationDocument,
};
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Size,
    Json,
    Shape,
    Effects,
    Idempotency,
    Authentication,
    Realization,
    Limits,
}

/// Shape and cross-field consistency only. The receiver must separately prove
/// supported effects/profile, complete curation, native repeat semantics, current
/// grants and enforcement of every declared limit before advertising a binding.
#[derive(Debug, Clone, PartialEq)]
pub struct Metadata {
    document: Document,
}

pub const CURATION_LIMIT: usize = 1024 * 1024;
pub const CURATION_FORMAT: &str = "connectors-operation-curation/1";

/// Protected declaration input. Selection against the exact native descriptor,
/// executable, auth requirement and supported execution binding is still required.
pub struct Curation {
    document: CurationDocument,
}
impl Curation {
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > CURATION_LIMIT {
            return Err(Error::Size);
        }
        let value = crate::json::decode(bytes, 16).map_err(|_| Error::Json)?;
        let document: CurationDocument =
            serde_json::from_value(value.clone()).map_err(|_| Error::Shape)?;
        if document.format != CURATION_FORMAT || document.adapters.len() > 1000 {
            return Err(Error::Shape);
        }
        let mut aliases = BTreeSet::new();
        for (index, adapter) in document.adapters.iter().enumerate() {
            if !crate::valid_id(&adapter.adapter_alias)
                || !aliases.insert(&adapter.adapter_alias)
                || !sha256(&adapter.executable_selection)
                || !sha256(&adapter.bootstrap_sha256)
                || adapter.operations.len() > 256
            {
                return Err(Error::Shape);
            }
            let mut operations = BTreeSet::new();
            for (position, operation) in adapter.operations.iter().enumerate() {
                if !crate::valid_id(&operation.operation)
                    || !operations.insert(&operation.operation)
                {
                    return Err(Error::Shape);
                }
                // Validate original JSON nodes before any generated numeric
                // representation can erase their identity. Profile binding is
                // deliberately deferred until the exact descriptor is selected.
                Metadata::from_value(
                    value["adapters"][index]["operations"][position]["metadata"].clone(),
                    None,
                )?;
            }
        }
        Ok(Self { document })
    }
    pub fn document(&self) -> &CurationDocument {
        &self.document
    }
}

impl Metadata {
    /// `profile` comes from the selected operation; it is not overwritten by
    /// metadata. The enclosing descriptor/configuration supplies the byte budget.
    pub fn parse(bytes: &[u8], profile: &str, maximum_bytes: usize) -> Result<Self, Error> {
        if maximum_bytes == 0 || bytes.len() > maximum_bytes {
            return Err(Error::Size);
        }
        if !crate::valid_id(profile) {
            return Err(Error::Shape);
        }
        let value = crate::json::decode(bytes, 16).map_err(|_| Error::Json)?;
        Self::from_value(value, Some(profile))
    }

    // None validates declaration shape only, and is used exclusively inside
    // protected curation decoding. No unbound Metadata escapes that path.
    fn from_value(value: Value, profile: Option<&str>) -> Result<Self, Error> {
        // Check JSON node identity before generated Number deserialization, which
        // otherwise accepts Serde's private object carrier as if it were a number.
        let limits = value.get("limits").ok_or(Error::Limits)?;
        for field in [
            "request_bytes",
            "result_bytes",
            "execution_ms",
            "provider_ms",
            "connect_ms",
        ] {
            if !limits
                .get(field)
                .and_then(Value::as_u64)
                .is_some_and(|n| n > 0)
            {
                return Err(Error::Limits);
            }
        }
        if let Some(seconds) = value
            .get("idempotency")
            .and_then(|v| v.get("retention_seconds"))
            && seconds.as_u64() != Some(86_400)
        {
            return Err(Error::Idempotency);
        }
        let document: Document = serde_json::from_value(value.clone()).map_err(|_| Error::Shape)?;
        if !distinct(&document.effects) || !distinct(&document.semantic_effects) {
            return Err(Error::Effects);
        }
        // Compare wire spellings, not generated ordinal enum names: adding an
        // unrelated model variant must not change what this discriminator means.
        let effects = value["effects"].as_array().ok_or(Error::Shape)?;
        let has = |effect: &str| effects.iter().any(|v| v.as_str() == Some(effect));
        if profile.is_some_and(|profile| has("external_write") != (profile == "mutation"))
            || ((has("send_external") || has("session_establishment")) && !has("external_write"))
        {
            return Err(Error::Effects);
        }
        let idempotency = &value["idempotency"];
        if idempotency["kind"] == "keyed" {
            if !has("external_write")
                || profile.is_some_and(|profile| profile != "mutation")
                || idempotency["key"] != "caller_supplied"
                || idempotency["retention_seconds"].as_u64() != Some(86_400)
            {
                return Err(Error::Idempotency);
            }
        } else if !document.idempotency.key.is_absent()
            || !document.idempotency.retention_seconds.is_absent()
        {
            return Err(Error::Idempotency);
        }
        if value.get("realization").is_some_and(|v| v == "unresolved") {
            return Err(Error::Realization);
        }
        if let EssPresence::Present(alternatives) = &document.requires_auth {
            if alternatives.is_empty() {
                return Err(Error::Authentication);
            }
            let mut seen = BTreeSet::new();
            for alternative in alternatives {
                let scopes: BTreeSet<_> = alternative.scopes.iter().map(String::as_str).collect();
                if !crate::valid_id(&alternative.profile)
                    || scopes.len() != alternative.scopes.len()
                    || scopes
                        .iter()
                        .any(|s| s.is_empty() || s.chars().any(char::is_control))
                    || !seen.insert((alternative.profile.as_str(), scopes))
                {
                    return Err(Error::Authentication);
                }
            }
        }
        Ok(Self { document })
    }

    pub fn document(&self) -> &Document {
        &self.document
    }

    pub fn to_value(&self) -> Result<Value, Error> {
        serde_json::to_value(&self.document).map_err(|_| Error::Shape)
    }
}

fn sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn distinct<T: serde::Serialize>(values: &[T]) -> bool {
    let mut seen = BTreeSet::new();
    values
        .iter()
        .all(|value| serde_json::to_string(value).is_ok_and(|value| seen.insert(value)))
}

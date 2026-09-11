//! Native protected entry for a PostgreSQL password. The host owns acquisition,
//! custody, publication, clocks and the credential capability; this module owns
//! only the document shape and the native failure vocabulary.
//!
//! There is no probe here, and deliberately so. For PostgreSQL the authenticated
//! session *is* the credential check: the server accepts or rejects the password
//! during the startup exchange, before any statement can run. Inventing a
//! validation query would add a business read that proves less than the
//! handshake already proved.
use connectors_sdk::Secret;
use serde::{Deserialize, Deserializer};
use zeroize::Zeroizing;

pub const PROFILE_ID: &str = "postgres.password";
pub const DOCUMENT_LIMIT: usize = 64 * 1024;
/// A session credential carries no expiry the client can read, so evidence is
/// bounded by the same short lifetime the other local profiles use.
pub const EVIDENCE_LIFETIME_MS: u64 = 60_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    InvalidEntry,
}

/// Sensitive input intentionally has no Debug or Serialize implementation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtectedEntry {
    #[serde(deserialize_with = "password")]
    password: Zeroizing<String>,
}

fn password<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Zeroizing<String>, D::Error> {
    let value = Zeroizing::new(String::deserialize(deserializer)?);
    // PostgreSQL accepts an empty password, but a saved connection that carries
    // one is indistinguishable from a missing field, so it is refused here.
    if value.is_empty() || value.len() > 8192 || value.bytes().any(|b| b == 0) {
        return Err(serde::de::Error::custom("invalid protected field"));
    }
    Ok(value)
}

impl ProtectedEntry {
    /// Own and erase the input even on rejection. Serde rejects duplicates,
    /// unknown fields, malformed UTF-8 and trailing documents without echoing.
    pub fn parse(document: Vec<u8>) -> Result<Self, Failure> {
        let document = Zeroizing::new(document);
        if document.len() > DOCUMENT_LIMIT {
            return Err(Failure::InvalidEntry);
        }
        serde_json::from_slice(&document).map_err(|_| Failure::InvalidEntry)
    }

    /// Transfer only the password into its trusted transport boundary.
    pub fn into_secret(mut self) -> Secret {
        Secret(std::mem::take(&mut *self.password).into_bytes())
    }
}

//! Native static entry and identity probe of the profile `prometheus.bearer`
//! (`spec/ess/domains/connection.yaml`; `contracts/series/v1alpha1/semantics.md`
//! §11.4). The host owns acquisition, custody, publication, clocks and the
//! authenticated capability; this module owns only the entry's shape and the
//! check.
//!
//! The check is `GET api/v1/status/buildinfo`: a 200 proves the token reaches
//! the deployment (or the Grafana data source proxying it) and runs no query.
//! Prometheus has no user or account read, so the answer's
//! body is not used and the identity is the configured connection: two tokens
//! the deployment accepts are the same identity, and a repair cannot detect a
//! token of another tenant.
use connectors_core::ErrorCode;
use connectors_sdk::{AuthenticatedHttp, Secret};
use serde::{Deserialize, Deserializer};
use zeroize::Zeroizing;

pub const PROFILE_ID: &str = "prometheus.bearer";
/// The identity kind recorded beside the configured `instance` subject.
pub const IDENTITY_KIND: &str = "prometheus.connection";
/// The identity probe, relative to the configured base URL.
pub const PROBE_PATH: [&str; 4] = ["api", "v1", "status", "buildinfo"];
pub const DOCUMENT_LIMIT: usize = 16 * 1024;
pub const TOKEN_BYTES: usize = 8192;
pub const EVIDENCE_LIFETIME_MS: u64 = 60_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    InvalidEntry,
    InvalidCredential,
    InvalidResponse,
    Unavailable,
    Deadline,
}

/// Sensitive input intentionally has no Debug or Serialize implementation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtectedEntry {
    #[serde(deserialize_with = "token")]
    token: Zeroizing<String>,
}

fn token<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Zeroizing<String>, D::Error> {
    let value = Zeroizing::new(String::deserialize(deserializer)?);
    if value.is_empty() || value.len() > TOKEN_BYTES || !value.bytes().all(|b| b.is_ascii_graphic())
    {
        return Err(serde::de::Error::custom("invalid protected field"));
    }
    Ok(value)
}

impl ProtectedEntry {
    /// Own and erase the input even on rejection.
    pub fn parse(document: Vec<u8>) -> Result<Self, Failure> {
        let document = Zeroizing::new(document);
        if document.len() > DOCUMENT_LIMIT {
            return Err(Failure::InvalidEntry);
        }
        // The derived reader refuses a duplicate member, so `{"token":"a","token":"b"}`
        // selects neither value, and it builds no unerased intermediate copy.
        serde_json::from_slice(&document).map_err(|_| Failure::InvalidEntry)
    }

    /// Transfer only the token into its trusted transport boundary.
    pub fn into_secret(mut self) -> Secret {
        Secret(std::mem::take(&mut *self.token).into_bytes())
    }
}

pub struct Validation {
    pub collected_at_ms: u64,
    pub valid_until_ms: u64,
}

/// `GET api/v1/status/buildinfo` with the token. 200 admits it; 401 and 403 refuse
/// it; 429 and 5xx are unavailable; any other status is a protocol failure.
pub async fn validate(
    http: &dyn AuthenticatedHttp,
    clock: impl Fn() -> u64,
) -> Result<Validation, Failure> {
    let collected_at_ms = clock();
    let response = http.get(&PROBE_PATH, &[]).await.map_err(|e| match e.code {
        ErrorCode::Timeout => Failure::Deadline,
        _ => Failure::Unavailable,
    })?;
    match response.status {
        200 => Ok(Validation {
            collected_at_ms,
            valid_until_ms: collected_at_ms.saturating_add(EVIDENCE_LIFETIME_MS),
        }),
        401 | 403 => Err(Failure::InvalidCredential),
        429 | 500..=599 => Err(Failure::Unavailable),
        _ => Err(Failure::InvalidResponse),
    }
}

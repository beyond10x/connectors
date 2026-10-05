//! Native static entry and baseline validation for a Tavily API key. The host
//! owns acquisition, custody, publication, clocks and the authenticated
//! capability; this module owns only the entry's shape and the check.
//!
//! The check is `GET /usage`, which reports the key's and the account's usage
//! and spends no search credit. Tavily exposes no account identifier, so the
//! connection's identity is the profile itself: two keys of one or two
//! accounts are not told apart, and a repair cannot detect a changed account.
use connectors_core::ErrorCode;
use connectors_sdk::{AuthenticatedHttp, Secret};
use serde::{Deserialize, Deserializer};
use zeroize::Zeroizing;

pub const PROFILE_ID: &str = "tavily.api-key";
pub const DOCUMENT_LIMIT: usize = 64 * 1024;
pub const EVIDENCE_LIFETIME_MS: u64 = 60_000;
pub const USAGE_ENDPOINT: [&str; 1] = ["usage"];

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
    #[serde(deserialize_with = "api_key")]
    api_key: Zeroizing<String>,
}

fn api_key<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Zeroizing<String>, D::Error> {
    let value = Zeroizing::new(String::deserialize(deserializer)?);
    if value.is_empty() || value.len() > 512 || !value.bytes().all(|b| b.is_ascii_graphic()) {
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
        serde_json::from_slice(&document).map_err(|_| Failure::InvalidEntry)
    }

    /// Transfer only the key into its trusted transport boundary.
    pub fn into_secret(mut self) -> Secret {
        Secret(std::mem::take(&mut *self.api_key).into_bytes())
    }
}

pub struct Validation {
    /// Tavily's plan name for the account, where it reports one.
    pub plan: Option<String>,
    pub collected_at_ms: u64,
    pub valid_until_ms: u64,
}

/// `GET /usage` with the key; a 200 with a `key` object is a usable key.
pub async fn validate(
    http: &dyn AuthenticatedHttp,
    clock: impl Fn() -> u64,
) -> Result<Validation, Failure> {
    let collected_at_ms = clock();
    let response = http
        .get(&USAGE_ENDPOINT, &[])
        .await
        .map_err(|e| match e.code {
            ErrorCode::Timeout => Failure::Deadline,
            _ => Failure::Unavailable,
        })?;
    match response.status {
        200 => {}
        401 | 403 => return Err(Failure::InvalidCredential),
        429 | 432 | 433 | 500..=599 => return Err(Failure::Unavailable),
        _ => return Err(Failure::InvalidResponse),
    }
    if response.body.len() > DOCUMENT_LIMIT {
        return Err(Failure::InvalidResponse);
    }
    let usage: serde_json::Value =
        connectors_core::read_json(&response.body).map_err(|_| Failure::InvalidResponse)?;
    if !usage.get("key").is_some_and(serde_json::Value::is_object) {
        return Err(Failure::InvalidResponse);
    }
    let plan = usage
        .pointer("/account/current_plan")
        .and_then(serde_json::Value::as_str)
        .filter(|p| {
            !p.is_empty() && p.len() <= 128 && p.bytes().all(|b| b.is_ascii_graphic() || b == b' ')
        })
        .map(str::to_owned);
    Ok(Validation {
        plan,
        collected_at_ms,
        valid_until_ms: collected_at_ms.saturating_add(EVIDENCE_LIFETIME_MS),
    })
}

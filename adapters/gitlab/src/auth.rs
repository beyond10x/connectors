//! Native static-entry and baseline validation; the host owns acquisition,
//! custody, publication, clocks and the exact authenticated capability.
use connectors_core::ErrorCode;
use connectors_sdk::{AuthenticatedHttp, HttpResponse, Secret};
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::collections::BTreeSet;
use zeroize::Zeroizing;

pub const PROFILE_ID: &str = "gitlab.pat";
pub const DOCUMENT_LIMIT: usize = 64 * 1024;
pub const VALIDATION_BUDGET_MS: u64 = 30_000;
pub const EVIDENCE_LIFETIME_MS: u64 = 60_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    InvalidEntry,
    InvalidCredential,
    InsufficientScope,
    IdentityMismatch,
    InvalidResponse,
    Unavailable,
    PermissionDenied,
    Expired,
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
    if value.is_empty() || value.len() > 8192 || !value.bytes().all(|b| b.is_ascii_graphic()) {
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

    /// Transfer only the token into its trusted transport boundary. Neither
    /// the original source nor its path is retained by the provider adapter.
    pub fn into_secret(mut self) -> Secret {
        Secret(std::mem::take(&mut *self.token).into_bytes())
    }
}

/// Native observation, not a host publication or dispatch permit. The owner
/// associates it with the exact captured generation and fixed connection.
pub struct BaselineValidation {
    pub user_id: i64,
    pub granted_scopes: BTreeSet<String>,
    pub collected_at_ms: u64,
    pub expires_at_ms: Option<u64>,
    pub valid_until_ms: u64,
}

impl BaselineValidation {
    pub fn is_current(&self, now_ms: u64) -> bool {
        self.collected_at_ms <= now_ms && now_ms < self.valid_until_ms
    }
}

#[derive(Deserialize)]
struct UserObservation {
    id: i64,
    state: String,
}

#[derive(Deserialize)]
struct TokenObservation {
    id: i64,
    user_id: i64,
    active: bool,
    revoked: bool,
    scopes: Vec<String>,
    // Value preserves the distinction between explicit null and a missing field.
    expires_at: Value,
}

fn response(response: HttpResponse) -> Result<Value, Failure> {
    match response.status {
        200 => {}
        401 => return Err(Failure::InvalidCredential),
        403 => return Err(Failure::PermissionDenied),
        429 | 500..=599 => return Err(Failure::Unavailable),
        _ => return Err(Failure::InvalidResponse),
    }
    if response.body.len() > DOCUMENT_LIMIT {
        return Err(Failure::InvalidResponse);
    }
    connectors_core::read_json(&response.body).map_err(|_| Failure::InvalidResponse)
}

fn transport(error: connectors_core::Error) -> Failure {
    match error.code {
        ErrorCode::Timeout => Failure::Deadline,
        _ => Failure::Unavailable,
    }
}

fn expiry(value: &Value) -> Result<Option<u64>, Failure> {
    if value.is_null() {
        return Ok(None);
    }
    let date = value.as_str().ok_or(Failure::InvalidResponse)?;
    if date.len() != 10 {
        return Err(Failure::InvalidResponse);
    }
    let date = time::Date::parse(
        date,
        time::macros::format_description!("[year]-[month]-[day]"),
    )
    .map_err(|_| Failure::InvalidResponse)?;
    let timestamp = date.midnight().assume_utc().unix_timestamp();
    let milliseconds = u64::try_from(timestamp)
        .ok()
        .and_then(|seconds| seconds.checked_mul(1000))
        .ok_or(Failure::InvalidResponse)?;
    Ok(Some(milliseconds))
}

/// The execution owner enforces the total timeout/cancellation and supplies a
/// trusted wall clock. Both reads use this one immutable authenticated capability.
/// This function performs no token exchange, refresh or implicit retries.
pub async fn validate(
    http: &dyn AuthenticatedHttp,
    clock: impl Fn() -> u64,
) -> Result<BaselineValidation, Failure> {
    let collected_at_ms = clock();
    let user: UserObservation = serde_json::from_value(response(
        http.get(&["user"], &[]).await.map_err(transport)?,
    )?)
    .map_err(|_| Failure::InvalidResponse)?;
    if user.id <= 0 {
        return Err(Failure::InvalidResponse);
    }
    if user.state != "active" {
        return Err(Failure::InvalidCredential);
    }
    let after_user = clock();
    if after_user < collected_at_ms || after_user - collected_at_ms >= VALIDATION_BUDGET_MS {
        return Err(Failure::Deadline);
    }
    let value = response(
        http.get(&["personal_access_tokens", "self"], &[])
            .await
            .map_err(transport)?,
    )?;
    // Granular scopes are a separate native profile, never an inferred grant.
    if value.get("expires_at").is_none() || value.get("granular_scopes").is_some() {
        return Err(Failure::InvalidResponse);
    }
    let token: TokenObservation =
        serde_json::from_value(value).map_err(|_| Failure::InvalidResponse)?;
    if token.id <= 0 || token.user_id <= 0 {
        return Err(Failure::InvalidResponse);
    }
    if token.user_id != user.id {
        return Err(Failure::IdentityMismatch);
    }
    if !token.active || token.revoked {
        return Err(Failure::InvalidCredential);
    }
    if token.scopes.is_empty()
        || token.scopes.len() > 64
        || token.scopes.iter().any(|scope| {
            scope.is_empty() || scope.len() > 256 || !scope.bytes().all(|b| b.is_ascii_graphic())
        })
    {
        return Err(Failure::InvalidResponse);
    }
    let mut granted_scopes: BTreeSet<_> = token.scopes.iter().cloned().collect();
    if granted_scopes.len() != token.scopes.len() {
        return Err(Failure::InvalidResponse);
    }
    if granted_scopes.contains("api") {
        granted_scopes.insert("read_api".into());
    }
    if granted_scopes.len() > 64 {
        return Err(Failure::InvalidResponse);
    }
    if !granted_scopes.contains("read_api") {
        return Err(Failure::InsufficientScope);
    }
    let expires_at_ms = expiry(&token.expires_at)?;
    let now = clock();
    if now < after_user || now - collected_at_ms >= VALIDATION_BUDGET_MS {
        return Err(Failure::Deadline);
    }
    if expires_at_ms.is_some_and(|expiry| expiry <= now) {
        return Err(Failure::Expired);
    }
    let valid_until_ms = collected_at_ms
        .checked_add(EVIDENCE_LIFETIME_MS)
        .ok_or(Failure::Deadline)?
        .min(expires_at_ms.unwrap_or(u64::MAX));
    Ok(BaselineValidation {
        user_id: user.id,
        granted_scopes,
        collected_at_ms,
        expires_at_ms,
        valid_until_ms,
    })
}

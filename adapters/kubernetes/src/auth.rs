//! Native static-entry and baseline validation for a Kubernetes cluster
//! credential. The host owns acquisition, custody, publication, clocks and the
//! exact authenticated capability; this module owns only the native probe and
//! its interpretation.
//!
//! `contracts/auth/v1alpha1/semantics.md` selects SelfSubjectReview as the
//! identity-validation probe and states that ordinary business reads cannot
//! invent it. It is therefore issued through a check-specific capability whose
//! endpoint the executable composition fixes, never through the read port.
use connectors_core::ErrorCode;
use connectors_sdk::{AuthProbe, HttpResponse, Secret};
use serde::{Deserialize, Deserializer};
use serde_json::{Value, json};
use zeroize::Zeroizing;

pub const PROFILE_ID: &str = "kubernetes.token";
pub const DOCUMENT_LIMIT: usize = 64 * 1024;
pub const VALIDATION_BUDGET_MS: u64 = 30_000;
/// The native auth contract admits at most 300 s of read evidence reuse. This
/// first profile takes the shorter lifetime the GitLab binding already uses.
pub const EVIDENCE_LIFETIME_MS: u64 = 60_000;
/// The fixed SelfSubjectReview endpoint. The composition passes these segments
/// to the probe capability once; nothing later can retarget them.
pub const REVIEW_ENDPOINT: [&str; 4] =
    ["apis", "authentication.k8s.io", "v1", "selfsubjectreviews"];
/// Kubernetes names this principal when a request carried no accepted
/// credential. A saved connection is an authenticated one, so it is refused
/// here rather than recorded as an identity.
pub const ANONYMOUS: &str = "system:anonymous";
/// The group every unauthenticated request carries, and the one RBAC binds such
/// requests to. A deployment can configure the anonymous principal's username,
/// so the username alone is not a sufficient test: a renamed anonymous
/// principal would otherwise be saved as an ordinary identity. This group is
/// checked as well, and it is the authorization-relevant half of the pair.
pub const UNAUTHENTICATED_GROUP: &str = "system:unauthenticated";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    InvalidEntry,
    InvalidCredential,
    InvalidResponse,
    Unavailable,
    PermissionDenied,
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

    /// Transfer only the token into its trusted transport boundary. Neither the
    /// original source nor its path is retained by the provider adapter.
    pub fn into_secret(mut self) -> Secret {
        Secret(std::mem::take(&mut *self.token).into_bytes())
    }
}

/// Native observation, not a host publication or dispatch permit. The owner
/// associates it with the exact captured generation and fixed connection.
pub struct BaselineValidation {
    /// The authenticated username. RBAC binds to this name, so it is the
    /// authorization-relevant identity. `uid` is authenticator-dependent and
    /// absent for several accepted credential kinds, so making it the subject
    /// would report an unchanged principal as a switch on those clusters.
    pub username: String,
    pub groups: Vec<String>,
    pub collected_at_ms: u64,
    pub valid_until_ms: u64,
}

impl BaselineValidation {
    pub fn is_current(&self, now_ms: u64) -> bool {
        self.collected_at_ms <= now_ms && now_ms < self.valid_until_ms
    }
}

#[derive(Deserialize)]
struct ReviewObservation {
    status: ReviewStatus,
}
#[derive(Deserialize)]
struct ReviewStatus {
    #[serde(rename = "userInfo")]
    user_info: UserInfo,
}
#[derive(Deserialize)]
struct UserInfo {
    username: String,
    #[serde(default)]
    groups: Vec<String>,
}

fn response(response: HttpResponse) -> Result<Value, Failure> {
    match response.status {
        200 | 201 => {}
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

fn name(value: &str, max: usize) -> bool {
    !value.is_empty() && value.len() <= max && value.bytes().all(|b| b.is_ascii_graphic())
}

/// The execution owner enforces the total timeout/cancellation and supplies a
/// trusted wall clock. One fixed-endpoint probe runs; this function performs no
/// token exchange, refresh, permission check or implicit retry, and it issues no
/// business read.
pub async fn validate(
    probe: &dyn AuthProbe,
    clock: impl Fn() -> u64,
) -> Result<BaselineValidation, Failure> {
    let collected_at_ms = clock();
    let value = response(
        probe
            .probe(&json!({
                "apiVersion": "authentication.k8s.io/v1",
                "kind": "SelfSubjectReview"
            }))
            .await
            .map_err(transport)?,
    )?;
    if value["apiVersion"] != json!("authentication.k8s.io/v1")
        || value["kind"] != json!("SelfSubjectReview")
    {
        return Err(Failure::InvalidResponse);
    }
    let review: ReviewObservation =
        serde_json::from_value(value).map_err(|_| Failure::InvalidResponse)?;
    let username = review.status.user_info.username;
    if !name(&username, 512) {
        return Err(Failure::InvalidResponse);
    }
    let groups = review.status.user_info.groups;
    if groups.len() > 256 || groups.iter().any(|group| !name(group, 512)) {
        return Err(Failure::InvalidResponse);
    }
    // An accepted anonymous request is a well-formed response and not a saved
    // identity. Deliberate anonymous access is a separate declared profile.
    // Either half refuses on its own: the well-known username, or the group
    // that is present whatever the deployment called the principal.
    if username == ANONYMOUS || groups.iter().any(|group| group == UNAUTHENTICATED_GROUP) {
        return Err(Failure::InvalidCredential);
    }
    let now = clock();
    if now < collected_at_ms || now - collected_at_ms >= VALIDATION_BUDGET_MS {
        return Err(Failure::Deadline);
    }
    let valid_until_ms = collected_at_ms
        .checked_add(EVIDENCE_LIFETIME_MS)
        .ok_or(Failure::Deadline)?;
    Ok(BaselineValidation {
        username,
        groups,
        collected_at_ms,
        valid_until_ms,
    })
}

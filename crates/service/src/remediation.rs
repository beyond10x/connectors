//! Non-consuming remediation ports.
//!
//! These internal values are not Grant proofs, dispatch authority, or output-safe DTOs.
//! The reviewed receiver and acquisition owner must supply current admission at every use.

use std::fmt;

use protocol::connection_v2::{
    BoundRemediationStatus, RemediationAcknowledgeRequest, RemediationAcknowledgement,
    RemediationStatusRequest,
};
use protocol::operation::v3::AuthenticationNeed;

use crate::PrincipalContext;

/// The five ESS observations, made for the exact selected credential generation.
///
/// This is a point-in-time classification, not a lease or a cached permission. The ordinary
/// execution/acquisition owner rechecks under its existing gate. Ready includes a credential
/// that the existing safe refresh path can use without interactive acquisition. Dependency
/// failures, generic 403 and channel state must not be classified as CredentialDegraded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialReadiness {
    Ready,
    MissingCredential,
    CredentialDegraded,
    DependencyUnavailable,
    Unsupported,
}

/// Closed value-free port errors. No arbitrary backend or daemon message is retained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RemediationError {
    #[error("authentication remediation is unsupported")]
    Unsupported,
    #[error("authentication remediation is not admitted")]
    Refused,
    #[error("authentication remediation is unavailable")]
    Unavailable,
    #[error("authentication remediation input is invalid")]
    InvalidInput,
    #[error("authentication remediation state conflicts with this request")]
    Conflict,
}

/// Exact caller-selected coordinates after wire validation; not proof of admission.
#[derive(Clone, Copy)]
pub struct RemediationTarget<'a> {
    pub operation_ref: &'a str,
    pub connection_ref: &'a str,
}

impl fmt::Debug for RemediationTarget<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RemediationTarget(<redacted>)")
    }
}

/// Registry ownership selection only. The predicate performs no credential/session work.
#[derive(Clone, Copy)]
pub enum RemediationRoute<'a> {
    Target(RemediationTarget<'a>),
    Session(&'a str),
}

impl fmt::Debug for RemediationRoute<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RemediationRoute(<redacted>)")
    }
}

/// Exact configured target and canonical catalog record for non-consuming grant evaluation.
///
/// Read from the actual owner after authenticating its context. This may describe a Created
/// binding internally; it must not publish a callable OperationDescription. The receiver uses
/// the actual catalog generation and operation/binding digest, never an invented public lease.
/// The record preserves the complete source, including its input schema and declared facts.
/// No constructor here invents a Connection initiation policy, route, generation or purpose.
pub struct RemediationMetadata<'a> {
    pub operation: catalog::reader::Operation<'a>,
    pub connection: domain::ConnectionAuthority,
    pub integration_ref: String,
    pub auth_profile: String,
    pub catalog_generation: String,
}

impl fmt::Debug for RemediationMetadata<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RemediationMetadata(<redacted>)")
    }
}

/// The existing ESS private value, stored beside the existing ephemeral ConnectSession.
///
/// It deliberately has neither serde support nor an authority-producing constructor. The
/// trusted receiver supplies these facts after exact current operation and management
/// admission. A field value or digest comparison alone cannot authorize any method.
///
/// Input is already validated and discarded: use the existing canonical_input_digest owner,
/// with its lowercase hexadecimal SHA256 result. Stable-authority and policy digest framing
/// require their explicitly reviewed owners; this module does not synthesize either. Keep a
/// real grant/policy reference and only an actual revision. In particular, never substitute
/// Agent revision for an absent personal-policy revision. Expiry is receiver-derived and
/// capped by both the session owner and applicable authority. Bounds are checked at admission.
#[derive(Clone, PartialEq, Eq)]
pub struct RemediationBinding {
    pub operation_ref: String,
    pub connection_ref: String,
    pub integration_ref: String,
    pub auth_profile: String,
    pub need: AuthenticationNeed,
    pub canonical_input_sha256: String,
    pub stable_authority_sha256: String,
    pub grant_ref: String,
    pub grant_revision: Option<u64>,
    pub admission_policy_sha256: String,
    pub expires_at_unix_ms: u64,
}

impl fmt::Debug for RemediationBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RemediationBinding(<redacted>)")
    }
}

/// Current receiver-policy capability; implemented by the admitted composition.
///
/// Revalidate current authenticated identity, stable owner/realm, exact metadata/binding,
/// real operation grant, management policy and deadline. A prior digest match is insufficient.
/// The caller invokes this before acquisition, status/ack and the publication decision where
/// the acquisition owner can observe it. Unknown/unadmitted facts yield the same Refused;
/// inability to establish current authority yields Unavailable.
///
/// This synchronous seam must not perform network or blocking store I/O while custody/session
/// locks are held. Personal OAuth's authoritative immutable process configuration can supply
/// its current policy, together with the existing mutable CurrentAuthority completion checks;
/// changing that configuration requires restart, which retires the ephemeral sessions. This
/// needs no new live-policy subsystem. A stale copy of a separately mutable hosted grant store
/// is different: it cannot supply current hosted authority. Hosted acquisition stays Unsupported
/// until its own composition implements that contract. No permissive implementation is supplied.
/// The retained Arc owns policy access, not the request bearer or raw invocation input.
/// The clock argument is supplied by the receiver's existing clock, never by a wire caller.
pub trait RemediationAuthority: Send + Sync + 'static {
    fn recheck(
        &self,
        context: &PrincipalContext,
        binding: &RemediationBinding,
        now_unix_ms: u64,
    ) -> Result<(), RemediationError>;

    /// Recheck current policy for observing an existing session, never for acquisition or
    /// publication. The default retains every strict check at the actual current time.
    /// An authoritative owner may separate expired session capability from still-current
    /// identity/grant/management admission so status can report Expired. It must preserve
    /// every current admission check; callers must propagate every rejection unchanged.
    fn recheck_status(
        &self,
        context: &PrincipalContext,
        binding: &RemediationBinding,
        now_unix_ms: u64,
    ) -> Result<(), RemediationError> {
        self.recheck(context, binding, now_unix_ms)
    }
}

/// Current personal-policy capability supplied by the actual configured owner.
/// The receiver still validates input and derives the exact private binding. This is neither
/// a dispatch proof nor hosted Grant authority; no input, bearer or credential is retained.
pub struct RemediationAdmission {
    pub grant_ref: String,
    pub grant_revision: Option<u64>,
    pub admission_policy_sha256: String,
    pub expires_at_unix_ms: u64,
    pub authority: std::sync::Arc<dyn RemediationAuthority>,
}

impl fmt::Debug for RemediationAdmission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RemediationAdmission(<redacted>)")
    }
}

/// Bound-only requests. Start carries admitted facts, never a raw input or v1 target selector.
pub enum RemediationRequest {
    Start(Box<RemediationBinding>),
    Status(RemediationStatusRequest),
    Acknowledge(RemediationAcknowledgeRequest),
}

impl fmt::Debug for RemediationRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RemediationRequest(<redacted>)")
    }
}

/// Trusted control-plane responses only. Never pass a contained value to a generic reducer.
///
/// DTO validity and field-name allowlists do not establish safe output: reference/message
/// strings can contain arbitrary private material even in a valid daemon envelope. Later
/// transport/client/console owners require closed or exact request-bound output projection.
pub enum RemediationResult {
    Status(Box<BoundRemediationStatus>),
    Acknowledged(RemediationAcknowledgement),
}

impl fmt::Debug for RemediationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RemediationResult(<redacted>)")
    }
}

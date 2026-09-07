//! Credential-safe values shared by the local and hosted Connector clients.

use std::io;
use std::path::PathBuf;

use protocol::connection;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

/// A transport, framing, or protocol validation failure.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("hosted token acquisition requires an exact provider auth profile")]
    HostedConnectProfile,
    #[error("credential input must be a nonempty UTF-8 regular file, owned by this user, with no group/other permissions and at most 8192 bytes")]
    UnsafeCredentialFile,
    #[error("the Connect Session does not provide a current token completion route at the selected Connector API base")]
    UnsafeHostedCompletion,
    #[error("credential submission could not be confirmed; inspect hosted Connections before starting another session, because the credential may already be stored")]
    CompletionUnconfirmed,
    #[error("authentication remediation was refused, expired, or returned invalid evidence")]
    RemediationRefused,
    #[error("personal OAuth instructions are unavailable or invalid")]
    PersonalOAuthInstructions,
    #[error("personal OAuth authorization expired or was refused")]
    PersonalOAuthRefused,
    #[error("Connector request was invalid: {0}")]
    InvalidRequest(String),
    #[error("Connector returned an invalid response")]
    InvalidResponse,
    #[error("hosted Connector base must be one explicit HTTPS or internal-cluster URL")]
    InvalidHostedBase,
    #[error("hosted Connector Identity bearer is invalid")]
    InvalidIdentityBearer,
    #[error("hosted Connector Identity authentication expired or was refused")]
    HostedAuthentication,
    #[error("hosted Connector Identity authority was refused")]
    HostedNotGranted,
    #[error("hosted Connector refused the Git fetch request with status {0}")]
    GitFetchRefused(u16),
    #[error("hosted Connector refused the subscription request with status {0}")]
    SubscriptionRefused(u16),
    #[error("hosted Connector refused the administrative request with status {0}")]
    AdminRefused(u16),
    #[error("Identity refused administrative login with status {0}")]
    AdminAuthenticationRefused(u16),
    #[error("hosted Connector is unavailable")]
    HostedUnavailable,
    #[error("hosted Connector returned a cacheable credential response")]
    CacheableCredentialResponse,
    #[error(
        "the Connect Session completion endpoint is not an owner-only socket under this state root"
    )]
    UnsafeCompletionEndpoint,
    #[error("the Connector refused the submitted credential")]
    CompletionRefused,
    #[error("the Connector refused the connection request: {0}")]
    ConnectionRefused(String),
    #[error("local Connector transport failed: {0}")]
    Io(#[from] io::Error),
    #[error("Connector protocol JSON was malformed: {0}")]
    Json(#[from] serde_json::Error),
}

/// Public facts needed to obtain one short-lived administrative access token from Identity.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AdminAuthMetadata {
    pub identity_origin: String,
    pub audience: String,
    pub scope: String,
}

/// Identity's public Authorization Code + PKCE discovery document.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AdminLoginMetadata {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub access_token_endpoint: String,
    pub cli_client_id: String,
    pub response_types_supported: Vec<String>,
    pub grant_types_supported: Vec<String>,
    pub code_challenge_methods_supported: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AdminCredentialState {
    Present,
    Missing,
    Unavailable,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AdminConfigurationField {
    pub name: String,
    pub state: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AdminCredentialStatus {
    pub name: String,
    pub required: bool,
    pub state: AdminCredentialState,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AdminIntegrationStatus {
    pub integration_ref: String,
    pub active: bool,
    pub configuration: Vec<AdminConfigurationField>,
    pub credentials: Vec<AdminCredentialStatus>,
    pub ready: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AdminStatus {
    pub integrations: Vec<AdminIntegrationStatus>,
    pub ready: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AdminCredentialWrite {
    pub request_id: String,
    pub integration_ref: String,
    pub credential: String,
    pub state: AdminCredentialState,
    pub replaced: bool,
}

/// Value-free result of beginning a Connector-owned credential acquisition session.
pub struct PendingConnection {
    pub session_ref: String,
    pub completion_endpoint: PathBuf,
}

/// Result of a generic candidate-selection and activation workflow.
pub enum CandidateActivationOutcome {
    SelectionRequired(Vec<connection::ConnectionCandidateSummary>),
    Connected {
        connection: connection::ConnectionDescription,
        observations: Vec<connection::DiscoveryObservationSummary>,
    },
}

/// Value-free result of materializing the recognized observations admitted by the Connector.
pub struct MaterializationOutcome {
    pub connections: Vec<connection::ConnectionSummary>,
    pub unsupported: usize,
    pub not_granted: usize,
}

/// One exact, bounded Git source grant. The source authority is wiped on drop and redacted.
pub struct GitFetchSession {
    pub session_ref: String,
    pub source: String,
    pub locator: String,
    pub reference: String,
    pub expected_commit: String,
    pub depth: u8,
    pub expires_at_unix_ms: u64,
    source_authorization: Zeroizing<String>,
}

impl GitFetchSession {
    pub(crate) fn from_wire(created: protocol::git_fetch::CreatedSession) -> Self {
        Self {
            session_ref: created.session_ref,
            source: created.source,
            locator: created.locator,
            reference: created.reference,
            expected_commit: created.expected_commit,
            depth: created.depth,
            expires_at_unix_ms: created.expires_at_unix_ms,
            source_authorization: Zeroizing::new(created.source_authorization),
        }
    }

    #[must_use]
    pub fn expose_at_source_boundary(&self) -> &str {
        self.source_authorization.as_str()
    }
}

impl std::fmt::Debug for GitFetchSession {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GitFetchSession")
            .field("session_ref", &self.session_ref)
            .field("source", &self.source)
            .field("locator", &self.locator)
            .field("reference", &self.reference)
            .field("expected_commit", &self.expected_commit)
            .field("depth", &self.depth)
            .field("expires_at_unix_ms", &self.expires_at_unix_ms)
            .field("source_authorization", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SubscriptionStatus {
    pub provider: String,
    pub connected: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SubscriptionOAuthStart {
    pub authorization_url: String,
    pub flow_id: String,
    pub expires_at: u64,
}

pub struct SubscriptionLease {
    pub lease_id: String,
    pub(crate) token: Zeroizing<String>,
    pub expires_at: u64,
}

impl SubscriptionLease {
    #[must_use]
    pub fn expose_at_redemption_boundary(&self) -> &str {
        self.token.as_str()
    }
}

impl std::fmt::Debug for SubscriptionLease {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SubscriptionLease")
            .field("lease_id", &self.lease_id)
            .field("token", &"[REDACTED]")
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

pub struct RedeemedSubscription {
    pub(crate) credential: Zeroizing<String>,
    pub kind: String,
}

impl RedeemedSubscription {
    #[must_use]
    pub fn expose_at_provider_boundary(&self) -> &str {
        self.credential.as_str()
    }
}

impl std::fmt::Debug for RedeemedSubscription {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RedeemedSubscription")
            .field("credential", &"[REDACTED]")
            .field("kind", &self.kind)
            .finish()
    }
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConnectSubscriptionRequest<'a> {
    pub credential: &'a str,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AdminCredentialRequest<'a> {
    pub request_id: &'a str,
    pub value: &'a str,
    pub replace: bool,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CompleteSubscriptionOAuthRequest<'a> {
    pub flow_id: &'a str,
    pub code: &'a str,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateSubscriptionLeaseRequest<'a> {
    pub attempt_id: &'a str,
    pub ttl_seconds: u64,
    pub maximum_uses: u16,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SubscriptionLeaseResponse {
    pub lease_id: String,
    pub lease_token: String,
    pub expires_at: u64,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RedeemSubscriptionLeaseRequest<'a> {
    pub attempt_id: &'a str,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RedeemedSubscriptionResponse {
    pub credential: String,
    pub kind: String,
}

/// Trusted local OAuth handoff. Deliberately neither Debug nor Serialize; it carries a capability.
/// Stopping the client does not cancel Connection v1 acquisition; daemon expiry/shutdown cleans it.
///
/// ```compile_fail
/// fn debug<T: std::fmt::Debug>() {}
/// debug::<connectors_client::PendingPersonalOAuth>();
/// ```
pub struct PendingPersonalOAuth {
    pub(crate) expected_connection_ref: String,
    pub(crate) integration_ref: String,
    pub(crate) auth_profile: String,
    pub(crate) session_ref: String,
    pub(crate) expires_at_unix_ms: u64,
    pub(crate) browser_url: Zeroizing<String>,
    pub(crate) deadline: tokio::time::Instant,
}
impl PendingPersonalOAuth {
    #[must_use]
    pub fn session_ref(&self) -> &str {
        &self.session_ref
    }
    #[must_use]
    pub fn expires_at_unix_ms(&self) -> u64 {
        self.expires_at_unix_ms
    }
}

/// Human-only OAuth instructions, held in zeroizing buffers and never serialized as command data.
///
/// ```compile_fail
/// fn serializable<T: serde::Serialize>() {}
/// serializable::<connectors_client::PersonalOAuthInstructions>();
/// ```
pub struct PersonalOAuthInstructions {
    pub(crate) url: Zeroizing<String>,
    pub(crate) user_code: Option<Zeroizing<String>>,
}
impl PersonalOAuthInstructions {
    /// The trusted console must select a controlling terminal or an owner-only file before
    /// starting a session. This writes only to that explicitly selected private destination.
    pub fn write_human(&self, destination: &mut impl io::Write) -> io::Result<()> {
        writeln!(
            destination,
            "Open this URL to authorize the configured connection:\n{}",
            *self.url
        )?;
        if let Some(code) = &self.user_code {
            writeln!(destination, "Enter this code: {}", **code)?;
        }
        destination.flush()
    }
}

impl PendingPersonalOAuth {
    /// Retire a private human handoff at the admitted session deadline even when a previously
    /// authorized commit is still settling. This timer sends no cancellation request.
    pub async fn instruction_expiry(&self) {
        tokio::time::sleep_until(self.deadline).await;
    }
}

/// Private local handoff for one bounded remediation. This is a response check, never authority.
/// It retains only this caller's bounded input for fresh-schema validation and is consumed on finish.
///
/// ```compile_fail
/// fn debug<T: std::fmt::Debug>() {}
/// debug::<connectors_client::PendingRemediation>();
/// ```
pub struct PendingRemediation {
    pub(crate) receiver: PathBuf,
    pub(crate) owner: protocol::operation::OwnerContext,
    pub(crate) operation_ref: String,
    pub(crate) connection_ref: String,
    pub(crate) integration_ref: String,
    pub(crate) auth_profile: String,
    pub(crate) input: serde_json::Value,
    pub(crate) session_ref: String,
    pub(crate) need: protocol::operation::v3::AuthenticationNeed,
    pub(crate) expires_at_unix_ms: u64,
    pub(crate) deadline: tokio::time::Instant,
    pub(crate) browser_url: Option<Zeroizing<String>>,
    pub(crate) ready: bool,
}
impl PendingRemediation {
    /// Deadline for the private human instruction destination; it is not an authority extension.
    #[must_use]
    pub fn instruction_deadline(&self) -> tokio::time::Instant {
        self.deadline
    }
}

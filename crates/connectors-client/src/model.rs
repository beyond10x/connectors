//! Credential-safe values shared by the local and hosted Connector clients.

use std::io;
use std::path::PathBuf;

use protocol::connection;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

/// A transport, framing, or protocol validation failure.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("Connector request was invalid: {0}")]
    InvalidRequest(String),
    #[error("Connector returned an invalid response")]
    InvalidResponse,
    #[error("hosted Connector base must be one explicit HTTPS or internal-cluster URL")]
    InvalidHostedBase,
    #[error("hosted Connector Identity bearer is invalid")]
    InvalidIdentityBearer,
    #[error("hosted Connector Identity authority was refused")]
    HostedNotGranted,
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

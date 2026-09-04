//! Application port for bounded, read-only Git byte sessions.

use async_trait::async_trait;
use zeroize::Zeroizing;

use crate::{EgressByteStream, PrincipalContext};

/// The only two Git Smart HTTP exchanges admitted by a fetch session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitFetchService {
    Discovery,
    UploadPack,
}

/// Source-capability authentication performed before an internal request body is read.
pub struct GitFetchAccess {
    pub session_ref: String,
    pub repository: String,
    pub source_authorization: Zeroizing<String>,
    pub service: GitFetchService,
    pub git_protocol: Option<String>,
}

/// One internal byte-plane request after route-shape validation.
pub struct GitFetchExchange {
    pub session_ref: String,
    pub repository: String,
    pub source_authorization: Zeroizing<String>,
    pub service: GitFetchService,
    pub git_protocol: Option<String>,
    pub body: Option<Vec<u8>>,
}

/// One successfully established source capability.
pub struct GitFetchGrant {
    pub session_ref: String,
    pub source: String,
    pub locator: String,
    pub reference: String,
    pub expected_commit: String,
    pub depth: u8,
    pub expires_at_unix_ms: u64,
    source_authorization: Zeroizing<String>,
}

impl GitFetchGrant {
    /// Construct a grant after the Integration has admitted all coordinates.
    pub fn admitted(
        session_ref: String,
        source: String,
        locator: String,
        request: &protocol::git_fetch::CreateRequest,
        expires_at_unix_ms: u64,
        source_authorization: Zeroizing<String>,
    ) -> Self {
        Self {
            session_ref,
            source,
            locator,
            reference: request.reference.clone(),
            expected_commit: request.expected_commit.clone(),
            depth: request.depth,
            expires_at_unix_ms,
            source_authorization,
        }
    }

    /// Expose the one-use capability only where the control response is serialized.
    #[must_use]
    pub fn expose_at_control_boundary(&self) -> &str {
        self.source_authorization.as_str()
    }
}

impl std::fmt::Debug for GitFetchGrant {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GitFetchGrant")
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

/// Selected response metadata and a bounded body stream.
pub struct GitFetchExchangeResponse {
    pub status: u16,
    pub content_type: String,
    pub body: Box<dyn EgressByteStream>,
}

/// Redaction-safe control-plane refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum GitFetchControlError {
    #[error("Git fetch session input was invalid")]
    Invalid,
    #[error("current Connector authority does not admit this repository")]
    NotGranted,
    #[error("Git fetch retry identity names different repository coordinates")]
    Conflict,
    #[error("Git fetch session establishment is unavailable")]
    Unavailable,
}

/// Redaction-safe data-plane refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum GitFetchDataError {
    #[error("Git fetch session was refused")]
    Refused,
    #[error("Git fetch provider exchange is unavailable")]
    Unavailable,
}

/// Connector-owned capability seam between authenticated control and internal Git bytes.
#[async_trait]
pub trait GitFetchBroker: Send + Sync + 'static {
    async fn create(
        &self,
        context: &PrincipalContext,
        request: protocol::git_fetch::CreateRequest,
    ) -> Result<GitFetchGrant, GitFetchControlError>;

    /// Authenticate a source capability before the hosted HTTP boundary reads an upload body.
    /// Exchange repeats the check while atomically consuming the session request budget.
    fn authorize(&self, request: &GitFetchAccess) -> Result<(), GitFetchDataError>;

    async fn exchange(
        &self,
        request: GitFetchExchange,
    ) -> Result<GitFetchExchangeResponse, GitFetchDataError>;
}

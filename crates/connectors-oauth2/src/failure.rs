use std::{fmt, time::Duration};

/// Every way a grant, exchange or receiver ends without a token: one closed set of codes.
///
/// A failure carries its code and, for [`Failure::RateLimited`], the delay the server asked
/// for. It never carries text a server or a redirect wrote, so `Display` and `Debug` are safe
/// to log.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    /// The grant is no longer usable (`invalid_grant`, `invalid_client`, HTTP 401): the person
    /// must authorize again.
    ReauthorizationRequired,
    /// HTTP 429, with the `Retry-After` delay when the server sent one.
    RateLimited { retry_after: Option<Duration> },
    /// HTTP 5xx, a transport failure, or a server that answered in a way no step can use.
    Unavailable,
    /// The person or the server refused the authorization (`access_denied`).
    AccessDenied,
    /// A redirect or pasted line whose `state` is absent or is not the flow's.
    StateMismatch,
    /// The granted scopes lack one of the declaration's `required_scopes`.
    InsufficientScope,
    /// The device code expired (`expired_token`, or its `expires_in` passed).
    ExpiredToken,
    /// The caller cancelled.
    Cancelled,
    /// The redirect did not arrive before the caller's deadline.
    Timeout,
    /// The server refused the request for another reason (`invalid_request`, `invalid_scope`,
    /// `unsupported_grant_type`, another 4xx, or an error redirect other than
    /// `access_denied`).
    Rejected,
    /// A response that is oversized, not JSON, or lacks a usable `access_token`, bearer
    /// `token_type` or positive `expires_in`; or a redirect without a code.
    InvalidResponse,
    /// The declaration or the client's credentials cannot be used as given.
    InvalidDeclaration,
    /// A refresh under a `non_rotating` declaration answered with a refresh token other than
    /// the one sent. Nothing of that response is returned.
    UnexpectedRotation,
}

impl Failure {
    /// The failure's code, in snake case.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::ReauthorizationRequired => "reauthorization_required",
            Self::RateLimited { .. } => "rate_limited",
            Self::Unavailable => "unavailable",
            Self::AccessDenied => "access_denied",
            Self::StateMismatch => "state_mismatch",
            Self::InsufficientScope => "insufficient_scope",
            Self::ExpiredToken => "expired_token",
            Self::Cancelled => "cancelled",
            Self::Timeout => "timeout",
            Self::Rejected => "rejected",
            Self::InvalidResponse => "invalid_response",
            Self::InvalidDeclaration => "invalid_declaration",
            Self::UnexpectedRotation => "unexpected_rotation",
        }
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for Failure {}

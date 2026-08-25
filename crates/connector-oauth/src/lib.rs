#![forbid(unsafe_code)]

//! **The authorization-code OAuth mechanics an Integration needs, with no transport.**
//!
//! # Why this exists
//!
//! Three Integrations shipped a browser OAuth flow and each wrote its own, because there was
//! nowhere to put a shared one. Measured 2026-08-25, before this crate:
//!
//! ```text
//! gitlab   authorization_code + refresh   PKCE S256      bare reqwest, form-encoded
//! jira     authorization_code + refresh   no PKCE        bare reqwest, JSON body
//! slack    authorization_code + refresh   no PKCE        egress gate, HTTP Basic
//! ```
//!
//! Three copies of state generation, three of the expiry sweep, three of "is this token response
//! actually usable". They do not agree — only GitLab derives a PKCE challenge, and only Jira
//! refreshes under a double-checked lock with a skew. A fourth connector would have inherited
//! whichever neighbour its author read first.
//!
//! # What it deliberately does not own
//!
//! **The request.** The three dial three different ways, and Slack's must stay on the egress gate
//! rather than a client of its own. A crate that owned the HTTP would have to abstract all three
//! transports and would put a client in the dependency graph of everything that links it.
//!
//! **The clock.** [`PendingStates::expire`], [`ValidatedToken::expires_at_unix_ms`] and
//! [`refresh_due`] take the instant as an argument. That is what lets expiry be tested without
//! sleeping, and it keeps this crate classifiable as a host library that reaches nothing.
//!
//! So the division is: this crate produces what goes *into* a request and validates what comes
//! *out* of one; the caller performs it.
//!
//! # The credential never gains a way to be printed
//!
//! [`ValidatedToken`] holds [`Zeroizing<String>`], not [`String`], and this crate derives no
//! [`Debug`] on any type that carries one. Callers wrap it in their own secret type on the way to
//! the store.

mod pkce;
mod state;
mod token;

pub use pkce::{authorize_url, random_token, AuthorizeParams, Pkce};
pub use state::{Pending, PendingStates, DEFAULT_PENDING_CAPACITY};
pub use token::{
    parse_scopes, refresh_due, validate, ScopePolicy, ScopeSeparator, TokenPolicy, TokenResponse,
    ValidatedToken,
};

/// Why an OAuth step refused.
///
/// Each variant carries a stable [`code`](OauthError::code) rather than a message, because the
/// three callers surface failures as their own error type over a short code string
/// (`GitlabError::new("oauth-exchange")`) and a prose message would have to be translated at every
/// call site. No variant carries a value: an `OauthError` is safe to log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OauthError {
    /// The system random source refused.
    #[error("randomness")]
    Randomness,
    /// The authorize endpoint could not be built from the configured origin.
    #[error("authorize-url")]
    AuthorizeUrl,
    /// The token response parsed but does not meet the caller's [`TokenPolicy`].
    #[error("oauth-exchange")]
    TokenResponse,
    /// The pending-state table is full of live entries.
    #[error("oauth-state-capacity")]
    StateCapacity,
    /// The clock is before the Unix epoch, or an instant arithmetic overflowed.
    #[error("clock")]
    Clock,
}

impl OauthError {
    /// The stable short code, for a caller mapping into its own error type.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Randomness => "randomness",
            Self::AuthorizeUrl => "authorize-url",
            Self::TokenResponse => "oauth-exchange",
            Self::StateCapacity => "oauth-state-capacity",
            Self::Clock => "clock",
        }
    }
}

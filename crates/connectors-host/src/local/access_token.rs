//! A current access token of one `oauth2_grant` connection, for a Rust caller
//! (story:connections-token-issues-an-access-token). [`access_token`] asks
//! the running owner, which issues the published generation's token, or
//! refreshes it first when it expires within the declaration's
//! `refresh_lead_seconds` (`contracts/auth/acquisition/v1alpha1/semantics.md`
//! "Access token issue"). `connectors connections token` makes the same
//! request through [`issue`]. Neither can ask for a refresh, and each issue is
//! recorded as one `connectors.oauth2.AccessTokenIssue` naming its delivery.
//!
//! [`AccessToken`]'s `Debug` and `Display` never show the token; only
//! [`AccessToken::token`] does.
use super::{config::Paths, owner};
use serde::{Deserialize, Serialize};
use std::fmt;

/// How the token leaves the host (`connectors.oauth2.Delivery`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Delivery {
    /// `connections token` without `--reveal`: the owner sends no token.
    MetadataOnly,
    /// `connections token --reveal`: the token is printed.
    Revealed,
    /// [`access_token`]: the token is handed to the calling process.
    Library,
}

impl Delivery {
    /// The name the record carries.
    pub fn name(self) -> &'static str {
        match self {
            Self::MetadataOnly => "metadata_only",
            Self::Revealed => "revealed",
            Self::Library => "library",
        }
    }
    /// Whether the owner sends the token to the caller.
    pub fn carries_token(self) -> bool {
        self != Self::MetadataOnly
    }
}

/// One issued access token and what the issue recorded.
pub struct AccessToken {
    pub(crate) connection: String,
    pub(crate) token: Option<connectors_oauth2::Secret>,
    pub(crate) expires_at_ms: u64,
    pub(crate) scopes: Vec<String>,
    pub(crate) refreshed: bool,
    pub(crate) issue: String,
}

impl AccessToken {
    /// The connection the token belongs to.
    pub fn connection(&self) -> &str {
        &self.connection
    }
    /// The access token; absent for a metadata-only issue.
    pub fn token(&self) -> Option<&str> {
        self.token.as_ref().map(connectors_oauth2::Secret::expose)
    }
    /// Until when the token is valid, in milliseconds since the Unix epoch.
    pub fn expires_at_ms(&self) -> u64 {
        self.expires_at_ms
    }
    /// The scopes the token was granted.
    pub fn scopes(&self) -> &[String] {
        &self.scopes
    }
    /// Whether a refresh published this token for this issue.
    pub fn refreshed(&self) -> bool {
        self.refreshed
    }
    /// The recorded `AccessTokenIssue`.
    pub fn issue(&self) -> &str {
        &self.issue
    }
    /// The access token alone, still redacting, for a caller that keeps only it.
    pub fn into_token(self) -> Option<connectors_oauth2::Secret> {
        self.token
    }
}

impl fmt::Debug for AccessToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccessToken")
            .field("connection", &self.connection)
            .field("token", &self.token.as_ref().map(|_| "<redacted>"))
            .field("expires_at_ms", &self.expires_at_ms)
            .field("scopes", &self.scopes)
            .field("refreshed", &self.refreshed)
            .field("issue", &self.issue)
            .finish()
    }
}

impl fmt::Display for AccessToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "access token of connection {} (redacted), valid until {} ms",
            self.connection, self.expires_at_ms
        )
    }
}

/// How long an issue may take, a refresh included.
const DEADLINE_MS: u64 = 30_000;

/// The current access token of `connection`, a connection of the configured
/// instance `adapter` (its alias) under the configuration and state at
/// `paths`. The owner the `connectors` CLI starts answers it; with none
/// running this is `unavailable`, since this process cannot start one.
pub fn access_token(paths: &Paths, adapter: &str, connection: &str) -> owner::Result<AccessToken> {
    issue(
        owner::Client::connect(paths, false)?,
        adapter,
        connection,
        Delivery::Library,
    )
}

/// Asks the owner `client` greeted for an access token of `connection` for
/// `delivery`; what [`access_token`] and `connections token` both call.
pub fn issue(
    client: owner::Client,
    adapter: &str,
    connection: &str,
    delivery: Delivery,
) -> owner::Result<AccessToken> {
    client.issue_access_token(
        adapter,
        connection,
        delivery,
        connectors_sdk::now_ms() + DEADLINE_MS,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_and_display_show_no_token() {
        let issued = AccessToken {
            connection: "fixture-connection".into(),
            token: Some(connectors_oauth2::Secret::new(
                "fixture-access-token-shown".into(),
            )),
            expires_at_ms: 1,
            scopes: vec!["read".into()],
            refreshed: false,
            issue: "fixture-issue".into(),
        };
        assert_eq!(issued.token(), Some("fixture-access-token-shown"));
        for text in [
            format!("{issued:?}"),
            format!("{issued:#?}"),
            issued.to_string(),
        ] {
            assert!(!text.contains("fixture-access-token-shown"), "{text}");
            assert!(text.contains("fixture-connection"), "{text}");
        }
    }
}

//! The owner's issue of a current access token of an `oauth2_grant`
//! connection (story:connections-token-issues-an-access-token,
//! `contracts/auth/acquisition/v1alpha1/semantics.md` "Access token issue").
//!
//! The published generation's access token is issued as it is (`current`)
//! while it stays valid for longer than the declaration's
//! `refresh_lead_seconds`; otherwise [`Refresher::refresh_generation`] runs
//! first, under the same `connectors.refresh` ledger serialization every
//! refresh takes, and its published generation is issued (`refreshed`). Its
//! reservation names the generation the issue read, in the transaction that
//! reserves: a caller whose generation another refresh already replaced
//! sends nothing and issues the successor. The caller cannot ask for a
//! refresh. Each issue is one recorded
//! `connectors.oauth2.AccessTokenIssue`; a refusal records nothing.
use super::*;
use crate::local::{
    access_token::Delivery,
    registry::{Issuance, Issue},
};

/// One issued access token. Its `Debug` shows no token.
pub struct Issued {
    pub connection: String,
    pub token: GrantSecret,
    pub expires_at_ms: u64,
    pub scopes: Vec<String>,
    pub refreshed: bool,
    /// The recorded `AccessTokenIssue`.
    pub issue: String,
}

impl std::fmt::Debug for Issued {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Issued")
            .field("connection", &self.connection)
            .field("token", &self.token)
            .field("expires_at_ms", &self.expires_at_ms)
            .field("scopes", &self.scopes)
            .field("refreshed", &self.refreshed)
            .field("issue", &self.issue)
            .finish()
    }
}

/// Why nothing was issued; nothing was recorded either.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    Registry(Failure),
    /// The connection's profile is not `oauth2_grant`.
    NotGrant,
    /// The refresh the issue needed did not publish a generation. A Stale
    /// connection is `ReauthorizationRequired` without a request.
    Refresh(Outcome),
    /// The published access token has expired and no refresh replaced it.
    Expired,
    /// The access token must be refreshed and this owner cannot: a
    /// confidential client's secret is not part of the stored entry.
    NotRefreshable,
    /// The caller's deadline passed before the issue was recorded.
    Deadline,
}

/// The stored entry's fields an issue reads (`oauth2_grant::entry`).
#[derive(Deserialize)]
struct Token {
    access_token: GrantSecret,
    expires_at_ms: u64,
    scopes: Vec<String>,
}

/// How often a generation replaced between its read and its record is read again.
const ROUNDS: usize = 3;

impl Refresher {
    /// Issues a current access token of `connection`, a connection of
    /// `instance` under `adapter`, for `delivery`, unless
    /// `deadline_ms` (Unix milliseconds) passes before it is recorded.
    pub fn issue_access_token(
        &self,
        instance: &str,
        adapter: &str,
        connection: &str,
        delivery: Delivery,
        deadline_ms: u64,
    ) -> Result<Issued, Refusal> {
        let mut refreshed = false;
        for _ in 0..ROUNDS {
            let published = match self
                .registry
                .issuance(instance, adapter, connection, now())
                .map_err(Refusal::Registry)?
            {
                Issuance::NotGrant => return Err(Refusal::NotGrant),
                Issuance::Stale => {
                    return Err(Refusal::Refresh(Outcome::ReauthorizationRequired));
                }
                Issuance::Published(published) => published,
            };
            // One refresh per call at most: a provider whose fresh token
            // already expires within the lead is issued what it published.
            if !refreshed && !published.current(now()) {
                if !published.refreshable() {
                    return Err(Refusal::NotRefreshable);
                }
                // The reservation names the generation decided on: once
                // another caller's refresh published its successor, this
                // one sends nothing and reads the successor.
                match self
                    .refresh_generation(connection, Some(published.generation()))
                    .map_err(Refusal::Registry)?
                {
                    Outcome::Refreshed => {
                        refreshed = true;
                        continue;
                    }
                    outcome => return Err(Refusal::Refresh(outcome)),
                }
            }
            let token = self
                .custody
                .read(published.version())
                .ok()
                .and_then(|material| serde_json::from_slice::<Token>(&material.0).ok())
                .ok_or(Refusal::Registry(Failure::CustodyUnavailable))?;
            if token.expires_at_ms <= now() {
                return Err(Refusal::Expired);
            }
            // A refresh this issue waited for may outlast its caller: an issue
            // never delivered is not recorded.
            if now() >= deadline_ms {
                return Err(Refusal::Deadline);
            }
            let issue = Issue {
                connection,
                generation: published.generation(),
                refreshed,
                expires_at_ms: token.expires_at_ms,
                scopes: &token.scopes,
                delivery: delivery.name(),
            };
            match self.registry.record_issue(&issue, now()) {
                Ok(recorded) => {
                    return Ok(Issued {
                        connection: connection.to_owned(),
                        token: token.access_token,
                        expires_at_ms: token.expires_at_ms,
                        scopes: token.scopes,
                        refreshed: recorded.refresh_attempt.is_some(),
                        issue: recorded.issue,
                    });
                }
                // A refresh or a repair replaced the generation: read again.
                Err(Failure::Conflict) => {}
                Err(failure) => return Err(Refusal::Registry(failure)),
            }
        }
        Err(Refusal::Refresh(Outcome::Uncertain))
    }
}

/// The owner's failure for a refusal, as `connectors.cli.FailureCode` names
/// it: `forbidden` for a profile that is not `oauth2_grant`, or for a token
/// inside the lead that this owner cannot refresh (a confidential client);
/// `not_granted` when a refresh no longer covers the profile's scopes;
/// `timeout` when the caller's deadline passed before the issue was recorded;
/// `service_failure` (next action `repair_connection`) when a refresh needs
/// reauthorization or its outcome is uncertain; `unavailable` when the token
/// endpoint rate limited it, with its `Retry-After` in whole seconds, or when
/// no current token is at hand.
impl From<Refusal> for crate::local::owner::Error {
    fn from(refusal: Refusal) -> Self {
        use crate::local::owner::{Code, Error};
        match refusal {
            Refusal::Registry(failure) => failure.into(),
            Refusal::NotGrant | Refusal::NotRefreshable => Code::Forbidden.into(),
            Refusal::Deadline => Code::Timeout.into(),
            Refusal::Expired => Code::Unavailable.into(),
            Refusal::Refresh(outcome) => match outcome {
                Outcome::ReauthorizationRequired | Outcome::Uncertain => {
                    Code::ServiceFailure.into()
                }
                Outcome::InsufficientScope => Code::NotGranted.into(),
                Outcome::InvalidOrRevoked => Code::Revoked.into(),
                Outcome::CustodyUnavailable => Code::CustodyUnavailable.into(),
                Outcome::RateLimited { retry_after_ms } => {
                    let mut error = Error::from(Code::Unavailable);
                    error.service_code = Some(connectors_core::ErrorCode::RateLimited);
                    error.retry_after_seconds = retry_after_ms.map(|ms| ms.div_ceil(1000));
                    error
                }
                Outcome::Refreshed | Outcome::Unavailable => Code::Unavailable.into(),
            },
        }
    }
}

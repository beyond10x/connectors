//! The registry side of `connectors.oauth2.IssueAccessToken`
//! (story:connections-token-issues-an-access-token,
//! `contracts/auth/acquisition/v1alpha1/semantics.md` "Access token issue").
//! [`Registry::issuance`] reads what an issue would hand out; the owner
//! refreshes through the `connectors.refresh` ledger when the access token
//! expires within the declaration's `refresh_lead_seconds`, and
//! [`Registry::record_issue`] records the issue in a ledger transaction that
//! checks the connection still publishes the generation it read. A refusal
//! records nothing.
use super::{
    Failure, Registry, Result, custody, lifecycle::custody_version, new_id, observation::visible,
};
use crate::local::metadata::{IssueRecord, LedgerStep};

/// What an issue found of a connection.
pub enum Issuance {
    /// The published generation, to issue or to refresh first.
    Published(Box<Published>),
    /// The connection is Stale: only a repair returns it.
    Stale,
    /// The connection's profile declares no authorization server: it is not
    /// `oauth2_grant`, and nothing is issued from it.
    NotGrant,
}

/// The connection's published generation, as an issue reads it.
pub struct Published {
    generation: String,
    version: custody::Version,
    expires_at_ms: Option<u64>,
    lead_ms: u64,
}

impl Published {
    pub fn generation(&self) -> &str {
        &self.generation
    }
    /// The custody version that holds the generation's entry.
    pub fn version(&self) -> custody::Version {
        self.version
    }
    /// Whether its access token stays valid for longer than the declaration's
    /// `refresh_lead_seconds` after `now`: the issue is then `current`.
    pub fn current(&self, now: u64) -> bool {
        self.expires_at_ms
            .is_some_and(|expires| expires.saturating_sub(now) > self.lead_ms)
    }
}

/// One recorded `AccessTokenIssue`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recorded {
    pub issue: String,
    /// The attempt that published the issued generation, on a refreshed issue.
    pub refresh_attempt: Option<String>,
}

/// What an issue records beside its decision.
pub struct Issue<'a> {
    pub connection: &'a str,
    pub generation: &'a str,
    /// `refreshed`: a refresh published the generation for this call.
    pub refreshed: bool,
    pub expires_at_ms: u64,
    pub scopes: &'a [String],
    /// `connectors.oauth2.Delivery`.
    pub delivery: &'a str,
}

impl Registry {
    /// The published generation of `connection`, a connection of `instance`
    /// under `adapter`, and whether it must be refreshed before an issue.
    pub(in crate::local) fn issuance(
        &self,
        instance: &str,
        adapter: &str,
        connection: &str,
        now: u64,
    ) -> Result<Issuance> {
        self.transaction_observation(now, |tx, authority, _| {
            let row = Self::connection(tx, connection)?;
            visible(&row, instance, adapter)?;
            if row.state == "revoked" {
                return Err(Failure::Revoked);
            }
            let Some(declaration) = row.binding.profile.oauth2.as_ref() else {
                return Ok(Issuance::NotGrant);
            };
            if row.kept.stale {
                return Ok(Issuance::Stale);
            }
            let (Some(generation), Some(material), Some(snapshot)) =
                (&row.generation, &row.material, &row.baseline)
            else {
                return Err(Failure::NotReady);
            };
            Ok(Issuance::Published(Box::new(Published {
                generation: generation.clone(),
                version: custody_version(authority, &row.scope_id, material)?,
                expires_at_ms: snapshot.credential_expires_at,
                lead_ms: u64::try_from(declaration.refresh_lead_seconds)
                    .unwrap_or(0)
                    .saturating_mul(1000),
            })))
        })
    }

    /// `connectors.oauth2.IssueAccessToken` of `issue.generation`, while the
    /// connection still publishes it: `Conflict` when a refresh or a repair
    /// replaced it since it was read, or when a refreshed issue's generation
    /// was not published by a refresh.
    pub(in crate::local) fn record_issue(&self, issue: &Issue<'_>, now: u64) -> Result<Recorded> {
        self.transaction_ledger(now, |tx, _, _, ledger| {
            let row = Self::connection(tx, issue.connection)?;
            if row.state == "revoked" {
                return Err(Failure::Revoked);
            }
            if row.kept.stale || row.generation.as_deref() != Some(issue.generation) {
                return Err(Failure::Conflict);
            }
            let refresh_attempt = if issue.refreshed {
                Some(
                    ledger
                        .published(issue.generation)
                        .ok_or(Failure::Conflict)?
                        .to_owned(),
                )
            } else {
                None
            };
            let recorded = Recorded {
                issue: new_id(),
                refresh_attempt,
            };
            ledger.stage(LedgerStep::Issue(Box::new(IssueRecord {
                issue: recorded.issue.clone(),
                connection: issue.connection.to_owned(),
                generation: issue.generation.to_owned(),
                refresh_attempt: recorded.refresh_attempt.clone(),
                expires_at_ms: issue.expires_at_ms,
                scopes: issue.scopes.to_vec(),
                delivery: issue.delivery.to_owned(),
            })));
            Ok(recorded)
        })
    }
}

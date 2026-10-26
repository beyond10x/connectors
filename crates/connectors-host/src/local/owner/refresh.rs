//! The owner's refresh of an `oauth2_grant` connection
//! (story:owner-rotating-refresh, `contracts/auth/acquisition/v1alpha1/semantics.md`
//! §4.1), `rotating` or `non_rotating`, through the `connectors.refresh`
//! ledger the registry records: ReserveRefresh, AuthorizeExchange (durable
//! before the request), one refresh request through `connectors-oauth2`,
//! StoreResponse (the complete response written as a new custody version of a
//! new generation), PublishRefresh (the active generation and version replaced
//! under the private publication fence, the old version retired).
//!
//! Callers that find another caller's attempt on the same source wait for its
//! outcome and send nothing. An exchange the token endpoint refused with HTTP
//! 429, or whose request never left (the connection, name resolution or TLS
//! handshake failed), is released and the connection left as it was. A
//! connection whose custody holds as many versions as it may retain is
//! refused before anything is reserved. At owner start, [`Refresher::recover`]
//! resolves the attempts a past owner left: an authorized exchange without a
//! stored response becomes `uncertain` and its connection Stale; a stored
//! response whose access token is still valid is published without a request,
//! an expired one discarded and its connection marked Stale.
//!
//! The entry point is [`Refresher::refresh_connection`]; the caller-facing
//! trigger is `connections token` (story:connections-token-issues-an-access-token).
use crate::local::{
    config::Selection,
    keyring::custody,
    oauth, oauth2_grant,
    registry::{
        self, Authorization, Failure, Lost, Publication, RefreshTicket, Registry, Reservation,
        ValidatedBaseline,
    },
};
use connectors_oauth2::{Client, Failure as GrantFailure, Granted, Secret as GrantSecret};
use connectors_sdk::Secret;
use serde::Deserialize;
use std::{
    collections::{BTreeSet, VecDeque},
    path::{Path, PathBuf},
    sync::{Condvar, Mutex, PoisonError},
    time::{Duration, Instant},
};

/// How long a caller waits for another caller's attempt on the same source.
const JOIN_WAIT: Duration = Duration::from_secs(60);
/// The outcomes the board keeps for callers that wait on them.
const BOARD: usize = 256;

/// The outcome a refresh's caller sees (§4.1's safe names, and `rate_limited`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Refreshed,
    /// `invalid_grant`, or a source an earlier attempt consumed: the
    /// connection is Stale, `next_action: repair_connection`.
    ReauthorizationRequired,
    InsufficientScope,
    InvalidOrRevoked,
    CustodyUnavailable,
    Uncertain,
    /// HTTP 429: the exchange is released and the connection unchanged.
    RateLimited {
        retry_after_ms: Option<u64>,
    },
    /// The token endpoint could not be reached and no byte of the request
    /// left (name resolution, connection or TLS handshake failed): the
    /// exchange is released and the connection unchanged.
    Unavailable,
}

impl Outcome {
    pub fn code(self) -> &'static str {
        match self {
            Self::Refreshed => "refreshed",
            Self::ReauthorizationRequired => "reauthorization_required",
            Self::InsufficientScope => "insufficient_scope",
            Self::InvalidOrRevoked => "invalid_or_revoked",
            Self::CustodyUnavailable => "custody_unavailable",
            Self::Uncertain => "uncertain",
            Self::RateLimited { .. } => "rate_limited",
            Self::Unavailable => "unavailable",
        }
    }
}

/// The custody a refresh reads its source from and writes its response to.
pub trait Custody: Send + Sync {
    fn read(&self, version: custody::Version) -> Result<Secret, custody::Failure>;
    /// Whether a write to `version`'s scope can succeed at all.
    fn writable(&self, version: custody::Version) -> bool;
    /// Writes `material` as `version` while `guard` holds.
    fn write_new(
        &self,
        version: custody::Version,
        material: &Secret,
        guard: &mut dyn FnMut() -> Result<(), custody::Failure>,
    ) -> Result<custody::WrittenVersion, custody::Failure>;
}

/// The binding the configuration selects, opened per call.
pub struct Selected {
    selection: Selection,
    state: PathBuf,
}

impl Selected {
    pub fn new(selection: Selection, state: &Path) -> Self {
        Self {
            selection,
            state: state.to_owned(),
        }
    }
    fn open(&self, version: custody::Version) -> Result<custody::Store, custody::Failure> {
        custody::Store::open_for(version.scope(), &self.selection, &self.state)
    }
}

impl Custody for Selected {
    fn read(&self, version: custody::Version) -> Result<Secret, custody::Failure> {
        self.open(version)?.read(version)
    }
    fn writable(&self, version: custody::Version) -> bool {
        self.open(version).is_ok_and(|store| !store.read_only())
    }
    fn write_new(
        &self,
        version: custody::Version,
        material: &Secret,
        guard: &mut dyn FnMut() -> Result<(), custody::Failure>,
    ) -> Result<custody::WrittenVersion, custody::Failure> {
        self.open(version)?
            .write_new_guarded(version, material, guard)
    }
}

/// The stored entry's fields a refresh reads (`oauth2_grant::entry`).
#[derive(Deserialize)]
struct Entry {
    client_id: String,
    refresh_token: Option<GrantSecret>,
    expires_at_ms: u64,
    scopes: Vec<String>,
    id_token: Option<GrantSecret>,
}

impl Entry {
    fn read(material: &Secret) -> Option<Self> {
        serde_json::from_slice(&material.0).ok()
    }
}

/// Outcomes of finished attempts, for the callers that joined them.
struct Board {
    outcomes: Mutex<VecDeque<(String, Outcome)>>,
    posted: Condvar,
}

static OUTCOMES: Board = Board {
    outcomes: Mutex::new(VecDeque::new()),
    posted: Condvar::new(),
};

impl Board {
    fn post(&self, attempt: &str, outcome: Outcome) {
        let mut outcomes = self.outcomes.lock().unwrap_or_else(PoisonError::into_inner);
        if outcomes.len() >= BOARD {
            outcomes.pop_front();
        }
        outcomes.push_back((attempt.to_owned(), outcome));
        self.posted.notify_all();
    }

    fn wait(&self, attempt: &str, until: Instant) -> Option<Outcome> {
        let mut outcomes = self.outcomes.lock().unwrap_or_else(PoisonError::into_inner);
        loop {
            if let Some((_, outcome)) = outcomes.iter().find(|(posted, _)| posted == attempt) {
                return Some(*outcome);
            }
            let left = until.checked_duration_since(Instant::now())?;
            outcomes = match self.posted.wait_timeout(outcomes, left) {
                Ok((outcomes, _)) => outcomes,
                Err(poisoned) => poisoned.into_inner().0,
            };
        }
    }
}

/// Where a test stops the lead as if its process died there.
#[cfg(test)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Crash {
    /// After the response arrived, before anything of it is stored.
    BeforeStore,
    /// After StoreResponse, before PublishRefresh.
    AfterStore,
}

/// One owner incarnation's refresh coordinator.
pub struct Refresher {
    registry: Registry,
    owner: String,
    trust: Option<Vec<u8>>,
    custody: Box<dyn Custody>,
    #[cfg(test)]
    crash: Option<Crash>,
}

fn now() -> u64 {
    connectors_sdk::now_ms()
}

impl Refresher {
    /// The owner incarnation `owner`'s coordinator over the state directory
    /// `state` and the custody binding `selection`.
    pub fn new(state: &Path, selection: Selection, owner: String) -> Self {
        Self {
            registry: Registry::with_system_clock(state),
            owner,
            trust: oauth::follow::trust().ok().flatten(),
            custody: Box::new(Selected::new(selection, state)),
            #[cfg(test)]
            crash: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn fixture(
        registry: Registry,
        owner: &str,
        trust: &[u8],
        custody: Box<dyn Custody>,
    ) -> Self {
        Self {
            registry,
            owner: owner.to_owned(),
            trust: Some(trust.to_vec()),
            custody,
            crash: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn crashing(mut self, crash: Crash) -> Self {
        self.crash = Some(crash);
        self
    }

    /// Refreshes `connection` once: this caller sends the one exchange, or
    /// waits for the caller that does.
    pub fn refresh_connection(&self, connection: &str) -> registry::Result<Outcome> {
        // A lost attempt is resolved once, then the source is reserved again.
        for _ in 0..2 {
            let reservation = match self
                .registry
                .reserve_refresh(connection, &self.owner, now())
            {
                Err(Failure::Revoked) => return Ok(Outcome::InvalidOrRevoked),
                other => other?,
            };
            match reservation {
                Reservation::Stale | Reservation::Consumed => {
                    return Ok(Outcome::ReauthorizationRequired);
                }
                Reservation::Full => return Ok(Outcome::CustodyUnavailable),
                Reservation::Lead(ticket) => {
                    let attempt = ticket.attempt().to_owned();
                    let outcome = self.lead(ticket);
                    // A lead that died posts nothing.
                    #[cfg(test)]
                    if self.crash.is_some() && outcome == Ok(Outcome::Uncertain) {
                        return outcome;
                    }
                    OUTCOMES.post(&attempt, *outcome.as_ref().unwrap_or(&Outcome::Uncertain));
                    return outcome;
                }
                Reservation::Join {
                    attempt,
                    lost: false,
                } => {
                    let until = Instant::now() + JOIN_WAIT;
                    return Ok(OUTCOMES.wait(&attempt, until).unwrap_or(Outcome::Uncertain));
                }
                Reservation::Join {
                    attempt,
                    lost: true,
                } => match self.resolve(&attempt) {
                    Err(Failure::CustodyUnavailable) => return Ok(Outcome::CustodyUnavailable),
                    other => other?,
                },
            }
        }
        Ok(Outcome::Uncertain)
    }

    /// Resolves every attempt a past owner left unfinished; at owner start.
    /// Answers how many it found.
    pub fn recover(&self) -> registry::Result<usize> {
        let open = self.registry.open_refreshes(now())?;
        let mut failed = None;
        for attempt in &open {
            if let Err(failure) = self.resolve(attempt) {
                failed.get_or_insert(failure);
            }
        }
        failed.map_or(Ok(open.len()), Err)
    }

    fn resolve(&self, attempt: &str) -> registry::Result<()> {
        let Lost::Stored(candidate) = self.registry.resolve_lost_refresh(attempt, now())? else {
            return Ok(());
        };
        // Recovery rechecks the stored response: read back from custody now,
        // its access token's expiry decides whether it is still current.
        let entry = self
            .custody
            .read(candidate.version())
            .ok()
            .and_then(|material| Entry::read(&material))
            .ok_or(Failure::CustodyUnavailable)?;
        let collected = now();
        let baseline = ValidatedBaseline {
            identity: candidate.identity().clone(),
            granted_scopes: Some(entry.scopes.into_iter().collect()),
            credential_expires_at_ms: Some(entry.expires_at_ms),
            collected_at_ms: collected,
            valid_until_ms: collected
                .saturating_add(candidate.evidence_lifetime_ms())
                .min(entry.expires_at_ms),
            binding: candidate.operation_binding().map(str::to_owned),
        };
        self.registry
            .recover_refresh(*candidate, baseline, &self.owner, now())?;
        Ok(())
    }

    /// The reserved attempt's exchange, storage and publication.
    fn lead(&self, ticket: Box<RefreshTicket>) -> registry::Result<Outcome> {
        let Some(declaration) = ticket.declaration().cloned() else {
            self.registry.fence_refresh(&ticket, now())?;
            return Ok(Outcome::Uncertain);
        };
        let Some(entry) = self
            .custody
            .read(ticket.version())
            .ok()
            .and_then(|material| Entry::read(&material))
        else {
            self.registry.fence_refresh(&ticket, now())?;
            return Ok(Outcome::CustodyUnavailable);
        };
        let Some(refresh_token) = entry.refresh_token.as_ref() else {
            self.registry.fence_refresh(&ticket, now())?;
            return Ok(Outcome::ReauthorizationRequired);
        };
        if !self.custody.writable(ticket.version()) {
            self.registry.fence_refresh(&ticket, now())?;
            return Ok(Outcome::CustodyUnavailable);
        }
        let Ok(https) = oauth2_grant::Https::new(self.trust.as_deref()) else {
            self.registry.fence_refresh(&ticket, now())?;
            return Ok(Outcome::Uncertain);
        };
        let Ok(client) = Client::new(declaration, Some(&entry.client_id), None, &https) else {
            self.registry.fence_refresh(&ticket, now())?;
            return Ok(Outcome::Uncertain);
        };
        match self.registry.authorize_refresh(&ticket, now())? {
            Authorization::Authorized => {}
            Authorization::Moved => return Ok(Outcome::Uncertain),
            Authorization::Full => return Ok(Outcome::CustodyUnavailable),
        }
        let sent_at = now();
        let response = client.refresh(refresh_token, &entry.scopes);
        #[cfg(test)]
        if self.crash == Some(Crash::BeforeStore) {
            return Ok(Outcome::Uncertain);
        }
        let refreshed = match response {
            Ok(refreshed) => refreshed,
            // A 429 consumed no source token: the exchange is released and
            // the connection left as it is (§4.1, the one exception).
            Err(GrantFailure::RateLimited { retry_after }) => {
                self.registry.release_refresh(&ticket, now())?;
                return Ok(Outcome::RateLimited {
                    retry_after_ms: retry_after
                        .map(|delay| u64::try_from(delay.as_millis()).unwrap_or(u64::MAX)),
                });
            }
            // No byte of the request left (§4.1, the other exception): the
            // source token cannot have been consumed.
            Err(_) if !https.may_have_sent() => {
                self.registry.release_refresh(&ticket, now())?;
                return Ok(Outcome::Unavailable);
            }
            Err(failure) => {
                let outcome = match failure {
                    GrantFailure::ReauthorizationRequired => Outcome::ReauthorizationRequired,
                    GrantFailure::InsufficientScope => Outcome::InsufficientScope,
                    _ => Outcome::Uncertain,
                };
                self.registry.quarantine_refresh(&ticket, true, now())?;
                return Ok(outcome);
            }
        };
        // Rotation never widens permission.
        let previous: BTreeSet<&str> = match ticket.granted_scopes() {
            Some(granted) => granted.iter().map(String::as_str).collect(),
            None => entry.scopes.iter().map(String::as_str).collect(),
        };
        if refreshed
            .scopes
            .iter()
            .any(|scope| !previous.contains(scope.as_str()))
        {
            self.registry.quarantine_refresh(&ticket, true, now())?;
            return Ok(Outcome::InsufficientScope);
        }
        let granted = Granted {
            access_token: refreshed.access_token,
            refresh_token: Some(refreshed.refresh_token),
            expires_in: refreshed.expires_in,
            scopes: refreshed.scopes,
            id_token: entry.id_token,
        };
        let expires_at = sent_at
            .saturating_add(u64::try_from(granted.expires_in.as_millis()).unwrap_or(u64::MAX));
        let Ok(material) = oauth2_grant::entry(&entry.client_id, &granted, sent_at) else {
            self.registry.quarantine_refresh(&ticket, true, now())?;
            return Ok(Outcome::Uncertain);
        };
        let collected = now();
        let baseline = ValidatedBaseline {
            identity: ticket.identity().clone(),
            granted_scopes: Some(granted.scopes.iter().cloned().collect()),
            credential_expires_at_ms: Some(expires_at),
            collected_at_ms: collected,
            valid_until_ms: collected
                .saturating_add(ticket.evidence_lifetime_ms())
                .min(expires_at),
            binding: ticket.operation_binding().map(str::to_owned),
        };
        drop(granted);
        let prepared =
            match self
                .registry
                .prepare_refresh(ticket, baseline, material.0.len(), now())
            {
                Ok(prepared) => prepared,
                Err((ticket, failure)) => {
                    let outcome = match failure {
                        Failure::InsufficientScope => Outcome::InsufficientScope,
                        Failure::Revoked => Outcome::InvalidOrRevoked,
                        _ => Outcome::Uncertain,
                    };
                    self.registry.quarantine_refresh(
                        &ticket,
                        failure != Failure::Revoked,
                        now(),
                    )?;
                    return Ok(outcome);
                }
            };
        let written = self
            .custody
            .write_new(prepared.version(), &material, &mut || {
                self.registry
                    .refresh_write_allowed(&prepared, now())
                    .map_err(|_| custody::Failure::Denied)
            });
        drop(material);
        let receipt = match written {
            Ok(receipt) => receipt,
            Err(_) => {
                self.registry
                    .quarantine_refresh(&prepared.into_ticket(), true, now())?;
                return Ok(Outcome::CustodyUnavailable);
            }
        };
        let stored = match self.registry.store_refresh(prepared, receipt, now()) {
            Ok(stored) => stored,
            // An unknown commit leaves the attempt to the next owner's recovery.
            Err((_, Failure::OutcomeUnknown)) => return Ok(Outcome::Uncertain),
            Err((ticket, _)) => {
                self.registry.quarantine_refresh(&ticket, true, now())?;
                return Ok(Outcome::Uncertain);
            }
        };
        #[cfg(test)]
        if self.crash == Some(Crash::AfterStore) {
            return Ok(Outcome::Uncertain);
        }
        Ok(match self.registry.publish_refresh(stored, now())? {
            Publication::Published => Outcome::Refreshed,
            Publication::Revoked => Outcome::InvalidOrRevoked,
            Publication::Superseded | Publication::Refused => Outcome::Uncertain,
        })
    }
}

pub mod issue;

#[cfg(test)]
mod tests;

//! The registry side of the owner's refresh of an `oauth2_grant` connection
//! (story:owner-rotating-refresh, `contracts/auth/acquisition/v1alpha1/semantics.md`
//! §4.1). Each step's SQL change and the `connectors.refresh` transition it
//! stages commit in one batch ([`Registry::transaction_ledger`]), so the
//! recorded ledger, read under the lifecycle lock, serializes the refreshes
//! of one source generation; no process mutex does.
//!
//! A refreshed custody version names its attempt as its acquisition
//! (`refresh:<attempt>`); its generation's capture is the same reference.
use super::{
    Binding, Failure, Registry, Result, ValidatedBaseline, custody, db, encode, host_failure,
    lifecycle::{bump, custody_version, mark_stale, recollected_snapshot, retire},
    new_id, read_optional_time, timestamp,
};
use crate::local::metadata::{
    LedgerStep, Metadata, REFRESH_ACQUISITION, RefreshAttempt, RefreshState,
    insert_refresh_acquisition,
};
use rusqlite::{OptionalExtension, Transaction, params};
use std::collections::BTreeSet;

/// The custody versions a refreshed connection may retain, retired ones
/// included until their deletion: an hourly refresh retires 24 a day.
const RETAINED: i64 = 64;

/// The attempts one registry transaction reads and the transitions it stages.
#[derive(Default)]
pub(super) struct Ledger {
    attempts: Vec<RefreshAttempt>,
    staged: Vec<LedgerStep>,
}

impl Ledger {
    pub(super) fn read(metadata: &Metadata) -> Result<Self> {
        Ok(Self {
            attempts: metadata.refresh_attempts().map_err(host_failure)?,
            staged: Vec::new(),
        })
    }

    pub(super) fn into_staged(self) -> Vec<LedgerStep> {
        self.staged
    }

    fn attempt(&self, id: &str) -> Result<RefreshAttempt> {
        self.attempts
            .iter()
            .find(|attempt| attempt.id == id)
            .cloned()
            .ok_or(Failure::NotFound)
    }

    /// Stages `step` and applies it to the attempts this transaction sees.
    fn stage(&mut self, step: LedgerStep) {
        if let LedgerStep::Reserve {
            attempt,
            source_generation,
            owner,
            token,
            expected_fence,
        } = &step
        {
            self.attempts.push(RefreshAttempt {
                id: attempt.clone(),
                state: RefreshState::Reserved,
                source_generation: source_generation.clone(),
                owner: owner.clone(),
                token: token.clone(),
                expected_fence: expected_fence.clone(),
                candidate_generation: None,
            });
        } else if let Some(attempt) = self.attempts.iter_mut().find(|a| a.id == step.attempt()) {
            attempt.state = match &step {
                LedgerStep::Authorize { .. } => RefreshState::Authorized,
                LedgerStep::Fence { .. } => RefreshState::Fenced,
                LedgerStep::Store {
                    candidate_generation,
                    ..
                } => {
                    attempt.candidate_generation = Some(candidate_generation.clone());
                    RefreshState::ResponseStored
                }
                LedgerStep::Quarantine { .. } => RefreshState::Uncertain,
                LedgerStep::Release { .. } => RefreshState::Released,
                LedgerStep::Recover { .. } => RefreshState::Recovering,
                LedgerStep::Publish { .. } => RefreshState::Published,
                LedgerStep::Discard { .. } | LedgerStep::Reserve { .. } => RefreshState::Discarded,
            };
        }
        self.staged.push(step);
    }
}

/// The lead of one refresh: its owner reserved the attempt, and only it may
/// authorize, store and publish it.
pub struct RefreshTicket {
    attempt: String,
    connection: String,
    owner: String,
    token: String,
    source: String,
    fence: String,
    version: custody::Version,
    binding: Binding,
    identity: super::ExternalIdentity,
    granted_scopes: Option<BTreeSet<String>>,
    operation_binding: Option<String>,
}

impl RefreshTicket {
    pub fn attempt(&self) -> &str {
        &self.attempt
    }
    /// The source generation's custody version, which holds the refresh token.
    pub fn version(&self) -> custody::Version {
        self.version
    }
    pub fn declaration(&self) -> Option<&connectors_oauth2::ServerDeclaration> {
        self.binding.profile.oauth2.as_ref()
    }
    pub fn evidence_lifetime_ms(&self) -> u64 {
        self.binding.profile.evidence_lifetime_ms
    }
    pub fn identity(&self) -> &super::ExternalIdentity {
        &self.identity
    }
    /// The scopes the source generation's evidence recorded.
    pub fn granted_scopes(&self) -> Option<&BTreeSet<String>> {
        self.granted_scopes.as_ref()
    }
    pub fn operation_binding(&self) -> Option<&str> {
        self.operation_binding.as_deref()
    }
}

/// What a refresh's reservation found.
pub enum Reservation {
    /// This caller reserved the attempt and sends the one exchange.
    Lead(Box<RefreshTicket>),
    /// An unfinished attempt holds the source; `lost` when an owner other than
    /// the caller reserved it, which only a past owner incarnation can have.
    Join { attempt: String, lost: bool },
    /// The connection is Stale: only a repair returns it.
    Stale,
    /// An earlier attempt consumed the source's one exchange without
    /// publishing a successor (§4.1: it is never reopened); the reservation
    /// marked the connection Stale.
    Consumed,
}

pub struct PreparedRefresh {
    ticket: Box<RefreshTicket>,
    generation: String,
    version_id: String,
    version: custody::Version,
    baseline: ValidatedBaseline,
}

impl PreparedRefresh {
    /// The candidate custody version the response is written to.
    pub fn version(&self) -> custody::Version {
        self.version
    }
    /// The attempt back, to quarantine when the write fails.
    pub fn into_ticket(self) -> Box<RefreshTicket> {
        self.ticket
    }
}

pub struct StoredRefresh {
    prepared: PreparedRefresh,
    acknowledged_at: u64,
}

/// How a publication ended. Every outcome but `Published` discarded the
/// candidate; `Refused` also marked the connection Stale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Publication {
    Published,
    Revoked,
    /// A repair replaced the source generation meanwhile.
    Superseded,
    /// The private publication fence moved, the connection went Stale or the
    /// candidate's evidence is no longer current.
    Refused,
}

/// A stored response a past owner left unpublished, as recovery reads it.
pub struct RecoveryCandidate {
    attempt: String,
    connection: String,
    source: String,
    fence: String,
    generation: String,
    version_id: String,
    version: custody::Version,
    acknowledged_at: u64,
    binding: Binding,
    identity: super::ExternalIdentity,
    operation_binding: Option<String>,
}

impl RecoveryCandidate {
    pub fn version(&self) -> custody::Version {
        self.version
    }
    pub fn acknowledged_at_ms(&self) -> u64 {
        self.acknowledged_at
    }
    pub fn evidence_lifetime_ms(&self) -> u64 {
        self.binding.profile.evidence_lifetime_ms
    }
    pub fn identity(&self) -> &super::ExternalIdentity {
        &self.identity
    }
    pub fn operation_binding(&self) -> Option<&str> {
        self.operation_binding.as_deref()
    }
}

/// What recovery found of an attempt a past owner left unfinished.
pub enum Lost {
    /// Resolved without a request: a reservation fenced, an authorized
    /// exchange quarantined (`uncertain`, connection Stale) or a lost recovery
    /// discarded (connection Stale).
    Resolved,
    /// A stored response, to publish.
    Stored(Box<RecoveryCandidate>),
}

fn acquisition(attempt: &str) -> String {
    format!("{REFRESH_ACQUISITION}{attempt}")
}

fn owned(attempt: &RefreshAttempt, ticket: &RefreshTicket, state: RefreshState) -> Result<()> {
    if attempt.state != state || attempt.owner != ticket.owner || attempt.token != ticket.token {
        return Err(Failure::Conflict);
    }
    Ok(())
}

/// Retires the attempt's candidate version if one was allocated.
fn retire_candidate(tx: &Transaction<'_>, attempt: &str, now: u64) -> Result<()> {
    let version: Option<String> = tx
        .query_row(
            "SELECT version_id FROM registry_materials WHERE acquisition_ref=?1",
            [acquisition(attempt)],
            |r| r.get(0),
        )
        .optional()
        .map_err(db)?;
    if let Some(version) = version {
        retire(tx, &version, now)?;
    }
    Ok(())
}

/// Marks the connection Stale while `source` is still its active generation:
/// the source's refresh token is spent and no successor was published.
fn stale_source(tx: &Transaction<'_>, connection: &str, source: &str) -> Result<()> {
    let row = Registry::connection(tx, connection)?;
    if row.state != "revoked" && row.generation.as_deref() == Some(source) && !row.kept.stale {
        mark_stale(tx, &row)?;
        bump(tx, &row.binding.instance_id)?;
    }
    Ok(())
}

/// The connection whose active or past generation `source` is.
fn source_connection(tx: &Transaction<'_>, source: &str) -> Result<String> {
    tx.query_row(
        "SELECT connection_ref FROM registry_generations WHERE generation_id=?1",
        [source],
        |r| r.get(0),
    )
    .map_err(db)
}

struct Candidate<'a> {
    attempt: &'a str,
    connection: &'a str,
    source: &'a str,
    fence: &'a str,
    generation: &'a str,
    version_id: &'a str,
    binding: &'a Binding,
    baseline: &'a ValidatedBaseline,
    acknowledged_at: u64,
}

/// PublishRefresh's checks and effect, after the caller checked the attempt
/// and its ownership. A refusal discards the candidate instead.
fn publish(
    tx: &Transaction<'_>,
    ledger: &mut Ledger,
    candidate: Candidate<'_>,
    publish: LedgerStep,
    now: u64,
) -> Result<Publication> {
    let row = Registry::connection(tx, candidate.connection)?;
    let refusal = if row.state == "revoked" {
        Some(Publication::Revoked)
    } else if row.generation.as_deref() != Some(candidate.source) {
        Some(Publication::Superseded)
    } else if row.kept.stale || row.fence != candidate.fence || row.binding != *candidate.binding {
        Some(Publication::Refused)
    } else {
        None
    };
    let collected = candidate.baseline.collected_at_ms;
    let snapshot = recollected_snapshot(
        &row.binding,
        &row.reference,
        candidate.generation,
        candidate.baseline,
        collected,
        candidate.acknowledged_at,
    )?;
    let current: bool = tx.query_row("SELECT acknowledged=1 AND deleted=0 AND retirement_fence IS NULL AND invalid_reason IS NULL FROM registry_materials WHERE version_id=?1 AND generation_id=?2 AND acquisition_ref=?3",
        params![candidate.version_id, candidate.generation, acquisition(candidate.attempt)], |r| r.get(0)).map_err(db)?;
    let refusal = refusal.or_else(|| {
        let valid = current
            && row.identity.as_ref() == Some(&candidate.baseline.identity)
            && candidate
                .baseline
                .validate(&row.binding, collected, now)
                .is_ok()
            && snapshot.current(now);
        (!valid).then_some(Publication::Refused)
    });
    if let Some(refusal) = refusal {
        retire(tx, candidate.version_id, now)?;
        if refusal == Publication::Refused {
            stale_source(tx, candidate.connection, candidate.source)?;
        }
        ledger.stage(LedgerStep::Discard {
            attempt: candidate.attempt.to_owned(),
        });
        return Ok(refusal);
    }
    if let Some(old) = &row.material {
        retire(tx, old, now)?;
    }
    tx.execute("UPDATE registry_connections SET active_generation=?2,active_material=?3,baseline=?4,publication_fence=?5 WHERE connection_ref=?1",
        params![row.reference, candidate.generation, candidate.version_id, encode(&snapshot)?, new_id()]).map_err(db)?;
    tx.execute(
        "UPDATE registry_uses SET released=1 WHERE connection_ref=?1 AND dispatched=0",
        [&row.reference],
    )
    .map_err(db)?;
    bump(tx, &row.binding.instance_id)?;
    ledger.stage(publish);
    Ok(Publication::Published)
}

impl Registry {
    /// `connectors.refresh.ReserveRefresh` on the connection's active
    /// generation, unless an attempt already holds or consumed it.
    pub(in crate::local) fn reserve_refresh(
        &self,
        connection: &str,
        owner: &str,
        now: u64,
    ) -> Result<Reservation> {
        self.transaction_ledger(now, |tx, authority, _, ledger| {
            let row = Self::connection(tx, connection)?;
            if row.state == "revoked" {
                return Err(Failure::Revoked);
            }
            let declaration = row
                .binding
                .profile
                .oauth2
                .as_ref()
                .ok_or(Failure::InvalidInput)?;
            // A confidential client's secret is not part of the stored entry.
            if !matches!(
                declaration.client_kind,
                connectors_oauth2::ClientKind::Public
            ) {
                return Err(Failure::InvalidInput);
            }
            if !row.public {
                return Err(Failure::NotFound);
            }
            if row.kept.stale {
                return Ok(Reservation::Stale);
            }
            let (Some(source), Some(material), Some(snapshot)) =
                (&row.generation, &row.material, &row.baseline)
            else {
                return Err(Failure::NotReady);
            };
            let mut held = ledger
                .attempts
                .iter()
                .filter(|attempt| attempt.source_generation == *source);
            if let Some(open) = held.clone().find(|attempt| !attempt.state.terminal()) {
                return Ok(Reservation::Join {
                    attempt: open.id.clone(),
                    lost: open.owner != owner,
                });
            }
            // A fenced or released attempt left the source refreshable.
            if held.any(|attempt| {
                !matches!(attempt.state, RefreshState::Fenced | RefreshState::Released)
            }) {
                mark_stale(tx, &row)?;
                bump(tx, &row.binding.instance_id)?;
                return Ok(Reservation::Consumed);
            }
            let ticket = RefreshTicket {
                attempt: new_id(),
                connection: row.reference.clone(),
                owner: owner.to_owned(),
                token: new_id(),
                source: source.clone(),
                fence: row.fence.clone(),
                version: custody_version(authority, &row.scope_id, material)?,
                binding: row.binding.clone(),
                identity: row.identity.clone().ok_or(Failure::MetadataUnavailable)?,
                granted_scopes: snapshot.granted_scopes.clone(),
                operation_binding: snapshot.operation_binding.clone(),
            };
            ledger.stage(LedgerStep::Reserve {
                attempt: ticket.attempt.clone(),
                source_generation: ticket.source.clone(),
                owner: ticket.owner.clone(),
                token: ticket.token.clone(),
                expected_fence: ticket.fence.clone(),
            });
            Ok(Reservation::Lead(Box::new(ticket)))
        })
    }

    /// `connectors.refresh.AuthorizeExchange`, durable before the caller may
    /// send. `false`: the source or the private fence moved, the attempt is
    /// fenced and nothing may be sent.
    pub(in crate::local) fn authorize_refresh(
        &self,
        ticket: &RefreshTicket,
        now: u64,
    ) -> Result<bool> {
        self.transaction_ledger(now, |tx, _, _, ledger| {
            owned(
                &ledger.attempt(&ticket.attempt)?,
                ticket,
                RefreshState::Reserved,
            )?;
            let row = Self::connection(tx, &ticket.connection)?;
            let current = row.state != "revoked"
                && !row.kept.stale
                && row.generation.as_deref() == Some(ticket.source.as_str())
                && row.fence == ticket.fence
                && row.binding == ticket.binding;
            ledger.stage(if current {
                LedgerStep::Authorize {
                    attempt: ticket.attempt.clone(),
                    owner: ticket.owner.clone(),
                    token: ticket.token.clone(),
                }
            } else {
                LedgerStep::Fence {
                    attempt: ticket.attempt.clone(),
                }
            });
            Ok(current)
        })
    }

    /// `connectors.refresh.FenceReserved` of the caller's own reservation,
    /// when it will not send: the source stays refreshable.
    pub(in crate::local) fn fence_refresh(&self, ticket: &RefreshTicket, now: u64) -> Result<()> {
        self.transaction_ledger(now, |_, _, _, ledger| {
            owned(
                &ledger.attempt(&ticket.attempt)?,
                ticket,
                RefreshState::Reserved,
            )?;
            ledger.stage(LedgerStep::Fence {
                attempt: ticket.attempt.clone(),
            });
            Ok(())
        })
    }

    /// `connectors.refresh.QuarantineRefresh` of the caller's authorized
    /// exchange that stored no response: the source stays consumed. `stale`
    /// also marks the connection Stale (`repair_connection`).
    pub(in crate::local) fn quarantine_refresh(
        &self,
        ticket: &RefreshTicket,
        stale: bool,
        now: u64,
    ) -> Result<()> {
        self.transaction_ledger(now, |tx, _, now, ledger| {
            owned(
                &ledger.attempt(&ticket.attempt)?,
                ticket,
                RefreshState::Authorized,
            )?;
            retire_candidate(tx, &ticket.attempt, now)?;
            if stale {
                stale_source(tx, &ticket.connection, &ticket.source)?;
            }
            ledger.stage(LedgerStep::Quarantine {
                attempt: ticket.attempt.clone(),
            });
            Ok(())
        })
    }

    /// `connectors.refresh.ReleaseRateLimited` of the caller's authorized
    /// exchange the token endpoint refused with HTTP 429 (§4.1, the one
    /// exception): the source may be reserved again and the connection is
    /// left as it was.
    pub(in crate::local) fn release_refresh(&self, ticket: &RefreshTicket, now: u64) -> Result<()> {
        self.transaction_ledger(now, |tx, _, now, ledger| {
            owned(
                &ledger.attempt(&ticket.attempt)?,
                ticket,
                RefreshState::Authorized,
            )?;
            retire_candidate(tx, &ticket.attempt, now)?;
            ledger.stage(LedgerStep::Release {
                attempt: ticket.attempt.clone(),
                owner: ticket.owner.clone(),
                token: ticket.token.clone(),
            });
            Ok(())
        })
    }

    /// Allocates the response's candidate generation and custody version
    /// (`RecordCredentialGeneration`, `RecordCustodyVersion`) before the
    /// write, for the caller's authorized attempt. `baseline` is the
    /// response's evidence under the connection's recorded identity.
    pub(in crate::local) fn prepare_refresh(
        &self,
        ticket: Box<RefreshTicket>,
        baseline: ValidatedBaseline,
        byte_size: usize,
        now: u64,
    ) -> std::result::Result<PreparedRefresh, (Box<RefreshTicket>, Failure)> {
        if !(1..=65536).contains(&byte_size) {
            return Err((ticket, Failure::InvalidInput));
        }
        let allocated = self.transaction_ledger(now, |tx, authority, now, ledger| {
            owned(
                &ledger.attempt(&ticket.attempt)?,
                &ticket,
                RefreshState::Authorized,
            )?;
            let row = Self::connection(tx, &ticket.connection)?;
            if row.state == "revoked" {
                return Err(Failure::Revoked);
            }
            if row.generation.as_deref() != Some(ticket.source.as_str())
                || row.fence != ticket.fence
                || row.binding != ticket.binding
            {
                return Err(Failure::Conflict);
            }
            baseline.validate(&row.binding, baseline.collected_at_ms, now)?;
            if row.identity.as_ref() != Some(&baseline.identity) {
                return Err(Failure::IdentityMismatch);
            }
            let retained: i64 = tx
                .query_row(
                    "SELECT count(*) FROM registry_materials WHERE connection_ref=?1 AND deleted=0",
                    [&row.reference],
                    |r| r.get(0),
                )
                .map_err(db)?;
            if retained >= RETAINED {
                return Err(Failure::Capacity);
            }
            let acquisition = acquisition(&ticket.attempt);
            let (generation, version_id) = (new_id(), new_id());
            tx.execute(
                "INSERT INTO registry_generations VALUES (?1,?2,?3,?4)",
                params![
                    generation,
                    row.reference,
                    acquisition,
                    encode(&baseline.identity)?
                ],
            )
            .map_err(db)?;
            insert_refresh_acquisition(tx, &acquisition, &row.reference, &generation)
                .map_err(host_failure)?;
            tx.execute("INSERT INTO registry_materials (version_id,connection_ref,acquisition_ref,generation_id,byte_size) VALUES (?1,?2,?3,?4,?5)",
                params![version_id, row.reference, acquisition, generation, byte_size as i64]).map_err(db)?;
            let version = custody_version(authority, &row.scope_id, &version_id)?;
            Ok((generation, version_id, version))
        });
        match allocated {
            Ok((generation, version_id, version)) => Ok(PreparedRefresh {
                ticket,
                generation,
                version_id,
                version,
                baseline,
            }),
            Err(failure) => Err((ticket, failure)),
        }
    }

    /// Called under the custody binding's writer lock: the attempt is still
    /// the caller's, authorized, and its candidate unacknowledged and unretired.
    pub(in crate::local) fn refresh_write_allowed(
        &self,
        prepared: &PreparedRefresh,
        now: u64,
    ) -> Result<()> {
        self.transaction_ledger(now, |tx, authority, _, ledger| {
            let ticket = &prepared.ticket;
            owned(
                &ledger.attempt(&ticket.attempt)?,
                ticket,
                RefreshState::Authorized,
            )?;
            let row = Self::connection(tx, &ticket.connection)?;
            if row.state == "revoked"
                || row.generation.as_deref() != Some(ticket.source.as_str())
                || row.fence != ticket.fence
                || prepared.version != custody_version(authority, &row.scope_id, &prepared.version_id)?
            {
                return Err(Failure::Conflict);
            }
            let (ack, retired, deleted): (bool, bool, bool) = tx
                .query_row("SELECT acknowledged,retirement_fence IS NOT NULL,deleted FROM registry_materials WHERE version_id=?1 AND acquisition_ref=?2",
                    params![prepared.version_id, acquisition(&ticket.attempt)], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
                .map_err(db)?;
            if ack || retired || deleted {
                return Err(Failure::Conflict);
            }
            Ok(())
        })
    }

    /// `connectors.refresh.StoreResponse`: the definite custody receipt
    /// acknowledges the candidate and attaches its generation to the attempt.
    pub(in crate::local) fn store_refresh(
        &self,
        prepared: PreparedRefresh,
        receipt: custody::WrittenVersion,
        now: u64,
    ) -> std::result::Result<StoredRefresh, (Box<RefreshTicket>, Failure)> {
        if !receipt.matches(prepared.version) {
            return Err((prepared.ticket, Failure::Conflict));
        }
        let stored = self.transaction_ledger(now, |tx, _, now, ledger| {
            let ticket = &prepared.ticket;
            owned(
                &ledger.attempt(&ticket.attempt)?,
                ticket,
                RefreshState::Authorized,
            )?;
            let deleted: bool = tx
                .query_row("SELECT deleted OR retirement_fence IS NOT NULL FROM registry_materials WHERE version_id=?1 AND acquisition_ref=?2 AND generation_id=?3",
                    params![prepared.version_id, acquisition(&ticket.attempt), prepared.generation], |r| r.get(0))
                .map_err(db)?;
            if deleted {
                return Err(Failure::Conflict);
            }
            tx.execute(
                "UPDATE registry_materials SET acknowledged=1,acknowledged_at_ms=?2 WHERE version_id=?1",
                params![prepared.version_id, timestamp(now)?],
            )
            .map_err(db)?;
            ledger.stage(LedgerStep::Store {
                attempt: ticket.attempt.clone(),
                owner: ticket.owner.clone(),
                token: ticket.token.clone(),
                candidate_generation: prepared.generation.clone(),
            });
            Ok(now)
        });
        match stored {
            Ok(acknowledged_at) => Ok(StoredRefresh {
                prepared,
                acknowledged_at,
            }),
            Err(failure) => Err((prepared.ticket, failure)),
        }
    }

    /// `connectors.refresh.PublishRefresh` under the private publication
    /// fence: the candidate becomes the active generation and custody
    /// version, the old version is retired. A refusal discards the candidate
    /// (`connectors.refresh.DiscardCandidate`).
    pub(in crate::local) fn publish_refresh(
        &self,
        stored: StoredRefresh,
        now: u64,
    ) -> Result<Publication> {
        let prepared = &stored.prepared;
        let ticket = &prepared.ticket;
        self.transaction_ledger(now, |tx, _, now, ledger| {
            let attempt = ledger.attempt(&ticket.attempt)?;
            owned(&attempt, ticket, RefreshState::ResponseStored)?;
            if attempt.candidate_generation.as_deref() != Some(prepared.generation.as_str()) {
                return Err(Failure::Conflict);
            }
            publish(
                tx,
                ledger,
                Candidate {
                    attempt: &ticket.attempt,
                    connection: &ticket.connection,
                    source: &ticket.source,
                    fence: &ticket.fence,
                    generation: &prepared.generation,
                    version_id: &prepared.version_id,
                    binding: &ticket.binding,
                    baseline: &prepared.baseline,
                    acknowledged_at: stored.acknowledged_at,
                },
                LedgerStep::Publish {
                    attempt: ticket.attempt.clone(),
                    owner: ticket.owner.clone(),
                    token: ticket.token.clone(),
                    candidate_generation: prepared.generation.clone(),
                },
                now,
            )
        })
    }

    /// Every attempt no transition has ended, oldest first by id.
    pub(in crate::local) fn open_refreshes(&self, now: u64) -> Result<Vec<String>> {
        self.transaction_ledger(now, |_, _, _, ledger| {
            let mut open = ledger
                .attempts
                .iter()
                .filter(|attempt| !attempt.state.terminal())
                .map(|attempt| attempt.id.clone())
                .collect::<Vec<_>>();
            open.sort();
            Ok(open)
        })
    }

    /// Resolves an attempt whose owner is gone (§4.1, "Resolve authorized
    /// owner loss"): a reservation is fenced; an authorized exchange without
    /// a stored response is quarantined and the connection marked Stale; a
    /// lost recovery is discarded and the connection marked Stale. A stored
    /// response is returned for publication.
    pub(in crate::local) fn resolve_lost_refresh(&self, attempt: &str, now: u64) -> Result<Lost> {
        self.transaction_ledger(now, |tx, authority, now, ledger| {
            let found = ledger.attempt(attempt)?;
            let step = match found.state {
                RefreshState::Reserved => LedgerStep::Fence {
                    attempt: found.id.clone(),
                },
                RefreshState::Authorized | RefreshState::Recovering => {
                    let connection = source_connection(tx, &found.source_generation)?;
                    retire_candidate(tx, &found.id, now)?;
                    stale_source(tx, &connection, &found.source_generation)?;
                    if found.state == RefreshState::Authorized {
                        LedgerStep::Quarantine {
                            attempt: found.id.clone(),
                        }
                    } else {
                        LedgerStep::Discard {
                            attempt: found.id.clone(),
                        }
                    }
                }
                RefreshState::ResponseStored => {
                    let connection = source_connection(tx, &found.source_generation)?;
                    let row = Self::connection(tx, &connection)?;
                    let generation = found
                        .candidate_generation
                        .clone()
                        .ok_or(Failure::MetadataUnavailable)?;
                    let (version_id, acknowledged_at): (String, Option<u64>) = tx
                        .query_row("SELECT version_id,acknowledged_at_ms FROM registry_materials WHERE acquisition_ref=?1 AND generation_id=?2",
                            params![acquisition(&found.id), generation], |r| Ok((r.get(0)?, read_optional_time(r, 1)?)))
                        .map_err(db)?;
                    let identity: String = tx
                        .query_row(
                            "SELECT expected_identity FROM registry_generations WHERE generation_id=?1",
                            [&generation],
                            |r| r.get(0),
                        )
                        .map_err(db)?;
                    return Ok(Lost::Stored(Box::new(RecoveryCandidate {
                        attempt: found.id.clone(),
                        version: custody_version(authority, &row.scope_id, &version_id)?,
                        connection,
                        source: found.source_generation.clone(),
                        fence: found.expected_fence.clone(),
                        generation,
                        version_id,
                        acknowledged_at: acknowledged_at.ok_or(Failure::MetadataUnavailable)?,
                        operation_binding: row
                            .baseline
                            .as_ref()
                            .and_then(|snapshot| snapshot.operation_binding.clone()),
                        binding: row.binding,
                        identity: super::decode(&identity)?,
                    })));
                }
                _ => return Err(Failure::Conflict),
            };
            ledger.stage(step);
            Ok(Lost::Resolved)
        })
    }

    /// `connectors.refresh.RecoverStoredResponse` then `PublishRefresh`, in
    /// one batch under the recovering `owner`: no request is sent. `baseline`
    /// is the stored response's evidence; one no longer current discards the
    /// candidate and marks the connection Stale.
    pub(in crate::local) fn recover_refresh(
        &self,
        candidate: RecoveryCandidate,
        baseline: ValidatedBaseline,
        owner: &str,
        now: u64,
    ) -> Result<Publication> {
        self.transaction_ledger(now, |tx, _, now, ledger| {
            let attempt = ledger.attempt(&candidate.attempt)?;
            if attempt.state != RefreshState::ResponseStored
                || attempt.candidate_generation.as_deref() != Some(candidate.generation.as_str())
            {
                return Err(Failure::Conflict);
            }
            let token = new_id();
            ledger.stage(LedgerStep::Recover {
                attempt: candidate.attempt.clone(),
                owner: owner.to_owned(),
                token: token.clone(),
            });
            publish(
                tx,
                ledger,
                Candidate {
                    attempt: &candidate.attempt,
                    connection: &candidate.connection,
                    source: &candidate.source,
                    fence: &candidate.fence,
                    generation: &candidate.generation,
                    version_id: &candidate.version_id,
                    binding: &candidate.binding,
                    baseline: &baseline,
                    acknowledged_at: candidate.acknowledged_at,
                },
                LedgerStep::Publish {
                    attempt: candidate.attempt.clone(),
                    owner: owner.to_owned(),
                    token,
                    candidate_generation: candidate.generation.clone(),
                },
                now,
            )
        })
    }
}

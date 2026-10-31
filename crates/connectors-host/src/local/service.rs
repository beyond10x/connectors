//! The HTTP service host's local state (contracts/service/compatibility.md
//! § 2.1, Service configuration and Host start): the recorded
//! `connectors.declarations.ServiceConfiguration` of its instance, the
//! execution-audit anchors of `POST /v1alpha2/invoke` and the attempt records
//! of its admitted writes, in the local metadata authority in the configured
//! `state` directory.
//!
//! An acknowledged anchor is one prerequisite for dispatch, never caller,
//! credential or dispatch authority by itself (contracts/service/audit.md § 1).
//! The attempt ledger follows `ess/domains/mutations.yaml`, "The HTTP host's
//! attempt ledger". Without `standing_grants` a write is recorded unkeyed
//! ([`ServiceState::prepare_attempt`]), as before standing grants existed.
//! With them ([`ServiceState::with_standing_grants`]) every write is keyed and
//! admitted under a standing grant (`ess/domains/standing_approvals.yaml`):
//! the keyed reservation is the ledger's own (`mutation_keys`, in the metadata
//! authority) and the hourly count is [`standing::Uses`] beside it. Both are
//! durable, so a restart keeps every key and every counted use.
use super::{
    Failure, Result, audit, filesystem as fs, metadata::Metadata, mutations as ledger, standing,
};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, Weak},
    time::{Duration, Instant},
};
use uuid::Uuid;

/// Replays that wait on one original this process is running
/// (compatibility.md § 2.1, Replay). A further replay of it is answered
/// `outcome_unknown` at once rather than wait.
pub const REPLAY_WAITERS: usize = 4;

/// The key lanes of this process: one async mutex per (principal, key) with
/// a keyed write between its key lookup and its prepared attempt. An entry
/// exists only while a request holds or waits for it.
type Lanes = Arc<Mutex<HashMap<(String, String), Arc<tokio::sync::Mutex<()>>>>>;
/// The keyed attempts this process prepared and has not yet settled or
/// aborted, with the replays waiting on each.
type Flights = Arc<Mutex<HashMap<Uuid, Flight>>>;

struct Flight {
    waiters: usize,
    /// Dropped when the attempt leaves the map, which wakes every waiter.
    landed: tokio::sync::watch::Sender<()>,
}

fn locked<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

/// One request's hold on a key lane ([`ServiceState::key_lane`]). Dropping
/// it lets the next request with that key in.
pub struct KeyLane {
    held: Option<tokio::sync::OwnedMutexGuard<()>>,
    lanes: Lanes,
    name: (String, String),
}

impl Drop for KeyLane {
    fn drop(&mut self) {
        let mut lanes = locked(&self.lanes);
        drop(self.held.take());
        // Only the map still refers to an idle lane: no holder, no waiter.
        if lanes
            .get(&self.name)
            .is_some_and(|lane| Arc::strong_count(lane) == 1)
        {
            lanes.remove(&self.name);
        }
    }
}

/// How a replay of a pending original may wait for it
/// ([`ServiceState::wait_on`]).
pub enum Waiting {
    /// This process runs the original; the wait ends when it settles.
    Joined(ReplayWait),
    /// This process does not run the original: an earlier process (a crash
    /// or a restart) or another process on the state directory left it
    /// pending. Answered `outcome_unknown` at once.
    NotRunHere,
    /// [`REPLAY_WAITERS`] replays already wait on it. Answered
    /// `outcome_unknown` at once.
    Full,
}

/// One replay's place among an original's waiters.
pub struct ReplayWait {
    landed: tokio::sync::watch::Receiver<()>,
    attempt: Uuid,
    flights: Flights,
}

impl ReplayWait {
    /// `true` once this process has settled or aborted the original, `false`
    /// when `within` passed first.
    pub async fn landed(&mut self, within: Duration) -> bool {
        // Nothing is ever sent: `changed` ends only when the sender drops.
        matches!(
            tokio::time::timeout(within, self.landed.changed()).await,
            Ok(Err(_))
        )
    }
}

impl Drop for ReplayWait {
    fn drop(&mut self) {
        if let Some(flight) = locked(&self.flights).get_mut(&self.attempt) {
            flight.waiters = flight.waiters.saturating_sub(1);
        }
    }
}

/// The static-bearer service principal (compatibility.md § 2.1, Audit anchor):
/// one receiver-configured principal, qualified by the anchor's instance, that
/// does not change when the credential bytes rotate.
pub const STATIC_BEARER_PRINCIPAL: &str = "static-bearer";
/// Retained records per instance (audit.md § 4); capacity refuses, never evicts.
const RECORDS_PER_INSTANCE: u32 = 100_000;
/// Final observations retained for recovery in this process
/// (execution_audit.yaml, Final-observation recovery). While this many are
/// retained the host admits no new invocation; none is ever evicted.
const PENDING_LIMIT: usize = 10_000;
/// The bound on one request's in-line final-append recovery (audit.md § 3).
const APPEND_BUDGET: Duration = Duration::from_millis(250);
/// How often the running host retries its retained final observations.
const RECOVERY_INTERVAL: Duration = Duration::from_secs(1);
/// Retained attempts per instance (mutations.yaml); capacity refuses before
/// the attempt and never evicts.
const ATTEMPTS_PER_INSTANCE: u32 = 100_000;
/// The terminal text of an HTTP-host attempt without its result: a bounded
/// safe observation.
const SETTLEMENT_BYTES: usize = 1024;
/// The largest write result an attempt's settlement retains for keyed replay;
/// a larger one is settled with `result_retained: false`.
pub const RETAINED_RESULT_BYTES: usize = 64 * 1024;
const CANONICALIZATION: &str = "adapter-v1-canonical-json";

/// The facts the host verified for one admitted invocation.
pub struct Admitted<'a> {
    pub request_id: &'a str,
    /// The admitted service principal: [`STATIC_BEARER_PRINCIPAL`], or the
    /// configured service account under `service-account-token`
    /// (compatibility.md § 2.1, Audit anchor).
    pub principal: &'a str,
    pub operation: &'a str,
    pub descriptor_revision: &'a str,
    pub access: audit::Access,
}

/// The facts of one admitted `external_write` the host records an attempt for.
pub struct Write<'a> {
    pub request_id: &'a str,
    /// The admitted service principal, the attempt's caller.
    pub principal: &'a str,
    pub operation: &'a str,
    pub contract: &'a str,
    pub profile: &'a str,
    pub descriptor_revision: &'a str,
    pub input: &'a Value,
}

/// The HTTP host's settlement time (mutations.yaml, Clock): a zero-width
/// system wall-clock sample, which the ledger floors with the mutation
/// `LocalClockFloor`. Record time only: this host runs no key expiry, so no
/// replay window reads it. Standing grants are read against the host's own
/// clock ([`now_ms`]), as its operator's configuration (standing_approvals.yaml).
struct RecordClock;
impl ledger::Clock for RecordClock {
    fn now(&self) -> ledger::Result<ledger::ClockInterval> {
        let now = i64::try_from(connectors_sdk::now_ms())
            .map_err(|_| ledger::Failure::ClockUnavailable)?;
        Ok(ledger::ClockInterval {
            lower_unix_ms: now,
            upper_unix_ms: now,
        })
    }
}

pub struct ServiceState {
    instance: String,
    adapter: String,
    configuration: String,
    path: PathBuf,
    audit: audit::Store,
    ledger: ledger::Store<RecordClock>,
    pending_limit: usize,
    /// Exact final observations whose append was not acknowledged, retained
    /// unchanged for idempotent recovery (audit.md § 3).
    pending: Mutex<Retained>,
    /// Present only when the service configuration sets `standing_grants`:
    /// then, and only then, writes are keyed and need a covering grant.
    standing: Option<Standing>,
    lanes: Lanes,
    flights: Flights,
}

/// A host's standing grants: the operator's grant file, read on every write,
/// and the hourly count of writes admitted under each grant.
struct Standing {
    grants: PathBuf,
    uses: standing::Uses,
}

/// How a keyed write's key stands before any grant is read.
pub enum KeyLookup {
    /// No live reservation: a new attempt may be admitted.
    Miss,
    /// The key names an attempt for this exact request: replay it.
    Replay(ledger::Observation),
}

/// How the grant check of one keyed write ended, before the anchor.
pub enum GrantCheck {
    /// A slot under this grant is taken and counts from now on.
    Admitted(standing::Use),
    /// The grant file could not be read safely (`unavailable`).
    Unreadable(standing::Unreadable),
    /// No grant covers the write (`approval_required`).
    Uncovered(standing::Uncovered),
    /// Every covering grant is at its cap (`rate_limited`).
    AtCap { retry_after_seconds: u64 },
    /// The use store failed (`unavailable`).
    UsesUnavailable,
}

/// How the keyed preparation of an admitted write ended.
pub enum KeyedPreparation {
    /// The attempt is prepared and its grant recorded with it.
    Prepared(ledger::Prepared),
    /// A concurrent request with the same key and input won: replay it.
    Existing(ledger::Observation),
    /// The attempt was prepared but its grant could not be recorded: it was
    /// aborted (`not_attempted`).
    GrantUnrecorded(ledger::AttemptRef),
}

/// The retained final observations: those waiting for a recovery pass and
/// those a pass has taken out and not yet resolved. Both count against the
/// bound (execution_audit.yaml, Final-observation recovery).
#[derive(Default)]
struct Retained {
    waiting: Vec<(audit::Reference, audit::FinalObservation)>,
    in_recovery: usize,
}

impl Retained {
    fn len(&self) -> usize {
        self.waiting.len() + self.in_recovery
    }
}

impl ServiceState {
    /// Host start: opens (creating if absent) the metadata authority in `path`
    /// and records the instance idempotently. Absent, it is recorded at
    /// registry epoch 1; the same adapter and revision change nothing; the same
    /// adapter at another revision moves the instance to it with the next epoch
    /// (`UpgradeServiceConfiguration`); another adapter refuses start.
    pub fn open(path: &Path, instance: &str, adapter: &str, revision: &str) -> Result<Self> {
        Self::open_bounded(path, instance, adapter, revision, PENDING_LIMIT)
    }

    /// [`Self::open`], naming the state directory or ancestor whose
    /// admission refused start, and why. Every other failure is the closed
    /// [`Failure`] `open` returns.
    pub fn open_explained(
        path: &Path,
        instance: &str,
        adapter: &str,
        revision: &str,
    ) -> std::result::Result<Self, fs::Refusal> {
        Self::open_bounded_explained(path, instance, adapter, revision, PENDING_LIMIT)
    }

    fn open_bounded(
        path: &Path,
        instance: &str,
        adapter: &str,
        revision: &str,
        pending_limit: usize,
    ) -> Result<Self> {
        Self::open_bounded_explained(path, instance, adapter, revision, pending_limit)
            .map_err(fs::Refusal::failure)
    }

    fn open_bounded_explained(
        path: &Path,
        instance: &str,
        adapter: &str,
        revision: &str,
        pending_limit: usize,
    ) -> std::result::Result<Self, fs::Refusal> {
        for value in [instance, adapter, revision] {
            if !connectors_core::valid_id(value) {
                return Err(Failure::InvalidConfiguration.into());
            }
        }
        drop(fs::explained_directory(path, true, true)?);
        Self::opened(path, instance, adapter, revision, pending_limit).map_err(fs::Refusal::Failure)
    }

    fn opened(
        path: &Path,
        instance: &str,
        adapter: &str,
        revision: &str,
        pending_limit: usize,
    ) -> Result<Self> {
        drop(Metadata::initialize(path)?);
        record_instance(path, instance, adapter, revision)?;
        Ok(Self {
            instance: instance.to_owned(),
            adapter: adapter.to_owned(),
            configuration: revision.to_owned(),
            path: path.to_owned(),
            audit: audit::Store::new(path, RECORDS_PER_INSTANCE)
                .map_err(|_| Failure::InvalidConfiguration)?,
            ledger: ledger::Store::new(
                path,
                RecordClock,
                ledger::Limits {
                    attempts_per_instance: ATTEMPTS_PER_INSTANCE,
                    result_bytes: SETTLEMENT_BYTES,
                },
            )
            .map_err(|_| Failure::InvalidConfiguration)?,
            pending_limit,
            pending: Mutex::new(Retained::default()),
            standing: None,
            lanes: Lanes::default(),
            flights: Flights::default(),
        })
    }

    /// The service configuration's `standing_grants`. `None` leaves the host
    /// as it was: writes unkeyed, nothing more opened. `Some(path)` makes every
    /// write keyed and admitted only under a grant the file at `path` holds
    /// when the write arrives; it opens the hourly use count
    /// (`standing-grants.sqlite3`) in the state directory, and lets an
    /// attempt's settlement retain its result for replay.
    pub fn with_standing_grants(mut self, grants: Option<PathBuf>) -> Result<Self> {
        let Some(grants) = grants else {
            return Ok(self);
        };
        self.ledger = ledger::Store::new(
            &self.path,
            RecordClock,
            ledger::Limits {
                attempts_per_instance: ATTEMPTS_PER_INSTANCE,
                result_bytes: RETAINED_RESULT_BYTES + SETTLEMENT_BYTES,
            },
        )
        .map_err(|_| Failure::InvalidConfiguration)?;
        self.standing = Some(Standing {
            grants,
            uses: standing::Uses::open(&self.path).map_err(|_| Failure::MetadataUnavailable)?,
        });
        Ok(self)
    }

    /// `true` when the host runs keyed writes under standing grants.
    pub fn keyed(&self) -> bool {
        self.standing.is_some()
    }

    pub fn instance(&self) -> &str {
        &self.instance
    }

    /// Waits for, then holds, the lane of `key` in `principal`'s namespace.
    /// A keyed write holds it from its key lookup until its attempt is
    /// prepared (the key reserved) or the write is refused, so in this
    /// process a second request with the key looks it up only after the
    /// first has reserved it or given up: a duplicate is always a replay
    /// (never `rate_limited` or `approval_required` for a write the first
    /// request was admitted for), and a request with other input is refused
    /// `idempotency_conflict` before its anchor. Lanes do not reach another
    /// process on the same state directory; the ledger's reservation and the
    /// recheck before a refusal remain the guard there.
    pub async fn key_lane(&self, principal: &str, key: &str) -> KeyLane {
        let name = (principal.to_owned(), key.to_owned());
        let lane = locked(&self.lanes).entry(name.clone()).or_default().clone();
        let held = lane.lock_owned().await;
        KeyLane {
            held: Some(held),
            lanes: self.lanes.clone(),
            name,
        }
    }

    /// How a replay may wait for the pending original `attempt`: join the
    /// original this process runs, unless [`REPLAY_WAITERS`] already wait on
    /// it (`Full`); `NotRunHere` when this process is not running it.
    pub fn wait_on(&self, attempt: Uuid) -> Waiting {
        let mut flights = locked(&self.flights);
        let Some(flight) = flights.get_mut(&attempt) else {
            return Waiting::NotRunHere;
        };
        if flight.waiters >= REPLAY_WAITERS {
            return Waiting::Full;
        }
        flight.waiters += 1;
        Waiting::Joined(ReplayWait {
            landed: flight.landed.subscribe(),
            attempt,
            flights: self.flights.clone(),
        })
    }

    /// The replays now waiting on originals this process runs.
    pub fn replays_waiting(&self) -> usize {
        locked(&self.flights)
            .values()
            .map(|flight| flight.waiters)
            .sum()
    }

    /// This process now runs the keyed attempt.
    fn take_off(&self, attempt: Uuid) {
        let (landed, _) = tokio::sync::watch::channel(());
        locked(&self.flights).insert(attempt, Flight { waiters: 0, landed });
    }

    /// This process no longer runs the attempt: its waiters wake.
    pub fn land(&self, attempt: Uuid) {
        locked(&self.flights).remove(&attempt);
    }

    fn keyed_candidate(&self, write: &Write<'_>, key: &str) -> ledger::Candidate {
        let mut candidate = self.candidate(write);
        candidate.caller_key = Some(key.to_owned());
        candidate
    }

    /// Operations § 4 step 4 for a keyed write: the reservation of `key` in
    /// this principal's namespace. `Err(Conflict)` is a live key with another
    /// fingerprint (`idempotency_conflict`); any other error is an unavailable
    /// store.
    pub fn lookup_key(&self, write: &Write<'_>, key: &str) -> ledger::Result<KeyLookup> {
        Ok(
            match self.ledger.lookup(&self.keyed_candidate(write, key))? {
                None => KeyLookup::Miss,
                Some(observation) => KeyLookup::Replay(observation),
            },
        )
    }

    /// The grant check that replaces operations § 4 steps 5 and 8 under a
    /// standing grant: the grant file read now, the grants covering this
    /// write now, and one slot under the first of them with room in its hour.
    pub fn check_grant(&self, write: &Write<'_>) -> GrantCheck {
        // A host without standing grants never reaches this check; were it
        // to, no grant covers anything.
        let Some(held) = &self.standing else {
            return GrantCheck::Uncovered(standing::Uncovered::NoGrant);
        };
        let grants = match standing::read(&held.grants) {
            Err(unreadable) => return GrantCheck::Unreadable(unreadable),
            Ok(None) => return GrantCheck::Uncovered(standing::Uncovered::NoGrant),
            Ok(Some(grants)) => grants,
        };
        let now = now_ms();
        let covering = match grants.covering(&self.instance, write.operation, write.input, now) {
            Ok(covering) => covering,
            Err(reason) => return GrantCheck::Uncovered(reason),
        };
        match held.uses.take(&self.instance, &covering, now) {
            Ok(standing::Taken::Slot(taken)) => GrantCheck::Admitted(taken),
            Ok(standing::Taken::AtCap {
                retry_after_seconds,
            }) => GrantCheck::AtCap {
                retry_after_seconds,
            },
            Err(standing::UsesUnavailable) => GrantCheck::UsesUnavailable,
        }
    }

    /// The slot is not used: no attempt will be prepared for it.
    pub fn release_grant(&self, taken: &standing::Use) {
        let released = self
            .standing
            .as_ref()
            .is_some_and(|held| held.uses.release(taken).is_ok());
        if !released {
            tracing::warn!(grant = %taken.grant_id, "a standing grant use was not released; it counts until its hour ends");
        }
    }

    /// `PrepareAttempt` for an admitted keyed write, reserving `key` in the
    /// same transaction, then the grant recorded with the attempt. A
    /// concurrent winner with the same key and input is returned for replay
    /// and the slot released; an unrecorded grant aborts the attempt.
    pub fn prepare_keyed(
        &self,
        write: &Write<'_>,
        key: &str,
        taken: &standing::Use,
    ) -> ledger::Result<KeyedPreparation> {
        let prepared = match self.ledger.prepare(&self.keyed_candidate(write, key)) {
            Ok(ledger::Preparation::Prepared(prepared)) => prepared,
            Ok(ledger::Preparation::Existing(winner)) => {
                self.release_grant(taken);
                return Ok(KeyedPreparation::Existing(winner));
            }
            Err(failure) => {
                self.release_grant(taken);
                return Err(failure);
            }
        };
        let attempt = prepared.reference();
        let bound = self
            .standing
            .as_ref()
            .is_some_and(|held| held.uses.bind(taken, attempt.attempt_id).is_ok());
        if !bound {
            // Refusal: no write is dispatched without its grant recorded.
            self.abort_attempt(attempt, "unavailable");
            return Ok(KeyedPreparation::GrantUnrecorded(attempt));
        }
        // Replays of the key in this process wait on this attempt until it
        // settles or is aborted ([`wait_on`](Self::wait_on)).
        self.take_off(attempt.attempt_id);
        Ok(KeyedPreparation::Prepared(prepared))
    }

    /// The standing grant recorded with one of this host's attempts; none on
    /// a host without standing grants.
    pub fn grant_of(&self, attempt_id: &str) -> ledger::Result<Option<String>> {
        let attempt_id = Uuid::parse_str(attempt_id).map_err(|_| ledger::Failure::InvalidInput)?;
        let Some(held) = &self.standing else {
            return Ok(None);
        };
        held.uses
            .grant_of(attempt_id)
            .map_err(|_| ledger::Failure::MetadataUnavailable)
    }

    /// The current observation of an attempt this host recorded.
    pub fn observe_attempt(
        &self,
        attempt: ledger::AttemptRef,
    ) -> ledger::Result<ledger::Observation> {
        self.ledger.observe(attempt)
    }

    /// `false` while the retained final observations are at their bound: the
    /// host then admits no new invocation (execution_audit.yaml,
    /// Final-observation recovery).
    pub fn accepting(&self) -> bool {
        self.pending_observations() < self.pending_limit
    }

    /// `AcknowledgeAnchor` for an admitted invocation (audit.md § 1-2). Only a
    /// definite acknowledgement returns the reference; a store that fails, does
    /// not answer or is at capacity returns none and grants no dispatch.
    pub fn anchor(&self, facts: &Admitted<'_>) -> audit::Result<audit::Reference> {
        let anchor = audit::Anchor {
            instance_id: self.instance.clone(),
            kind: audit::Kind::AdmittedExecution,
            activity: Some(audit::Activity::Invoke),
            access: Some(facts.access),
            hop: audit::Hop::Execution,
            stage: audit::Stage::Admission,
            request_id: Some(facts.request_id.to_owned()),
            principal_ref: Some(facts.principal.to_owned()),
            operation_id: Some(facts.operation.to_owned()),
            connection_ref: None,
            descriptor_revision: Some(facts.descriptor_revision.to_owned()),
            recorded_at_ms: now_ms(),
            attempt_id: None,
        };
        match self.audit.anchor(&anchor)? {
            audit::Acknowledgement::Execution(receipt) => Ok(receipt.reference().clone()),
            audit::Acknowledgement::Refusal(_) => Err(audit::Failure::MetadataUnavailable),
        }
    }

    /// `PrepareAttempt` for an admitted write: no connection, no key,
    /// `not_required` approval, fenced by the instance's configuration
    /// revision. Capacity and an unavailable store record nothing.
    pub fn prepare_attempt(&self, write: &Write<'_>) -> ledger::Result<ledger::Prepared> {
        match self.ledger.prepare(&self.candidate(write))? {
            ledger::Preparation::Prepared(prepared) => Ok(prepared),
            // Without a key no existing attempt can answer a preparation.
            ledger::Preparation::Existing(_) => Err(ledger::Failure::Conflict),
        }
    }

    /// The attempt candidate of an admitted write: no connection (the
    /// instance's one configured credential, fenced by its configuration
    /// revision), `not_required` per-input approval (on a host with standing
    /// grants, the grant replaces steps 5 and 8), no key until
    /// [`keyed_candidate`](Self::keyed_candidate).
    fn candidate(&self, write: &Write<'_>) -> ledger::Candidate {
        ledger::Candidate {
            namespace: ledger::Namespace {
                receiver_instance: self.instance.clone(),
                tenant: None,
                realm: None,
                caller: write.principal.to_owned(),
                executor: None,
                origin: ledger::Origin::Direct,
            },
            fingerprint: ledger::Fingerprint {
                operation: ledger::OperationRef {
                    instance: self.instance.clone(),
                    adapter: self.adapter.clone(),
                    operation: write.operation.to_owned(),
                },
                connection_ref: None,
                connection_revision: None,
                contract_ref: write.contract.to_owned(),
                profile: write.profile.to_owned(),
                descriptor_revision: write.descriptor_revision.to_owned(),
                configuration_revision: self.configuration.clone(),
                canonicalization_version: CANONICALIZATION.to_owned(),
                input_digest: connectors_core::digest(write.input),
                route: None,
            },
            caller_key: None,
            request_id: write.request_id.to_owned(),
            approval: ledger::Approval::NotRequired,
        }
    }

    /// `LinkAnchorAttempt`: `true` only when the anchor names the attempt.
    pub fn link_attempt(&self, reference: &audit::Reference, attempt: ledger::AttemptRef) -> bool {
        self.audit.link(reference, attempt.attempt_id).is_ok()
    }

    /// `OpenDispatch`: the one-shot dispatch permission.
    pub fn open_attempt(&self, prepared: ledger::Prepared) -> ledger::Result<ledger::AttemptRef> {
        self.ledger.open_dispatch(prepared)?.consume()
    }

    /// `AbortPrepared`: the attempt was not dispatched (`not_attempted`).
    pub fn abort_attempt(&self, attempt: ledger::AttemptRef, cause: &str) -> bool {
        let aborted = self
            .ledger
            .abort(attempt, settlement("not_attempted", Some(cause)))
            .is_ok();
        self.land(attempt.attempt_id);
        aborted
    }

    /// `RecordCompletion` when `applied`, else `RecordUncertainty` with its
    /// cause. `true` when the ledger acknowledged the settlement.
    pub fn settle_attempt(
        &self,
        attempt: ledger::AttemptRef,
        applied: bool,
        cause: Option<&str>,
    ) -> bool {
        let outcome = if applied {
            ledger::Outcome::Applied(settlement("applied", cause))
        } else {
            ledger::Outcome::Unknown(settlement("unknown", cause))
        };
        self.ledger.settle(attempt, &outcome).is_ok()
    }

    /// [`settle_attempt`](Self::settle_attempt) for a keyed write: an applied
    /// write's `result` (`Some`) is retained for replay when it fits
    /// [`RETAINED_RESULT_BYTES`]; otherwise the settlement says
    /// `result_retained: false`. `None` is not applied.
    pub fn settle_keyed(
        &self,
        attempt: ledger::AttemptRef,
        result: Option<&Value>,
        cause: Option<&str>,
    ) -> bool {
        let outcome = match result {
            Some(result) => {
                let mut settled = settlement("applied", cause);
                if serde_json::to_vec(result)
                    .is_ok_and(|bytes| bytes.len() <= RETAINED_RESULT_BYTES)
                {
                    settled["result"] = result.clone();
                } else {
                    settled["result_retained"] = json!(false);
                }
                ledger::Outcome::Applied(settled)
            }
            None => ledger::Outcome::Unknown(settlement("unknown", cause)),
        };
        let settled = self.ledger.settle(attempt, &outcome).is_ok();
        self.land(attempt.attempt_id);
        settled
    }

    /// `RecordRefusal`: the provider definitely refused the write, so nothing
    /// was performed (`Failed`, classification `refused`). The refusal's
    /// `code` and safe `message` are kept beside the classification, so a
    /// replay of its key answers them again. `true` when the ledger
    /// acknowledged the settlement.
    pub fn settle_refused(&self, attempt: ledger::AttemptRef, code: &str, message: &str) -> bool {
        let mut settled = settlement("refused", Some(code));
        settled["message"] = json!(message);
        let settled = self
            .ledger
            .settle(attempt, &ledger::Outcome::Refused(settled))
            .is_ok();
        self.land(attempt.attempt_id);
        settled
    }

    /// Internal owner read of one of this host's attempts by its public id.
    pub fn attempt(&self, attempt_id: &str) -> ledger::Result<ledger::Observation> {
        let attempt_id = Uuid::parse_str(attempt_id).map_err(|_| ledger::Failure::InvalidInput)?;
        let authority = Metadata::authority_at(&self.path).map_err(ledger_failure)?;
        self.ledger.observe(ledger::AttemptRef {
            authority,
            attempt_id,
        })
    }

    /// The attempts recorded for this host's instance.
    pub fn attempt_count(&self) -> ledger::Result<u32> {
        let metadata = Metadata::inspect(&self.path).map_err(ledger_failure)?;
        metadata
            .connection
            .query_row(
                "SELECT count(*) FROM mutation_attempts WHERE instance_id=?1",
                [&self.instance],
                |row| row.get(0),
            )
            .map_err(|_| ledger::Failure::MetadataUnavailable)
    }

    /// Records the one final observation of an anchored invocation. `true`
    /// when it was acknowledged (`complete`); otherwise the exact observation
    /// is retained for [`recover_observations`](Self::recover_observations)
    /// and the record stays `Anchored` (`incomplete`). A retained observation
    /// is never evicted.
    pub fn finish(
        &self,
        reference: &audit::Reference,
        outcome: audit::Outcome,
        code: Option<String>,
    ) -> bool {
        let observation = audit::FinalObservation {
            observation_id: Uuid::new_v4(),
            outcome,
            code,
            recorded_at_ms: now_ms(),
        };
        let until = Instant::now() + APPEND_BUDGET;
        if self
            .audit
            .append_recovering_with_now(reference, &observation, until, Instant::now)
            .is_ok()
        {
            return true;
        }
        self.retained()
            .waiting
            .push((reference.clone(), observation));
        false
    }

    fn retained(&self) -> std::sync::MutexGuard<'_, Retained> {
        self.pending.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Final observations retained for recovery, including those a running
    /// recovery pass is retrying.
    pub fn pending_observations(&self) -> usize {
        self.retained().len()
    }

    /// Retries each retained final observation with its exact original fields
    /// (audit.md § 3): an acknowledged or already-identical append completes
    /// it, a different final observation leaves the original and is dropped,
    /// and an unavailable store keeps it. Never dispatches anything. An
    /// observation the pass is retrying still counts against the bound until
    /// it is resolved, so a pass never reopens admission by itself. Returns
    /// how many remain.
    pub fn recover_observations(&self) -> usize {
        let taken = {
            let mut retained = self.retained();
            let taken = std::mem::take(&mut retained.waiting);
            retained.in_recovery += taken.len();
            taken
        };
        let mut remaining = Vec::new();
        for (reference, observation) in taken {
            match self.audit.append(&reference, &observation) {
                Ok(_) | Err(audit::Failure::Conflict | audit::Failure::NotFound) => {
                    self.retained().in_recovery -= 1;
                }
                Err(_) => remaining.push((reference, observation)),
            }
        }
        let mut retained = self.retained();
        retained.in_recovery -= remaining.len();
        remaining.append(&mut retained.waiting);
        retained.waiting = remaining;
        retained.len()
    }

    /// The running host's recovery: every [`RECOVERY_INTERVAL`] it retries the
    /// retained final observations, until the state is dropped. Needs a Tokio
    /// runtime; without one nothing is spawned.
    pub fn spawn_recovery(state: &Arc<Self>) {
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            return;
        };
        let state: Weak<Self> = Arc::downgrade(state);
        runtime.spawn(async move {
            loop {
                tokio::time::sleep(RECOVERY_INTERVAL).await;
                let Some(current) = state.upgrade() else {
                    return;
                };
                if current.pending_observations() == 0 {
                    continue;
                }
                let _ = tokio::task::spawn_blocking(move || current.recover_observations()).await;
            }
        });
    }
}

/// The terminal text of an HTTP-host attempt: the canonical JSON of a
/// `connectors.mutations.Observation` without `attempt_id`.
fn settlement(classification: &str, cause: Option<&str>) -> Value {
    match cause {
        Some(cause) => json!({"cause": cause, "classification": classification}),
        None => json!({"classification": classification}),
    }
}

fn record_instance(path: &Path, instance: &str, adapter: &str, revision: &str) -> Result<()> {
    let mut metadata = Metadata::update_audit(path)?;
    let tx = metadata
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| Failure::MetadataUnavailable)?;
    let recorded: Option<(String, String)> = tx
        .query_row(
            "SELECT adapter_id,configuration_revision FROM registry_instances WHERE instance_id=?1",
            [instance],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|_| Failure::MetadataUnavailable)?;
    match recorded {
        None => {
            tx.execute(
                "INSERT INTO registry_instances(instance_id,adapter_id,configuration_revision,epoch) VALUES (?1,?2,?3,1)",
                params![instance, adapter, revision],
            )
            .map_err(|_| Failure::MetadataUnavailable)?;
        }
        Some((recorded, _)) if recorded != adapter => {
            return Err(Failure::InvalidConfiguration);
        }
        Some((_, recorded)) if recorded == revision => return Ok(()),
        Some(_) => {
            tx.execute(
                "UPDATE registry_instances SET configuration_revision=?2,epoch=epoch+1 WHERE instance_id=?1",
                params![instance, revision],
            )
            .map_err(|_| Failure::MetadataUnavailable)?;
        }
    }
    tx.commit().map_err(|_| Failure::OutcomeUnknown)?;
    metadata.persist()
}

/// A ledger read's failed open: a store a newer binary with another record
/// model wrote keeps its name and release (semantics.md §3); any other is
/// `MetadataUnavailable`.
fn ledger_failure(failure: Failure) -> ledger::Failure {
    match failure {
        Failure::MetadataSchemaNewer(store) => ledger::Failure::MetadataSchemaNewer(store),
        _ => ledger::Failure::MetadataUnavailable,
    }
}

fn now_ms() -> i64 {
    i64::try_from(connectors_sdk::now_ms()).unwrap_or(i64::MAX)
}

// Withdraws the store directory's permission through mode bits.
#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn mode(path: &Path, mode: u32) {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
    }

    /// semantics.md §3: the HTTP host's attempt reads name a store a newer
    /// binary with another record model wrote, with its release, as every
    /// other open does; never `metadata_unavailable`.
    #[test]
    fn the_attempt_reads_name_a_store_a_newer_binary_wrote() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("state");
        std::fs::create_dir(&path).unwrap();
        mode(&path, 0o700);
        let state = ServiceState::open(&path, "leaf", "fixture", "rev-1").unwrap();
        Metadata::record_writer_for_tests(&path, "999.0.0", &"0".repeat(64)).unwrap();
        let refused =
            ledger::Failure::MetadataSchemaNewer(Some(super::super::Release::new(999, 0, 0)));
        assert_eq!(state.attempt_count().err(), Some(refused));
        assert_eq!(
            state.attempt(&Uuid::new_v4().to_string()).err(),
            Some(refused)
        );
    }

    /// execution_audit.yaml, Final-observation recovery: a full retained set
    /// refuses new invocations and evicts nothing; recovery reopens it.
    #[test]
    fn a_full_retained_set_refuses_new_work_and_evicts_nothing() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("state");
        std::fs::create_dir(&path).unwrap();
        mode(&path, 0o700);
        let state = ServiceState::open_bounded(&path, "leaf", "fixture", "rev-1", 2).unwrap();
        let admitted = |request_id: &'static str| Admitted {
            request_id,
            principal: STATIC_BEARER_PRINCIPAL,
            operation: "read",
            descriptor_revision: "rev-1",
            access: audit::Access::Read,
        };
        let first = state.anchor(&admitted("one")).unwrap();
        let second = state.anchor(&admitted("two")).unwrap();
        assert!(state.accepting());
        mode(&path, 0o750);
        assert!(!state.finish(&first, audit::Outcome::Success, None));
        assert!(state.accepting());
        assert!(!state.finish(&second, audit::Outcome::Success, None));
        assert!(!state.accepting());
        assert_eq!(state.pending_observations(), 2);
        // An unavailable store keeps both.
        assert_eq!(state.recover_observations(), 2);
        mode(&path, 0o700);
        assert_eq!(state.recover_observations(), 0);
        assert!(state.accepting());
        for reference in [first, second] {
            let record = state.audit.observe(&reference).unwrap().unwrap();
            assert!(record.final_observation.is_some());
        }
    }
}

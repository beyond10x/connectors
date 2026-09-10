//! Host-owned mutation metadata ports. No public request codec, caller policy,
//! approval verifier, audit admission, credential access or provider dispatch.
//! Embedders must supply those independently before using a ledger gate receipt.
mod types;
pub use types::*;
#[cfg(test)]
mod tests;

use super::metadata::Metadata;
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde_json::Value;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const RETENTION_MS: i64 = 86_400_000;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    MetadataUnavailable,
    OutcomeUnknown,
    ClockUnavailable,
    InvalidInput,
    NotFound,
    Conflict,
    Capacity,
    BindingChanged,
}
pub type Result<T> = std::result::Result<T, Failure>;

/// No default unqualified wall clock is provided. Each method closes the
/// SQLite handle before returning, including before any receipt can be used.
pub struct Store<C> {
    path: PathBuf,
    clock: C,
    limits: Limits,
    #[cfg(test)]
    fault: std::sync::atomic::AtomicU8,
}
struct GateBinding {
    instance: String,
    adapter: String,
    configuration: String,
    connection: String,
    revision: String,
    fence: String,
}

impl<C: Clock> Store<C> {
    pub fn new(path: &Path, clock: C, limits: Limits) -> Result<Self> {
        if !(1..=100_000).contains(&limits.attempts_per_instance)
            || !(64..=1_048_576).contains(&limits.result_bytes)
        {
            return Err(Failure::InvalidInput);
        }
        Ok(Self {
            path: path.to_owned(),
            clock,
            limits,
            #[cfg(test)]
            fault: std::sync::atomic::AtomicU8::new(0),
        })
    }

    fn transaction<T>(
        &self,
        migrate: bool,
        action: impl FnOnce(&Transaction<'_>, Uuid) -> Result<T>,
    ) -> Result<T> {
        self.transaction_schema(if migrate { 4 } else { 0 }, action)
    }

    fn transaction_schema<T>(
        &self,
        migrate: u8,
        action: impl FnOnce(&Transaction<'_>, Uuid) -> Result<T>,
    ) -> Result<T> {
        let mut metadata = if migrate == 6 {
            Metadata::update_approvals(&self.path)
        } else if migrate == 4 {
            Metadata::update_mutations(&self.path)
        } else {
            Metadata::update(&self.path, false)
        }
        .map_err(host_error)?;
        let authority = metadata.authority().map_err(host_error)?;
        require_schema(&metadata.connection)?;
        let tx = metadata
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db)?;
        let result = action(&tx, authority)?;
        #[cfg(test)]
        if self.take_fault(1) {
            return Err(Failure::MetadataUnavailable);
        }
        tx.commit().map_err(|_| Failure::OutcomeUnknown)?;
        #[cfg(test)]
        if self.take_fault(2) {
            return Err(Failure::OutcomeUnknown);
        }
        Ok(result)
    }

    /// Atomic reserve-and-prepare. The host has already admitted this candidate.
    /// An existing reservation wins before a new preparation can be created.
    pub fn prepare(&self, candidate: &Candidate) -> Result<Preparation> {
        self.prepare_inner(candidate, None)
    }

    /// Initial verification is bound to this preparation, then rechecked by the
    /// spend owner. Existing-key observations do not recreate approval authority.
    pub fn prepare_approved(
        &self,
        candidate: &Candidate,
        verified: &super::approvals::VerifiedApproval,
    ) -> Result<Preparation> {
        let approval = verified
            .for_candidate(candidate)
            .map_err(|_| Failure::InvalidInput)?;
        self.prepare_inner(candidate, Some(Box::new(approval)))
    }

    fn prepare_inner(
        &self,
        candidate: &Candidate,
        approval: Option<Box<super::approvals::ProofBinding>>,
    ) -> Result<Preparation> {
        let (key, fingerprint) = candidate.encode()?;
        self.transaction_schema(if approval.is_some() { 6 } else { 4 }, |tx, authority| {
            if let Some(key) = &key
                && let Some(existing) = lookup(tx, authority, key, &fingerprint)?
            { return Ok(Preparation::Existing(existing)); }
            let f = &candidate.fingerprint;
            let binding = binding(tx, f)?;
            let count: u32 = tx.query_row("SELECT count(*) FROM mutation_attempts WHERE instance_id=?1", [&f.operation.instance], |r| r.get(0)).map_err(db)?;
            if count >= self.limits.attempts_per_instance { return Err(Failure::Capacity); }
            let reference = AttemptRef { authority, attempt_id: Uuid::new_v4() };
            let nonce = Uuid::new_v4();
            let (approval_mode, approval_ref) = candidate.approval.coordinates();
            tx.execute("INSERT INTO mutation_attempts(attempt_id,instance_id,connection_ref,request_id,fingerprint,approval_mode,approval_ref,owner_nonce,publication_fence,state) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,'prepared')",
                params![reference.attempt_id.to_string(), f.operation.instance, f.connection_ref, candidate.request_id, fingerprint, approval_mode, approval_ref, nonce.to_string(), binding.fence]).map_err(db)?;
            if let Some(approved) = &approval {
                let subject = String::from_utf8(approved.subject.canonical_bytes().map_err(|_| Failure::InvalidInput)?).map_err(|_| Failure::InvalidInput)?;
                tx.execute("UPDATE mutation_attempts SET approval_subject=?2 WHERE attempt_id=?1", params![reference.attempt_id.to_string(), subject]).map_err(db)?;
            }
            #[cfg(test)]
            if self.take_fault(3) { return Err(Failure::MetadataUnavailable); }
            if let Some(key) = key {
                tx.execute("INSERT INTO mutation_keys(reservation_id,namespace_key,attempt_id,fingerprint,state) VALUES (?1,?2,?3,?4,'pending')",
                    params![Uuid::new_v4().to_string(), key, reference.attempt_id.to_string(), fingerprint]).map_err(db)?;
            }
            Ok(Preparation::Prepared(Prepared { reference, nonce, process: std::process::id(), binding, approval, spend_started: std::sync::atomic::AtomicBool::new(false) }))
        })
    }

    /// Read-only and authoritative, but not disclosure authority. In particular
    /// this is also the host's miss/refusal winner-recheck primitive.
    pub fn lookup(&self, candidate: &Candidate) -> Result<Option<Observation>> {
        let (key, fingerprint) = candidate.encode()?;
        let key = key.ok_or(Failure::InvalidInput)?;
        let metadata = Metadata::inspect(&self.path).map_err(host_error)?;
        require_schema(&metadata.connection)?;
        lookup(
            &metadata.connection,
            metadata.authority().map_err(host_error)?,
            &key,
            &fingerprint,
        )
    }

    pub fn observe(&self, reference: AttemptRef) -> Result<Observation> {
        let metadata = Metadata::inspect(&self.path).map_err(host_error)?;
        require_schema(&metadata.connection)?;
        check_authority(reference, metadata.authority().map_err(host_error)?)?;
        observation(&metadata.connection, reference)
    }

    /// Separately acknowledged one-shot CAS. This receipt proves only the
    /// metadata gate; it is not sufficient authority for a provider call.
    pub fn open_dispatch(&self, prepared: Prepared) -> Result<GateWinner> {
        self.dispatch(prepared, None)
    }

    /// Only the original acknowledged spend receipt opens an approved gate.
    /// Other required/event-claim preparations cannot use ordinary dispatch.
    pub fn open_approved_dispatch(
        &self,
        prepared: Prepared,
        spent: super::approvals::SpendReceipt,
    ) -> Result<GateWinner> {
        self.dispatch(prepared, Some(spent))
    }

    fn dispatch(
        &self,
        prepared: Prepared,
        spent: Option<super::approvals::SpendReceipt>,
    ) -> Result<GateWinner> {
        if prepared.process != std::process::id() {
            return Err(Failure::Conflict);
        }
        self.transaction(false, |tx, authority| {
            check_authority(prepared.reference, authority)?;
            check_binding(tx, &prepared.binding)?;
            if let Some(spent) = spent {
                prepared.check_live_approval(tx, authority)?;
                spent.check(tx, &prepared).map_err(|failure| match failure {
                    super::approvals::Failure::MetadataUnavailable => Failure::MetadataUnavailable,
                    _ => Failure::Conflict,
                })?;
            } else {
                let unapproved: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM mutation_attempts WHERE attempt_id=?1 AND approval_mode='not_required')", [prepared.reference.attempt_id.to_string()], |r| r.get(0)).map_err(db)?;
                if !unapproved { return Err(Failure::Conflict); }
            }
            let changed = tx.execute("UPDATE mutation_attempts SET state='dispatching' WHERE attempt_id=?1 AND owner_nonce=?2 AND state='prepared' AND publication_fence=?3",
                params![prepared.reference.attempt_id.to_string(), prepared.nonce.to_string(), prepared.binding.fence]).map_err(db)?;
            if changed != 1 { return Err(Failure::Conflict); }
            Ok(GateWinner { reference: prepared.reference, process: std::process::id() })
        })
    }

    /// Caller must own cancellation of the original, not a duplicate waiter.
    pub fn abort(&self, reference: AttemptRef, safe_result: Value) -> Result<Observation> {
        let bytes = self.result_bytes(&safe_result)?;
        self.transaction(false, |tx, authority| {
            check_authority(reference, authority)?;
            let current = observation(tx, reference)?;
            if current.state != State::Prepared {
                return Err(Failure::Conflict);
            }
            self.terminal(tx, reference, "prepared", "aborted", &bytes, true)
        })
    }

    /// Persist host-selected native outcome evidence. On failure the caller
    /// keeps its definitive live answer; this result describes storage only.
    pub fn settle(&self, reference: AttemptRef, outcome: &Outcome) -> Result<Observation> {
        let (state, value, known) = match outcome {
            Outcome::Applied(v) => ("completed", v, true),
            Outcome::Refused(v) => ("failed", v, true),
            Outcome::Unknown(v) => ("indeterminate", v, false),
        };
        let bytes = self.result_bytes(value)?;
        self.transaction(false, |tx, authority| {
            check_authority(reference, authority)?;
            let current = observation(tx, reference)?;
            if current.state.terminal() {
                return if current.state == State::parse(state)?
                    && current.result.as_ref() == Some(value)
                {
                    Ok(current)
                } else {
                    Err(Failure::Conflict)
                };
            }
            if current.state != State::Dispatching {
                return Err(Failure::Conflict);
            }
            self.terminal(tx, reference, "dispatching", state, &bytes, known)
        })
    }

    /// Never sends or recreates a handle. The coordinator must first establish
    /// recovery ownership; this can race only through the same durable fences.
    pub fn recover(&self, reference: AttemptRef) -> Result<Observation> {
        self.transaction(false, |tx, authority| {
            check_authority(reference, authority)?;
            let current = observation(tx, reference)?;
            match current.state {
                State::Prepared => self.terminal(
                    tx,
                    reference,
                    "prepared",
                    "aborted",
                    r#"{"cause":"recovery_before_dispatch"}"#,
                    true,
                ),
                State::Dispatching => self.terminal(
                    tx,
                    reference,
                    "dispatching",
                    "indeterminate",
                    r#"{"cause":"recovery_without_outcome"}"#,
                    false,
                ),
                _ => Ok(current),
            }
        })
    }

    /// Generation-safe retirement. A replacement never matches an old ID, and
    /// pending or quarantined keys cannot expire at any clock value.
    pub fn expire(
        &self,
        candidate: &Candidate,
        reference: AttemptRef,
        reservation_id: Uuid,
    ) -> Result<bool> {
        let (key, _) = candidate.encode()?;
        let key = key.ok_or(Failure::InvalidInput)?;
        self.transaction(false, |tx, authority| {
            check_authority(reference, authority)?;
            let row: Option<(String, Option<i64>)> = tx.query_row("SELECT state,replay_expires_at_ms FROM mutation_keys WHERE namespace_key=?1 AND reservation_id=?2 AND attempt_id=?3",
                params![key, reservation_id.to_string(), reference.attempt_id.to_string()], |r| Ok((r.get(0)?, r.get(1)?))).optional().map_err(db)?;
            let Some((state, Some(expiry))) = row else { return Ok(false); };
            if state != "replayable" { return Ok(false); }
            // Validate coupled durable state before trusting a retirement row.
            let observed = observation(tx, reference)?;
            if observed.reservation_id != Some(reservation_id) || observed.replay_expires_at_ms != Some(expiry) { return Err(Failure::MetadataUnavailable); }
            if self.time(tx)?.lower_unix_ms < expiry { return Ok(false); }
            let changed = tx.execute("UPDATE mutation_keys SET state='expired' WHERE namespace_key=?1 AND reservation_id=?2 AND attempt_id=?3 AND state='replayable'",
                params![key, reservation_id.to_string(), reference.attempt_id.to_string()]).map_err(db)?;
            Ok(changed == 1)
        })
    }

    fn terminal(
        &self,
        tx: &Transaction<'_>,
        reference: AttemptRef,
        from: &str,
        to: &str,
        result: &str,
        known: bool,
    ) -> Result<Observation> {
        let (settled, expiry) = if known {
            let upper = self.time(tx)?.upper_unix_ms;
            (
                Some(upper),
                Some(
                    upper
                        .checked_add(RETENTION_MS)
                        .ok_or(Failure::ClockUnavailable)?,
                ),
            )
        } else {
            (None, None)
        };
        let changed = tx.execute("UPDATE mutation_attempts SET state=?2,result_json=?3,settled_at_ms=?4 WHERE attempt_id=?1 AND state=?5",
            params![reference.attempt_id.to_string(), to, result, settled, from]).map_err(db)?;
        if changed != 1 {
            return Err(Failure::Conflict);
        }
        tx.execute("UPDATE mutation_keys SET state=?2,settled_at_ms=?3,replay_expires_at_ms=?4 WHERE attempt_id=?1 AND state='pending'",
            params![reference.attempt_id.to_string(), if known { "replayable" } else { "quarantined" }, settled, expiry]).map_err(db)?;
        observation(tx, reference)
    }

    fn time(&self, tx: &Transaction<'_>) -> Result<ClockInterval> {
        let value = self.clock.now().map_err(|_| Failure::ClockUnavailable)?;
        let width = value
            .upper_unix_ms
            .checked_sub(value.lower_unix_ms)
            .ok_or(Failure::ClockUnavailable)?;
        if value.lower_unix_ms < 0 || !(0..=4000).contains(&width) {
            return Err(Failure::ClockUnavailable);
        }
        let previous: i64 = tx
            .query_row(
                "SELECT last_lower_ms FROM mutation_clock WHERE singleton=1",
                [],
                |r| r.get(0),
            )
            .map_err(db)?;
        if value.lower_unix_ms < previous {
            return Err(Failure::ClockUnavailable);
        }
        tx.execute(
            "UPDATE mutation_clock SET last_lower_ms=?1 WHERE singleton=1",
            [value.lower_unix_ms],
        )
        .map_err(db)?;
        Ok(value)
    }
    fn result_bytes(&self, value: &Value) -> Result<String> {
        let bytes = serde_json::to_string(value).map_err(|_| Failure::InvalidInput)?;
        if bytes.len() > self.limits.result_bytes {
            return Err(Failure::Capacity);
        }
        Ok(bytes)
    }
    #[cfg(test)]
    fn take_fault(&self, expected: u8) -> bool {
        self.fault
            .compare_exchange(
                expected,
                0,
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
            )
            .is_ok()
    }
}

fn require_schema(connection: &Connection) -> Result<()> {
    let version: i64 = connection
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(db)?;
    if !(4..=7).contains(&version) {
        return Err(Failure::MetadataUnavailable);
    }
    Ok(())
}
impl Prepared {
    pub(in crate::local) fn claim_spend(&self) -> Result<()> {
        if self.process != std::process::id()
            || self.approval.is_none()
            || self
                .spend_started
                .swap(true, std::sync::atomic::Ordering::SeqCst)
        {
            return Err(Failure::Conflict);
        }
        Ok(())
    }
    pub(in crate::local) fn approval_binding(&self) -> Option<&super::approvals::ProofBinding> {
        self.approval.as_deref()
    }

    pub(in crate::local) fn check_live_approval(
        &self,
        tx: &Transaction<'_>,
        authority: Uuid,
    ) -> Result<()> {
        if self.process != std::process::id() {
            return Err(Failure::Conflict);
        }
        check_authority(self.reference, authority)?;
        check_binding(tx, &self.binding)?;
        let approval = self.approval.as_ref().ok_or(Failure::Conflict)?;
        let subject = String::from_utf8(
            approval
                .subject
                .canonical_bytes()
                .map_err(|_| Failure::Conflict)?,
        )
        .map_err(|_| Failure::Conflict)?;
        let live: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM mutation_attempts WHERE attempt_id=?1 AND owner_nonce=?2 AND state='prepared' AND publication_fence=?3 AND approval_mode='required' AND approval_ref=?4 AND approval_subject=?5)",
            params![self.reference.attempt_id.to_string(), self.nonce.to_string(), self.binding.fence, approval.reference, subject], |r| r.get(0)).map_err(db)?;
        if !live {
            return Err(Failure::Conflict);
        }
        Ok(())
    }
}
fn check_authority(reference: AttemptRef, authority: Uuid) -> Result<()> {
    if reference.authority != authority || reference.attempt_id.is_nil() {
        return Err(Failure::Conflict);
    }
    Ok(())
}
fn binding(tx: &Transaction<'_>, f: &Fingerprint) -> Result<GateBinding> {
    let mut result = GateBinding {
        instance: f.operation.instance.clone(),
        adapter: f.operation.adapter.clone(),
        configuration: f.configuration_revision.clone(),
        connection: f.connection_ref.clone(),
        revision: f.connection_revision.clone(),
        fence: String::new(),
    };
    result.fence = tx
        .query_row(
            "SELECT publication_fence FROM registry_connections WHERE connection_ref=?1",
            [&result.connection],
            |r| r.get(0),
        )
        .optional()
        .map_err(db)?
        .ok_or(Failure::BindingChanged)?;
    check_binding(tx, &result)?;
    Ok(result)
}
fn check_binding(tx: &Transaction<'_>, b: &GateBinding) -> Result<()> {
    let matches: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM registry_connections c JOIN registry_instances i ON i.instance_id=c.instance_id WHERE c.connection_ref=?1 AND c.instance_id=?2 AND c.semantic_revision=?3 AND c.publication_fence=?4 AND c.state='live' AND c.public=1 AND i.adapter_id=?5 AND i.configuration_revision=?6)",
        params![b.connection,b.instance,b.revision,b.fence,b.adapter,b.configuration], |r| r.get(0)).map_err(db)?;
    if matches {
        Ok(())
    } else {
        Err(Failure::BindingChanged)
    }
}
fn lookup(
    connection: &Connection,
    authority: Uuid,
    key: &str,
    fingerprint: &str,
) -> Result<Option<Observation>> {
    let row: Option<(String, String)> = connection.query_row("SELECT attempt_id,fingerprint FROM mutation_keys WHERE namespace_key=?1 AND state!='expired'", [key], |r| Ok((r.get(0)?, r.get(1)?))).optional().map_err(db)?;
    let Some((attempt, original)) = row else {
        return Ok(None);
    };
    if original != fingerprint {
        return Err(Failure::Conflict);
    }
    let reference = AttemptRef {
        authority,
        attempt_id: Uuid::parse_str(&attempt).map_err(|_| Failure::MetadataUnavailable)?,
    };
    observation(connection, reference).map(Some)
}
fn observation(connection: &Connection, reference: AttemptRef) -> Result<Observation> {
    let mut stmt = connection.prepare("SELECT a.request_id,a.state,a.result_json,a.settled_at_ms,k.reservation_id,k.state,k.settled_at_ms,k.replay_expires_at_ms,a.fingerprint,k.fingerprint FROM mutation_attempts a LEFT JOIN mutation_keys k ON k.attempt_id=a.attempt_id WHERE a.attempt_id=?1").map_err(db)?;
    let mut rows = stmt.query([reference.attempt_id.to_string()]).map_err(db)?;
    let row = rows.next().map_err(db)?.ok_or(Failure::NotFound)?;
    let state = State::parse(&row.get::<_, String>(1).map_err(db)?)?;
    let result: Option<String> = row.get(2).map_err(db)?;
    let settled: Option<i64> = row.get(3).map_err(db)?;
    let key_id: Option<String> = row.get(4).map_err(db)?;
    if let Some(key_state) = row.get::<_, Option<String>>(5).map_err(db)? {
        let expected = match state {
            State::Prepared | State::Dispatching => "pending",
            State::Indeterminate => "quarantined",
            _ => "replayable",
        };
        let key_settled: Option<i64> = row.get(6).map_err(db)?;
        let expires: Option<i64> = row.get(7).map_err(db)?;
        if (key_state != expected && !(expected == "replayable" && key_state == "expired"))
            || settled != key_settled
            || expires != settled.and_then(|v| v.checked_add(RETENTION_MS))
            || row.get::<_, String>(8).map_err(db)? != row.get::<_, String>(9).map_err(db)?
        {
            return Err(Failure::MetadataUnavailable);
        }
    }
    if state.terminal() != result.is_some() {
        return Err(Failure::MetadataUnavailable);
    }
    let result = result
        .map(|v| {
            if v.len() > 1_048_576 {
                return Err(Failure::MetadataUnavailable);
            }
            connectors_core::read_json(v.as_bytes()).map_err(|_| Failure::MetadataUnavailable)
        })
        .transpose()?;
    Ok(Observation {
        reference,
        request_id: row.get(0).map_err(db)?,
        state,
        result,
        reservation_id: key_id
            .map(|v| Uuid::parse_str(&v).map_err(|_| Failure::MetadataUnavailable))
            .transpose()?,
        settled_at_ms: settled,
        replay_expires_at_ms: row.get(7).map_err(db)?,
    })
}
fn db(_: rusqlite::Error) -> Failure {
    Failure::MetadataUnavailable
}
fn host_error(error: super::Failure) -> Failure {
    match error {
        super::Failure::OutcomeUnknown => Failure::OutcomeUnknown,
        _ => Failure::MetadataUnavailable,
    }
}

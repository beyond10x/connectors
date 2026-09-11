use super::types::ProofBinding;
use super::*;
use crate::local::{
    metadata::Metadata,
    mutations::{AttemptRef, Clock, Prepared},
};
use rusqlite::{Transaction, TransactionBehavior, params};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Private authority binding. No read API reconstructs a spend receipt; every
/// ingress of the same logical leaf must use this one retained authority.
pub struct Store<C> {
    path: PathBuf,
    clock: C,
    capacity_per_instance: u32,
    #[cfg(test)]
    pub(super) fault: std::sync::atomic::AtomicU8,
}
/// Only a definite, original acknowledgement creates this process-local value.
/// Abort never refunds it; recovery cannot reconstruct it from its durable row.
pub struct SpendReceipt {
    receipt_id: Uuid,
    attempt: AttemptRef,
    process: u32,
    binding: ProofBinding,
}
impl<C: Clock> Store<C> {
    pub fn new(path: &Path, clock: C, capacity_per_instance: u32) -> Result<Self> {
        if !(1..=100_000).contains(&capacity_per_instance) {
            return Err(Failure::Refused);
        }
        Ok(Self {
            path: path.to_owned(),
            clock,
            capacity_per_instance,
            #[cfg(test)]
            fault: std::sync::atomic::AtomicU8::new(0),
        })
    }

    /// One original spend attempt per live preparation. Any failure requires
    /// coordinator abort/recovery, never a retry which could manufacture an
    /// acknowledgement after an ambiguous original decision.
    pub fn spend(
        &self,
        prepared: &Prepared,
        evidence: &Evidence,
        expected: &Subject,
        policy: &impl ReceiverPolicy,
    ) -> Result<SpendReceipt> {
        prepared.claim_spend().map_err(|_| Failure::Refused)?;
        let decoded = proof::decode(evidence)?;
        let mut metadata = Metadata::update(&self.path, false).map_err(host_error)?;
        let version: i64 = metadata
            .connection
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(db)?;
        if !(6..=8).contains(&version) {
            return Err(Failure::MetadataUnavailable);
        }
        let authority = metadata.authority().map_err(host_error)?;
        let tx = metadata
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db)?;
        prepared
            .check_live_approval(&tx, authority)
            .map_err(|failure| match failure {
                crate::local::mutations::Failure::MetadataUnavailable => {
                    Failure::MetadataUnavailable
                }
                crate::local::mutations::Failure::OutcomeUnknown => Failure::OutcomeUnknown,
                _ => Failure::Refused,
            })?;
        // Lock ordering: local metadata first, bounded consumer policy second.
        // Keep this guard alive explicitly THROUGH transaction acknowledgement.
        let guard = policy.admit(expected, decoded.kid())?;
        let time = proof::now(&self.clock)?;
        let binding = decoded.verify(expected, guard.key(), time)?;
        if prepared.approval_binding() != Some(&binding) {
            return Err(Failure::Refused);
        }
        let duplicate: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM approval_redemptions WHERE (issuer=?1 AND reference=?2) OR attempt_id=?3)",
            params![binding.issuer, binding.reference, prepared.reference().attempt_id.to_string()], |r| r.get(0)).map_err(db)?;
        if duplicate {
            return Err(Failure::Replayed);
        }
        let instance = &binding.subject.target.instance;
        let count: u32 = tx
            .query_row(
                "SELECT count(*) FROM approval_redemptions WHERE instance_id=?1",
                [instance],
                |r| r.get(0),
            )
            .map_err(db)?;
        if count >= self.capacity_per_instance {
            return Err(Failure::Capacity);
        }
        let subject =
            String::from_utf8(binding.subject.canonical_bytes()?).map_err(|_| Failure::Refused)?;
        let receipt_id = Uuid::new_v4();
        tx.execute("INSERT INTO approval_redemptions(receipt_id,issuer,reference,instance_id,attempt_id,subject,spent_at_ms) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![receipt_id.to_string(), binding.issuer, binding.reference, instance, prepared.reference().attempt_id.to_string(), subject, time.upper_unix_ms]).map_err(db)?;
        #[cfg(test)]
        if self.take_fault(1) {
            return Err(Failure::MetadataUnavailable);
        }
        // No stale pre-storage clock observation grants a suspended decision.
        decoded.verify(expected, guard.key(), proof::now(&self.clock)?)?;
        #[cfg(test)]
        if self.take_fault(7) {
            std::process::exit(73);
        }
        tx.commit().map_err(|_| Failure::OutcomeUnknown)?;
        #[cfg(test)]
        if self.take_fault(8) {
            std::process::exit(73);
        }
        #[cfg(test)]
        if self.take_fault(2) {
            return Err(Failure::OutcomeUnknown);
        }
        // Expiry or lost clock certainty during commit leaves the row spent,
        // but supplies no live receipt. The original preparation stays fenced.
        decoded.verify(expected, guard.key(), proof::now(&self.clock)?)?;
        let receipt = SpendReceipt {
            receipt_id,
            attempt: prepared.reference(),
            process: std::process::id(),
            binding,
        };
        drop(guard);
        Ok(receipt)
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
impl SpendReceipt {
    pub(in crate::local) fn check(self, tx: &Transaction<'_>, prepared: &Prepared) -> Result<()> {
        if self.process != std::process::id()
            || self.attempt != prepared.reference()
            || prepared.approval_binding() != Some(&self.binding)
        {
            return Err(Failure::Refused);
        }
        let subject = String::from_utf8(self.binding.subject.canonical_bytes()?)
            .map_err(|_| Failure::Refused)?;
        let found: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM approval_redemptions WHERE receipt_id=?1 AND attempt_id=?2 AND issuer=?3 AND reference=?4 AND subject=?5 AND instance_id=?6)",
            params![self.receipt_id.to_string(), self.attempt.attempt_id.to_string(), self.binding.issuer, self.binding.reference, subject, self.binding.subject.target.instance], |r| r.get(0)).map_err(db)?;
        if !found {
            return Err(Failure::Refused);
        }
        Ok(())
    }
}
fn db(_: rusqlite::Error) -> Failure {
    Failure::MetadataUnavailable
}
fn host_error(value: crate::local::Failure) -> Failure {
    match value {
        crate::local::Failure::OutcomeUnknown => Failure::OutcomeUnknown,
        _ => Failure::MetadataUnavailable,
    }
}

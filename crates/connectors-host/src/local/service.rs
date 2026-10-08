//! The HTTP service host's local state (contracts/service/compatibility.md
//! § 2.1, Service configuration and Host start): the recorded
//! `connectors.declarations.ServiceConfiguration` of its instance and the
//! execution-audit anchors of `POST /v1alpha2/invoke`, in the local metadata
//! authority in the configured `state` directory.
//!
//! An acknowledged anchor is one prerequisite for dispatch, never caller,
//! credential or dispatch authority by itself (contracts/service/audit.md § 1).
use super::{Failure, Result, audit, filesystem as fs, metadata::Metadata};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use std::{
    path::Path,
    sync::Mutex,
    time::{Duration, Instant},
};
use uuid::Uuid;

/// The static-bearer service principal (compatibility.md § 2.1, Audit anchor):
/// one receiver-configured principal, qualified by the anchor's instance, that
/// does not change when the credential bytes rotate.
pub const STATIC_BEARER_PRINCIPAL: &str = "static-bearer";
/// Retained records per instance (audit.md § 4); capacity refuses, never evicts.
const RECORDS_PER_INSTANCE: u32 = 100_000;
/// Final observations kept for recovery in this process. Past it, the oldest
/// stays `Anchored` (answered `incomplete`) and is no longer retried here.
const PENDING_LIMIT: usize = 10_000;
/// The bound on one request's in-line final-append recovery (audit.md § 3).
const APPEND_BUDGET: Duration = Duration::from_millis(250);

/// The facts the host verified for one admitted invocation.
pub struct Admitted<'a> {
    pub request_id: &'a str,
    pub operation: &'a str,
    pub descriptor_revision: &'a str,
    pub access: audit::Access,
}

pub struct ServiceState {
    instance: String,
    audit: audit::Store,
    /// Exact final observations whose append was not acknowledged, retained
    /// unchanged for idempotent recovery (audit.md § 3).
    pending: Mutex<Vec<(audit::Reference, audit::FinalObservation)>>,
}

impl ServiceState {
    /// Host start: opens (creating if absent) the metadata authority in `path`
    /// and records the instance idempotently. Absent, it is recorded at
    /// registry epoch 1; the same adapter and revision change nothing; the same
    /// adapter at another revision moves the instance to it with the next epoch
    /// (`UpgradeServiceConfiguration`); another adapter refuses start.
    pub fn open(path: &Path, instance: &str, adapter: &str, revision: &str) -> Result<Self> {
        for value in [instance, adapter, revision] {
            if !connectors_core::valid_id(value) {
                return Err(Failure::InvalidConfiguration);
            }
        }
        drop(fs::directory(path, true, true)?);
        drop(Metadata::initialize(path)?);
        record_instance(path, instance, adapter, revision)?;
        Ok(Self {
            instance: instance.to_owned(),
            audit: audit::Store::new(path, RECORDS_PER_INSTANCE)
                .map_err(|_| Failure::InvalidConfiguration)?,
            pending: Mutex::new(Vec::new()),
        })
    }

    pub fn instance(&self) -> &str {
        &self.instance
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
            principal_ref: Some(STATIC_BEARER_PRINCIPAL.to_owned()),
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

    /// Records the one final observation of an anchored invocation. `true`
    /// when it was acknowledged (`complete`); otherwise the exact observation
    /// is retained for [`recover_observations`](Self::recover_observations)
    /// and the record stays `Anchored` (`incomplete`).
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
        let mut pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        if pending.len() >= PENDING_LIMIT {
            pending.remove(0);
        }
        pending.push((reference.clone(), observation));
        false
    }

    /// Final observations retained for recovery.
    pub fn pending_observations(&self) -> usize {
        self.pending.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    /// Retries each retained final observation with its exact original fields
    /// (audit.md § 3): an acknowledged or already-identical append completes
    /// it, a different final observation leaves the original and is dropped,
    /// and an unavailable store keeps it. Never dispatches anything. Returns
    /// how many remain.
    pub fn recover_observations(&self) -> usize {
        let retained = std::mem::take(&mut *self.pending.lock().unwrap_or_else(|e| e.into_inner()));
        let mut remaining = Vec::new();
        for (reference, observation) in retained {
            match self.audit.append(&reference, &observation) {
                Ok(_) | Err(audit::Failure::Conflict | audit::Failure::NotFound) => {}
                Err(_) => remaining.push((reference, observation)),
            }
        }
        let mut pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        remaining.append(&mut pending);
        *pending = remaining;
        pending.len()
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

fn now_ms() -> i64 {
    i64::try_from(connectors_sdk::now_ms()).unwrap_or(i64::MAX)
}

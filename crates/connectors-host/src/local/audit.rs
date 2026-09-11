//! Private host execution-audit port. An acknowledged audit is one prerequisite
//! for governed execution, never caller, approval, credential or dispatch authority.
mod types;
pub use types::*;
#[cfg(test)]
pub(crate) mod tests;

use super::metadata::Metadata;
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    InvalidInput,
    MetadataUnavailable,
    OutcomeUnknown,
    NotFound,
    Conflict,
    Capacity,
}
pub type Result<T> = std::result::Result<T, Failure>;

pub struct Store {
    path: PathBuf,
    capacity_per_instance: u32,
    #[cfg(test)]
    pub(crate) fault: std::sync::atomic::AtomicU8,
    #[cfg(test)]
    appends: std::sync::Mutex<Vec<FinalObservation>>,
    #[cfg(test)]
    reads: std::sync::atomic::AtomicUsize,
}
impl Store {
    /// Capacity may be lowered by the operator, never raised above the selected
    /// 100,000-record binding. Existing records survive lower limits.
    pub fn new(path: &Path, capacity_per_instance: u32) -> Result<Self> {
        if !(1..=100_000).contains(&capacity_per_instance) {
            return Err(Failure::InvalidInput);
        }
        Ok(Self {
            path: path.to_owned(),
            capacity_per_instance,
            #[cfg(test)]
            fault: std::sync::atomic::AtomicU8::new(0),
            #[cfg(test)]
            appends: std::sync::Mutex::new(Vec::new()),
            #[cfg(test)]
            reads: std::sync::atomic::AtomicUsize::new(0),
        })
    }

    fn transaction<T>(
        &self,
        migrate: bool,
        action: impl FnOnce(&Transaction<'_>, Uuid) -> Result<T>,
    ) -> Result<T> {
        let mut metadata = if migrate {
            Metadata::update_audit(&self.path)
        } else {
            Metadata::update(&self.path, false)
        }
        .map_err(host_error)?;
        require_schema(&metadata.connection)?;
        let authority = metadata.authority().map_err(host_error)?;
        let tx = metadata
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db)?;
        let result = action(&tx, authority)?;
        #[cfg(test)]
        if self.take_fault(1) {
            return Err(Failure::MetadataUnavailable);
        }
        #[cfg(test)]
        if self.take_fault(7) {
            std::process::exit(73);
        }
        tx.commit().map_err(|_| Failure::OutcomeUnknown)?;
        #[cfg(test)]
        if self.take_fault(2) {
            return Err(Failure::OutcomeUnknown);
        }
        #[cfg(test)]
        if self.take_fault(8) {
            std::process::exit(73);
        }
        Ok(result)
    }

    /// The host supplies verified facts and this owner allocates the opaque ref.
    /// Failed or ambiguous acknowledgement returns no receipt or public ref.
    pub fn anchor(&self, anchor: &Anchor) -> Result<Acknowledgement> {
        self.insert(anchor, Uuid::new_v4().to_string())
    }

    fn insert(&self, anchor: &Anchor, audit_ref: String) -> Result<Acknowledgement> {
        let record = Record {
            reference: Reference {
                instance: anchor.instance_id.clone(),
                audit_ref,
            },
            anchor: anchor.clone(),
            final_observation: None,
        };
        let bytes = record.encode()?;
        let key = record.reference.encode_private()?;
        self.transaction(true, |tx, authority| {
            references(tx, anchor)?;
            if load(tx, &record.reference)?.is_some() { return Err(Failure::Conflict); }
            let count: u32 = tx.query_row("SELECT count(*) FROM execution_audits WHERE instance_id=?1", [&anchor.instance_id], |r| r.get(0)).map_err(db)?;
            if count >= self.capacity_per_instance { return Err(Failure::Capacity); }
            tx.execute("INSERT INTO execution_audits(audit_record_ref,instance_id,audit_ref,connection_ref,attempt_id,record_json) VALUES (?1,?2,?3,?4,?5,?6)", params![key, anchor.instance_id, record.reference.audit_ref, anchor.connection_ref, anchor.attempt_id.map(|id| id.to_string()), bytes]).map_err(db)?;
            Ok(match anchor.kind {
                Kind::AdmittedExecution => Acknowledgement::Execution(Admission { record: Box::new(record), authority, process: std::process::id() }),
                Kind::EarlyRefusal => Acknowledgement::Refusal(record.reference),
            })
        })
    }

    /// Consume the original acknowledged receipt, checking its exact facts and
    /// metadata authority. This proves audit only; it does not open a mutation
    /// gate or perform provider work. No read or recovery creates this receipt.
    pub fn confirm(&self, receipt: Admission, expected: &Anchor) -> Result<Reference> {
        if receipt.process != std::process::id() || &receipt.record.anchor != expected {
            return Err(Failure::Conflict);
        }
        let metadata = Metadata::inspect(&self.path).map_err(host_error)?;
        require_schema(&metadata.connection)?;
        if metadata.authority().map_err(host_error)? != receipt.authority {
            return Err(Failure::Conflict);
        }
        let current =
            load(&metadata.connection, &receipt.record.reference)?.ok_or(Failure::NotFound)?;
        if current != *receipt.record {
            return Err(Failure::Conflict);
        }
        Ok(receipt.record.reference)
    }

    /// Internal exact-pair resolution. None means definite absence in an
    /// admitted schema; unavailable state is an error, never an empty history.
    pub fn observe(&self, reference: &Reference) -> Result<Option<Record>> {
        #[cfg(test)]
        self.reads.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        reference.encode_private()?;
        let metadata = Metadata::inspect(&self.path).map_err(host_error)?;
        require_schema(&metadata.connection)?;
        load(&metadata.connection, reference)
    }

    /// Separate final append, borrowing the owner's retained exact observation.
    /// Storage failure cannot replace the caller's known live business result.
    pub fn append(
        &self,
        reference: &Reference,
        final_observation: &FinalObservation,
    ) -> Result<Record> {
        #[cfg(test)]
        self.appends.lock().unwrap().push(final_observation.clone());
        reference.encode_private()?;
        final_observation.validate()?;
        self.transaction(false, |tx, _| {
            let mut record = load(tx, reference)?.ok_or(Failure::NotFound)?;
            if let Some(existing) = &record.final_observation {
                return if existing == final_observation {
                    Ok(record)
                } else {
                    Err(Failure::Conflict)
                };
            }
            record.final_observation = Some(final_observation.clone());
            let encoded = record.encode()?;
            let changed = tx
                .execute(
                    "UPDATE execution_audits SET record_json=?2 WHERE audit_record_ref=?1",
                    params![reference.encode_private()?, encoded],
                )
                .map_err(db)?;
            if changed != 1 {
                return Err(Failure::MetadataUnavailable);
            }
            Ok(record)
        })
    }

    /// Recover acknowledgement of this exact live observation only. No read can
    /// reconstruct admission or substitute another observation after a crash.
    /// Extra calls check both deadlines; each retains the metadata port's wait.
    pub(crate) fn append_recovering(
        &self,
        reference: &Reference,
        observation: &FinalObservation,
        until: Instant,
    ) -> Result<Record> {
        let first_error = match self.append(reference, observation) {
            Ok(record) => return Ok(record),
            Err(error) => error,
        };
        let until = until.min(Instant::now() + Duration::from_millis(250));
        if Instant::now() >= until {
            return Err(first_error);
        }
        if let Some(record) = self.matching_final(reference, observation)? {
            return Ok(record);
        }
        if Instant::now() >= until {
            return Err(first_error);
        }
        // Definite absence of a final observation permits one exact append
        // retry. It does not permit a new UUID, timestamp or business operation.
        match self.append(reference, observation) {
            Ok(record) => Ok(record),
            Err(error) => {
                if Instant::now() >= until {
                    return Err(error);
                }
                self.matching_final(reference, observation)?.ok_or(error)
            }
        }
    }

    fn matching_final(
        &self,
        reference: &Reference,
        observation: &FinalObservation,
    ) -> Result<Option<Record>> {
        let record = self.observe(reference)?.ok_or(Failure::NotFound)?;
        match &record.final_observation {
            Some(retained) if retained == observation => Ok(Some(record)),
            Some(_) => Err(Failure::Conflict),
            None => Ok(None),
        }
    }

    #[cfg(test)]
    fn take_fault(&self, expected: u8) -> bool {
        // Two-call scripts for bounded final-append recovery: first roll back,
        // then lose the commit acknowledgement (9) or roll back again (10).
        if expected == 1 {
            for (script, next) in [(9, 2), (10, 1)] {
                if self
                    .fault
                    .compare_exchange(
                        script,
                        next,
                        std::sync::atomic::Ordering::SeqCst,
                        std::sync::atomic::Ordering::SeqCst,
                    )
                    .is_ok()
                {
                    return true;
                }
            }
        }
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
    if !(5..=8).contains(&version) {
        return Err(Failure::MetadataUnavailable);
    }
    Ok(())
}
fn references(connection: &Connection, anchor: &Anchor) -> Result<()> {
    let instance: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM registry_instances WHERE instance_id=?1)",
            [&anchor.instance_id],
            |r| r.get(0),
        )
        .map_err(db)?;
    if !instance {
        return Err(Failure::InvalidInput);
    }
    if let Some(selected) = &anchor.connection_ref {
        let same: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM registry_connections WHERE connection_ref=?1 AND instance_id=?2)", params![selected, anchor.instance_id], |r| r.get(0)).map_err(db)?;
        if !same {
            return Err(Failure::InvalidInput);
        }
    }
    if let Some(attempt) = anchor.attempt_id {
        let selected: Option<(String, String)> = connection
            .query_row(
                "SELECT instance_id,connection_ref FROM mutation_attempts WHERE attempt_id=?1",
                [attempt.to_string()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(db)?;
        let Some((instance, selected)) = selected else {
            return Err(Failure::InvalidInput);
        };
        if instance != anchor.instance_id
            || anchor
                .connection_ref
                .as_ref()
                .is_some_and(|value| value != &selected)
        {
            return Err(Failure::InvalidInput);
        }
    }
    Ok(())
}
fn load(connection: &Connection, reference: &Reference) -> Result<Option<Record>> {
    let key = reference.encode_private()?;
    let mut query = connection.prepare("SELECT audit_record_ref,instance_id,audit_ref,connection_ref,attempt_id,record_json FROM execution_audits WHERE instance_id=?1 AND audit_ref=?2").map_err(db)?;
    let mut rows = query
        .query(params![reference.instance, reference.audit_ref])
        .map_err(db)?;
    let Some(row) = rows.next().map_err(db)? else {
        return Ok(None);
    };
    let encoded: String = row.get(5).map_err(db)?;
    if encoded.len() + key.len() > 4096 {
        return Err(Failure::MetadataUnavailable);
    }
    let record: Record =
        connectors_core::read_json(encoded.as_bytes()).map_err(|_| Failure::MetadataUnavailable)?;
    if row.get::<_, String>(0).map_err(db)? != key
        || row.get::<_, String>(1).map_err(db)? != reference.instance
        || row.get::<_, String>(2).map_err(db)? != reference.audit_ref
        || row.get::<_, Option<String>>(3).map_err(db)? != record.anchor.connection_ref
        || row.get::<_, Option<String>>(4).map_err(db)?
            != record.anchor.attempt_id.map(|id| id.to_string())
        || &record.reference != reference
        || record.encode().map_err(|_| Failure::MetadataUnavailable)? != encoded
        || rows.next().map_err(db)?.is_some()
    {
        return Err(Failure::MetadataUnavailable);
    }
    references(connection, &record.anchor).map_err(|_| Failure::MetadataUnavailable)?;
    Ok(Some(record))
}
fn db(_: rusqlite::Error) -> Failure {
    Failure::MetadataUnavailable
}
fn host_error(value: super::Failure) -> Failure {
    match value {
        super::Failure::OutcomeUnknown => Failure::OutcomeUnknown,
        _ => Failure::MetadataUnavailable,
    }
}

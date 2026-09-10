//! Retained local approval-policy metadata and current-use exclusion.
//!
//! This is the storage port, not caller authentication or operation admission.
//! The host coordinator must select current required-approval write operations
//! from admitted metadata before `set_admitted`, and independently check caller,
//! target, key and time before using a lease. Adapters never receive this store.
use super::{approvals::Snapshot, filesystem as fs, metadata::Metadata};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    fs::File,
    os::{fd::AsRawFd, unix::fs::MetadataExt},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use uuid::Uuid;

const LOCK: &str = "approval-policy.lock";
const MAX_REVISION: i64 = 9_007_199_254_740_991;
const MAX_OPERATIONS: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    InvalidInput,
    Conflict,
    NotFound,
    NotAdmitted,
    MetadataUnavailable,
    OutcomeUnknown,
    Capacity,
}
pub type Result<T> = std::result::Result<T, Failure>;
impl From<super::Failure> for Failure {
    fn from(value: super::Failure) -> Self {
        match value {
            super::Failure::OutcomeUnknown => Self::OutcomeUnknown,
            _ => Self::MetadataUnavailable,
        }
    }
}
fn db(_: rusqlite::Error) -> Failure {
    Failure::MetadataUnavailable
}
fn identifier(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256 && connectors_core::valid_id(value)
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub configuration_revision: String,
    pub descriptor_revision: String,
    pub executable_selection: String,
    pub clock_configuration_sha256: String,
}
impl Selection {
    fn validate(&self) -> Result<()> {
        if identifier(&self.configuration_revision)
            && identifier(&self.descriptor_revision)
            && digest(&self.executable_selection)
            && digest(&self.clock_configuration_sha256)
        {
            Ok(())
        } else {
            Err(Failure::InvalidInput)
        }
    }
}

/// Public observation. Cloning it does not retain a current-use lease.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct View {
    pub policy_id: Uuid,
    pub instance_id: String,
    pub issuer_id: Uuid,
    pub revision: i64,
    pub owner_uid: u32,
    pub selection: Selection,
    pub operations: Vec<String>,
}
impl View {
    pub fn snapshot(&self) -> Result<Snapshot> {
        self.validate()?;
        Ok(Snapshot {
            id: format!(
                "connectors.local-policy/{}/{}",
                self.policy_id, self.revision
            ),
            sha256: connectors_core::digest(
                &serde_json::to_value(self).map_err(|_| Failure::InvalidInput)?,
            ),
        })
    }
    pub fn principal(&self) -> String {
        format!(
            "connectors.local-owner/{}/{}",
            self.issuer_id, self.owner_uid
        )
    }
    fn validate(&self) -> Result<()> {
        self.selection.validate()?;
        if self.policy_id.is_nil()
            || self.issuer_id.is_nil()
            || !identifier(&self.instance_id)
            || !(1..=MAX_REVISION).contains(&self.revision)
            || self.owner_uid != fs::uid()
            || self.operations.len() > MAX_OPERATIONS
            || self.operations.iter().any(|op| !identifier(op))
            || self.operations.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(Failure::InvalidInput);
        }
        Ok(())
    }
}

pub struct Store {
    path: PathBuf,
    instance: String,
    adapter: String,
}
impl Store {
    /// The host supplies the metadata path and configured identities. They are
    /// checked against retained registry and issuer bindings before publication.
    pub fn new(path: &Path, instance: &str, adapter: &str) -> Result<Self> {
        if !identifier(instance) || !identifier(adapter) {
            return Err(Failure::InvalidInput);
        }
        Ok(Self {
            path: path.into(),
            instance: instance.into(),
            adapter: adapter.into(),
        })
    }

    /// Passive: no migration, lease creation, custody, clock or provider access.
    pub fn status(&self) -> Result<Option<View>> {
        let metadata = Metadata::inspect(&self.path)?;
        self.load(&metadata.connection)
    }

    /// Publish an already admitted operation set. The coordinator, not this
    /// metadata port, checks that every operation is a supported approval-gated
    /// write under current native metadata and owner permissions.
    pub fn set_admitted(
        &self,
        selection: &Selection,
        operations: Vec<String>,
        expected_revision: Option<i64>,
    ) -> Result<View> {
        self.publish(selection, operations, expected_revision, || Ok(()))
    }

    fn publish(
        &self,
        selection: &Selection,
        mut operations: Vec<String>,
        expected_revision: Option<i64>,
        acknowledge: impl FnOnce() -> Result<()>,
    ) -> Result<View> {
        selection.validate()?;
        if operations.len() > MAX_OPERATIONS
            || operations.iter().any(|op| !identifier(op))
            || expected_revision.is_some_and(|r| !(1..=MAX_REVISION).contains(&r))
        {
            return Err(Failure::InvalidInput);
        }
        operations.sort();
        if operations.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(Failure::InvalidInput);
        }
        // Reject a missing issuer/changed retained binding without installing a
        // policy schema or a lock. Repeat admission inside the publication tx.
        {
            let metadata = Metadata::inspect(&self.path)?;
            self.issuer(&metadata.connection, Some(selection))?;
        }
        let lease = Lease::acquire(&self.path, true)?;
        let mut metadata = Metadata::update_approval_policy(&self.path)?;
        let tx = metadata.connection.transaction().map_err(db)?;
        let issuer = self.issuer(&tx, Some(selection))?;
        let old = self.load(&tx)?;
        if old.as_ref().map(|p| p.revision) != expected_revision {
            return Err(Failure::Conflict);
        }
        let revision = old.as_ref().map_or(Ok(1), |p| {
            p.revision
                .checked_add(1)
                .filter(|v| *v <= MAX_REVISION)
                .ok_or(Failure::Capacity)
        })?;
        let view = View {
            policy_id: old.as_ref().map_or_else(Uuid::new_v4, |p| p.policy_id),
            instance_id: self.instance.clone(),
            issuer_id: issuer,
            revision,
            owner_uid: fs::uid(),
            selection: selection.clone(),
            operations,
        };
        view.validate()?;
        let selected = serde_json::to_string(&view.selection).map_err(|_| Failure::InvalidInput)?;
        let operations =
            serde_json::to_string(&view.operations).map_err(|_| Failure::InvalidInput)?;
        match old {
            None => {
                tx.execute("INSERT INTO local_approval_policies VALUES (?1,?2,?3,?4,?5,?6,?7)",
                    params![view.policy_id.to_string(), self.instance, issuer.to_string(), revision,
                        view.owner_uid, selected, operations]).map_err(db)?;
            }
            Some(old) => {
                if tx.execute("UPDATE local_approval_policies SET revision=?1,selection=?2,operations=?3 WHERE policy_id=?4 AND revision=?5",
                    params![revision, selected, operations, old.policy_id.to_string(), old.revision]).map_err(db)? != 1 {
                    return Err(Failure::Conflict);
                }
            }
        }
        lease.check()?;
        tx.commit().map_err(|_| Failure::OutcomeUnknown)?;
        // FULL/WAL acknowledges the transaction. Unknown acknowledgement never
        // returns the proposed view; later inspection cannot recreate a lease.
        acknowledge().map_err(|_| Failure::OutcomeUnknown)?;
        lease.check().map_err(|_| Failure::OutcomeUnknown)?;
        Ok(view)
    }

    /// Serialize the selected policy through the caller's final acknowledgement.
    /// This alone grants neither proof issuance nor business dispatch authority.
    pub fn acquire(&self, selection: &Selection, operation: &str) -> Result<PolicyUse> {
        selection.validate()?;
        if !identifier(operation) {
            return Err(Failure::InvalidInput);
        }
        let lease = Lease::acquire(&self.path, false)?;
        let metadata = Metadata::inspect(&self.path)?;
        self.issuer(&metadata.connection, Some(selection))?;
        let view = self.load(&metadata.connection)?.ok_or(Failure::NotFound)?;
        if &view.selection != selection
            || view
                .operations
                .binary_search_by(|s| s.as_str().cmp(operation))
                .is_err()
        {
            return Err(Failure::NotAdmitted);
        }
        lease.check()?;
        Ok(PolicyUse { lease, view })
    }

    fn issuer(
        &self,
        connection: &rusqlite::Connection,
        selection: Option<&Selection>,
    ) -> Result<Uuid> {
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(db)?;
        if version < 7 {
            return Err(Failure::NotFound);
        }
        let row: Option<(String, String, String)> = connection.query_row(
            "SELECT i.issuer_id,r.adapter_id,r.configuration_revision FROM local_approval_issuers i JOIN registry_instances r USING(instance_id) WHERE i.instance_id=?1",
            [&self.instance], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).optional().map_err(db)?;
        let (issuer, adapter, configuration) = row.ok_or(Failure::NotFound)?;
        if adapter != self.adapter
            || selection.is_some_and(|s| s.configuration_revision != configuration)
        {
            return Err(Failure::Conflict);
        }
        let id = Uuid::parse_str(&issuer).map_err(|_| Failure::MetadataUnavailable)?;
        if id.is_nil() || id.to_string() != issuer {
            return Err(Failure::MetadataUnavailable);
        }
        Ok(id)
    }

    fn load(&self, connection: &rusqlite::Connection) -> Result<Option<View>> {
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(db)?;
        if version < 8 {
            return Ok(None);
        }
        let row = connection.query_row(
            "SELECT policy_id,issuer_id,revision,owner_uid,selection,operations FROM local_approval_policies WHERE instance_id=?1",
            [&self.instance], |r| Ok((r.get::<_,String>(0)?, r.get::<_,String>(1)?, r.get::<_,i64>(2)?, r.get::<_,u32>(3)?, r.get::<_,String>(4)?, r.get::<_,String>(5)?)))
            .optional().map_err(db)?;
        let Some((policy, issuer, revision, owner_uid, selection, operations)) = row else {
            return Ok(None);
        };
        if selection.len() > 4096 || operations.len() > 131072 {
            return Err(Failure::MetadataUnavailable);
        }
        let view = View {
            policy_id: Uuid::parse_str(&policy).map_err(|_| Failure::MetadataUnavailable)?,
            instance_id: self.instance.clone(),
            issuer_id: Uuid::parse_str(&issuer).map_err(|_| Failure::MetadataUnavailable)?,
            revision,
            owner_uid,
            selection: connectors_core::read_json(selection.as_bytes())
                .map_err(|_| Failure::MetadataUnavailable)?,
            operations: connectors_core::read_json(operations.as_bytes())
                .map_err(|_| Failure::MetadataUnavailable)?,
        };
        view.validate().map_err(|_| Failure::MetadataUnavailable)?;
        if view.policy_id.to_string() != policy
            || view.issuer_id.to_string() != issuer
            || self.issuer(connection, None)? != view.issuer_id
        {
            return Err(Failure::MetadataUnavailable);
        }
        Ok(Some(view))
    }
}

/// Non-clone, process-bound shared policy exclusion; never a persisted grant.
pub struct PolicyUse {
    lease: Lease,
    view: View,
}
impl PolicyUse {
    pub fn policy(&self) -> Result<&View> {
        self.lease.check()?;
        Ok(&self.view)
    }
}

struct Lease {
    _file: File,
    pid: u32,
}
impl Lease {
    fn acquire(path: &Path, exclusive: bool) -> Result<Self> {
        let directory = fs::directory(path, false, true)?;
        if exclusive {
            match fs::publish_new(&directory, OsStr::new(LOCK), &[]) {
                Ok(()) | Err(super::Failure::ConfigurationExists) => {}
                Err(error) => return Err(error.into()),
            }
        }
        let file = fs::private_file_at(&directory, OsStr::new(LOCK))?;
        let until = Instant::now() + Duration::from_secs(2);
        loop {
            // SAFETY: a live owned descriptor; flock is released at close.
            let result = unsafe {
                libc::flock(
                    file.as_raw_fd(),
                    libc::LOCK_NB
                        | if exclusive {
                            libc::LOCK_EX
                        } else {
                            libc::LOCK_SH
                        },
                )
            };
            if result == 0 {
                break;
            }
            if std::io::Error::last_os_error().kind() != std::io::ErrorKind::WouldBlock
                || Instant::now() >= until
            {
                return Err(Failure::Conflict);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let current = fs::private_file_at(&directory, OsStr::new(LOCK))?;
        let a = file.metadata().map_err(|_| Failure::MetadataUnavailable)?;
        let b = current
            .metadata()
            .map_err(|_| Failure::MetadataUnavailable)?;
        if (a.dev(), a.ino()) != (b.dev(), b.ino()) {
            return Err(Failure::Conflict);
        }
        Ok(Self {
            _file: file,
            pid: std::process::id(),
        })
    }
    fn check(&self) -> Result<()> {
        if self.pid == std::process::id() {
            Ok(())
        } else {
            Err(Failure::Conflict)
        }
    }
}

#[cfg(test)]
mod tests;

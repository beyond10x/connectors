use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use super::lock;

const MAX_AUDIT_BYTES: u64 = 16 * 1024 * 1024;

pub(super) struct AuditJournal {
    path: PathBuf,
    state: Mutex<AuditJournalState>,
}

#[derive(Default)]
struct AuditJournalState {
    /// Bytes promised to terminal records for effects that have an attempted record and may be in
    /// flight. Holding this reservation prevents a full journal from accepting an effect and then
    /// lacking room to record its outcome.
    terminal_reservations: BTreeMap<String, u64>,
}

#[derive(Clone, Copy, Serialize)]
pub(super) struct AuditEvent<'a> {
    pub(super) audit_ref: &'a str,
    pub(super) operation_ref: &'a str,
    pub(super) connection_ref: &'a str,
    pub(super) tenant_id: &'a str,
    pub(super) actor_subject: &'a str,
    pub(super) outcome: &'a str,
}

impl AuditJournal {
    pub(super) fn new(path: PathBuf) -> Self {
        Self {
            path,
            state: Mutex::new(AuditJournalState::default()),
        }
    }

    /// Durably record an attempted effect and reserve room for its terminal outcome.
    pub(super) fn begin(&self, event: AuditEvent<'_>) -> Result<(), std::io::Error> {
        if event.outcome != "attempted" {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "initial Connector audit outcome must be attempted",
            ));
        }
        let mut state = lock(&self.state);
        if state.terminal_reservations.contains_key(event.audit_ref) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "Connector audit reference is already active",
            ));
        }
        let attempted = audit_line(event, now_seconds()?)?;
        let terminal_reservation = u64::try_from(
            audit_line(
                AuditEvent {
                    outcome: "indeterminate",
                    ..event
                },
                u64::MAX,
            )?
            .len(),
        )
        .map_err(std::io::Error::other)?;
        let outstanding = state
            .terminal_reservations
            .values()
            .try_fold(0_u64, |sum, value| sum.checked_add(*value))
            .ok_or_else(|| std::io::Error::other("Connector audit reservation overflow"))?;
        let mut file = self.open_file()?;
        let current = file.metadata()?.len();
        let attempted_bytes = u64::try_from(attempted.len()).map_err(std::io::Error::other)?;
        if current
            .checked_add(outstanding)
            .and_then(|length| length.checked_add(attempted_bytes))
            .and_then(|length| length.checked_add(terminal_reservation))
            .is_none_or(|length| length > MAX_AUDIT_BYTES)
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::FileTooLarge,
                "connector audit bound reached",
            ));
        }
        file.write_all(&attempted)?;
        file.sync_data()?;
        state
            .terminal_reservations
            .insert(event.audit_ref.to_owned(), terminal_reservation);
        Ok(())
    }

    /// Durably record exactly one terminal outcome for an earlier attempted effect.
    pub(super) fn finish(&self, event: AuditEvent<'_>) -> Result<(), std::io::Error> {
        if !matches!(event.outcome, "completed" | "indeterminate") {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "terminal Connector audit outcome is invalid",
            ));
        }
        let mut state = lock(&self.state);
        let reservation = state
            .terminal_reservations
            .get(event.audit_ref)
            .copied()
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Connector audit attempt is not active",
                )
            })?;
        let terminal = audit_line(event, now_seconds()?)?;
        let terminal_bytes = u64::try_from(terminal.len()).map_err(std::io::Error::other)?;
        if terminal_bytes > reservation {
            return Err(std::io::Error::other(
                "Connector audit terminal record exceeded its reservation",
            ));
        }
        let outstanding_other = state
            .terminal_reservations
            .values()
            .try_fold(0_u64, |sum, value| sum.checked_add(*value))
            .and_then(|sum| sum.checked_sub(reservation))
            .ok_or_else(|| std::io::Error::other("Connector audit reservation overflow"))?;
        let mut file = self.open_file()?;
        if file
            .metadata()?
            .len()
            .checked_add(outstanding_other)
            .and_then(|length| length.checked_add(terminal_bytes))
            .is_none_or(|length| length > MAX_AUDIT_BYTES)
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::FileTooLarge,
                "connector audit bound reached",
            ));
        }
        file.write_all(&terminal)?;
        file.sync_data()?;
        state.terminal_reservations.remove(event.audit_ref);
        Ok(())
    }

    fn open_file(&self) -> Result<fs::File, std::io::Error> {
        let parent = self.path.parent().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "audit path has no parent")
        })?;
        let parent_metadata = fs::symlink_metadata(parent)?;
        if !parent_metadata.file_type().is_dir()
            || parent_metadata.uid() != rustix::process::geteuid().as_raw()
            || parent_metadata.permissions().mode() & 0o077 != 0
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "unsafe audit state root",
            ));
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .mode(0o600)
            .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
            .open(&self.path)?;
        let metadata = file.metadata()?;
        if !metadata.file_type().is_file()
            || metadata.uid() != rustix::process::geteuid().as_raw()
            || metadata.permissions().mode() & 0o077 != 0
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "unsafe audit file",
            ));
        }
        Ok(file)
    }
}

fn audit_line(event: AuditEvent<'_>, at_unix_seconds: u64) -> Result<Vec<u8>, std::io::Error> {
    let mut line = serde_json::to_vec(&serde_json::json!({
        "at_unix_seconds": at_unix_seconds,
        "event": event,
    }))
    .map_err(std::io::Error::other)?;
    line.push(b'\n');
    Ok(line)
}

fn now_seconds() -> Result<u64, std::io::Error> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(std::io::Error::other)?
        .as_secs())
}

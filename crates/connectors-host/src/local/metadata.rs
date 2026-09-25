//! A single local SQLite authority. Schema installation grants no connection,
//! credential custody, approval or business dispatch authority.
use super::{Failure, Result, filesystem as fs};
mod er;
use rusqlite::{Connection, OpenFlags, TransactionBehavior};
use sha2::{Digest, Sha256};
use std::{
    ffi::OsStr,
    os::fd::AsRawFd,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const APPLICATION_ID: i64 = 0x434e4354;
const MIGRATION: &str = "CREATE TABLE local_authority (singleton INTEGER PRIMARY KEY CHECK(singleton=1), authority_id TEXT NOT NULL UNIQUE, owner_uid INTEGER NOT NULL); CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, digest TEXT NOT NULL);";
const APPROVAL_MIGRATION: &str = include_str!("metadata/approvals.sql");
const APPROVAL_KEYS_MIGRATION: &str = include_str!("metadata/approval_keys.sql");
const APPROVAL_POLICY_MIGRATION: &str = include_str!("metadata/approval_policy.sql");
const REGISTRY_MIGRATION: &str = include_str!("metadata/registry.sql");
const RUNTIME_MIGRATION: &str = include_str!("metadata/runtime.sql");
const MUTATION_MIGRATION: &str = include_str!("metadata/mutations.sql");
const AUDIT_MIGRATION: &str = include_str!("metadata/audit.sql");
const NAME: &str = "metadata.sqlite3";
const LOCK: &str = "metadata.lock";
type SchemaObject = (String, String, String, Option<String>);

#[cfg(test)]
thread_local! {
    static MIGRATION_FAULT: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
thread_local! {
    static RELOCK_HOOK: std::cell::RefCell<Option<Box<dyn FnMut()>>> =
        const { std::cell::RefCell::new(None) };
}

/// Runs on this thread each time an observation, having replayed ER outside
/// the lifecycle lock, is about to take that lock back: the exact window in
/// which another writer can advance what the observation read.
#[cfg(test)]
pub(super) fn set_relock_hook(hook: Option<Box<dyn FnMut()>>) {
    RELOCK_HOOK.with(|slot| *slot.borrow_mut() = hook);
}

#[cfg(test)]
fn run_relock_hook() {
    // Taken out while it runs: the hook itself opens metadata.
    if let Some(mut hook) = RELOCK_HOOK.with(|slot| slot.borrow_mut().take()) {
        hook();
        RELOCK_HOOK.with(|slot| {
            let mut slot = slot.borrow_mut();
            if slot.is_none() {
                *slot = Some(hook);
            }
        });
    }
}

#[cfg(test)]
fn take_migration_fault(point: u8) -> bool {
    MIGRATION_FAULT.with(|fault| {
        if fault.get() == point {
            fault.set(0);
            true
        } else {
            false
        }
    })
}

pub struct Metadata {
    // Field drop order matters: SQLite closes and retires sidecars while the
    // lifecycle lock is still held. Releasing at open/validate left a race with
    // another process admitting a sidecar that the last connection was deleting.
    pub(super) connection: Connection,
    er: Option<er::ErAuthority>,
    durable_path: PathBuf,
    _directory: std::fs::File,
    _lifecycle_lock: std::fs::File,
    concurrent_observation: bool,
}

impl Metadata {
    pub fn initialize(path: &Path) -> Result<Self> {
        let dir = fs::directory(path, false, true).map_err(|_| Failure::MetadataUnavailable)?;
        match fs::publish_new(&dir, OsStr::new(LOCK), &[]) {
            Ok(()) | Err(Failure::ConfigurationExists) => {}
            Err(Failure::OutcomeUnknown) => return Err(Failure::OutcomeUnknown),
            Err(_) => return Err(Failure::MetadataUnavailable),
        }
        let lock = lifecycle_lock(&dir)?;
        match fs::publish_new(&dir, OsStr::new(NAME), &[]) {
            Ok(()) | Err(Failure::ConfigurationExists) => {}
            Err(Failure::OutcomeUnknown) => return Err(Failure::OutcomeUnknown),
            Err(_) => return Err(Failure::MetadataUnavailable),
        }
        let mut metadata = Self::open_connection(path, dir, lock, false)?;
        let version: i64 = metadata
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(unavailable)?;
        let app: i64 = metadata
            .connection
            .pragma_query_value(None, "application_id", |row| row.get(0))
            .map_err(unavailable)?;
        if version != 0 || app != 0 {
            metadata.validate()?;
            if version == er::LEVEL {
                metadata.activate_er(false)?;
                metadata.migrate(3)?;
                return Ok(metadata);
            }
            metadata.adopt_er()?;
            metadata.migrate(3)?;
            return Ok(metadata);
        }
        let count: i64 = metadata
            .connection
            .query_row(
                "SELECT count(*) FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%'",
                [],
                |row| row.get(0),
            )
            .map_err(unavailable)?;
        if count != 0 {
            return Err(Failure::MetadataUnavailable);
        }
        // Install WAL outside a transaction. A conflicting startup waits within
        // its bounded busy timeout; migration admission is repeated under lock.
        metadata
            .connection
            .pragma_update(None, "journal_mode", "WAL")
            .map_err(unavailable)?;
        let tx = metadata
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let version: i64 = tx
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(unavailable)?;
        let app: i64 = tx
            .pragma_query_value(None, "application_id", |row| row.get(0))
            .map_err(unavailable)?;
        if version == 0 && app == 0 {
            let count: i64 = tx
                .query_row(
                    "SELECT count(*) FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%'",
                    [],
                    |row| row.get(0),
                )
                .map_err(unavailable)?;
            if count != 0 {
                return Err(Failure::MetadataUnavailable);
            }
            tx.execute_batch(MIGRATION).map_err(unavailable)?;
            tx.execute(
                "INSERT INTO local_authority VALUES (1, ?1, ?2)",
                rusqlite::params![uuid::Uuid::new_v4().to_string(), fs::uid()],
            )
            .map_err(unavailable)?;
            tx.execute(
                "INSERT INTO schema_migrations VALUES (1, ?1)",
                [migration_digest()],
            )
            .map_err(unavailable)?;
            tx.pragma_update(None, "application_id", APPLICATION_ID)
                .map_err(unavailable)?;
            tx.pragma_update(None, "user_version", 1)
                .map_err(unavailable)?;
        }
        tx.commit().map_err(|_| Failure::OutcomeUnknown)?;
        metadata
            ._directory
            .sync_all()
            .map_err(|_| Failure::OutcomeUnknown)?;
        metadata.validate()?;
        metadata.adopt_er()?;
        metadata.migrate(3)?;
        Ok(metadata)
    }

    /// Inspection never creates a database, migrates it, or interprets absence
    /// as an empty authoritative registry.
    pub fn inspect(path: &Path) -> Result<Self> {
        let dir = fs::directory(path, false, true).map_err(|_| Failure::MetadataUnavailable)?;
        let lock = lifecycle_lock(&dir)?;
        let mut metadata = Self::open_connection(path, dir, lock, true)?;
        metadata.validate()?;
        let version: i64 = metadata
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(unavailable)?;
        if version == er::LEVEL {
            metadata.activate_er(true)?;
        }
        Ok(metadata)
    }

    /// An admitted mutating action may upgrade an existing, recognized database.
    /// Missing authority is never recreated by connection management.
    pub(super) fn update(path: &Path, migrate: bool) -> Result<Self> {
        Self::update_with_observation(path, migrate, false)
    }

    pub(super) fn update_observation(path: &Path) -> Result<Self> {
        Self::update_with_observation(path, false, true)
    }

    /// The established physical source was validated and closed before ER
    /// replay. Exclude all clock writers again before sampling time or acting
    /// on that freshly replayed projection. Legacy adoption never unlocks.
    pub(super) fn relock_observation(&mut self) -> Result<()> {
        if !self.concurrent_observation {
            return Err(Failure::MetadataUnavailable);
        }
        #[cfg(test)]
        run_relock_hook();
        acquire_lifecycle_lock(&self._lifecycle_lock)?;
        self.concurrent_observation = false;
        Ok(())
    }

    fn update_with_observation(
        path: &Path,
        migrate: bool,
        concurrent_observation: bool,
    ) -> Result<Self> {
        let dir = fs::directory(path, false, true).map_err(|_| Failure::MetadataUnavailable)?;
        let lock = lifecycle_lock(&dir)?;
        let mut metadata = Self::open_connection(path, dir, lock, false)?;
        metadata.validate()?;
        let version: i64 = metadata
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(unavailable)?;
        if version == er::LEVEL {
            metadata.activate_er(concurrent_observation)?;
        }
        if migrate {
            if metadata.er.is_none() {
                metadata.adopt_er()?;
            }
            metadata.migrate(3)?;
        } else {
            metadata.require_registry()?;
            if metadata.er.is_none() {
                metadata.adopt_er()?;
            }
        }
        metadata.require_registry()?;
        metadata.concurrent_observation = concurrent_observation;
        Ok(metadata)
    }

    /// Only the admitted mutation port installs its schema. Existing setup and
    /// read owners must not upgrade storage for an unused business-write port.
    pub(super) fn update_mutations(path: &Path) -> Result<Self> {
        let mut metadata = Self::update(path, true)?;
        metadata.migrate(4)?;
        Ok(metadata)
    }

    /// Audit is independently acknowledged; only admitted anchoring installs
    /// this port. Its retained attempt references require the earlier schema.
    pub(super) fn update_audit(path: &Path) -> Result<Self> {
        let mut metadata = Self::update(path, true)?;
        metadata.migrate(5)?;
        Ok(metadata)
    }

    pub(super) fn update_approvals(path: &Path) -> Result<Self> {
        let mut metadata = Self::update(path, true)?;
        metadata.migrate(6)?;
        Ok(metadata)
    }

    pub(super) fn require_registry(&self) -> Result<()> {
        let version: i64 = self
            .connection
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(unavailable)?;
        if !(2..=8).contains(&version) {
            return Err(Failure::MetadataUnavailable);
        }
        Ok(())
    }

    pub(super) fn update_approval_keys(path: &Path) -> Result<Self> {
        let mut metadata = Self::update(path, true)?;
        metadata.migrate(7)?;
        Ok(metadata)
    }

    /// Only admitted policy publication installs this port. Inspection and
    /// existing setup/key management retain their original migration targets.
    pub(super) fn update_approval_policy(path: &Path) -> Result<Self> {
        let mut metadata = Self::update(path, true)?;
        metadata.migrate(8)?;
        Ok(metadata)
    }

    pub(super) fn authority(&self) -> Result<uuid::Uuid> {
        let text: String = self
            .connection
            .query_row(
                "SELECT authority_id FROM local_authority WHERE singleton=1",
                [],
                |r| r.get(0),
            )
            .map_err(unavailable)?;
        uuid::Uuid::parse_str(&text).map_err(|_| Failure::MetadataUnavailable)
    }

    fn migrate(&mut self, target: i64) -> Result<()> {
        let version: i64 = self
            .connection
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(unavailable)?;
        for (next, sql) in [
            (2, REGISTRY_MIGRATION),
            (3, RUNTIME_MIGRATION),
            (4, MUTATION_MIGRATION),
            (5, AUDIT_MIGRATION),
            (6, APPROVAL_MIGRATION),
            (7, APPROVAL_KEYS_MIGRATION),
            (8, APPROVAL_POLICY_MIGRATION),
        ] {
            if version >= next || next > target {
                continue;
            }
            let tx = self
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(unavailable)?;
            tx.execute_batch(sql).map_err(unavailable)?;
            tx.execute(
                "INSERT INTO schema_migrations VALUES (?1, ?2)",
                rusqlite::params![next, hex::encode(Sha256::digest(sql.as_bytes()))],
            )
            .map_err(unavailable)?;
            tx.pragma_update(None, "user_version", next)
                .map_err(unavailable)?;
            tx.commit().map_err(|_| Failure::OutcomeUnknown)?;
        }
        self.persist()?;
        if self.er.is_some() {
            let level: i64 = self
                .connection
                .pragma_query_value(None, "user_version", |row| row.get(0))
                .map_err(unavailable)?;
            let authority_id = self.authority()?;
            let authority = self.er.as_mut().ok_or(Failure::MetadataUnavailable)?;
            er::advance_projection_level(authority, level)?;
            self.connection = er::projection(authority_id, authority)?;
        }
        self._directory
            .sync_all()
            .map_err(|_| Failure::OutcomeUnknown)?;
        self.validate()
    }

    pub(super) fn persist(&mut self) -> Result<()> {
        self.persist_with_admission(false)
    }

    pub(super) fn persist_admission(&mut self) -> Result<()> {
        self.persist_with_admission(true)
    }

    pub(super) fn persist_prepared_observation(&mut self) -> Result<()> {
        if self.concurrent_observation {
            return Err(Failure::MetadataUnavailable);
        }
        let authority = self.er.as_mut().ok_or(Failure::MetadataUnavailable)?;
        er::persist(
            authority,
            &self.connection,
            er::PersistMode::PreparedObservation,
        )
    }

    /// Runtime suppression and remembered bootstrap read only their own rows.
    /// This named path cannot persist registry-dependent business changes.
    pub(super) fn persist_runtime_state(&mut self) -> Result<()> {
        if self.concurrent_observation {
            return Err(Failure::MetadataUnavailable);
        }
        let authority = self.er.as_mut().ok_or(Failure::MetadataUnavailable)?;
        er::persist(
            authority,
            &self.connection,
            er::PersistMode::RuntimeStateOnly,
        )
    }

    fn persist_with_admission(&mut self, admitted_registry_use: bool) -> Result<()> {
        if let Some(authority) = &mut self.er {
            er::persist(
                authority,
                &self.connection,
                er::PersistMode::Standard {
                    admitted_registry_use,
                    concurrent_observation: self.concurrent_observation,
                },
            )?;
        }
        Ok(())
    }

    fn adopt_er(&mut self) -> Result<()> {
        if self.er.is_some() {
            return Ok(());
        }
        let level: i64 = self
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(unavailable)?;
        if !(1..=8).contains(&level) {
            return Err(Failure::MetadataUnavailable);
        }
        let authority_id = self.authority()?;
        let rows = er::capture(&self.connection, level)?;
        let (authority, coordinates, source_digest) =
            er::provision(&self.durable_path, authority_id, level, rows)?;
        let projection = er::projection(authority_id, &authority)?;
        if er::capture_digest(&projection, level)? != source_digest {
            return Err(Failure::MetadataUnavailable);
        }
        #[cfg(test)]
        if take_migration_fault(1) {
            return Err(Failure::MetadataUnavailable);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        tx.execute_batch(er::MIGRATION).map_err(unavailable)?;
        tx.execute(
            "INSERT INTO connectors_er_authority VALUES (1,?1,?2,?3,?4,?5,?6,?7)",
            rusqlite::params![
                authority_id.to_string(),
                coordinates.logical_scope,
                coordinates.tenant,
                coordinates.stream_identity,
                level,
                level,
                source_digest
            ],
        )
        .map_err(unavailable)?;
        tx.execute(
            "INSERT INTO schema_migrations VALUES (9,?1)",
            [er::migration_digest()],
        )
        .map_err(unavailable)?;
        tx.pragma_update(None, "user_version", er::LEVEL)
            .map_err(unavailable)?;
        tx.commit().map_err(|_| Failure::OutcomeUnknown)?;
        self._directory
            .sync_all()
            .map_err(|_| Failure::OutcomeUnknown)?;
        #[cfg(test)]
        if take_migration_fault(2) {
            return Err(Failure::OutcomeUnknown);
        }
        self.connection = projection;
        self.er = Some(authority);
        self.validate()
    }

    fn activate_er(&mut self, release_physical: bool) -> Result<()> {
        if self.er.is_some() {
            return Ok(());
        }
        let authority_id = self.authority()?;
        let (marked_authority, logical_scope, tenant, stream_identity, source_level, projection_level, digest): (
            String,
            String,
            String,
            String,
            i64,
            i64,
            String,
        ) = self
            .connection
            .query_row(
                "SELECT authority_id,logical_scope,tenant,stream_identity,source_level,projection_level,source_digest FROM connectors_er_authority WHERE singleton=1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?)),
            )
            .map_err(unavailable)?;
        if marked_authority != authority_id.to_string()
            || logical_scope != er::LOGICAL_SCOPE
            || tenant != authority_id.to_string()
            || !(1..=8).contains(&source_level)
            || !(source_level..=8).contains(&projection_level)
            || digest.len() != 64
            || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(Failure::MetadataUnavailable);
        }
        let physical = er::capture_digest(&self.connection, source_level)?;
        if physical != digest {
            return Err(Failure::MetadataUnavailable);
        }
        if release_physical {
            // The LEVEL 9 marker and legacy source are immutable here. Close the
            // physical SQLite handle while its lifecycle lock still excludes
            // another metadata open, then release it before replaying the ER
            // authority. Eventlog's SQLite provider owns its own concurrent
            // handle and guarded append lifecycle after this point.
            let scratch = Connection::open_in_memory().map_err(unavailable)?;
            let physical_connection = std::mem::replace(&mut self.connection, scratch);
            physical_connection
                .close()
                .map_err(|_| Failure::MetadataUnavailable)?;
            // SAFETY: the live file owns this descriptor; Drop still closes it.
            if unsafe { libc::flock(self._lifecycle_lock.as_raw_fd(), libc::LOCK_UN) } != 0 {
                return Err(Failure::MetadataUnavailable);
            }
        }
        let (authority, imported_digest) = er::open(
            &self.durable_path,
            entity_eventlog::Authority {
                logical_scope,
                tenant,
                stream_identity,
            },
            source_level,
            projection_level,
        )?;
        if imported_digest != digest {
            return Err(Failure::MetadataUnavailable);
        }
        let projection = er::projection(authority_id, &authority)?;
        self.connection = projection;
        self.er = Some(authority);
        self.validate()
    }

    fn open_connection(
        path: &Path,
        dir: std::fs::File,
        lock: std::fs::File,
        read_only: bool,
    ) -> Result<Self> {
        // The WAL-reset concurrency defect is fixed in 3.51.3 and later.
        // Refuse an ambient library override that predates that fix, even if
        // the build environment overrides our bundled Cargo dependency.
        if rusqlite::version_number() < 3_051_003 {
            return Err(Failure::MetadataUnavailable);
        }
        fs::private_file_metadata_at(&dir, OsStr::new(NAME))
            .map_err(|_| Failure::MetadataUnavailable)?;
        admit_sidecars(&dir)?;
        let access = if read_only {
            OpenFlags::SQLITE_OPEN_READ_ONLY
        } else {
            OpenFlags::SQLITE_OPEN_READ_WRITE
        };
        let connection = Connection::open_with_flags(
            path.join(NAME),
            access | OpenFlags::SQLITE_OPEN_NO_MUTEX | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .map_err(unavailable)?;
        // How long a writer waits for another to release the database before reporting
        // it unavailable. Two seconds was enough for one CLI and its owner; it is not
        // enough when the machine is loaded, and an exhausted busy timeout surfaces as
        // `MetadataUnavailable` — indistinguishable from a store that is genuinely gone.
        // Every caller above this carries its own deadline and cancels on it, so waiting
        // longer here cannot hang a request; it only stops a contended write from
        // reporting the wrong failure. See story:host-suite-load-sensitivity.
        connection
            .busy_timeout(Duration::from_secs(30))
            .map_err(unavailable)?;
        connection
            .pragma_update(None, "trusted_schema", false)
            .map_err(unavailable)?;
        connection
            .pragma_update(None, "foreign_keys", true)
            .map_err(unavailable)?;
        connection
            .pragma_update(None, "synchronous", "FULL")
            .map_err(unavailable)?;
        connection
            .pragma_update(None, "temp_store", "MEMORY")
            .map_err(unavailable)?;
        Ok(Self {
            connection,
            er: None,
            durable_path: path.join(NAME),
            _directory: dir,
            _lifecycle_lock: lock,
            concurrent_observation: false,
        })
    }

    fn validate(&self) -> Result<()> {
        let app: i64 = self
            .connection
            .pragma_query_value(None, "application_id", |row| row.get(0))
            .map_err(unavailable)?;
        let version: i64 = self
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(unavailable)?;
        let mode: String = self
            .connection
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .map_err(unavailable)?;
        let recorded = self.er.is_some();
        if app != APPLICATION_ID
            || if recorded {
                !(1..=8).contains(&version) || mode != "memory"
            } else {
                !(1..=er::LEVEL).contains(&version) || mode != "wal"
            }
        {
            return Err(Failure::MetadataUnavailable);
        }
        let (authority, owner): (String, u32) = self
            .connection
            .query_row(
                "SELECT authority_id, owner_uid FROM local_authority WHERE singleton=1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(unavailable)?;
        if owner != fs::uid() || uuid::Uuid::parse_str(&authority).map_or(true, |id| id.is_nil()) {
            return Err(Failure::MetadataUnavailable);
        }
        let migrations = self
            .connection
            .prepare("SELECT version, digest FROM schema_migrations ORDER BY version")
            .map_err(unavailable)?
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(unavailable)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(unavailable)?;
        let expected_level = if version == er::LEVEL {
            self.connection
                .query_row(
                    "SELECT source_level FROM connectors_er_authority WHERE singleton=1",
                    [],
                    |row| row.get(0),
                )
                .map_err(unavailable)?
        } else {
            version
        };
        validate_schema(
            &self.connection,
            expected_level,
            !recorded && version == er::LEVEL,
        )?;
        let mut expected = vec![(1, migration_digest())];
        if expected_level >= 2 {
            expected.push((
                2,
                hex::encode(Sha256::digest(REGISTRY_MIGRATION.as_bytes())),
            ));
        }
        if expected_level >= 3 {
            expected.push((3, hex::encode(Sha256::digest(RUNTIME_MIGRATION.as_bytes()))));
        }
        if expected_level >= 4 {
            expected.push((
                4,
                hex::encode(Sha256::digest(MUTATION_MIGRATION.as_bytes())),
            ));
        }
        if expected_level >= 5 {
            expected.push((5, hex::encode(Sha256::digest(AUDIT_MIGRATION.as_bytes()))));
        }
        if expected_level >= 6 {
            expected.push((
                6,
                hex::encode(Sha256::digest(APPROVAL_MIGRATION.as_bytes())),
            ));
        }
        if expected_level >= 7 {
            expected.push((
                7,
                hex::encode(Sha256::digest(APPROVAL_KEYS_MIGRATION.as_bytes())),
            ));
        }
        if expected_level >= 8 {
            expected.push((
                8,
                hex::encode(Sha256::digest(APPROVAL_POLICY_MIGRATION.as_bytes())),
            ));
        }
        if version == er::LEVEL {
            expected.push((er::LEVEL, er::migration_digest()));
        }
        if migrations != expected {
            return Err(Failure::MetadataUnavailable);
        }
        Ok(())
    }
}

fn validate_schema(connection: &Connection, level: i64, marker: bool) -> Result<()> {
    let integrity: String = connection
        .query_row("PRAGMA quick_check(1)", [], |row| row.get(0))
        .map_err(unavailable)?;
    if integrity != "ok" {
        return Err(Failure::MetadataUnavailable);
    }
    let foreign_key_errors: i64 = connection
        .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })
        .map_err(unavailable)?;
    if foreign_key_errors != 0 {
        return Err(Failure::MetadataUnavailable);
    }

    let expected = Connection::open_in_memory().map_err(unavailable)?;
    expected.execute_batch(MIGRATION).map_err(unavailable)?;
    for (next, source) in er::migrations() {
        if next > level {
            break;
        }
        expected.execute_batch(source).map_err(unavailable)?;
    }
    if marker {
        expected.execute_batch(er::MIGRATION).map_err(unavailable)?;
    }
    if schema_objects(connection, true)? != schema_objects(&expected, false)? {
        return Err(Failure::MetadataUnavailable);
    }
    Ok(())
}

fn schema_objects(connection: &Connection, omit_eventlog: bool) -> Result<Vec<SchemaObject>> {
    let eventlog_prefix = format!("{}_", er::PREFIX);
    let mut statement = connection
        .prepare(
            "SELECT type,name,tbl_name,sql FROM sqlite_schema \
             WHERE name NOT LIKE 'sqlite_%' ORDER BY type,name",
        )
        .map_err(unavailable)?;
    let objects = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })
        .map_err(unavailable)?
        .filter_map(|object| match object {
            Ok(object)
                if omit_eventlog
                    && object.1.starts_with(&eventlog_prefix)
                    && object.1 != er::MARKER =>
            {
                None
            }
            other => Some(other),
        })
        .collect::<std::result::Result<Vec<_>, _>>();
    objects.map_err(unavailable)
}

fn migration_digest() -> String {
    hex::encode(Sha256::digest(MIGRATION.as_bytes()))
}

fn unavailable(_: rusqlite::Error) -> Failure {
    Failure::MetadataUnavailable
}

/// Eventlog's SQLite handle is not under the lifecycle lock, so its last close
/// may retire a sidecar at any point here. One `fstatat` per sidecar decides
/// present-and-private or absent; a present sidecar is never admitted weaker.
fn admit_sidecars(dir: &std::fs::File) -> Result<()> {
    for suffix in ["-wal", "-shm", "-journal"] {
        let name = format!("{NAME}{suffix}");
        fs::private_file_metadata_if_present_at(dir, OsStr::new(&name))
            .map_err(|_| Failure::MetadataUnavailable)?;
    }
    Ok(())
}

// Protect physical SQLite admission, initial WAL installation, migration and
// last-close sidecar retirement. Established ER observations close that physical
// handle before releasing the OS lock; ordinary business writes retain it.
// No handle may be retained across provider work.
fn lifecycle_lock(directory: &std::fs::File) -> Result<std::fs::File> {
    let file = fs::private_file_at(directory, OsStr::new(LOCK))
        .map_err(|_| Failure::MetadataUnavailable)?;
    acquire_lifecycle_lock(&file)?;
    Ok(file)
}

fn acquire_lifecycle_lock(file: &std::fs::File) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        // SAFETY: file owns the live descriptor. Dropping it releases the lock.
        let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_NB | libc::LOCK_EX) };
        if result == 0 {
            return Ok(());
        }
        let kind = std::io::Error::last_os_error().kind();
        if kind != std::io::ErrorKind::WouldBlock || Instant::now() >= deadline {
            return Err(Failure::MetadataUnavailable);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[cfg(test)]
pub(super) fn legacy_fixture(path: &Path, target: i64) -> uuid::Uuid {
    let directory = fs::directory(path, true, true).unwrap();
    fs::publish_new(&directory, OsStr::new(NAME), &[]).unwrap();
    fs::publish_new(&directory, OsStr::new(LOCK), &[]).unwrap();
    let connection = Connection::open(path.join(NAME)).unwrap();
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .unwrap();
    connection.execute_batch(MIGRATION).unwrap();
    connection
        .execute(
            "INSERT INTO schema_migrations VALUES (1,?1)",
            [migration_digest()],
        )
        .unwrap();
    for (version, source) in er::migrations().filter(|(version, _)| *version <= target) {
        connection.execute_batch(source).unwrap();
        connection
            .execute(
                "INSERT INTO schema_migrations VALUES (?1,?2)",
                rusqlite::params![version, hex::encode(Sha256::digest(source.as_bytes()))],
            )
            .unwrap();
    }
    let authority = uuid::Uuid::new_v4();
    connection
        .execute(
            "INSERT INTO local_authority VALUES (1,?1,?2)",
            rusqlite::params![authority.to_string(), fs::uid()],
        )
        .unwrap();
    connection
        .pragma_update(None, "application_id", APPLICATION_ID)
        .unwrap();
    connection
        .pragma_update(None, "user_version", target)
        .unwrap();
    authority
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wal_writer_lock_held(shm: &Path) -> bool {
        use std::os::unix::ffi::OsStrExt;
        let path = std::ffi::CString::new(shm.as_os_str().as_bytes()).unwrap();
        // SAFETY: the child performs only libc calls and _exit after fork. It
        // never touches the inherited SQLite handle or Rust test runtime.
        let child = unsafe { libc::fork() };
        assert!(child >= 0);
        if child == 0 {
            // SAFETY: path is a live NUL-terminated string prepared pre-fork.
            let fd = unsafe { libc::open(path.as_ptr(), libc::O_RDONLY | libc::O_CLOEXEC) };
            if fd < 0 {
                unsafe { libc::_exit(2) };
            }
            // SQLite's bundled unix VFS locks WAL_WRITE_LOCK at -shm byte 120.
            let mut lock = libc::flock {
                l_type: libc::F_WRLCK as i16,
                l_whence: libc::SEEK_SET as i16,
                l_start: 120,
                l_len: 1,
                l_pid: 0,
            };
            let result = unsafe { libc::fcntl(fd, libc::F_GETLK, &mut lock) };
            unsafe { libc::close(fd) };
            unsafe {
                libc::_exit(if result < 0 {
                    2
                } else if lock.l_type == libc::F_UNLCK as i16 {
                    1
                } else {
                    0
                })
            };
        }
        let mut status = 0;
        assert_eq!(unsafe { libc::waitpid(child, &mut status, 0) }, child);
        assert_eq!(status & 0x7f, 0, "lock probe child terminated abnormally");
        let exit = (status >> 8) & 0xff;
        assert!(exit <= 1, "lock probe child could not inspect WAL lock");
        exit == 0
    }

    #[test]
    fn metadata_file_verification_preserves_an_existing_sqlite_wal_lock() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join(NAME);
        let connection = Connection::open(&db).unwrap();
        std::fs::set_permissions(&db, std::fs::Permissions::from_mode(0o600)).unwrap();
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .unwrap();
        connection.execute_batch("CREATE TABLE lock_probe(value INTEGER); BEGIN IMMEDIATE; INSERT INTO lock_probe VALUES (1)").unwrap();
        let shm = root.path().join(format!("{NAME}-shm"));
        assert!(
            wal_writer_lock_held(&shm),
            "SQLite must hold the WAL writer lock before verification"
        );
        let directory = std::fs::File::open(root.path()).unwrap();
        fs::private_file_metadata_at(&directory, OsStr::new(&format!("{NAME}-shm"))).unwrap();
        assert!(
            wal_writer_lock_held(&shm),
            "metadata verification must not clear an active SQLite WAL lock"
        );
        connection.execute_batch("ROLLBACK").unwrap();
    }

    /// Eventlog's own SQLite handle is outside the lifecycle lock, so its last
    /// close retires sidecars while another process is admitting the file. A
    /// sidecar that is gone by the time it is checked is absent, not unsafe.
    #[test]
    fn a_sidecar_retired_during_admission_is_absent_not_unavailable() {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt, symlink};
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let directory = fs::directory(root.path(), false, true).unwrap();
        let sidecar = root.path().join(format!("{NAME}-journal"));
        let stop = Arc::new(AtomicBool::new(false));
        let retiring = {
            let stop = stop.clone();
            let sidecar = sidecar.clone();
            std::thread::spawn(move || {
                while !stop.load(Ordering::SeqCst) {
                    drop(
                        std::fs::OpenOptions::new()
                            .write(true)
                            .create_new(true)
                            .mode(0o600)
                            .open(&sidecar)
                            .unwrap(),
                    );
                    std::fs::remove_file(&sidecar).unwrap();
                }
            })
        };
        let refused = (0..20_000)
            .filter(|_| admit_sidecars(&directory).is_err())
            .count();
        stop.store(true, Ordering::SeqCst);
        retiring.join().unwrap();
        assert_eq!(refused, 0, "a concurrently retired sidecar was refused");

        // Absence is not a weaker admission of what is present.
        std::fs::write(&sidecar, b"").unwrap();
        std::fs::set_permissions(&sidecar, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(
            admit_sidecars(&directory),
            Err(Failure::MetadataUnavailable)
        );
        std::fs::remove_file(&sidecar).unwrap();
        symlink(root.path().join("elsewhere"), &sidecar).unwrap();
        assert_eq!(
            admit_sidecars(&directory),
            Err(Failure::MetadataUnavailable)
        );
    }

    #[test]
    fn a_business_batch_cannot_commit_behind_an_unrecorded_clock_advance() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("state");
        fs::directory(&path, true, true).unwrap();
        drop(Metadata::initialize(&path).unwrap());
        let mut observation = Metadata::update_observation(&path).unwrap();
        let mut business = Metadata::update(&path, false).unwrap();
        // The business handle read the same clock value as its baseline. Its
        // business row is changed later, while an already-open observer can
        // durably advance the clock despite the business lifecycle lock.
        observation
            .connection
            .execute("UPDATE registry_clock SET last_seen_ms=1", [])
            .unwrap();
        observation.persist().unwrap();
        business
            .connection
            .execute("INSERT INTO registry_instances(instance_id,adapter_id,configuration_revision,epoch) VALUES ('fixture','adapter','revision',0)", [])
            .unwrap();
        let result = business.persist();
        drop(business);
        drop(observation);
        let reopened = Metadata::inspect(&path).unwrap();
        let count: i64 = reopened
            .connection
            .query_row(
                "SELECT count(*) FROM registry_instances WHERE instance_id='fixture'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0, "a stale business batch must not commit");
        assert_eq!(result, Err(Failure::ConcurrentRevision));
    }

    #[test]
    fn unchanged_clock_business_batch_succeeds_without_advancing_time() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("state");
        fs::directory(&path, true, true).unwrap();
        drop(Metadata::initialize(&path).unwrap());
        let mut business = Metadata::update(&path, false).unwrap();
        business
            .connection
            .execute("INSERT INTO registry_instances(instance_id,adapter_id,configuration_revision,epoch) VALUES ('fixture','adapter','revision',0)", [])
            .unwrap();
        business.persist().unwrap();
        drop(business);
        let reopened = Metadata::inspect(&path).unwrap();
        let (count, floor): (i64, i64) = reopened
            .connection
            .query_row("SELECT (SELECT count(*) FROM registry_instances WHERE instance_id='fixture'),last_seen_ms FROM registry_clock", [], |row| Ok((row.get(0)?,row.get(1)?)))
            .unwrap();
        assert_eq!((count, floor), (1, 0));
    }

    #[test]
    fn runtime_state_persist_refuses_a_changed_registry_clock() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("state");
        fs::directory(&path, true, true).unwrap();
        drop(Metadata::initialize(&path).unwrap());
        let mut runtime = Metadata::update(&path, true).unwrap();
        runtime
            .connection
            .execute("UPDATE registry_clock SET last_seen_ms=1", [])
            .unwrap();
        assert_eq!(
            runtime.persist_runtime_state(),
            Err(Failure::MetadataUnavailable)
        );
        drop(runtime);
        let reopened = Metadata::inspect(&path).unwrap();
        let floor: i64 = reopened
            .connection
            .query_row(
                "SELECT last_seen_ms FROM registry_clock WHERE singleton=1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(floor, 0);
    }

    #[test]
    fn prepared_observation_commits_across_an_interleaved_runtime_record_write() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("state");
        fs::directory(&path, true, true).unwrap();
        drop(Metadata::initialize(&path).unwrap());
        // The observation has verified and closed the physical source and
        // replayed ER without the lifecycle lock. Its baseline has no runtime
        // record yet.
        let mut prepared = Metadata::update_observation(&path).unwrap();
        // A runtime-record write lands before the observation relocks. This
        // path deliberately omits the registry-clock guard, so the observation's
        // clock revision cannot detect it.
        let mut runtime = Metadata::update(&path, true).unwrap();
        runtime
            .connection
            .execute(
                "INSERT INTO local_runtime_instances (instance_id,suppressed) VALUES ('fixture',1)",
                [],
            )
            .unwrap();
        runtime.persist_runtime_state().unwrap();
        drop(runtime);

        prepared.relock_observation().unwrap();
        prepared
            .connection
            .execute("UPDATE registry_clock SET last_seen_ms=last_seen_ms", [])
            .unwrap();
        // The observation never reads or writes runtime records; the
        // interleaved write is not evidence of a bad own projection.
        assert_eq!(prepared.persist_prepared_observation(), Ok(()));
        drop(prepared);
        let reopened = Metadata::inspect(&path).unwrap();
        let (suppressed, floor): (bool, i64) = reopened
            .connection
            .query_row(
                "SELECT (SELECT suppressed FROM local_runtime_instances WHERE instance_id='fixture'),last_seen_ms FROM registry_clock WHERE singleton=1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert!(
            suppressed,
            "the runtime record must survive the observation"
        );
        assert_eq!(floor, 0);
    }

    fn disk_version(path: &Path) -> i64 {
        Connection::open(path.join(NAME))
            .unwrap()
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap()
    }

    #[test]
    fn fresh_setup_selects_the_recorded_authority_and_reopens_after_a_real_mutation() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("state");
        fs::directory(&path, true, true).unwrap();
        let authority = Metadata::initialize(&path).unwrap().authority().unwrap();
        assert_eq!(
            disk_version(&path),
            9,
            "fresh setup must select ER/Eventlog"
        );

        let state = crate::local::runtime::state::State::new(&path);
        state.suppress("fixture", true).unwrap();
        drop(state);
        assert!(
            crate::local::runtime::state::State::new(&path)
                .suppressed("fixture")
                .unwrap()
        );
        assert_eq!(
            Metadata::inspect(&path).unwrap().authority().unwrap(),
            authority
        );
    }

    #[test]
    fn an_unmodeled_sql_deletion_cannot_erase_recorded_metadata() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("state");
        fs::directory(&path, true, true).unwrap();
        let mut metadata = Metadata::initialize(&path).unwrap();
        metadata
            .connection
            .execute(
                "INSERT INTO registry_instances VALUES ('instance','adapter','revision',1)",
                [],
            )
            .unwrap();
        metadata.persist().unwrap();
        metadata
            .connection
            .execute(
                "DELETE FROM registry_instances WHERE instance_id='instance'",
                [],
            )
            .unwrap();
        assert_eq!(metadata.persist(), Err(Failure::MetadataUnavailable));
        drop(metadata);
        let restored: String = Metadata::inspect(&path)
            .unwrap()
            .connection
            .query_row(
                "SELECT instance_id FROM registry_instances WHERE instance_id='instance'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(restored, "instance");
    }

    #[test]
    fn nonempty_level_eight_migrates_without_changing_identity_or_retained_facts() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("state");
        let directory = fs::directory(&path, true, true).unwrap();
        fs::publish_new(&directory, OsStr::new(NAME), &[]).unwrap();
        fs::publish_new(&directory, OsStr::new(LOCK), &[]).unwrap();
        let connection = Connection::open(path.join(NAME)).unwrap();
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .unwrap();
        let authority = uuid::Uuid::new_v4();
        for (version, source) in [
            (1, MIGRATION),
            (2, REGISTRY_MIGRATION),
            (3, RUNTIME_MIGRATION),
            (4, MUTATION_MIGRATION),
            (5, AUDIT_MIGRATION),
            (6, APPROVAL_MIGRATION),
            (7, APPROVAL_KEYS_MIGRATION),
            (8, APPROVAL_POLICY_MIGRATION),
        ] {
            connection.execute_batch(source).unwrap();
            connection
                .execute(
                    "INSERT INTO schema_migrations VALUES (?1,?2)",
                    rusqlite::params![
                        version,
                        if version == 1 {
                            migration_digest()
                        } else {
                            hex::encode(Sha256::digest(source.as_bytes()))
                        }
                    ],
                )
                .unwrap();
        }
        connection
            .execute(
                "INSERT INTO local_authority VALUES (1,?1,?2)",
                rusqlite::params![authority.to_string(), fs::uid()],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO registry_instances VALUES ('instance','adapter','cfg',7)",
                [],
            )
            .unwrap();
        let binding = crate::local::registry::fixture_binding("instance");
        connection
            .execute(
                "INSERT INTO registry_profiles VALUES ('profile','adapter','pat','1',?1)",
                [serde_json::to_string(&binding.profile).unwrap()],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO registry_connections(connection_ref,instance_id,profile_key,binding,scope_id,semantic_revision,publication_fence,state,public,created_at_ms) \
                 VALUES ('connection','instance','profile',?1,'scope','semantic-revision','private-fence','live',1,41)",
                [serde_json::to_string(&binding).unwrap()],
            )
            .unwrap();
        let audit_rows = [
            ("first", None),
            (
                "second",
                Some(crate::local::audit::FinalObservation {
                    observation_id: uuid::Uuid::from_u128(7),
                    outcome: crate::local::audit::Outcome::Success,
                    code: None,
                    recorded_at_ms: 47,
                }),
            ),
        ]
        .into_iter()
        .map(|(audit_ref, final_observation)| {
            let reference = crate::local::audit::Reference {
                instance: "instance".into(),
                audit_ref: audit_ref.into(),
            };
            let record = crate::local::audit::Record {
                reference: reference.clone(),
                anchor: crate::local::audit::Anchor {
                    instance_id: "instance".into(),
                    kind: crate::local::audit::Kind::AdmittedExecution,
                    activity: Some(crate::local::audit::Activity::Invoke),
                    hop: crate::local::audit::Hop::Execution,
                    stage: crate::local::audit::Stage::Admission,
                    request_id: Some("request".into()),
                    principal_ref: Some("caller".into()),
                    operation_id: Some("operation".into()),
                    connection_ref: Some("connection".into()),
                    descriptor_revision: Some("descriptor".into()),
                    recorded_at_ms: 45,
                    attempt_id: None,
                },
                final_observation,
            };
            let encoded = serde_json::to_string(&record).unwrap();
            connection
                .execute(
                    "INSERT INTO execution_audits(audit_record_ref,instance_id,audit_ref,connection_ref,record_json) \
                     VALUES (?1,'instance',?2,'connection',?3)",
                    rusqlite::params![reference.encode_private().unwrap(), audit_ref, encoded],
                )
                .unwrap();
            serde_json::to_value(record).unwrap()
        })
        .collect::<Vec<_>>();
        let descriptor = serde_json::json!({
            "version": "v1alpha1", "instance": "instance", "adapter": "adapter",
            "revision": "descriptor", "configuration_schema": {"type": "object"},
            "operations": [{"id": "read", "description": "Fixture read", "contract": "fixture/1",
                "profile": "pat", "input_schema": {"type": "object"},
                "output_schema": {"type": "object"}}]
        });
        let bootstrap: crate::local::runtime::Bootstrap = serde_json::from_value(serde_json::json!({
            "instance": "instance", "adapter": "adapter", "protocol": "v1alpha1",
            "configuration_revision": "cfg", "provider_authority": "fixture-authority",
            "descriptor": descriptor.to_string(),
            "profiles": [{"id": "pat", "revision": "1", "purpose": "delegated_user",
                "subject": "user", "scheme": "http_bearer", "capability": "http-bearer",
                "minimum_scopes": [], "evidence_lifetime_ms": 60000,
                "fields": [{"name": "token", "label": "Fictional credential", "max_bytes": 100}]}],
            "requirements": [{"operation": "read", "profile": "pat", "scopes": [], "effect": "read"}]
        })).unwrap();
        bootstrap.validate().unwrap();
        let bootstrap_bytes = serde_json::to_string(&bootstrap).unwrap();
        connection
            .execute(
                "INSERT INTO local_runtime_instances VALUES ('instance',1,'selection',?1,44)",
                [&bootstrap_bytes],
            )
            .unwrap();
        let mut alternate = bootstrap.clone();
        alternate.instance = "other".into();
        let mut alternate_descriptor = descriptor;
        alternate_descriptor["instance"] = serde_json::json!("other");
        alternate.descriptor = alternate_descriptor.to_string();
        alternate.validate().unwrap();
        let alternate_bytes = serde_json::to_value(&alternate).unwrap().to_string();
        assert_ne!(alternate_bytes, serde_json::to_string(&alternate).unwrap());
        connection
            .execute(
                "INSERT INTO local_runtime_instances VALUES ('other',0,'selection',?1,45)",
                [&alternate_bytes],
            )
            .unwrap();
        connection
            .pragma_update(None, "application_id", APPLICATION_ID)
            .unwrap();
        connection.pragma_update(None, "user_version", 8).unwrap();
        drop(connection);

        let prior_cached = crate::local::runtime::state::State::new(&path)
            .cached("other", "selection")
            .unwrap()
            .unwrap();
        assert_eq!(
            serde_json::to_value(prior_cached).unwrap(),
            serde_json::to_value(&alternate).unwrap()
        );

        let passive = Metadata::inspect(&path).unwrap();
        assert_eq!(passive.authority().unwrap(), authority);
        drop(passive);
        let untouched = Connection::open(path.join(NAME)).unwrap();
        let created: i64 = untouched
            .query_row(
                "SELECT count(*) FROM sqlite_schema WHERE name LIKE 'connectors_er_%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(created, 0, "passive inspection cannot start migration");
        drop(untouched);

        drop(Metadata::update(&path, true).unwrap());
        assert_eq!(disk_version(&path), 9);
        assert_eq!(
            Metadata::inspect(&path).unwrap().authority().unwrap(),
            authority
        );
        let state = crate::local::runtime::state::State::new(&path);
        assert!(state.suppressed("instance").unwrap());
        let retained_runtime: (String, String, i64) = Metadata::inspect(&path)
            .unwrap()
            .connection
            .query_row(
                "SELECT selection,bootstrap,observed_at_ms FROM local_runtime_instances WHERE instance_id='instance'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(retained_runtime, ("selection".into(), bootstrap_bytes, 44));
        let retained_other: String = Metadata::inspect(&path)
            .unwrap()
            .connection
            .query_row(
                "SELECT bootstrap FROM local_runtime_instances WHERE instance_id='other'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&retained_other).unwrap(),
            serde_json::from_str::<serde_json::Value>(&alternate_bytes).unwrap()
        );
        assert_eq!(
            serde_json::to_value(state.cached("other", "selection").unwrap().unwrap()).unwrap(),
            serde_json::to_value(&alternate).unwrap()
        );
        let retained_binding: (String, String, String, String) = Metadata::inspect(&path)
            .unwrap()
            .connection
            .query_row(
                "SELECT connection_ref,scope_id,semantic_revision,publication_fence FROM registry_connections WHERE connection_ref='connection'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(
            retained_binding,
            (
                "connection".into(),
                "scope".into(),
                "semantic-revision".into(),
                "private-fence".into(),
            )
        );
        let metadata = Metadata::inspect(&path).unwrap();
        let mut audit_statement = metadata
            .connection
            .prepare("SELECT record_json FROM execution_audits ORDER BY audit_ref")
            .unwrap();
        let retained_audits = audit_statement
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .map(|row| serde_json::from_str::<serde_json::Value>(&row.unwrap()).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(retained_audits, audit_rows);
        drop(audit_statement);
        drop(metadata);
        state.suppress("instance", false).unwrap();
        drop(state);
        assert!(
            !crate::local::runtime::state::State::new(&path)
                .suppressed("instance")
                .unwrap()
        );

        let recovery = Connection::open(path.join(NAME)).unwrap();
        let retained: (String, i64) = recovery
            .query_row(
                "SELECT instance_id,epoch FROM registry_instances WHERE instance_id='instance'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(retained, ("instance".into(), 7));
    }

    #[test]
    fn revoked_legacy_connection_retains_both_read_use_histories_without_invented_admission() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("state");
        let authority = legacy_fixture(&path, 8);
        let database = Connection::open(path.join(NAME)).unwrap();
        database.pragma_update(None, "foreign_keys", true).unwrap();
        let binding = crate::local::registry::fixture_binding("instance");
        let generation = "a1df8a49-81fd-44c3-bcac-5085042a649d";
        let version = "d8391aa2-3ca8-4763-9b3d-d2fac9f35341";
        let dispatched_use = "6a49ddf5-76be-43d6-a7af-d7da384910e1";
        let cancelled_use = "e3a4d0c5-ea75-4e8e-a9ad-52421dd52712";
        database
            .execute(
                "INSERT INTO registry_instances VALUES ('instance','adapter','config',7)",
                [],
            )
            .unwrap();
        database
            .execute(
                "INSERT INTO registry_profiles VALUES ('profile','adapter',?1,?2,?3)",
                rusqlite::params![
                    binding.profile.id,
                    binding.profile.revision,
                    serde_json::to_string(&binding.profile).unwrap()
                ],
            )
            .unwrap();
        database
            .execute(
                "INSERT INTO registry_connections(connection_ref,instance_id,profile_key,binding,scope_id,semantic_revision,publication_fence,state,public,created_at_ms,revoked_at_ms) \
                 VALUES ('connection','instance','profile',?1,'7a57b043-81ed-440c-b3c6-1586fc8fe126','revision','fence','revoked',1,41,90)",
                [serde_json::to_string(&binding).unwrap()],
            )
            .unwrap();
        database
            .execute(
                "INSERT INTO registry_acquisitions(acquisition_ref,connection_ref,owner_token,expected_fence,state,created_at_ms,expires_at_ms,consumed_at_ms,generation_id,capture_id) \
                 VALUES ('acquisition','connection','owner-token','fence','completed',42,10000,43,?1,'capture')",
                [generation],
            )
            .unwrap();
        database
            .execute(
                "INSERT INTO registry_generations VALUES (?1,'connection','capture',?2)",
                rusqlite::params![
                    generation,
                    serde_json::to_string(&crate::local::registry::ExternalIdentity {
                        kind: "fixture".into(),
                        subject: "one".into(),
                    })
                    .unwrap()
                ],
            )
            .unwrap();
        database
            .execute(
                "INSERT INTO registry_materials(version_id,connection_ref,acquisition_ref,generation_id,acknowledged,acknowledged_at_ms,byte_size,retirement_fence,retired_at_ms,delete_not_before_ms) \
                 VALUES (?1,'connection','acquisition',?2,1,44,17,'retirement-fence',90,100000)",
                rusqlite::params![version, generation],
            )
            .unwrap();
        database
            .execute(
                "UPDATE registry_acquisitions SET candidate_id=?1 WHERE acquisition_ref='acquisition'",
                [version],
            )
            .unwrap();
        for (id, dispatched) in [(dispatched_use, true), (cancelled_use, false)] {
            database
                .execute(
                    "INSERT INTO registry_uses VALUES (?1,'connection',?2,?3,'fence',90000,?4,1)",
                    rusqlite::params![id, generation, version, dispatched],
                )
                .unwrap();
        }
        let read_uses = |database: &Connection| {
            database
                .prepare("SELECT use_id,connection_ref,generation_id,version_id,publication_fence,expires_at_ms,dispatched,released FROM registry_uses ORDER BY use_id")
                .unwrap()
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, bool>(6)?,
                        row.get::<_, bool>(7)?,
                    ))
                })
                .unwrap()
                .map(|row| row.unwrap())
                .collect::<Vec<_>>()
        };
        let original = read_uses(&database);
        drop(database);

        let migrated = Metadata::update(&path, true).unwrap();
        assert_eq!(migrated.authority().unwrap(), authority);
        assert_eq!(read_uses(&migrated.connection), original);
        let subjects = er::snapshot_facts(migrated.er.as_ref().unwrap());
        assert!(
            !subjects
                .iter()
                .any(|row| row.0 == "connectors.credential_evidence.DispatchAdmission")
        );
        let retained = subjects
            .iter()
            .filter(|row| row.0 == "connectors.credential_evidence.ReadUse")
            .collect::<Vec<_>>();
        assert_eq!(retained.len(), 2);
        for (_, _, state, fields) in retained {
            assert_eq!(state, "Released");
            assert_eq!(fields["released"], true);
            assert!(!fields.contains_key("evidence"));
            assert!(!fields.contains_key("request_id"));
            assert!(!fields.contains_key("operation_ref"));
            assert_eq!(fields["connection_ref"], "connection");
            assert_eq!(fields["generation_id"], generation);
            assert_eq!(fields["custody_version_ref"], version);
            assert_eq!(fields["publication_fence"], "fence");
        }
        drop(migrated);
        let reopened = Metadata::inspect(&path).unwrap();
        assert_eq!(read_uses(&reopened.connection), original);
        assert_eq!(er::snapshot_facts(reopened.er.as_ref().unwrap()), subjects);
    }

    #[test]
    fn every_recognized_legacy_level_uses_the_same_explicit_cutover() {
        for level in 1..=8 {
            let root = tempfile::tempdir().unwrap();
            let path = root.path().join(format!("level-{level}"));
            let authority = legacy_fixture(&path, level);
            assert_eq!(
                Metadata::inspect(&path).unwrap().authority().unwrap(),
                authority
            );
            drop(Metadata::update(&path, true).unwrap());
            assert_eq!(disk_version(&path), er::LEVEL);
            let physical = Connection::open(path.join(NAME)).unwrap();
            let retained_level: i64 = physical
                .query_row(
                    "SELECT source_level FROM connectors_er_authority WHERE singleton=1",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(retained_level, level);
            drop(physical);
            assert_eq!(
                Metadata::inspect(&path).unwrap().authority().unwrap(),
                authority
            );
        }
    }

    #[test]
    fn interrupted_import_and_committed_switch_resume_without_resetting_authority() {
        for point in [1, 2] {
            let root = tempfile::tempdir().unwrap();
            let path = root.path().join(format!("fault-{point}"));
            let authority = legacy_fixture(&path, 3);
            MIGRATION_FAULT.with(|fault| fault.set(point));
            let failure = match Metadata::update(&path, true) {
                Ok(_) => panic!("migration fault unexpectedly succeeded"),
                Err(failure) => failure,
            };
            assert_eq!(
                failure,
                if point == 1 {
                    Failure::MetadataUnavailable
                } else {
                    Failure::OutcomeUnknown
                }
            );
            assert_eq!(disk_version(&path), if point == 1 { 3 } else { er::LEVEL });
            drop(Metadata::update(&path, true).unwrap());
            assert_eq!(disk_version(&path), er::LEVEL);
            assert_eq!(
                Metadata::inspect(&path).unwrap().authority().unwrap(),
                authority
            );
        }
    }

    #[test]
    fn initialize_resumes_a_committed_switch_before_installing_its_required_projection() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("state");
        let authority = legacy_fixture(&path, 1);
        MIGRATION_FAULT.with(|fault| fault.set(2));
        assert!(matches!(
            Metadata::initialize(&path),
            Err(Failure::OutcomeUnknown)
        ));
        assert_eq!(disk_version(&path), er::LEVEL);

        let metadata = Metadata::initialize(&path).unwrap();
        assert_eq!(metadata.authority().unwrap(), authority);
        assert_eq!(
            metadata
                .connection
                .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
                .unwrap(),
            3
        );
        metadata.require_registry().unwrap();
    }

    #[test]
    fn altered_legacy_and_retained_recovery_schemas_are_refused() {
        let root = tempfile::tempdir().unwrap();
        let legacy = root.path().join("legacy");
        legacy_fixture(&legacy, 3);
        Connection::open(legacy.join(NAME))
            .unwrap()
            .execute_batch("CREATE TABLE undeclared(value TEXT);")
            .unwrap();
        assert!(matches!(
            Metadata::inspect(&legacy),
            Err(Failure::MetadataUnavailable)
        ));

        let recorded = root.path().join("recorded");
        fs::directory(&recorded, true, true).unwrap();
        drop(Metadata::initialize(&recorded).unwrap());
        Connection::open(recorded.join(NAME))
            .unwrap()
            .execute_batch("CREATE TABLE undeclared(value TEXT);")
            .unwrap();
        assert!(matches!(
            Metadata::inspect(&recorded),
            Err(Failure::MetadataUnavailable)
        ));
    }

    #[test]
    fn passive_v1_inspection_cannot_migrate_or_claim_an_empty_registry() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("state");
        let directory = fs::directory(&path, true, true).unwrap();
        fs::publish_new(&directory, OsStr::new(NAME), &[]).unwrap();
        fs::publish_new(&directory, OsStr::new(LOCK), &[]).unwrap();
        // Build the exact previous schema in a disposable database. This fixture
        // does not rewrite a current database or bypass a production migration.
        let connection = Connection::open(path.join(NAME)).unwrap();
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .unwrap();
        connection.execute_batch(MIGRATION).unwrap();
        let authority = uuid::Uuid::new_v4();
        connection
            .execute(
                "INSERT INTO local_authority VALUES (1,?1,?2)",
                rusqlite::params![authority.to_string(), fs::uid()],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO schema_migrations VALUES (1,?1)",
                [migration_digest()],
            )
            .unwrap();
        connection
            .pragma_update(None, "application_id", APPLICATION_ID)
            .unwrap();
        connection.pragma_update(None, "user_version", 1).unwrap();
        drop(connection);
        assert!(Metadata::inspect(&path).is_ok());
        assert!(matches!(
            Metadata::update(&path, false),
            Err(Failure::MetadataUnavailable)
        ));
        let registry = crate::local::registry::Registry::new(&path);
        assert!(matches!(
            registry.list(
                "one",
                "fixture",
                "cfg",
                crate::local::registry::PageOptions {
                    limit: 10,
                    cursor: None
                },
                1000,
                false
            ),
            Err(crate::local::registry::Failure::MetadataUnavailable)
        ));
        let metadata = Metadata::inspect(&path).unwrap();
        assert_eq!(metadata.authority().unwrap(), authority);
        assert_eq!(
            metadata
                .connection
                .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        drop(metadata);
        let metadata = Metadata::update(&path, true).unwrap();
        assert_eq!(metadata.authority().unwrap(), authority);
        metadata.require_registry().unwrap();
        drop(metadata);
        assert!(
            registry
                .list(
                    "one",
                    "fixture",
                    "cfg",
                    crate::local::registry::PageOptions {
                        limit: 10,
                        cursor: None
                    },
                    1000,
                    false
                )
                .unwrap()
                .connections
                .is_empty()
        );
    }

    #[test]
    fn runtime_migration_is_admitted_and_retains_the_v2_authority() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("state");
        let directory = fs::directory(&path, true, true).unwrap();
        fs::publish_new(&directory, OsStr::new(NAME), &[]).unwrap();
        fs::publish_new(&directory, OsStr::new(LOCK), &[]).unwrap();
        let connection = Connection::open(path.join(NAME)).unwrap();
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .unwrap();
        connection.execute_batch(MIGRATION).unwrap();
        connection.execute_batch(REGISTRY_MIGRATION).unwrap();
        let authority = uuid::Uuid::new_v4();
        connection
            .execute(
                "INSERT INTO local_authority VALUES (1,?1,?2)",
                rusqlite::params![authority.to_string(), fs::uid()],
            )
            .unwrap();
        for (version, source) in [(1, MIGRATION), (2, REGISTRY_MIGRATION)] {
            connection
                .execute(
                    "INSERT INTO schema_migrations VALUES (?1,?2)",
                    rusqlite::params![version, hex::encode(Sha256::digest(source.as_bytes()))],
                )
                .unwrap();
        }
        connection
            .pragma_update(None, "application_id", APPLICATION_ID)
            .unwrap();
        connection.pragma_update(None, "user_version", 2).unwrap();
        drop(connection);
        let passive = Metadata::inspect(&path).unwrap();
        assert_eq!(passive.authority().unwrap(), authority);
        assert_eq!(
            passive
                .connection
                .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            2
        );
        drop(passive);
        let active = Metadata::update(&path, true).unwrap();
        assert_eq!(active.authority().unwrap(), authority);
        assert_eq!(
            active
                .connection
                .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            3
        );
        drop(active);
        let state = crate::local::runtime::state::State::new(&path);
        state.suppress("fixture", true).unwrap();
        drop(state);
        assert!(
            crate::local::runtime::state::State::new(&path)
                .suppressed("fixture")
                .unwrap()
        );
    }
}

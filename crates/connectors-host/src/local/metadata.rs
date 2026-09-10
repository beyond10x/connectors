//! A single local SQLite authority. Schema installation grants no connection,
//! credential custody, approval or business dispatch authority.
use super::{Failure, Result, filesystem as fs};
use rusqlite::{Connection, OpenFlags, TransactionBehavior};
use sha2::{Digest, Sha256};
use std::{
    ffi::OsStr,
    os::fd::AsRawFd,
    path::Path,
    time::{Duration, Instant},
};

const APPLICATION_ID: i64 = 0x434e4354;
const MIGRATION: &str = "CREATE TABLE local_authority (singleton INTEGER PRIMARY KEY CHECK(singleton=1), authority_id TEXT NOT NULL UNIQUE, owner_uid INTEGER NOT NULL); CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, digest TEXT NOT NULL);";
const REGISTRY_MIGRATION: &str = include_str!("metadata/registry.sql");
const RUNTIME_MIGRATION: &str = include_str!("metadata/runtime.sql");
const MUTATION_MIGRATION: &str = include_str!("metadata/mutations.sql");
const NAME: &str = "metadata.sqlite3";
const LOCK: &str = "metadata.lock";

pub struct Metadata {
    // Field drop order matters: SQLite closes and retires sidecars while the
    // lifecycle lock is still held. Releasing at open/validate left a race with
    // another process admitting a sidecar that the last connection was deleting.
    pub(super) connection: Connection,
    _directory: std::fs::File,
    _lifecycle_lock: std::fs::File,
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
        metadata.migrate(3)?;
        Ok(metadata)
    }

    /// Inspection never creates a database, migrates it, or interprets absence
    /// as an empty authoritative registry.
    pub fn inspect(path: &Path) -> Result<Self> {
        let dir = fs::directory(path, false, true).map_err(|_| Failure::MetadataUnavailable)?;
        let lock = lifecycle_lock(&dir)?;
        let metadata = Self::open_connection(path, dir, lock, true)?;
        metadata.validate()?;
        Ok(metadata)
    }

    /// An admitted mutating action may upgrade an existing, recognized database.
    /// Missing authority is never recreated by connection management.
    pub(super) fn update(path: &Path, migrate: bool) -> Result<Self> {
        let dir = fs::directory(path, false, true).map_err(|_| Failure::MetadataUnavailable)?;
        let lock = lifecycle_lock(&dir)?;
        let mut metadata = Self::open_connection(path, dir, lock, false)?;
        metadata.validate()?;
        if migrate {
            metadata.migrate(3)?;
        }
        metadata.require_registry()?;
        Ok(metadata)
    }

    /// Only the admitted mutation port installs its schema. Existing setup and
    /// read owners must not upgrade storage for an unused business-write port.
    pub(super) fn update_mutations(path: &Path) -> Result<Self> {
        let mut metadata = Self::update(path, true)?;
        metadata.migrate(4)?;
        Ok(metadata)
    }

    pub(super) fn require_registry(&self) -> Result<()> {
        let version: i64 = self
            .connection
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(unavailable)?;
        if !(2..=4).contains(&version) {
            return Err(Failure::MetadataUnavailable);
        }
        Ok(())
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
        self._directory
            .sync_all()
            .map_err(|_| Failure::OutcomeUnknown)?;
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
        fs::private_file_at(&dir, OsStr::new(NAME)).map_err(|_| Failure::MetadataUnavailable)?;
        for suffix in ["-wal", "-shm", "-journal"] {
            let name = format!("{NAME}{suffix}");
            match std::fs::symlink_metadata(path.join(&name)) {
                Ok(_) => {
                    fs::private_file_at(&dir, OsStr::new(&name))
                        .map_err(|_| Failure::MetadataUnavailable)?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => return Err(Failure::MetadataUnavailable),
            }
        }
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
        connection
            .busy_timeout(Duration::from_secs(2))
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
            _directory: dir,
            _lifecycle_lock: lock,
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
        if app != APPLICATION_ID || !(1..=4).contains(&version) || mode != "wal" {
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
        let mut expected = vec![(1, migration_digest())];
        if version >= 2 {
            expected.push((
                2,
                hex::encode(Sha256::digest(REGISTRY_MIGRATION.as_bytes())),
            ));
        }
        if version >= 3 {
            expected.push((3, hex::encode(Sha256::digest(RUNTIME_MIGRATION.as_bytes()))));
        }
        if version >= 4 {
            expected.push((
                4,
                hex::encode(Sha256::digest(MUTATION_MIGRATION.as_bytes())),
            ));
        }
        if migrations != expected {
            return Err(Failure::MetadataUnavailable);
        }
        Ok(())
    }
}

fn migration_digest() -> String {
    hex::encode(Sha256::digest(MIGRATION.as_bytes()))
}

fn unavailable(_: rusqlite::Error) -> Failure {
    Failure::MetadataUnavailable
}

// Serialize metadata handle lifetime, including initial WAL installation and
// SQLite's last-close sidecar retirement. This is only an OS lock, released on
// crash; all durable authority remains in SQLite. These bounded setup/inspection
// handles must not be retained across provider work.
fn lifecycle_lock(directory: &std::fs::File) -> Result<std::fs::File> {
    let file = fs::private_file_at(directory, OsStr::new(LOCK))
        .map_err(|_| Failure::MetadataUnavailable)?;
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        // SAFETY: file owns the live descriptor. Dropping it releases the lock.
        let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_NB | libc::LOCK_EX) };
        if result == 0 {
            return Ok(file);
        }
        if std::io::Error::last_os_error().kind() != std::io::ErrorKind::WouldBlock
            || Instant::now() >= deadline
        {
            return Err(Failure::MetadataUnavailable);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

#![forbid(unsafe_code)]

//! **SQLite-backed state cells**: `:memory:` for tests, a file for a workstation.
//!
//! # Why SQLite rather than files
//!
//! Local state used to be owner-only files beside the socket, and that is a different *semantic*
//! from the hosted PostgreSQL backend, not just a different medium: no transactions, different
//! concurrency, no atomic read-modify-write. Testing against files and shipping on PostgreSQL tests
//! a different thing. SQLite and PostgreSQL are both SQL and both transactional, so a bug that
//! appears in one is far likelier to appear in the other — and the shared
//! [`connector_state::conformance`] suite is what turns "likelier" into "checked".
//!
//! It also makes every Integration testable without a database. `integration-jira` cannot be
//! composed on a workstation at all today, because its only constructor takes a concrete
//! `PostgresState`; with a port and an in-memory backend, that stops being a category of code that
//! only CI can run.
//!
//! # `rusqlite`, bundled
//!
//! The same crate and version this family already uses in `substrate` and `identity`. Bundled, so
//! the build does not depend on a system `libsqlite3` — the connectors image is distroless and has
//! none.
//!
//! # Why `append` is a transaction rather than one clever statement
//!
//! The obvious SQLite append is `ON CONFLICT DO UPDATE SET body = body || excluded.body`. It is
//! wrong here: SQLite's `||` operates on text, and applying it to a BLOB coerces both operands to
//! TEXT — which silently truncates at the first zero byte and mangles anything that is not valid
//! UTF-8. State cells carry whatever an Integration encoded. So the append reads, checks the bound,
//! and writes inside one immediate transaction, which is both correct for binary and atomic.

use std::path::Path;
use std::sync::Mutex;

use connector_state::{validate_key, validate_request, StateError, StateStore};
use rusqlite::{Connection, OptionalExtension as _};

/// State cells in one SQLite database.
pub struct SqliteState {
    connection: Mutex<Connection>,
}

enum Synchronization {
    Normal,
    Full,
}

impl SqliteState {
    /// Open or create a database file.
    ///
    /// # Errors
    ///
    /// [`StateError::Unavailable`] when the file cannot be opened or the schema cannot be created.
    pub fn open(path: &Path) -> Result<Self, StateError> {
        let connection = Connection::open(path).map_err(|_| StateError::Unavailable)?;
        Self::prepare(connection, Synchronization::Normal)
    }

    /// Open a file-backed WAL database with SQLite's FULL synchronization at every commit.
    ///
    /// Use this port for decision journals whose committed record must be synchronized before
    /// another durable effect. The mode belongs to this connection: every journal writer must
    /// choose it explicitly. [`Self::open`] and [`Self::in_memory`] retain NORMAL synchronization.
    ///
    /// # Errors
    ///
    /// [`StateError::Unavailable`] if opening, WAL/FULL configuration or schema creation fails.
    /// In-memory and temporary databases cannot enter WAL mode and are refused.
    pub fn open_full(path: &Path) -> Result<Self, StateError> {
        let connection = Connection::open(path).map_err(|_| StateError::Unavailable)?;
        Self::prepare(connection, Synchronization::Full)
    }

    /// A database that lives only as long as this value. For tests, and for a deployment that
    /// wants nothing on disk.
    ///
    /// # Errors
    ///
    /// [`StateError::Unavailable`] when SQLite cannot allocate it.
    pub fn in_memory() -> Result<Self, StateError> {
        let connection = Connection::open_in_memory().map_err(|_| StateError::Unavailable)?;
        Self::prepare(connection, Synchronization::Normal)
    }

    fn prepare(
        connection: Connection,
        synchronization: Synchronization,
    ) -> Result<Self, StateError> {
        // WAL preserves concurrent readers. Existing openers keep NORMAL, which can lose the
        // last commit on power loss; decision-journal callers explicitly select FULL instead.
        connection
            .execute_batch("PRAGMA journal_mode = WAL;")
            .map_err(|_| StateError::Unavailable)?;
        let synchronous = match synchronization {
            Synchronization::Normal => "NORMAL",
            Synchronization::Full => "FULL",
        };
        connection
            .pragma_update(None, "synchronous", synchronous)
            .map_err(|_| StateError::Unavailable)?;
        if matches!(synchronization, Synchronization::Full) {
            let journal_mode: String = connection
                .pragma_query_value(None, "journal_mode", |row| row.get(0))
                .map_err(|_| StateError::Unavailable)?;
            if journal_mode != "wal" {
                return Err(StateError::Unavailable);
            }
        }
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON;
                 CREATE TABLE IF NOT EXISTS connector_state_cells (
                     state_key TEXT PRIMARY KEY NOT NULL,
                     body BLOB NOT NULL,
                     updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
                     CHECK (length(state_key) BETWEEN 1 AND 128)
                 );",
            )
            .map_err(|_| StateError::Unavailable)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
}

impl StateStore for SqliteState {
    fn read(&self, key: &str, maximum: usize) -> Result<Option<Vec<u8>>, StateError> {
        validate_request(key, maximum)?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| StateError::Unavailable)?;
        let body: Option<Vec<u8>> = connection
            .query_row(
                "SELECT body FROM connector_state_cells WHERE state_key = ?1",
                [key],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| StateError::Unavailable)?;
        match body {
            Some(body) if body.len() > maximum => Err(StateError::Capacity),
            other => Ok(other),
        }
    }

    fn replace(&self, key: &str, body: &[u8], maximum: usize) -> Result<(), StateError> {
        validate_request(key, maximum)?;
        if body.len() > maximum {
            return Err(StateError::Capacity);
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| StateError::Unavailable)?;
        connection
            .execute(
                "INSERT INTO connector_state_cells (state_key, body)
                 VALUES (?1, ?2)
                 ON CONFLICT(state_key) DO UPDATE
                 SET body = excluded.body, updated_at = unixepoch()",
                rusqlite::params![key, body],
            )
            .map(|_| ())
            .map_err(|_| StateError::Unavailable)
    }

    fn append(&self, key: &str, suffix: &[u8], maximum: usize) -> Result<usize, StateError> {
        validate_request(key, maximum)?;
        if suffix.len() > maximum {
            return Err(StateError::Capacity);
        }
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| StateError::Unavailable)?;
        // `Immediate` takes the write lock at BEGIN rather than at first write, so a concurrent
        // appender cannot read the same length and then both write.
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| StateError::Unavailable)?;
        let existing: Option<Vec<u8>> = transaction
            .query_row(
                "SELECT body FROM connector_state_cells WHERE state_key = ?1",
                [key],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| StateError::Unavailable)?;
        let mut body = existing.unwrap_or_default();
        // Checked before anything is written, so a refusal rolls back nothing and the cell is
        // exactly as it was.
        if body.len() + suffix.len() > maximum {
            return Err(StateError::Capacity);
        }
        body.extend_from_slice(suffix);
        transaction
            .execute(
                "INSERT INTO connector_state_cells (state_key, body)
                 VALUES (?1, ?2)
                 ON CONFLICT(state_key) DO UPDATE
                 SET body = excluded.body, updated_at = unixepoch()",
                rusqlite::params![key, &body],
            )
            .map_err(|_| StateError::Unavailable)?;
        transaction.commit().map_err(|_| StateError::Unavailable)?;
        Ok(body.len())
    }

    fn delete(&self, key: &str) -> Result<(), StateError> {
        validate_key(key)?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| StateError::Unavailable)?;
        connection
            .execute(
                "DELETE FROM connector_state_cells WHERE state_key = ?1",
                [key],
            )
            .map(|_| ())
            .map_err(|_| StateError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(store: &SqliteState) -> (String, u32, bool) {
        let connection = store.connection.lock().expect("connection lock");
        (
            connection
                .pragma_query_value(None, "journal_mode", |row| row.get(0))
                .unwrap(),
            connection
                .pragma_query_value(None, "synchronous", |row| row.get(0))
                .unwrap(),
            connection
                .pragma_query_value(None, "foreign_keys", |row| row.get(0))
                .unwrap(),
        )
    }

    #[test]
    fn full_open_configures_wal_and_full_synchronization_on_every_open() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("journal.db");
        for _ in 0..2 {
            let full = SqliteState::open_full(&path).unwrap();
            assert_eq!(settings(&full), ("wal".into(), 2, true));
            let normal = SqliteState::open(&path).unwrap();
            assert_eq!(settings(&normal), ("wal".into(), 1, true));
            assert_eq!(
                settings(&full),
                ("wal".into(), 2, true),
                "another opener must not lower this connection's synchronization"
            );
        }
    }

    #[test]
    fn existing_openers_keep_normal_synchronization() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("legacy.db");
        assert_eq!(
            settings(&SqliteState::open(&path).unwrap()),
            ("wal".into(), 1, true)
        );
        assert_eq!(
            settings(&SqliteState::in_memory().unwrap()),
            ("memory".into(), 1, true)
        );
    }

    #[test]
    fn the_full_file_backend_preserves_state_and_grant_conformance() {
        let directory = tempfile::tempdir().unwrap();
        let store = SqliteState::open_full(&directory.path().join("journal.db")).unwrap();
        connector_state::conformance::run(&store);
        domain::grant_conformance::run(std::sync::Arc::new(store));
    }

    #[test]
    fn full_commits_are_visible_before_close_and_survive_reopening() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("journal.db");
        let value = b"\x00decision\xff";
        {
            let store = SqliteState::open_full(&path).unwrap();
            assert_eq!(settings(&store).1, 2);
            store
                .replace("custody.decision", b"\x00decision", 32)
                .unwrap();
            assert_eq!(
                store.append("custody.decision", b"\xff", 32),
                Ok(value.len())
            );
            assert_eq!(
                store.append("custody.decision", b"overflow", value.len()),
                Err(StateError::Capacity)
            );
            // An independent connection sees the committed record while the writer is still open.
            let reader = Connection::open(&path).unwrap();
            let body: Vec<u8> = reader
                .query_row(
                    "SELECT body FROM connector_state_cells WHERE state_key = 'custody.decision'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(body, value);
        }
        {
            let reopened = SqliteState::open_full(&path).unwrap();
            assert_eq!(settings(&reopened), ("wal".into(), 2, true));
            assert_eq!(
                reopened.read("custody.decision", 32),
                Ok(Some(value.to_vec()))
            );
            reopened.delete("custody.decision").unwrap();
        }
        assert_eq!(
            SqliteState::open_full(&path)
                .unwrap()
                .read("custody.decision", 32),
            Ok(None)
        );
    }

    #[test]
    fn full_open_refuses_unusable_paths() {
        assert!(matches!(
            SqliteState::open_full(Path::new(":memory:")),
            Err(StateError::Unavailable)
        ));
        assert!(matches!(
            SqliteState::open_full(Path::new("")),
            Err(StateError::Unavailable)
        ));
        let directory = tempfile::tempdir().unwrap();
        assert!(matches!(
            SqliteState::open_full(directory.path()),
            Err(StateError::Unavailable)
        ));
        assert!(matches!(
            SqliteState::open_full(&directory.path().join("missing/journal.db")),
            Err(StateError::Unavailable)
        ));
    }

    #[test]
    fn the_in_memory_backend_conforms() {
        connector_state::conformance::run(&SqliteState::in_memory().expect("in-memory database"));
    }

    #[test]
    fn the_file_backend_conforms() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let store = SqliteState::open(&directory.path().join("state.db")).expect("a database file");
        connector_state::conformance::run(&store);
    }

    #[test]
    fn the_in_memory_backend_serves_grant_evaluation() {
        domain::grant_conformance::run(std::sync::Arc::new(
            SqliteState::in_memory().expect("in-memory database"),
        ));
    }

    #[test]
    fn the_file_backend_serves_grant_evaluation() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let store = SqliteState::open(&directory.path().join("state.db")).expect("a database file");
        domain::grant_conformance::run(std::sync::Arc::new(store));
    }

    #[test]
    fn a_cell_survives_reopening_the_file() {
        // The property the whole crate exists for on a workstation. Asserted rather than assumed,
        // because a `journal_mode` or `synchronous` change could quietly cost it.
        let directory = tempfile::tempdir().expect("a temporary directory");
        let path = directory.path().join("state.db");
        {
            let store = SqliteState::open(&path).expect("first open");
            store
                .replace("survives.restart", b"\x00\xffbytes", 64)
                .expect("write");
        }
        let store = SqliteState::open(&path).expect("second open");
        assert_eq!(
            store.read("survives.restart", 64),
            Ok(Some(b"\x00\xffbytes".to_vec())),
            "and byte-exact: the value contains a zero byte and invalid UTF-8 on purpose"
        );
    }

    #[test]
    fn concatenation_would_have_corrupted_binary_and_the_transaction_does_not() {
        // The specific bug this implementation avoids: SQLite's `||` coerces BLOB operands to TEXT,
        // truncating at the first zero byte. Appending across one is the case that would expose it.
        let store = SqliteState::in_memory().expect("in-memory database");
        store
            .append("binary.append", b"\x00\x01", 64)
            .expect("first");
        store
            .append("binary.append", b"\x00\xfe", 64)
            .expect("second");
        assert_eq!(
            store.read("binary.append", 64),
            Ok(Some(vec![0, 1, 0, 254])),
            "four bytes, two of them zero"
        );
    }
}

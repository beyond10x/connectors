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

impl SqliteState {
    /// Open or create a database file.
    ///
    /// # Errors
    ///
    /// [`StateError::Unavailable`] when the file cannot be opened or the schema cannot be created.
    pub fn open(path: &Path) -> Result<Self, StateError> {
        let connection = Connection::open(path).map_err(|_| StateError::Unavailable)?;
        Self::prepare(connection)
    }

    /// A database that lives only as long as this value. For tests, and for a deployment that
    /// wants nothing on disk.
    ///
    /// # Errors
    ///
    /// [`StateError::Unavailable`] when SQLite cannot allocate it.
    pub fn in_memory() -> Result<Self, StateError> {
        let connection = Connection::open_in_memory().map_err(|_| StateError::Unavailable)?;
        Self::prepare(connection)
    }

    fn prepare(connection: Connection) -> Result<Self, StateError> {
        connection
            .execute_batch(
                // WAL for a workstation: a reader — `connectors doctor`, or a second one-shot
                // command — must not block on the daemon's writes. `synchronous = NORMAL` is the
                // documented companion to WAL and survives process death, which is the failure
                // that matters here; only host power loss can lose the last commit.
                "PRAGMA journal_mode = WAL;
                 PRAGMA synchronous = NORMAL;
                 PRAGMA foreign_keys = ON;
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
        let connection = self.connection.lock().map_err(|_| StateError::Unavailable)?;
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
        let connection = self.connection.lock().map_err(|_| StateError::Unavailable)?;
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
        let mut connection = self.connection.lock().map_err(|_| StateError::Unavailable)?;
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
        let connection = self.connection.lock().map_err(|_| StateError::Unavailable)?;
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
    fn a_cell_survives_reopening_the_file() {
        // The property the whole crate exists for on a workstation. Asserted rather than assumed,
        // because a `journal_mode` or `synchronous` change could quietly cost it.
        let directory = tempfile::tempdir().expect("a temporary directory");
        let path = directory.path().join("state.db");
        {
            let store = SqliteState::open(&path).expect("first open");
            store.replace("survives.restart", b"\x00\xffbytes", 64).expect("write");
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
        store.append("binary.append", b"\x00\x01", 64).expect("first");
        store.append("binary.append", b"\x00\xfe", 64).expect("second");
        assert_eq!(
            store.read("binary.append", 64),
            Ok(Some(vec![0, 1, 0, 254])),
            "four bytes, two of them zero"
        );
    }
}

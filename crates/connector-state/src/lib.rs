#![forbid(unsafe_code)]

//! **Where an Integration's durable state lives, as a port.**
//!
//! # Why this exists
//!
//! Secrets became a port ([`connector_secrets::SecretStore`]) and egress became a port
//! (`service::EgressTransport`). State never did, so each Integration answered "where does my state
//! live" in its own shape:
//!
//! ```text
//! slack        Option<PostgresState> + state_root: PathBuf
//! gitlab       enum GitlabState { Postgres, Local { root } }
//! jira         state_store: PostgresState        ← no local branch at all
//! b10x   enum PersistedState + Option<PostgresState>
//! sip          Option<PostgresState>
//! monitoring   state_root: PathBuf only
//! ```
//!
//! Because the shapes differ, composition cannot be uniform: `connectors-runtime` has two
//! hand-written ladders, one per posture, each of which has to know which shape each Integration
//! wants. And `jira` cannot be composed on a workstation at all — not by policy, but because nobody
//! wrote the other branch.
//!
//! With one port, **posture becomes which backend a deployment binds, not which branch it takes**.
//!
//! # The surface is deliberately not SQL
//!
//! Four operations over keyed byte cells. That is what the hosted PostgreSQL implementation already
//! was — bounded cells, not a schema — and keeping it that narrow is what lets a SQLite backend be
//! a faithful substitute rather than an approximation. SQLite and PostgreSQL diverge on type
//! affinity, concurrent writers and `SELECT FOR UPDATE`; a cell API never exposes any of it, and a
//! SQL-passthrough port would leak all three into every Integration.
//!
//! # Conformance, not hope
//!
//! [`conformance`] is a suite every backend runs against itself. Without it "the SQLite backend
//! behaves like the PostgreSQL one" is a claim in a comment; with it, it is a test that fails.
//! [`append`](StateStore::append) is the reason it matters: it is atomic, and it must leave the
//! cell **unchanged** when the result would exceed the caller's bound, which is easy to implement
//! three subtly different ways.

use std::collections::BTreeMap;
use std::sync::Mutex;

/// The longest state key any backend accepts.
pub const MAX_KEY_BYTES: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum StateError {
    #[error("state request is invalid")]
    Invalid,
    #[error("state is unavailable")]
    Unavailable,
    #[error("state exceeded its configured byte bound")]
    Capacity,
}

/// Durable keyed byte cells.
///
/// Synchronous by design: every implementation serializes its own access, and an Integration that
/// reads its connection list on a control-plane request has nothing useful to do while it waits.
/// The hosted PostgreSQL backend was already shaped this way.
pub trait StateStore: Send + Sync {
    /// The cell's bytes, or `None` when it has never been written.
    ///
    /// A cell longer than `maximum` is [`StateError::Capacity`] rather than a truncated read: a
    /// caller that asked for a bound and got a prefix would parse garbage.
    fn read(&self, key: &str, maximum: usize) -> Result<Option<Vec<u8>>, StateError>;

    /// Replace the cell's bytes, creating it when absent.
    fn replace(&self, key: &str, body: &[u8], maximum: usize) -> Result<(), StateError>;

    /// Append atomically and return the resulting cell length.
    ///
    /// **Refuses without writing** when the result would exceed `maximum`. An implementation that
    /// appends and then notices has already corrupted an append-only log.
    fn append(&self, key: &str, suffix: &[u8], maximum: usize) -> Result<usize, StateError>;

    /// Remove the cell. Removing an absent cell succeeds — a caller cleaning up should not have to
    /// know whether it got there first.
    fn delete(&self, key: &str) -> Result<(), StateError>;
}

/// The key grammar every backend enforces identically.
///
/// Closed and lowercase so a key is safe to embed in any backend's addressing without quoting, and
/// bounded so a key cannot become the payload.
///
/// # Errors
///
/// [`StateError::Invalid`] for an empty, over-long, or out-of-grammar key.
pub fn validate_key(key: &str) -> Result<(), StateError> {
    if key.is_empty()
        || key.len() > MAX_KEY_BYTES
        || !key.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._:-".contains(&byte)
        })
    {
        return Err(StateError::Invalid);
    }
    Ok(())
}

/// A key, plus a bound that must be a bound.
///
/// # Errors
///
/// [`StateError::Invalid`] for a bad key or a zero maximum — zero would make every operation fail
/// as `Capacity`, which reads as a full store rather than a caller mistake.
pub fn validate_request(key: &str, maximum: usize) -> Result<(), StateError> {
    validate_key(key)?;
    if maximum == 0 {
        return Err(StateError::Invalid);
    }
    Ok(())
}

/// State that does not survive the process.
///
/// For tests, and for a deployment that genuinely wants nothing on disk. It is a real backend
/// rather than a stub: it runs the same conformance suite as the other two.
#[derive(Debug, Default)]
pub struct MemoryState {
    cells: Mutex<BTreeMap<String, Vec<u8>>>,
}

impl MemoryState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl StateStore for MemoryState {
    fn read(&self, key: &str, maximum: usize) -> Result<Option<Vec<u8>>, StateError> {
        validate_request(key, maximum)?;
        let cells = self.cells.lock().map_err(|_| StateError::Unavailable)?;
        match cells.get(key) {
            Some(body) if body.len() > maximum => Err(StateError::Capacity),
            Some(body) => Ok(Some(body.clone())),
            None => Ok(None),
        }
    }

    fn replace(&self, key: &str, body: &[u8], maximum: usize) -> Result<(), StateError> {
        validate_request(key, maximum)?;
        if body.len() > maximum {
            return Err(StateError::Capacity);
        }
        let mut cells = self.cells.lock().map_err(|_| StateError::Unavailable)?;
        cells.insert(key.to_owned(), body.to_vec());
        Ok(())
    }

    fn append(&self, key: &str, suffix: &[u8], maximum: usize) -> Result<usize, StateError> {
        validate_request(key, maximum)?;
        if suffix.len() > maximum {
            return Err(StateError::Capacity);
        }
        let mut cells = self.cells.lock().map_err(|_| StateError::Unavailable)?;
        let existing = cells.get(key).map_or(0, Vec::len);
        // Checked before the mutation, not after: the cell must be unchanged when this refuses.
        if existing + suffix.len() > maximum {
            return Err(StateError::Capacity);
        }
        let cell = cells.entry(key.to_owned()).or_default();
        cell.extend_from_slice(suffix);
        Ok(cell.len())
    }

    fn delete(&self, key: &str) -> Result<(), StateError> {
        validate_key(key)?;
        let mut cells = self.cells.lock().map_err(|_| StateError::Unavailable)?;
        cells.remove(key);
        Ok(())
    }
}

pub mod conformance;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_memory_backend_conforms() {
        conformance::run(&MemoryState::new());
    }

    #[test]
    fn the_key_grammar_is_closed() {
        for valid in ["slack.connections", "b10x:work-events", "vault_journal-v1"] {
            assert!(validate_key(valid).is_ok(), "{valid}");
        }
        for invalid in ["", "Upper", "has space", "semi;colon", "sl/ash"] {
            assert_eq!(validate_key(invalid), Err(StateError::Invalid), "{invalid:?}");
        }
        assert!(validate_key(&"a".repeat(MAX_KEY_BYTES)).is_ok());
        assert_eq!(
            validate_key(&"a".repeat(MAX_KEY_BYTES + 1)),
            Err(StateError::Invalid)
        );
    }

    #[test]
    fn a_zero_bound_is_a_caller_mistake_not_a_full_store() {
        assert_eq!(validate_request("key", 0), Err(StateError::Invalid));
    }
}

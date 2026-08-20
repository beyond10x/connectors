#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc)] // This crate is an internal hosted persistence boundary.

//! Bounded PostgreSQL-backed state cells for hosted Connector integrations.
//!
//! The database owns every non-configurable durable value. Integrations retain their existing
//! domain-specific validation and encoding; this crate supplies one serialized database worker,
//! exact byte bounds, and atomic replace/append operations without leaking connection details.

use std::sync::{mpsc, Arc};

use postgres::{Client, NoTls};

const MAX_KEY_BYTES: usize = 128;
const REQUEST_QUEUE_DEPTH: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum StateError {
    #[error("hosted state configuration is invalid")]
    Invalid,
    #[error("hosted state is unavailable")]
    Unavailable,
    #[error("hosted state exceeded its configured byte bound")]
    Capacity,
}

#[derive(Clone)]
pub struct PostgresState {
    inner: Arc<Inner>,
}

struct Inner {
    requests: mpsc::SyncSender<Request>,
}

enum Request {
    Read {
        key: String,
        maximum: usize,
        response: mpsc::Sender<Result<Option<Vec<u8>>, StateError>>,
    },
    Replace {
        key: String,
        body: Vec<u8>,
        maximum: usize,
        response: mpsc::Sender<Result<(), StateError>>,
    },
    Append {
        key: String,
        suffix: Vec<u8>,
        maximum: usize,
        response: mpsc::Sender<Result<usize, StateError>>,
    },
    Delete {
        key: String,
        response: mpsc::Sender<Result<(), StateError>>,
    },
}

impl PostgresState {
    /// Connect to one service-owned PostgreSQL database and bootstrap its fresh schema.
    pub fn connect(database_url: &str) -> Result<Self, StateError> {
        if database_url.trim().is_empty() {
            return Err(StateError::Invalid);
        }
        let (requests, receiver) = mpsc::sync_channel(REQUEST_QUEUE_DEPTH);
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let database_url = database_url.to_owned();
        std::thread::Builder::new()
            .name("connectors-postgresql-state".to_owned())
            .spawn(move || {
                let client = Client::connect(&database_url, NoTls)
                    .and_then(|mut client| {
                        bootstrap_schema(&mut client)?;
                        Ok(client)
                    })
                    .map_err(|_| StateError::Unavailable);
                match client {
                    Ok(mut client) => {
                        let _ = ready_sender.send(Ok(()));
                        serve(&mut client, receiver);
                    }
                    Err(error) => {
                        let _ = ready_sender.send(Err(error));
                    }
                }
            })
            .map_err(|_| StateError::Unavailable)?;
        ready_receiver
            .recv()
            .map_err(|_| StateError::Unavailable)??;
        Ok(Self {
            inner: Arc::new(Inner { requests }),
        })
    }

    pub fn read(&self, key: &str, maximum: usize) -> Result<Option<Vec<u8>>, StateError> {
        validate_request(key, maximum)?;
        let (response, receiver) = mpsc::channel();
        self.inner
            .requests
            .send(Request::Read {
                key: key.to_owned(),
                maximum,
                response,
            })
            .map_err(|_| StateError::Unavailable)?;
        receiver.recv().map_err(|_| StateError::Unavailable)?
    }

    pub fn replace(&self, key: &str, body: &[u8], maximum: usize) -> Result<(), StateError> {
        validate_request(key, maximum)?;
        if body.len() > maximum {
            return Err(StateError::Capacity);
        }
        let (response, receiver) = mpsc::channel();
        self.inner
            .requests
            .send(Request::Replace {
                key: key.to_owned(),
                body: body.to_vec(),
                maximum,
                response,
            })
            .map_err(|_| StateError::Unavailable)?;
        receiver.recv().map_err(|_| StateError::Unavailable)?
    }

    /// Append bytes atomically and return the resulting cell length.
    pub fn append(&self, key: &str, suffix: &[u8], maximum: usize) -> Result<usize, StateError> {
        validate_request(key, maximum)?;
        if suffix.len() > maximum {
            return Err(StateError::Capacity);
        }
        let (response, receiver) = mpsc::channel();
        self.inner
            .requests
            .send(Request::Append {
                key: key.to_owned(),
                suffix: suffix.to_vec(),
                maximum,
                response,
            })
            .map_err(|_| StateError::Unavailable)?;
        receiver.recv().map_err(|_| StateError::Unavailable)?
    }

    pub fn delete(&self, key: &str) -> Result<(), StateError> {
        validate_key(key)?;
        let (response, receiver) = mpsc::channel();
        self.inner
            .requests
            .send(Request::Delete {
                key: key.to_owned(),
                response,
            })
            .map_err(|_| StateError::Unavailable)?;
        receiver.recv().map_err(|_| StateError::Unavailable)?
    }
}

/// The hosted backend behind the shared port.
///
/// Additive: `PostgresState`'s inherent methods stay, because existing Integrations call them
/// directly and will move to the port one at a time. What this gains is that a deployment can hold
/// `Arc<dyn StateStore>` without knowing which backend it bound — the property that collapses two
/// composition ladders into one.
///
/// The errors are re-mapped rather than shared because this crate's `StateError` predates the port
/// and is part of its published surface. The variants correspond one to one.
impl connector_state::StateStore for PostgresState {
    fn read(
        &self,
        key: &str,
        maximum: usize,
    ) -> Result<Option<Vec<u8>>, connector_state::StateError> {
        Self::read(self, key, maximum).map_err(port_error)
    }

    fn replace(
        &self,
        key: &str,
        body: &[u8],
        maximum: usize,
    ) -> Result<(), connector_state::StateError> {
        Self::replace(self, key, body, maximum).map_err(port_error)
    }

    fn append(
        &self,
        key: &str,
        suffix: &[u8],
        maximum: usize,
    ) -> Result<usize, connector_state::StateError> {
        Self::append(self, key, suffix, maximum).map_err(port_error)
    }

    fn delete(&self, key: &str) -> Result<(), connector_state::StateError> {
        Self::delete(self, key).map_err(port_error)
    }
}

const fn port_error(error: StateError) -> connector_state::StateError {
    match error {
        StateError::Invalid => connector_state::StateError::Invalid,
        StateError::Unavailable => connector_state::StateError::Unavailable,
        StateError::Capacity => connector_state::StateError::Capacity,
    }
}

fn bootstrap_schema(client: &mut Client) -> Result<(), postgres::Error> {
    client.batch_execute(
        "CREATE TABLE IF NOT EXISTS connector_state_cells (
            state_key TEXT PRIMARY KEY,
            body BYTEA NOT NULL,
            updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
            CONSTRAINT connector_state_key_length CHECK (octet_length(state_key) BETWEEN 1 AND 128)
        );",
    )
}

fn serve(client: &mut Client, receiver: mpsc::Receiver<Request>) {
    while let Ok(request) = receiver.recv() {
        match request {
            Request::Read {
                key,
                maximum,
                response,
            } => {
                let result = client
                    .query_opt(
                        "SELECT body FROM connector_state_cells WHERE state_key = $1",
                        &[&key],
                    )
                    .map_err(|_| StateError::Unavailable)
                    .and_then(|row| {
                        row.map(|row| row.get::<_, Vec<u8>>(0))
                            .map(|body| {
                                if body.len() > maximum {
                                    Err(StateError::Capacity)
                                } else {
                                    Ok(body)
                                }
                            })
                            .transpose()
                    });
                let _ = response.send(result);
            }
            Request::Replace {
                key,
                body,
                maximum,
                response,
            } => {
                let result = if body.len() > maximum {
                    Err(StateError::Capacity)
                } else {
                    client
                        .execute(
                            "INSERT INTO connector_state_cells (state_key, body)
                             VALUES ($1, $2)
                             ON CONFLICT (state_key) DO UPDATE
                             SET body = EXCLUDED.body, updated_at = CURRENT_TIMESTAMP",
                            &[&key, &body],
                        )
                        .map(|_| ())
                        .map_err(|_| StateError::Unavailable)
                };
                let _ = response.send(result);
            }
            Request::Append {
                key,
                suffix,
                maximum,
                response,
            } => {
                let result = append(client, &key, &suffix, maximum);
                let _ = response.send(result);
            }
            Request::Delete { key, response } => {
                let result = client
                    .execute(
                        "DELETE FROM connector_state_cells WHERE state_key = $1",
                        &[&key],
                    )
                    .map(|_| ())
                    .map_err(|_| StateError::Unavailable);
                let _ = response.send(result);
            }
        }
    }
}

fn append(
    client: &mut Client,
    key: &str,
    suffix: &[u8],
    maximum: usize,
) -> Result<usize, StateError> {
    let maximum = i64::try_from(maximum).map_err(|_| StateError::Invalid)?;
    client
        .query_opt(
            "INSERT INTO connector_state_cells (state_key, body)
             VALUES ($1, $2)
             ON CONFLICT (state_key) DO UPDATE
             SET body = connector_state_cells.body || EXCLUDED.body,
                 updated_at = CURRENT_TIMESTAMP
             WHERE (octet_length(connector_state_cells.body) + octet_length(EXCLUDED.body))::bigint
                   <= $3::bigint
             RETURNING octet_length(body)::bigint",
            &[&key, &suffix, &maximum],
        )
        .map_err(|_| StateError::Unavailable)?
        .map(|row| usize::try_from(row.get::<_, i64>(0)).map_err(|_| StateError::Unavailable))
        .transpose()?
        .ok_or(StateError::Capacity)
}

fn validate_request(key: &str, maximum: usize) -> Result<(), StateError> {
    validate_key(key)?;
    if maximum == 0 {
        return Err(StateError::Invalid);
    }
    Ok(())
}

fn validate_key(key: &str) -> Result<(), StateError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_keys_are_closed_and_bounded() {
        for valid in [
            "slack.connections",
            "b10x:work-events",
            "vault_journal-v1",
        ] {
            assert_eq!(validate_key(valid), Ok(()));
        }
        for invalid in ["", "Slack", "../state", "with space", "state/value"] {
            assert_eq!(validate_key(invalid), Err(StateError::Invalid));
        }
        assert_eq!(validate_key(&"a".repeat(129)), Err(StateError::Invalid));
    }

    #[test]
    fn live_postgres_round_trip_is_bounded_and_atomic() {
        let Ok(database_url) = std::env::var("CONNECTORS_TEST_DATABASE_URL") else {
            return;
        };
        let key = format!("test.{}", std::process::id());
        let first = PostgresState::connect(&database_url).unwrap();
        let second = PostgresState::connect(&database_url).unwrap();
        first.delete(&key).unwrap();
        assert_eq!(first.read(&key, 8).unwrap(), None);
        first.replace(&key, b"ab", 8).unwrap();
        assert_eq!(second.append(&key, b"cd", 8).unwrap(), 4);
        assert_eq!(first.read(&key, 8).unwrap(), Some(b"abcd".to_vec()));
        assert_eq!(first.append(&key, b"efghi", 8), Err(StateError::Capacity));
        assert_eq!(second.read(&key, 8).unwrap(), Some(b"abcd".to_vec()));
        first.delete(&key).unwrap();
    }
}

#[cfg(test)]
mod port_tests {
    use super::*;

    /// The conformance suite against a live PostgreSQL, when one is offered.
    ///
    /// `#[ignore]`d by default and pointed at `CONNECTORS_DATABASE_URL`, mirroring
    /// `connector-secrets`'s `vault_live.rs`: a suite that silently passes because no database was
    /// present would be worse than one that is visibly skipped. The other two backends run the
    /// same suite unconditionally, so a divergence is caught the moment anyone runs this one.
    ///
    /// ```text
    /// CONNECTORS_DATABASE_URL=postgres://… cargo test -p hosted-state -- --ignored
    /// ```
    #[test]
    #[ignore = "requires a PostgreSQL named by CONNECTORS_DATABASE_URL"]
    fn the_postgres_backend_conforms() {
        let url = std::env::var("CONNECTORS_DATABASE_URL")
            .expect("CONNECTORS_DATABASE_URL names the database to test against");
        let store = PostgresState::connect(&url).expect("the database is reachable");
        connector_state::conformance::run(&store);
    }
}

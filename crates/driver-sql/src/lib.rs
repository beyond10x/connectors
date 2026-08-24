#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! The closed built-in `sql_v1` protocol driver.
//!
//! The only Connectors crate allowed to open a database socket and speak a relational engine's
//! native wire protocol. One driver word covers both shipped engines — MySQL over its
//! client/server protocol, PostgreSQL over its frontend/backend protocol — the way `cdp_v1`
//! covers Chrome, Chromium and Brave: the engines differ in wire bytes, not in what the
//! operation surface means, and the engine is a Connection-owned fact a caller never selects.
//!
//! # What the shipped surface is
//!
//! Four operations per engine: one bounded free-form read (`*-query`) and three fixed
//! `information_schema` projections (`*-schemas-list`, `*-tables-list`, `*-table-describe`).
//! Everything is read-only. Mutation waits on the grant-gated write story; adding it here
//! without that story would turn a read-only capability into an unapproved write.
//!
//! Four properties this crate exists to hold:
//!
//! 1. **A write is refused before any connection exists.** [`admission`] parses every statement
//!    with a real SQL parser and admits only a single `SELECT`/`SHOW`/`DESCRIBE`-class read;
//!    refusal happens before the credential is even resolved, so a refused statement leaves no
//!    network trace and reads no secret. Database grants are the deployment's own second fence,
//!    never the first.
//! 2. **The credential is a reference end to end.** [`credentials`] resolves a custody reference
//!    (file, environment, or — once composed — a Kubernetes secret named by an S-059 descriptor)
//!    into a [`credentials::ResolvedSecret`] that redacts itself in `Debug`, zeroizes on drop,
//!    and never appears in configuration, results, errors or logs. Every error detail passes
//!    through [`scrub`] before it can carry wire text.
//! 3. **Truncation is honest.** [`bounds::RowAccumulator`] owns the row and byte caps once for
//!    both engines: a page that stopped early says so, names the cap it hit, and carries the
//!    returned row count and counted bytes rather than silently cutting.
//! 4. **The transaction is read-only at the server too.** Both engines run the free-form read
//!    inside an explicitly read-only transaction, so even a statement class the parser admits
//!    cannot commit a write — defense in depth behind admission, not a substitute for it.
//!
//! What the read-only transaction does **not** close: an admitted `SELECT` can still call a
//! side-effectful server function the database account is privileged for (for example
//! `pg_terminate_backend`). That residual is governed by the account's own grants, which is why
//! the deployment discipline pairs this driver with least-privilege read accounts and the
//! catalog claims `risk = "medium"` for the free-form read.

pub mod admission;
pub mod bounds;
pub mod credentials;
mod mysql;
mod postgres;

use protocol::sql::{
    QueryResultPage, SchemaList, SqlQueryInput, TableDescription, TableList, MAX_RESULT_ROWS,
};

use crate::admission::admit_read_statement;
use crate::credentials::{CredentialError, CredentialReference, CredentialSource, ResolvedSecret};

/// The engine a Connection resolved. Never caller-selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlEngine {
    /// MySQL, over the MySQL client/server protocol.
    MySql,
    /// PostgreSQL, over the PostgreSQL frontend/backend protocol.
    Postgres,
}

/// One admitted Connection's deployment-owned route to a database.
///
/// Every field is a deployment fact resolved after grant admission — never caller input — and
/// none is secret: the credential is a [`CredentialReference`], so this struct can derive
/// `Debug` without a redaction hazard.
#[derive(Debug, Clone)]
pub struct SqlConnectionConfig {
    /// The engine behind this route.
    pub engine: SqlEngine,
    /// Database host, as the deployment names it.
    pub host: String,
    /// Database port.
    pub port: u16,
    /// The database (PostgreSQL) or default schema (MySQL) to connect to.
    pub database: String,
    /// The database account. Deployment-owned; pairs with the referenced credential.
    pub user: String,
    /// Where the password lives — a custody reference, never a value.
    pub credential: CredentialReference,
    /// Server-side statement timeout applied to every call, in milliseconds.
    pub statement_timeout_ms: u32,
}

/// The default server-side statement timeout.
pub const DEFAULT_STATEMENT_TIMEOUT_MS: u32 = 10_000;

/// Named refusal or failure from the SQL driver.
///
/// No variant ever carries a credential value: admission refusals precede resolution, and every
/// wire-derived detail passes through [`scrub`] with the resolved secret before construction.
#[derive(Debug, thiserror::Error)]
pub enum SqlDriverError {
    /// The caller's input was refused before any connection was opened.
    #[error("invalid_input: {reason}")]
    InvalidInput {
        /// Why the input was refused.
        reason: String,
    },
    /// The credential reference could not be resolved. Carries the reference's description,
    /// never a value.
    #[error(transparent)]
    Credential(#[from] CredentialError),
    /// Connecting to the database failed.
    #[error("connection failed: {detail}")]
    Connection {
        /// Scrubbed client-library detail.
        detail: String,
    },
    /// The admitted statement failed at the server.
    #[error("query failed: {detail}")]
    Query {
        /// Scrubbed client-library detail.
        detail: String,
    },
}

/// Remove every occurrence of the resolved secret from a wire- or client-derived detail string.
///
/// Client libraries do not normally echo passwords, but this driver does not build its
/// credential hygiene on "normally": every detail that could have seen wire text passes through
/// here before it becomes an error a caller or log can read.
#[must_use]
pub fn scrub(detail: &str, secret: &ResolvedSecret) -> String {
    let value = secret.expose();
    if value.is_empty() {
        return detail.to_owned();
    }
    detail.replace(value, "<redacted>")
}

fn resolve(
    config: &SqlConnectionConfig,
    source: &dyn CredentialSource,
) -> Result<ResolvedSecret, SqlDriverError> {
    Ok(source.resolve(&config.credential)?)
}

fn effective_max_rows(requested: Option<u32>) -> Result<u32, SqlDriverError> {
    match requested {
        None => Ok(MAX_RESULT_ROWS),
        Some(0) => Err(SqlDriverError::InvalidInput {
            reason: "max_rows must be at least 1".to_owned(),
        }),
        Some(value) => Ok(value.min(MAX_RESULT_ROWS)),
    }
}

/// Run one bounded read-only statement against the configured database.
///
/// Order is the contract: the statement is admitted first, so a refused statement resolves no
/// credential and opens no socket.
///
/// # Errors
///
/// [`SqlDriverError::InvalidInput`] for a refused statement or bound, before any connection;
/// [`SqlDriverError::Credential`] when the reference cannot be resolved;
/// [`SqlDriverError::Connection`]/[`SqlDriverError::Query`] for wire failures, scrubbed.
pub async fn run_query(
    config: &SqlConnectionConfig,
    source: &dyn CredentialSource,
    input: &SqlQueryInput,
) -> Result<QueryResultPage, SqlDriverError> {
    let admitted =
        admit_read_statement(config.engine, &input.statement).map_err(|refusal| {
            SqlDriverError::InvalidInput {
                reason: refusal.reason,
            }
        })?;
    let max_rows = effective_max_rows(input.max_rows)?;
    let secret = resolve(config, source)?;
    match config.engine {
        SqlEngine::Postgres => postgres::run_query(config, &secret, admitted.text(), max_rows).await,
        SqlEngine::MySql => mysql::run_query(config, &secret, admitted.text(), max_rows).await,
    }
}

/// List the server's schemas from `information_schema.schemata`.
///
/// # Errors
///
/// [`SqlDriverError::Credential`] when the reference cannot be resolved;
/// [`SqlDriverError::Connection`]/[`SqlDriverError::Query`] for wire failures, scrubbed.
pub async fn list_schemas(
    config: &SqlConnectionConfig,
    source: &dyn CredentialSource,
) -> Result<SchemaList, SqlDriverError> {
    let secret = resolve(config, source)?;
    match config.engine {
        SqlEngine::Postgres => postgres::list_schemas(config, &secret).await,
        SqlEngine::MySql => mysql::list_schemas(config, &secret).await,
    }
}

/// List one schema's tables and views from `information_schema.tables`.
///
/// # Errors
///
/// [`SqlDriverError::InvalidInput`] for an empty schema name;
/// [`SqlDriverError::Credential`] when the reference cannot be resolved;
/// [`SqlDriverError::Connection`]/[`SqlDriverError::Query`] for wire failures, scrubbed.
pub async fn list_tables(
    config: &SqlConnectionConfig,
    source: &dyn CredentialSource,
    schema: &str,
) -> Result<TableList, SqlDriverError> {
    require_identifier("schema", schema)?;
    let secret = resolve(config, source)?;
    match config.engine {
        SqlEngine::Postgres => postgres::list_tables(config, &secret, schema).await,
        SqlEngine::MySql => mysql::list_tables(config, &secret, schema).await,
    }
}

/// Describe one table's columns from `information_schema.columns`.
///
/// # Errors
///
/// [`SqlDriverError::InvalidInput`] for an empty schema or table name, or a table with no
/// visible columns; [`SqlDriverError::Credential`] when the reference cannot be resolved;
/// [`SqlDriverError::Connection`]/[`SqlDriverError::Query`] for wire failures, scrubbed.
pub async fn describe_table(
    config: &SqlConnectionConfig,
    source: &dyn CredentialSource,
    schema: &str,
    table: &str,
) -> Result<TableDescription, SqlDriverError> {
    require_identifier("schema", schema)?;
    require_identifier("table", table)?;
    let secret = resolve(config, source)?;
    match config.engine {
        SqlEngine::Postgres => postgres::describe_table(config, &secret, schema, table).await,
        SqlEngine::MySql => mysql::describe_table(config, &secret, schema, table).await,
    }
}

/// The identifier bound published by the catalog: MySQL admits 64 characters, PostgreSQL 63
/// bytes; one shared refusal at the larger bound keeps the driver engine-neutral and the
/// per-engine JSON Schemas carry the exact vendor bound.
const MAX_IDENTIFIER_CHARACTERS: usize = 64;

fn require_identifier(field: &str, value: &str) -> Result<(), SqlDriverError> {
    if value.trim().is_empty() {
        return Err(SqlDriverError::InvalidInput {
            reason: format!("{field} must not be empty"),
        });
    }
    if value.chars().count() > MAX_IDENTIFIER_CHARACTERS {
        return Err(SqlDriverError::InvalidInput {
            reason: format!("{field} exceeds {MAX_IDENTIFIER_CHARACTERS} characters"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credentials::FileEnvCredentialSource;

    fn config(engine: SqlEngine) -> SqlConnectionConfig {
        SqlConnectionConfig {
            engine,
            host: "127.0.0.1".to_owned(),
            port: 1,
            database: "db".to_owned(),
            user: "reader".to_owned(),
            credential: CredentialReference::File {
                path: "/nonexistent/driver-sql-test-credential".into(),
            },
            statement_timeout_ms: DEFAULT_STATEMENT_TIMEOUT_MS,
        }
    }

    /// **A write statement is refused before any connection — and before the credential is even
    /// resolved.** The credential reference points at a file that does not exist, so reaching
    /// resolution (let alone the socket at port 1) would fail with a different error class than
    /// the `invalid_input` asserted here.
    #[tokio::test]
    async fn a_write_statement_is_refused_before_credential_resolution_and_connection() {
        for engine in [SqlEngine::MySql, SqlEngine::Postgres] {
            let error = run_query(
                &config(engine),
                &FileEnvCredentialSource,
                &SqlQueryInput {
                    statement: "INSERT INTO t (a) VALUES (1)".to_owned(),
                    max_rows: None,
                },
            )
            .await
            .expect_err("a write statement must be refused");
            assert!(
                matches!(error, SqlDriverError::InvalidInput { .. }),
                "expected invalid_input, got: {error}"
            );
        }
    }

    /// An admitted statement proceeds past admission and fails at credential resolution — the
    /// pre-connection ordering made observable from the public seam.
    #[tokio::test]
    async fn an_admitted_statement_reaches_credential_resolution() {
        let error = run_query(
            &config(SqlEngine::Postgres),
            &FileEnvCredentialSource,
            &SqlQueryInput {
                statement: "SELECT 1".to_owned(),
                max_rows: None,
            },
        )
        .await
        .expect_err("the credential file does not exist");
        assert!(
            matches!(error, SqlDriverError::Credential(_)),
            "expected a credential error, got: {error}"
        );
    }

    #[test]
    fn scrub_removes_every_secret_occurrence() {
        let secret = ResolvedSecret::new("S3CRET-SENTINEL".to_owned());
        let scrubbed = scrub(
            "auth failed: password \"S3CRET-SENTINEL\" rejected (S3CRET-SENTINEL)",
            &secret,
        );
        assert!(!scrubbed.contains("S3CRET-SENTINEL"), "{scrubbed}");
        assert_eq!(
            scrubbed,
            "auth failed: password \"<redacted>\" rejected (<redacted>)"
        );
    }

    #[test]
    fn max_rows_narrows_but_never_widens() {
        assert_eq!(effective_max_rows(None).unwrap(), MAX_RESULT_ROWS);
        assert_eq!(effective_max_rows(Some(10)).unwrap(), 10);
        assert_eq!(
            effective_max_rows(Some(MAX_RESULT_ROWS + 1)).unwrap(),
            MAX_RESULT_ROWS
        );
        assert!(effective_max_rows(Some(0)).is_err());
    }
}

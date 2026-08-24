//! The PostgreSQL wire, over tokio-postgres and the frontend/backend protocol's simple query
//! form.
//!
//! The free-form read runs as a cursor inside an explicitly read-only transaction:
//!
//! ```text
//! BEGIN READ ONLY
//! DECLARE b10x_sql_cursor NO SCROLL CURSOR FOR <admitted statement>
//! FETCH FORWARD <batch> FROM b10x_sql_cursor   (repeated)
//! ROLLBACK
//! ```
//!
//! Three properties fall out of that shape. The simple protocol returns every value as text, so
//! arbitrary column types need no per-type rendering; the cursor batches keep the transferred
//! rows near the caps instead of pulling a whole result set; and the read-only transaction is
//! the server-side fence behind admission — a locking or writing statement that somehow passed
//! the parser still dies at `cannot execute … in a read-only transaction`.
//!
//! The admitted statement is embedded after `CURSOR FOR` verbatim. That is sound because
//! admission guarantees exactly one statement: a `;` smuggling a second one fails the
//! single-statement parse long before this module runs.

use tokio_postgres::{Config, NoTls, SimpleQueryMessage};

use protocol::sql::{
    ColumnDescription, QueryResultPage, SchemaList, TableDescription, TableList, MAX_RESULT_ROWS,
};

use crate::bounds::{Offer, RowAccumulator};
use crate::credentials::ResolvedSecret;
use crate::{scrub, SqlConnectionConfig, SqlDriverError};

/// Rows fetched per cursor round trip: small enough that a byte-cap stop wastes little
/// transfer, large enough that a full page needs few round trips.
const FETCH_BATCH: u32 = 64;

async fn connect(
    config: &SqlConnectionConfig,
    secret: &ResolvedSecret,
) -> Result<tokio_postgres::Client, SqlDriverError> {
    let timeout = config.statement_timeout_ms;
    let (client, connection) = Config::new()
        .host(&config.host)
        .port(config.port)
        .dbname(&config.database)
        .user(&config.user)
        .password(secret.expose())
        .application_name("b10x-driver-sql")
        // Server-side bounds ride the startup packet: every statement in the session inherits
        // them, and no admitted statement can widen them (SET is a denied keyword).
        .options(format!(
            "-c statement_timeout={timeout} -c idle_in_transaction_session_timeout={timeout}"
        ))
        .connect_timeout(std::time::Duration::from_millis(u64::from(timeout)))
        .connect(NoTls)
        .await
        .map_err(|error| SqlDriverError::Connection {
            detail: scrub(&error.to_string(), secret),
        })?;
    // The connection task owns the socket; it ends when the client drops.
    tokio::spawn(async move {
        let _ = connection.await;
    });
    Ok(client)
}

fn query_error(error: &tokio_postgres::Error, secret: &ResolvedSecret) -> SqlDriverError {
    SqlDriverError::Query {
        detail: scrub(&error.to_string(), secret),
    }
}

pub(crate) async fn run_query(
    config: &SqlConnectionConfig,
    secret: &ResolvedSecret,
    statement: &str,
    max_rows: u32,
) -> Result<QueryResultPage, SqlDriverError> {
    let client = connect(config, secret).await?;
    client
        .simple_query("BEGIN READ ONLY")
        .await
        .map_err(|error| query_error(&error, secret))?;
    client
        .simple_query(&format!(
            "DECLARE b10x_sql_cursor NO SCROLL CURSOR FOR {statement}"
        ))
        .await
        .map_err(|error| query_error(&error, secret))?;

    let mut columns: Vec<String> = Vec::new();
    let mut accumulator = RowAccumulator::new(max_rows, protocol::sql::MAX_RESULT_BYTES);
    // Offer up to max_rows + 1 rows so a result of exactly max_rows reports untruncated while
    // one more row flips the row cap.
    let mut remaining: u32 = max_rows + 1;
    'fetch: while remaining > 0 {
        let batch_size = remaining.min(FETCH_BATCH);
        let messages = client
            .simple_query(&format!("FETCH FORWARD {batch_size} FROM b10x_sql_cursor"))
            .await
            .map_err(|error| query_error(&error, secret))?;
        let mut batch_rows: u32 = 0;
        for message in messages {
            match message {
                SimpleQueryMessage::RowDescription(description) => {
                    if columns.is_empty() {
                        columns = description
                            .iter()
                            .map(|column| column.name().to_owned())
                            .collect();
                    }
                }
                SimpleQueryMessage::Row(row) => {
                    batch_rows += 1;
                    if columns.is_empty() {
                        columns = row
                            .columns()
                            .iter()
                            .map(|column| column.name().to_owned())
                            .collect();
                    }
                    let rendered = (0..row.len())
                        .map(|index| row.get(index).map(str::to_owned))
                        .collect();
                    if let Offer::Full(_) = accumulator.offer(rendered) {
                        break 'fetch;
                    }
                }
                _ => {}
            }
        }
        if batch_rows < batch_size {
            break;
        }
        remaining -= batch_rows;
    }

    let _ = client.simple_query("ROLLBACK").await;

    let (rows, bytes, truncation) = accumulator.finish();
    Ok(QueryResultPage {
        columns,
        rows_returned: rows.len() as u32,
        bytes_returned: bytes,
        truncated: truncation.is_some(),
        truncation_cause: truncation,
        rows,
    })
}

/// The inventory reads carry an explicit engine-side `LIMIT` of one past the row cap, so the
/// transfer is bounded and the flag stays honest.
const INVENTORY_LIMIT: u32 = MAX_RESULT_ROWS + 1;

pub(crate) async fn list_schemas(
    config: &SqlConnectionConfig,
    secret: &ResolvedSecret,
) -> Result<SchemaList, SqlDriverError> {
    let client = connect(config, secret).await?;
    let rows = client
        .query(
            &format!(
                "SELECT schema_name::text FROM information_schema.schemata \
                 ORDER BY schema_name LIMIT {INVENTORY_LIMIT}"
            ),
            &[],
        )
        .await
        .map_err(|error| query_error(&error, secret))?;
    let truncated = rows.len() as u32 > MAX_RESULT_ROWS;
    let schemas = rows
        .iter()
        .take(MAX_RESULT_ROWS as usize)
        .map(|row| row.try_get::<_, String>(0))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| query_error(&error, secret))?;
    Ok(SchemaList { schemas, truncated })
}

pub(crate) async fn list_tables(
    config: &SqlConnectionConfig,
    secret: &ResolvedSecret,
    schema: &str,
) -> Result<TableList, SqlDriverError> {
    let client = connect(config, secret).await?;
    let rows = client
        .query(
            &format!(
                "SELECT table_name::text FROM information_schema.tables \
                 WHERE table_schema = $1 ORDER BY table_name LIMIT {INVENTORY_LIMIT}"
            ),
            &[&schema],
        )
        .await
        .map_err(|error| query_error(&error, secret))?;
    let truncated = rows.len() as u32 > MAX_RESULT_ROWS;
    let tables = rows
        .iter()
        .take(MAX_RESULT_ROWS as usize)
        .map(|row| row.try_get::<_, String>(0))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| query_error(&error, secret))?;
    Ok(TableList {
        schema: schema.to_owned(),
        tables,
        truncated,
    })
}

pub(crate) async fn describe_table(
    config: &SqlConnectionConfig,
    secret: &ResolvedSecret,
    schema: &str,
    table: &str,
) -> Result<TableDescription, SqlDriverError> {
    let client = connect(config, secret).await?;
    let rows = client
        .query(
            &format!(
                "SELECT column_name::text, data_type::text, is_nullable::text, \
                 ordinal_position::int4 FROM information_schema.columns \
                 WHERE table_schema = $1 AND table_name = $2 \
                 ORDER BY ordinal_position LIMIT {INVENTORY_LIMIT}"
            ),
            &[&schema, &table],
        )
        .await
        .map_err(|error| query_error(&error, secret))?;
    if rows.is_empty() {
        return Err(SqlDriverError::InvalidInput {
            reason: format!(
                "table {schema}.{table} does not exist or has no columns visible to this account"
            ),
        });
    }
    let truncated = rows.len() as u32 > MAX_RESULT_ROWS;
    let columns = rows
        .iter()
        .take(MAX_RESULT_ROWS as usize)
        .map(|row| {
            Ok(ColumnDescription {
                name: row.try_get::<_, String>(0)?,
                data_type: row.try_get::<_, String>(1)?,
                nullable: row.try_get::<_, String>(2)? == "YES",
                ordinal: row.try_get::<_, i32>(3)?.unsigned_abs(),
            })
        })
        .collect::<Result<Vec<_>, tokio_postgres::Error>>()
        .map_err(|error| query_error(&error, secret))?;
    Ok(TableDescription {
        schema: schema.to_owned(),
        table: table.to_owned(),
        columns,
        truncated,
    })
}

//! The MySQL wire, over mysql_async and the MySQL client/server protocol.
//!
//! The free-form read streams through the text protocol inside an explicitly read-only
//! transaction:
//!
//! ```text
//! SET SESSION max_execution_time = <timeout>   (connection init)
//! START TRANSACTION READ ONLY
//! <admitted statement>                          (streamed row by row)
//! ROLLBACK                                      (clean path)
//! ```
//!
//! `query_iter` yields rows as the server sends them, so the row and byte caps stop the read
//! near the caps rather than after a full transfer. When a cap stops the read mid-result the
//! connection is dropped rather than drained: draining a deliberately truncated result would
//! transfer exactly the bytes the cap exists to avoid, and the server ends the read-only
//! transaction with the closed socket.
//!
//! `max_execution_time` bounds `SELECT` execution server-side and rides connection setup, not
//! caller SQL — `SET` is a denied keyword at admission, so no statement can widen it.

use mysql_async::prelude::Queryable;
use mysql_async::{Conn, Opts, OptsBuilder, Row, Value};

use protocol::sql::{
    ColumnDescription, QueryResultPage, SchemaList, TableDescription, TableList, MAX_RESULT_ROWS,
};

use crate::bounds::{Offer, RowAccumulator};
use crate::credentials::ResolvedSecret;
use crate::{scrub, SqlConnectionConfig, SqlDriverError};

fn opts(config: &SqlConnectionConfig, secret: &ResolvedSecret) -> Opts {
    OptsBuilder::default()
        .ip_or_hostname(config.host.clone())
        .tcp_port(config.port)
        .db_name(Some(config.database.clone()))
        .user(Some(config.user.clone()))
        .pass(Some(secret.expose().to_owned()))
        .init(vec![format!(
            "SET SESSION max_execution_time = {}",
            config.statement_timeout_ms
        )])
        .into()
}

async fn connect(
    config: &SqlConnectionConfig,
    secret: &ResolvedSecret,
) -> Result<Conn, SqlDriverError> {
    Conn::new(opts(config, secret))
        .await
        .map_err(|error| SqlDriverError::Connection {
            detail: scrub(&error.to_string(), secret),
        })
}

fn query_error(error: &mysql_async::Error, secret: &ResolvedSecret) -> SqlDriverError {
    SqlDriverError::Query {
        detail: scrub(&error.to_string(), secret),
    }
}

/// Render one MySQL value as text. The text protocol delivers nearly everything as
/// [`Value::Bytes`]; the remaining variants arrive from the binary protocol the inventory
/// reads use.
fn render(value: &Value) -> Option<String> {
    match value {
        Value::NULL => None,
        Value::Bytes(bytes) => Some(String::from_utf8_lossy(bytes).into_owned()),
        Value::Int(value) => Some(value.to_string()),
        Value::UInt(value) => Some(value.to_string()),
        Value::Float(value) => Some(value.to_string()),
        Value::Double(value) => Some(value.to_string()),
        Value::Date(year, month, day, hour, minute, second, micro) => Some(format!(
            "{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}.{micro:06}"
        )),
        Value::Time(negative, days, hours, minutes, seconds, micro) => {
            let sign = if *negative { "-" } else { "" };
            let total_hours = u32::from(*hours) + days * 24;
            Some(format!(
                "{sign}{total_hours:02}:{minutes:02}:{seconds:02}.{micro:06}"
            ))
        }
    }
}

fn rendered_row(row: &Row) -> Vec<Option<String>> {
    (0..row.len())
        .map(|index| row.as_ref(index).and_then(render))
        .collect()
}

pub(crate) async fn run_query(
    config: &SqlConnectionConfig,
    secret: &ResolvedSecret,
    statement: &str,
    max_rows: u32,
) -> Result<QueryResultPage, SqlDriverError> {
    let mut conn = connect(config, secret).await?;
    conn.query_drop("START TRANSACTION READ ONLY")
        .await
        .map_err(|error| query_error(&error, secret))?;

    let mut result = conn
        .query_iter(statement)
        .await
        .map_err(|error| query_error(&error, secret))?;
    let columns: Vec<String> = result
        .columns_ref()
        .iter()
        .map(|column| column.name_str().into_owned())
        .collect();

    let mut accumulator = RowAccumulator::new(max_rows, protocol::sql::MAX_RESULT_BYTES);
    let mut stopped_early = false;
    while let Some(row) = result
        .next()
        .await
        .map_err(|error| query_error(&error, secret))?
    {
        if let Offer::Full(_) = accumulator.offer(rendered_row(&row)) {
            stopped_early = true;
            break;
        }
    }
    drop(result);

    if stopped_early {
        // Draining a truncated result would transfer the bytes the cap refused; the dropped
        // connection ends the read-only transaction instead.
        drop(conn);
    } else {
        let _ = conn.query_drop("ROLLBACK").await;
        let _ = conn.disconnect().await;
    }

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

async fn fetch_first_column(
    conn: &mut Conn,
    secret: &ResolvedSecret,
    statement: &str,
    params: Vec<String>,
) -> Result<(Vec<String>, bool), SqlDriverError> {
    let rows: Vec<Row> = conn
        .exec(statement, params)
        .await
        .map_err(|error| query_error(&error, secret))?;
    let truncated = rows.len() as u32 > MAX_RESULT_ROWS;
    let values = rows
        .iter()
        .take(MAX_RESULT_ROWS as usize)
        .map(|row| {
            row.as_ref(0)
                .and_then(render)
                .ok_or_else(|| SqlDriverError::Query {
                    detail: "information_schema returned an unexpected NULL name".to_owned(),
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((values, truncated))
}

pub(crate) async fn list_schemas(
    config: &SqlConnectionConfig,
    secret: &ResolvedSecret,
) -> Result<SchemaList, SqlDriverError> {
    let mut conn = connect(config, secret).await?;
    let (schemas, truncated) = fetch_first_column(
        &mut conn,
        secret,
        &format!(
            "SELECT SCHEMA_NAME FROM INFORMATION_SCHEMA.SCHEMATA \
             ORDER BY SCHEMA_NAME LIMIT {INVENTORY_LIMIT}"
        ),
        Vec::new(),
    )
    .await?;
    let _ = conn.disconnect().await;
    Ok(SchemaList { schemas, truncated })
}

pub(crate) async fn list_tables(
    config: &SqlConnectionConfig,
    secret: &ResolvedSecret,
    schema: &str,
) -> Result<TableList, SqlDriverError> {
    let mut conn = connect(config, secret).await?;
    let (tables, truncated) = fetch_first_column(
        &mut conn,
        secret,
        &format!(
            "SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES \
             WHERE TABLE_SCHEMA = ? ORDER BY TABLE_NAME LIMIT {INVENTORY_LIMIT}"
        ),
        vec![schema.to_owned()],
    )
    .await?;
    let _ = conn.disconnect().await;
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
    let mut conn = connect(config, secret).await?;
    let rows: Vec<Row> = conn
        .exec(
            format!(
                "SELECT COLUMN_NAME, DATA_TYPE, IS_NULLABLE, ORDINAL_POSITION \
                 FROM INFORMATION_SCHEMA.COLUMNS \
                 WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ? \
                 ORDER BY ORDINAL_POSITION LIMIT {INVENTORY_LIMIT}"
            ),
            vec![schema.to_owned(), table.to_owned()],
        )
        .await
        .map_err(|error| query_error(&error, secret))?;
    let _ = conn.disconnect().await;
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
            let cell = |index: usize| {
                row.as_ref(index)
                    .and_then(render)
                    .ok_or_else(|| SqlDriverError::Query {
                        detail: "information_schema returned an unexpected NULL column fact"
                            .to_owned(),
                    })
            };
            let ordinal: u32 = cell(3)?.parse().map_err(|_| SqlDriverError::Query {
                detail: "information_schema returned a non-numeric ordinal".to_owned(),
            })?;
            Ok(ColumnDescription {
                name: cell(0)?,
                data_type: cell(1)?,
                nullable: cell(2)? == "YES",
                ordinal,
            })
        })
        .collect::<Result<Vec<_>, SqlDriverError>>()?;
    Ok(TableDescription {
        schema: schema.to_owned(),
        table: table.to_owned(),
        columns,
        truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_render_as_text_and_null_stays_null() {
        assert_eq!(render(&Value::NULL), None);
        assert_eq!(
            render(&Value::Bytes(b"hello".to_vec())).as_deref(),
            Some("hello")
        );
        assert_eq!(render(&Value::Int(-7)).as_deref(), Some("-7"));
        assert_eq!(render(&Value::UInt(7)).as_deref(), Some("7"));
        assert_eq!(
            render(&Value::Date(2026, 8, 24, 12, 30, 5, 0)).as_deref(),
            Some("2026-08-24 12:30:05.000000")
        );
        assert_eq!(
            render(&Value::Time(true, 1, 2, 3, 4, 5)).as_deref(),
            Some("-26:03:04.000005")
        );
    }
}

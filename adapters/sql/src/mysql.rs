//! The MySQL engine: the same two operations, bounds and output shape as the
//! PostgreSQL path, over the MySQL wire protocol (`mysql_async` on rustls with
//! the ring provider).
//!
//! A read opens one fresh session, makes it READ ONLY and bounds it before the
//! caller's statement is prepared, refuses a statement that returns no columns
//! (any write) before executing it, and runs it inside a derived table that
//! returns at most one row past the requested limit. Values arrive through the
//! binary result protocol and are written by the `mysql-native-text` rules
//! (`Family::rendering`), which the adapter's ESS model fixes.
use super::{
    CLEANUP_TIMEOUT, CONNECT_TIMEOUT, Config, QUERY_TIMEOUT, Query, REQUEST_TIMEOUT, column_bounds,
    database_certificates, query_timeout, sqlstate_error,
};
use base64::Engine as _;
use connectors_contracts::{Column, QueryResult};
use connectors_core::{Error, ErrorCode, Result};
use connectors_sdk::{Credential, encode, provenance};
use mysql_async::{
    Conn, Opts, OptsBuilder, Params, SslOpts, Value as Cell,
    consts::{ColumnFlags, ColumnType},
    prelude::Queryable,
};
use serde_json::Value;
use tokio::{sync::oneshot, time::Instant};

/// Every later transaction of the session, including each autocommitted
/// statement, is read-only: the server refuses a write with error 1792.
pub const SESSION_READ_ONLY: &str = "SET SESSION TRANSACTION READ ONLY";
/// Server-side guards inside the adapter's own deadline: a SELECT runs for at
/// most 10 s, and a metadata or row lock is waited on for at most 2 s.
pub const SESSION_BOUNDS: &str = "SET SESSION max_execution_time = 10000, SESSION lock_wait_timeout = 2, SESSION innodb_lock_wait_timeout = 2";
/// `schema.list` on MySQL: the column metadata of one schema (a MySQL database)
/// visible to the configured user, in the PostgreSQL path's column order and
/// names. `udt_name` carries MySQL's full `COLUMN_TYPE`.
pub const SCHEMA_QUERY: &str = "SELECT TABLE_SCHEMA AS table_schema, TABLE_NAME AS table_name, COLUMN_NAME AS column_name, DATA_TYPE AS data_type, COLUMN_TYPE AS udt_name, IS_NULLABLE AS is_nullable, ORDINAL_POSITION AS ordinal_position FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = ? ORDER BY TABLE_NAME, ORDINAL_POSITION";

/// The binary character set: a string column in it holds bytes, not text.
const BINARY_CHARSET: u16 = 63;

/// The MySQL column families the binary result protocol distinguishes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    SignedInteger,
    UnsignedInteger,
    Decimal,
    Floating,
    Year,
    Date,
    Datetime,
    Timestamp,
    Time,
    BinaryString,
    Bit,
    Geometry,
    TextString,
    Json,
    NullType,
}
/// How a non-NULL cell of a family is written. NULL is always JSON null.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rendering {
    IntegerText,
    DecimalText,
    FloatText,
    Iso8601Date,
    Iso8601Datetime,
    SignedElapsedTime,
    Base64,
    Utf8Text,
    JsonNull,
}
impl Rendering {
    /// The model's name for the rendering (`connectors_sql.reads.CellRendering`).
    pub fn name(self) -> &'static str {
        match self {
            Rendering::IntegerText => "integer_text",
            Rendering::DecimalText => "decimal_text",
            Rendering::FloatText => "float_text",
            Rendering::Iso8601Date => "iso8601_date",
            Rendering::Iso8601Datetime => "iso8601_datetime",
            Rendering::SignedElapsedTime => "signed_elapsed_time",
            Rendering::Base64 => "base64",
            Rendering::Utf8Text => "utf8_text",
            Rendering::JsonNull => "json_null",
        }
    }
}
impl Family {
    pub const ALL: [Family; 15] = [
        Family::SignedInteger,
        Family::UnsignedInteger,
        Family::Decimal,
        Family::Floating,
        Family::Year,
        Family::Date,
        Family::Datetime,
        Family::Timestamp,
        Family::Time,
        Family::BinaryString,
        Family::Bit,
        Family::Geometry,
        Family::TextString,
        Family::Json,
        Family::NullType,
    ];
    /// The model's name for the family (`connectors_sql.reads.MysqlTypeFamily`).
    pub fn name(self) -> &'static str {
        match self {
            Family::SignedInteger => "signed_integer",
            Family::UnsignedInteger => "unsigned_integer",
            Family::Decimal => "decimal",
            Family::Floating => "floating",
            Family::Year => "year",
            Family::Date => "date",
            Family::Datetime => "datetime",
            Family::Timestamp => "timestamp",
            Family::Time => "time",
            Family::BinaryString => "binary_string",
            Family::Bit => "bit",
            Family::Geometry => "geometry",
            Family::TextString => "text_string",
            Family::Json => "json",
            Family::NullType => "null_type",
        }
    }
    /// The `mysql-native-text` rule for the family.
    pub fn rendering(self) -> Rendering {
        match self {
            Family::SignedInteger | Family::UnsignedInteger | Family::Year => {
                Rendering::IntegerText
            }
            Family::Decimal => Rendering::DecimalText,
            Family::Floating => Rendering::FloatText,
            Family::Date => Rendering::Iso8601Date,
            Family::Datetime | Family::Timestamp => Rendering::Iso8601Datetime,
            Family::Time => Rendering::SignedElapsedTime,
            Family::BinaryString | Family::Bit | Family::Geometry => Rendering::Base64,
            Family::TextString | Family::Json => Rendering::Utf8Text,
            Family::NullType => Rendering::JsonNull,
        }
    }
    fn of(column: &mysql_async::Column) -> Family {
        use ColumnType::*;
        let unsigned = column.flags().contains(ColumnFlags::UNSIGNED_FLAG);
        match column.column_type() {
            MYSQL_TYPE_TINY | MYSQL_TYPE_SHORT | MYSQL_TYPE_INT24 | MYSQL_TYPE_LONG
            | MYSQL_TYPE_LONGLONG => {
                if unsigned {
                    Family::UnsignedInteger
                } else {
                    Family::SignedInteger
                }
            }
            MYSQL_TYPE_YEAR => Family::Year,
            MYSQL_TYPE_DECIMAL | MYSQL_TYPE_NEWDECIMAL => Family::Decimal,
            MYSQL_TYPE_FLOAT | MYSQL_TYPE_DOUBLE => Family::Floating,
            MYSQL_TYPE_DATE | MYSQL_TYPE_NEWDATE => Family::Date,
            MYSQL_TYPE_DATETIME | MYSQL_TYPE_DATETIME2 => Family::Datetime,
            MYSQL_TYPE_TIMESTAMP | MYSQL_TYPE_TIMESTAMP2 => Family::Timestamp,
            MYSQL_TYPE_TIME | MYSQL_TYPE_TIME2 => Family::Time,
            MYSQL_TYPE_BIT => Family::Bit,
            MYSQL_TYPE_GEOMETRY => Family::Geometry,
            MYSQL_TYPE_JSON => Family::Json,
            MYSQL_TYPE_NULL => Family::NullType,
            _ if column.character_set() == BINARY_CHARSET => Family::BinaryString,
            _ => Family::TextString,
        }
    }
}

/// The native type name of a column: MySQL's own lower-case type name, as
/// `information_schema.COLUMNS.DATA_TYPE` writes it, with ` unsigned` for an
/// unsigned integer.
fn native_type(column: &mysql_async::Column) -> String {
    use ColumnType::*;
    let flags = column.flags();
    let binary = column.character_set() == BINARY_CHARSET;
    let name = match column.column_type() {
        MYSQL_TYPE_TINY => "tinyint",
        MYSQL_TYPE_SHORT => "smallint",
        MYSQL_TYPE_INT24 => "mediumint",
        MYSQL_TYPE_LONG => "int",
        MYSQL_TYPE_LONGLONG => "bigint",
        MYSQL_TYPE_DECIMAL | MYSQL_TYPE_NEWDECIMAL => "decimal",
        MYSQL_TYPE_FLOAT => "float",
        MYSQL_TYPE_DOUBLE => "double",
        MYSQL_TYPE_YEAR => "year",
        MYSQL_TYPE_DATE | MYSQL_TYPE_NEWDATE => "date",
        MYSQL_TYPE_DATETIME | MYSQL_TYPE_DATETIME2 => "datetime",
        MYSQL_TYPE_TIMESTAMP | MYSQL_TYPE_TIMESTAMP2 => "timestamp",
        MYSQL_TYPE_TIME | MYSQL_TYPE_TIME2 => "time",
        MYSQL_TYPE_BIT => "bit",
        MYSQL_TYPE_JSON => "json",
        MYSQL_TYPE_GEOMETRY => "geometry",
        MYSQL_TYPE_NULL => "null",
        MYSQL_TYPE_VECTOR => "vector",
        _ if flags.contains(ColumnFlags::ENUM_FLAG) => "enum",
        _ if flags.contains(ColumnFlags::SET_FLAG) => "set",
        MYSQL_TYPE_VARCHAR | MYSQL_TYPE_VAR_STRING => {
            if binary {
                "varbinary"
            } else {
                "varchar"
            }
        }
        MYSQL_TYPE_STRING => {
            if binary {
                "binary"
            } else {
                "char"
            }
        }
        MYSQL_TYPE_TINY_BLOB | MYSQL_TYPE_MEDIUM_BLOB | MYSQL_TYPE_LONG_BLOB | MYSQL_TYPE_BLOB => {
            if binary {
                "blob"
            } else {
                "text"
            }
        }
        _ => "unknown",
    };
    let integer = matches!(
        column.column_type(),
        MYSQL_TYPE_TINY
            | MYSQL_TYPE_SHORT
            | MYSQL_TYPE_INT24
            | MYSQL_TYPE_LONG
            | MYSQL_TYPE_LONGLONG
    );
    if integer && flags.contains(ColumnFlags::UNSIGNED_FLAG) {
        format!("{name} unsigned")
    } else {
        name.into()
    }
}

fn unsupported_value() -> Error {
    Error::new(
        ErrorCode::Unsupported,
        "query returned a value this profile cannot write",
    )
}

/// The fractional seconds of a temporal value: as many digits as the column
/// declares, or six when it declares none and the value has a fraction.
fn fraction(micros: u32, decimals: u8) -> String {
    let digits = match decimals {
        1..=6 => decimals as usize,
        _ if micros != 0 => 6,
        _ => 0,
    };
    if digits == 0 {
        return String::new();
    }
    format!(".{:06}", micros)[..digits + 1].to_owned()
}

/// The shortest text that reads back as the same floating value. Both
/// candidates carry the shortest round-trip digits; the positional form
/// (`1.5`, `0.01`) wins a tie, the exponent form (`1e300`, `1.5e-7`) wins
/// when it is shorter. The value never passes through a JSON number.
fn shortest(positional: String, exponent: String) -> String {
    if exponent.len() < positional.len() {
        exponent
    } else {
        positional
    }
}

/// Write one cell by its column's family. The mapping is total over the values
/// the binary protocol produces for each family; anything else is refused
/// rather than coerced.
fn cell(value: Cell, column: &mysql_async::Column) -> Result<Value> {
    if value == Cell::NULL {
        return Ok(Value::Null);
    }
    let text = match (Family::of(column).rendering(), value) {
        (Rendering::IntegerText, Cell::Int(value)) => value.to_string(),
        (Rendering::IntegerText, Cell::UInt(value)) => value.to_string(),
        (Rendering::FloatText, Cell::Double(value)) => {
            shortest(value.to_string(), format!("{value:e}"))
        }
        (Rendering::FloatText, Cell::Float(value)) => {
            shortest(value.to_string(), format!("{value:e}"))
        }
        (Rendering::DecimalText | Rendering::Utf8Text, Cell::Bytes(bytes)) => {
            String::from_utf8(bytes).map_err(|_| unsupported_value())?
        }
        (Rendering::Base64, Cell::Bytes(bytes)) => {
            base64::engine::general_purpose::STANDARD.encode(bytes)
        }
        (Rendering::Iso8601Date, Cell::Date(year, month, day, ..)) => {
            format!("{year:04}-{month:02}-{day:02}")
        }
        (
            Rendering::Iso8601Datetime,
            Cell::Date(year, month, day, hour, minute, second, micros),
        ) => format!(
            "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}{}",
            fraction(micros, column.decimals())
        ),
        (
            Rendering::SignedElapsedTime,
            Cell::Time(negative, days, hours, minutes, seconds, micros),
        ) => format!(
            "{}{:02}:{minutes:02}:{seconds:02}{}",
            if negative { "-" } else { "" },
            u64::from(days) * 24 + u64::from(hours),
            fraction(micros, column.decimals())
        ),
        _ => return Err(unsupported_value()),
    };
    Ok(Value::String(text))
}

/// The session options: the configured endpoint only (never a Unix socket the
/// server names), no compression, no local-infile handler and no client-side
/// settings query. Without `allow_plaintext` TLS is required before any
/// credential is sent; a configured CA bundle replaces the public roots.
async fn options(config: &Config, password: &dyn Credential) -> Result<Opts> {
    let ssl = if config.allow_plaintext {
        None
    } else {
        // mysql_async builds its rustls client from the process default
        // provider. Install ring, the only provider this adapter selects; an
        // already installed default is kept.
        let _ = rustls::crypto::ring::default_provider().install_default();
        Some(match config.ca_file.as_deref() {
            Some(path) => SslOpts::default()
                .with_root_certs(
                    database_certificates(path)?
                        .into_iter()
                        .map(|certificate| certificate.to_vec().into())
                        .collect(),
                )
                .with_disable_built_in_roots(true),
            None => SslOpts::default(),
        })
    };
    let secret = password.resolve().await?;
    let password = std::str::from_utf8(&secret.0)
        .map_err(|_| Error::invalid("invalid database password"))?
        .to_owned();
    Ok(OptsBuilder::default()
        .ip_or_hostname(config.host.clone())
        .tcp_port(config.port)
        .user(Some(config.user.clone()))
        .pass(Some(password))
        .db_name(Some(config.database.clone()))
        .prefer_socket(false)
        .ssl_opts(ssl)
        .max_allowed_packet(Some(connectors_core::RESPONSE_LIMIT))
        .wait_timeout(Some(28_800))
        .stmt_cache_size(0)
        .into())
}

async fn connect(opts: Opts) -> Result<Conn> {
    tokio::time::timeout(CONNECT_TIMEOUT, Conn::new(opts))
        .await
        .map_err(|_| query_timeout())?
        .map_err(mysql_error)
}

/// Native credential validation: the handshake accepts or rejects the
/// password. No statement runs.
pub(crate) async fn validate_session(config: &Config, password: &dyn Credential) -> Result<()> {
    let conn = connect(options(config, password).await?).await?;
    let _ = tokio::time::timeout(CLEANUP_TIMEOUT, conn.disconnect()).await;
    Ok(())
}

pub(crate) async fn query(
    config: &Config,
    password: &dyn Credential,
    instance: &str,
    args: Query,
) -> Result<Value> {
    let deadline = Instant::now() + REQUEST_TIMEOUT;
    let opts = options(config, password).await?;
    let mut conn = connect(opts.clone()).await?;
    // Reserve cleanup time inside the 15 s request bound, as on PostgreSQL.
    let work_deadline = (Instant::now() + QUERY_TIMEOUT).min(deadline - CLEANUP_TIMEOUT);
    let instance = instance.to_owned();
    let database = config.database.clone();
    let (mut send, receive) = oneshot::channel();
    // The supervisor outlives a dropped invocation long enough to kill the
    // statement: dropping the session alone can leave MySQL executing it.
    tokio::spawn(async move {
        let result = tokio::select! {
            biased;
            _ = send.closed() => None,
            result = tokio::time::timeout_at(work_deadline, read(&mut conn, args, &instance, &database)) => {
                Some(result.unwrap_or_else(|_| Err(query_timeout())))
            }
        };
        // Kill only a statement that may still be running: a dropped call or
        // the adapter's own deadline. A server answer ended its statement.
        let kill = match &result {
            None => true,
            Some(Err(error)) => error.code == ErrorCode::Timeout && !error.upstream_answer,
            Some(Ok(_)) => false,
        };
        close(conn, opts, kill).await;
        if let Some(result) = result {
            let _ = send.send(result);
        }
    });
    receive.await.map_err(|_| Error::internal())?
}

/// Close a read's session within the cleanup budget, first killing its
/// statement from a second session when it may still be running. `KILL QUERY`
/// names the session's own connection id, which the handshake reported.
async fn close(conn: Conn, opts: Opts, kill: bool) {
    let id = conn.id();
    let cleanup = async move {
        if kill
            && let Ok(Ok(mut killer)) = tokio::time::timeout(CLEANUP_TIMEOUT, Conn::new(opts)).await
        {
            let _ = killer.query_drop(format!("KILL QUERY {id}")).await;
            let _ = killer.disconnect().await;
        }
        // Disconnecting drains what remains of the result, at most one row.
        let _ = conn.disconnect().await;
    };
    // A server that ignores the kill cannot hold the session past the budget:
    // dropping the future closes the socket.
    let _ = tokio::time::timeout(CLEANUP_TIMEOUT, cleanup).await;
}

async fn read(conn: &mut Conn, args: Query, instance: &str, database: &str) -> Result<Value> {
    conn.query_drop(SESSION_READ_ONLY)
        .await
        .map_err(mysql_error)?;
    conn.query_drop(SESSION_BOUNDS).await.map_err(mysql_error)?;
    let query = args
        .query
        .trim()
        .strip_suffix(';')
        .unwrap_or(args.query.trim());
    let original = conn.prep(query).await.map_err(mysql_error)?;
    column_bounds(original.columns().len())?;
    if original.num_params() as usize != args.parameters.len() {
        return Err(Error::invalid("parameter count does not match query"));
    }
    let columns = original
        .columns()
        .iter()
        .map(|c| Column {
            name: c.name_str().into_owned(),
            native_type: native_type(c),
        })
        .collect::<Vec<_>>();
    conn.close(original).await.map_err(mysql_error)?;
    let aliases = (0..columns.len())
        .map(|i| format!("c{i}"))
        .collect::<Vec<_>>()
        .join(",");
    // A single-statement derived table with positional aliases, as on
    // PostgreSQL: only a query can be a derived table, duplicate result names
    // cannot collide, and the server sends at most one row past the limit.
    // The derived table closes on its own line, so a trailing `--` or `#`
    // comment in the caller's statement cannot reach the aliases or LIMIT.
    let wrapped = format!(
        "SELECT * FROM ({query}\n) AS result ({aliases}) LIMIT {}",
        u32::from(args.limit) + 1
    );
    let statement = conn.prep(&wrapped).await.map_err(mysql_error)?;
    let parameters = if args.parameters.is_empty() {
        Params::Empty
    } else {
        Params::Positional(
            args.parameters
                .into_iter()
                .map(|p| {
                    p.map(|text| Cell::Bytes(text.into_bytes()))
                        .unwrap_or(Cell::NULL)
                })
                .collect(),
        )
    };
    let mut result = conn
        .exec_iter(&statement, parameters)
        .await
        .map_err(mysql_error)?;
    let mut truncated = false;
    let mut rows = Vec::new();
    let mut total = 0;
    while let Some(row) = result.next().await.map_err(mysql_error)? {
        if rows.len() == args.limit as usize {
            truncated = true;
            break;
        }
        let types = row.columns();
        let values = row
            .unwrap()
            .into_iter()
            .zip(types.iter())
            .map(|(value, column)| cell(value, column))
            .collect::<Result<Vec<_>>>()?;
        total += serde_json::to_vec(&values)
            .map_err(|_| Error::internal())?
            .len();
        if total > connectors_core::RESPONSE_LIMIT - 65536 {
            return Err(Error::new(
                ErrorCode::Capacity,
                "query result exceeds byte limit",
            ));
        }
        rows.push(values);
    }
    result.drop_result().await.map_err(mysql_error)?;
    conn.close(statement).await.map_err(mysql_error)?;
    encode(QueryResult {
        columns,
        rows,
        truncated,
        provenance: provenance(instance, database, None),
    })
}

fn mysql_error(error: mysql_async::Error) -> Error {
    match error {
        mysql_async::Error::Server(error) => server_error(error.code, &error.state),
        _ => sqlstate_error(None),
    }
}
/// Classify a MySQL server error by its error number where the SQLSTATE class
/// is too coarse (MySQL reports access refusals and syntax errors alike as
/// 42000), and otherwise by its SQLSTATE as on PostgreSQL. The server's
/// message never reaches the caller.
fn server_error(code: u16, state: &str) -> Error {
    let classified = match code {
        // Database, table, column and privilege access refusals, and a write
        // refused by the read-only session.
        1044 | 1142 | 1143 | 1227 | 1370 | 1792 => ErrorCode::Forbidden,
        1045 => ErrorCode::Unauthorized,
        1235 => ErrorCode::Unsupported,
        // The server's own statement deadline (max_execution_time).
        3024 => ErrorCode::Timeout,
        // Too many connections, for the server or for the user.
        1040 | 1203 => ErrorCode::Capacity,
        _ => return sqlstate_error(Some(state)),
    };
    let answered = matches!(classified, ErrorCode::Timeout | ErrorCode::Capacity);
    let error = Error::new(classified, "database could not complete the requested read");
    if answered { error.answered() } else { error }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn float_text_is_the_shortest_text_that_reads_back() {
        for (value, expected) in [
            (1.5_f64, "1.5"),
            (0.0, "0"),
            (-2.25, "-2.25"),
            (100.0, "100"),
            (1000.0, "1e3"),
            (0.01, "0.01"),
            (0.001, "1e-3"),
            (0.0001, "1e-4"),
            (1e300, "1e300"),
            (1e-300, "1e-300"),
            (-1.7976931348623157e308, "-1.7976931348623157e308"),
            (123456789.125, "123456789.125"),
        ] {
            let text = shortest(value.to_string(), format!("{value:e}"));
            assert_eq!(text, expected, "{value:e}");
            assert_eq!(text.parse::<f64>().unwrap(), value);
        }
        let single = 1e30_f32;
        assert_eq!(shortest(single.to_string(), format!("{single:e}")), "1e30");
    }

    #[test]
    fn fractions_follow_the_declared_precision() {
        assert_eq!(fraction(123, 6), ".000123");
        assert_eq!(fraction(120_000, 2), ".12");
        assert_eq!(fraction(0, 0), "");
        assert_eq!(fraction(5, 0), ".000005");
        assert_eq!(fraction(5, 31), ".000005");
        assert_eq!(fraction(0, 31), "");
    }

    #[test]
    fn server_errors_classify_by_number_then_by_sqlstate() {
        for (code, state, expected, answered) in [
            (1792, "25006", ErrorCode::Forbidden, false),
            (1142, "42000", ErrorCode::Forbidden, false),
            (1045, "28000", ErrorCode::Unauthorized, false),
            (3024, "HY000", ErrorCode::Timeout, true),
            (1040, "08004", ErrorCode::Capacity, true),
            (1064, "42000", ErrorCode::InvalidInput, false),
            (1205, "HY000", ErrorCode::Unavailable, false),
        ] {
            let error = server_error(code, state);
            assert_eq!(error.code, expected, "{code}");
            assert_eq!(error.upstream_answer, answered, "{code}");
        }
    }
}

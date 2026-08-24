//! Public input/output projection for the bounded read-only SQL surface.
//!
//! A caller supplies at most one read statement, or the coordinates of a schema object, and
//! nothing else. No host, port, database name, user, or credential value crosses this boundary in
//! either direction: those are deployment-owned Connection facts resolved behind the closed
//! `sql_v1` driver after admission, and the credential is a custody reference end to end.
//!
//! **The surface is read-only by construction, not by grant.** The driver refuses any statement
//! that does not parse as a single read (`SELECT`/`SHOW`/`DESCRIBE`-class) before opening a
//! connection, so a write statement never reaches a database even when the database account
//! itself would have allowed it.

use serde::{Deserialize, Serialize};

/// Canonical catalog id. Connector tool projection renders this as `mysql.query`.
pub const MYSQL_QUERY_OPERATION: &str = "mysql-query";

/// Canonical catalog id. Connector tool projection renders this as `mysql.schemas-list`.
pub const MYSQL_SCHEMAS_LIST_OPERATION: &str = "mysql-schemas-list";

/// Canonical catalog id. Connector tool projection renders this as `mysql.tables-list`.
pub const MYSQL_TABLES_LIST_OPERATION: &str = "mysql-tables-list";

/// Canonical catalog id. Connector tool projection renders this as `mysql.table-describe`.
pub const MYSQL_TABLE_DESCRIBE_OPERATION: &str = "mysql-table-describe";

/// Canonical catalog id. Connector tool projection renders this as `postgresql.query`.
pub const POSTGRESQL_QUERY_OPERATION: &str = "postgresql-query";

/// Canonical catalog id. Connector tool projection renders this as `postgresql.schemas-list`.
pub const POSTGRESQL_SCHEMAS_LIST_OPERATION: &str = "postgresql-schemas-list";

/// Canonical catalog id. Connector tool projection renders this as `postgresql.tables-list`.
pub const POSTGRESQL_TABLES_LIST_OPERATION: &str = "postgresql-tables-list";

/// Canonical catalog id. Connector tool projection renders this as `postgresql.table-describe`.
pub const POSTGRESQL_TABLE_DESCRIBE_OPERATION: &str = "postgresql-table-describe";

/// The exact admitted MySQL surface, in catalog order.
///
/// Mutation — `INSERT`, `UPDATE`, DDL — is **deliberately absent**. It acts on governed data on
/// the operator's behalf, so it is a write, and it waits on the grant-gated mutation story.
/// Adding an entry here without that story would turn a read-only surface into an unapproved
/// write.
pub const MYSQL_OPERATIONS: [&str; 4] = [
    MYSQL_QUERY_OPERATION,
    MYSQL_SCHEMAS_LIST_OPERATION,
    MYSQL_TABLES_LIST_OPERATION,
    MYSQL_TABLE_DESCRIBE_OPERATION,
];

/// The exact admitted PostgreSQL surface, in catalog order. The same read-only rule as
/// [`MYSQL_OPERATIONS`] applies.
pub const POSTGRESQL_OPERATIONS: [&str; 4] = [
    POSTGRESQL_QUERY_OPERATION,
    POSTGRESQL_SCHEMAS_LIST_OPERATION,
    POSTGRESQL_TABLES_LIST_OPERATION,
    POSTGRESQL_TABLE_DESCRIBE_OPERATION,
];

/// Stable Provider id for the MySQL server capability.
pub const MYSQL_PROVIDER: &str = "mysql";

/// Permanent Provider authority for the MySQL server capability.
pub const MYSQL_PROVIDER_AUTHORITY: &str = "com.mysql.server";

/// Stable Provider id for the PostgreSQL server capability.
pub const POSTGRESQL_PROVIDER: &str = "postgresql";

/// Permanent Provider authority for the PostgreSQL server capability.
pub const POSTGRESQL_PROVIDER_AUTHORITY: &str = "org.postgresql.server";

/// The published bound on one statement, in characters.
pub const MAX_STATEMENT_CHARACTERS: usize = 8_192;

/// The published bound on how many rows one query result returns.
pub const MAX_RESULT_ROWS: u32 = 500;

/// The published bound on one query result's cell payload, in bytes.
///
/// Counted over the returned cell text, so it bounds what actually travels. Calibrated to keep a
/// serialized result page far inside the 256 KiB operation-result bound even with the column and
/// envelope overhead on top.
pub const MAX_RESULT_BYTES: u64 = 131_072;

/// One bounded read statement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SqlQueryInput {
    /// Exactly one read statement. Anything that is not a single `SELECT`/`SHOW`/`DESCRIBE`-class
    /// statement is refused before any connection is opened.
    pub statement: String,
    /// Optional caller-side row bound. The effective bound is the smaller of this and
    /// [`MAX_RESULT_ROWS`]; a caller can narrow the cap, never widen it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_rows: Option<u32>,
}

/// Why a result page stopped early.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TruncationCause {
    /// The row cap was reached and at least one more row existed.
    RowCap,
    /// The byte cap was reached before the row cap.
    ByteCap,
}

/// One bounded query result page.
///
/// Truncation is honest: a page that stopped early says so, names why, and carries both the
/// returned row count and the counted bytes rather than silently cutting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryResultPage {
    /// Result column names, in wire order.
    pub columns: Vec<String>,
    /// Returned rows. Every cell is the engine's text rendering; `null` is an SQL `NULL`.
    pub rows: Vec<Vec<Option<String>>>,
    /// How many rows were returned — always `rows.len()`, restated so a truncated page carries
    /// the number beside the flag.
    pub rows_returned: u32,
    /// Counted cell bytes across the returned rows.
    pub bytes_returned: u64,
    /// Whether the result stopped before the statement's full result set.
    pub truncated: bool,
    /// Why the page stopped early; absent when `truncated` is false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub truncation_cause: Option<TruncationCause>,
}

/// The schema inventory of one database server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaList {
    /// Schema names, sorted, system schemas included — hiding them would misreport the server.
    pub schemas: Vec<String>,
    /// Whether the list stopped at the row cap.
    pub truncated: bool,
}

/// The table inventory of one schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableList {
    /// The schema that was listed.
    pub schema: String,
    /// Table and view names in that schema, sorted.
    pub tables: Vec<String>,
    /// Whether the list stopped at the row cap.
    pub truncated: bool,
}

/// One column of a described table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnDescription {
    /// The column name.
    pub name: String,
    /// The engine's own data-type word for the column.
    pub data_type: String,
    /// Whether the column admits SQL `NULL`.
    pub nullable: bool,
    /// The column's 1-based position in the table.
    pub ordinal: u32,
}

/// The described shape of one table.
///
/// Deliberately structure only — no default expressions, no comments, no row estimates: those can
/// embed deployment facts, and a model needs the shape, not the DDL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableDescription {
    /// The schema holding the table.
    pub schema: String,
    /// The described table.
    pub table: String,
    /// The table's columns in ordinal order.
    pub columns: Vec<ColumnDescription>,
    /// Whether the column list stopped at the row cap.
    pub truncated: bool,
}

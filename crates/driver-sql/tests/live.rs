//! Live-database proof of the wire modules, against real servers.
//!
//! These are `#[ignore]`d: they need a reachable MySQL and PostgreSQL, which the gate may not
//! assume. Point them at throwaway containers and run them explicitly:
//!
//! ```text
//! docker run --rm -d -p 15432:5432 -e POSTGRES_PASSWORD=live-pg-password postgres:17
//! docker run --rm -d -p 13306:3306 -e MYSQL_ROOT_PASSWORD=live-my-password mysql:8.4
//! B10X_SQL_TEST_PG_PASSWORD=live-pg-password \
//! B10X_SQL_TEST_MYSQL_PASSWORD=live-my-password \
//!   cargo test --test live -- --ignored
//! ```
//!
//! Ports, databases and users are overridable through the same `B10X_SQL_TEST_*` family. The
//! password rides an `Env` credential reference, so the tests exercise the custody seam rather
//! than bypassing it.

use driver_sql::credentials::{CredentialReference, FileEnvCredentialSource};
use driver_sql::{
    describe_table, list_schemas, list_tables, run_query, SqlConnectionConfig, SqlDriverError,
    SqlEngine, DEFAULT_STATEMENT_TIMEOUT_MS,
};
use protocol::sql::{SqlQueryInput, TruncationCause, MAX_RESULT_ROWS};

fn env_or(name: &str, fallback: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| fallback.to_owned())
}

fn postgres_config() -> SqlConnectionConfig {
    SqlConnectionConfig {
        engine: SqlEngine::Postgres,
        host: env_or("B10X_SQL_TEST_PG_HOST", "127.0.0.1"),
        port: env_or("B10X_SQL_TEST_PG_PORT", "15432").parse().expect("port"),
        database: env_or("B10X_SQL_TEST_PG_DB", "postgres"),
        user: env_or("B10X_SQL_TEST_PG_USER", "postgres"),
        credential: CredentialReference::Env {
            variable: "B10X_SQL_TEST_PG_PASSWORD".to_owned(),
        },
        statement_timeout_ms: DEFAULT_STATEMENT_TIMEOUT_MS,
    }
}

fn mysql_config() -> SqlConnectionConfig {
    SqlConnectionConfig {
        engine: SqlEngine::MySql,
        host: env_or("B10X_SQL_TEST_MYSQL_HOST", "127.0.0.1"),
        port: env_or("B10X_SQL_TEST_MYSQL_PORT", "13306")
            .parse()
            .expect("port"),
        database: env_or("B10X_SQL_TEST_MYSQL_DB", "mysql"),
        user: env_or("B10X_SQL_TEST_MYSQL_USER", "root"),
        credential: CredentialReference::Env {
            variable: "B10X_SQL_TEST_MYSQL_PASSWORD".to_owned(),
        },
        statement_timeout_ms: DEFAULT_STATEMENT_TIMEOUT_MS,
    }
}

fn query(statement: &str) -> SqlQueryInput {
    SqlQueryInput {
        statement: statement.to_owned(),
        max_rows: None,
    }
}

// ---------------------------------------------------------------------------------------------
// PostgreSQL
// ---------------------------------------------------------------------------------------------

#[tokio::test]
#[ignore = "needs a live PostgreSQL; see the module docs"]
async fn postgres_small_result_returns_untruncated() {
    let page = run_query(
        &postgres_config(),
        &FileEnvCredentialSource,
        &query("SELECT 1 AS one, 'two' AS two, NULL AS three"),
    )
    .await
    .expect("query runs");
    assert_eq!(page.columns, ["one", "two", "three"]);
    assert_eq!(page.rows_returned, 1);
    assert_eq!(
        page.rows,
        [vec![
            Some("1".to_owned()),
            Some("two".to_owned()),
            None
        ]]
    );
    assert!(!page.truncated);
    assert_eq!(page.truncation_cause, None);
}

#[tokio::test]
#[ignore = "needs a live PostgreSQL; see the module docs"]
async fn postgres_row_cap_truncates_honestly() {
    let page = run_query(
        &postgres_config(),
        &FileEnvCredentialSource,
        &query("SELECT generate_series(1, 2000) AS n"),
    )
    .await
    .expect("query runs");
    assert_eq!(page.rows_returned, MAX_RESULT_ROWS);
    assert!(page.truncated);
    assert_eq!(page.truncation_cause, Some(TruncationCause::RowCap));
}

#[tokio::test]
#[ignore = "needs a live PostgreSQL; see the module docs"]
async fn postgres_byte_cap_truncates_honestly() {
    let page = run_query(
        &postgres_config(),
        &FileEnvCredentialSource,
        &query("SELECT repeat('x', 10000) FROM generate_series(1, 400)"),
    )
    .await
    .expect("query runs");
    assert!(page.truncated);
    assert_eq!(page.truncation_cause, Some(TruncationCause::ByteCap));
    assert!(page.bytes_returned <= protocol::sql::MAX_RESULT_BYTES);
    assert!(page.rows_returned < 400);
}

#[tokio::test]
#[ignore = "needs a live PostgreSQL; see the module docs"]
async fn postgres_caller_max_rows_narrows_the_cap() {
    let page = run_query(
        &postgres_config(),
        &FileEnvCredentialSource,
        &SqlQueryInput {
            statement: "SELECT generate_series(1, 100) AS n".to_owned(),
            max_rows: Some(7),
        },
    )
    .await
    .expect("query runs");
    assert_eq!(page.rows_returned, 7);
    assert!(page.truncated);
}

#[tokio::test]
#[ignore = "needs a live PostgreSQL; see the module docs"]
async fn postgres_inventory_reads_work() {
    let config = postgres_config();
    let schemas = list_schemas(&config, &FileEnvCredentialSource)
        .await
        .expect("schemas list");
    assert!(
        schemas.schemas.iter().any(|s| s == "information_schema"),
        "{:?}",
        schemas.schemas
    );

    let tables = list_tables(&config, &FileEnvCredentialSource, "information_schema")
        .await
        .expect("tables list");
    assert!(tables.tables.iter().any(|t| t == "tables"), "{:?}", tables.tables);

    let description = describe_table(&config, &FileEnvCredentialSource, "information_schema", "tables")
        .await
        .expect("table describe");
    assert!(
        description.columns.iter().any(|c| c.name == "table_name"),
        "{:?}",
        description.columns
    );
    assert!(description.columns.iter().all(|c| c.ordinal >= 1));
}

/// A write that somehow reached the server would still die in the read-only transaction; here
/// the parser refuses it first, against a fully real config, without a connection.
#[tokio::test]
#[ignore = "needs a live PostgreSQL; see the module docs"]
async fn postgres_write_statement_refused_pre_connection() {
    let error = run_query(
        &postgres_config(),
        &FileEnvCredentialSource,
        &query("INSERT INTO t (a) VALUES (1)"),
    )
    .await
    .expect_err("writes are refused");
    assert!(matches!(error, SqlDriverError::InvalidInput { .. }));
}

#[tokio::test]
#[ignore = "needs a live PostgreSQL; see the module docs"]
async fn postgres_auth_failure_never_echoes_the_password() {
    let mut config = postgres_config();
    config.user = "definitely-not-a-user".to_owned();
    let password = std::env::var("B10X_SQL_TEST_PG_PASSWORD").expect("test password set");
    let error = run_query(&config, &FileEnvCredentialSource, &query("SELECT 1"))
        .await
        .expect_err("auth must fail");
    let rendered = error.to_string();
    assert!(!rendered.contains(&password), "{rendered}");
}

// ---------------------------------------------------------------------------------------------
// MySQL
// ---------------------------------------------------------------------------------------------

#[tokio::test]
#[ignore = "needs a live MySQL; see the module docs"]
async fn mysql_small_result_returns_untruncated() {
    let page = run_query(
        &mysql_config(),
        &FileEnvCredentialSource,
        &query("SELECT 1 AS one, 'two' AS two, NULL AS three"),
    )
    .await
    .expect("query runs");
    assert_eq!(page.columns, ["one", "two", "three"]);
    assert_eq!(page.rows_returned, 1);
    assert_eq!(
        page.rows,
        [vec![
            Some("1".to_owned()),
            Some("two".to_owned()),
            None
        ]]
    );
    assert!(!page.truncated);
}

#[tokio::test]
#[ignore = "needs a live MySQL; see the module docs"]
async fn mysql_row_cap_truncates_honestly() {
    // cte_max_recursion_depth defaults to 1000; 900 rows is over the 500-row cap and under it.
    let page = run_query(
        &mysql_config(),
        &FileEnvCredentialSource,
        &query(
            "WITH RECURSIVE cte (n) AS (SELECT 1 UNION ALL SELECT n + 1 FROM cte WHERE n < 900) \
             SELECT n FROM cte",
        ),
    )
    .await
    .expect("query runs");
    assert_eq!(page.rows_returned, MAX_RESULT_ROWS);
    assert!(page.truncated);
    assert_eq!(page.truncation_cause, Some(TruncationCause::RowCap));
}

#[tokio::test]
#[ignore = "needs a live MySQL; see the module docs"]
async fn mysql_byte_cap_truncates_honestly() {
    let page = run_query(
        &mysql_config(),
        &FileEnvCredentialSource,
        &query(
            "WITH RECURSIVE cte (n) AS (SELECT 1 UNION ALL SELECT n + 1 FROM cte WHERE n < 400) \
             SELECT REPEAT('x', 10000) FROM cte",
        ),
    )
    .await
    .expect("query runs");
    assert!(page.truncated);
    assert_eq!(page.truncation_cause, Some(TruncationCause::ByteCap));
    assert!(page.bytes_returned <= protocol::sql::MAX_RESULT_BYTES);
}

#[tokio::test]
#[ignore = "needs a live MySQL; see the module docs"]
async fn mysql_show_and_describe_run() {
    let page = run_query(&mysql_config(), &FileEnvCredentialSource, &query("SHOW DATABASES"))
        .await
        .expect("SHOW DATABASES runs");
    assert!(page.rows_returned >= 1);

    let page = run_query(
        &mysql_config(),
        &FileEnvCredentialSource,
        &query("DESCRIBE information_schema.TABLES"),
    )
    .await
    .expect("DESCRIBE runs");
    assert!(page.rows_returned >= 1);
}

#[tokio::test]
#[ignore = "needs a live MySQL; see the module docs"]
async fn mysql_inventory_reads_work() {
    let config = mysql_config();
    let schemas = list_schemas(&config, &FileEnvCredentialSource)
        .await
        .expect("schemas list");
    assert!(
        schemas.schemas.iter().any(|s| s == "information_schema"),
        "{:?}",
        schemas.schemas
    );

    let tables = list_tables(&config, &FileEnvCredentialSource, "information_schema")
        .await
        .expect("tables list");
    assert!(
        tables.tables.iter().any(|t| t == "TABLES"),
        "{:?}",
        tables.tables
    );

    let description = describe_table(&config, &FileEnvCredentialSource, "information_schema", "TABLES")
        .await
        .expect("table describe");
    assert!(
        description.columns.iter().any(|c| c.name == "TABLE_NAME"),
        "{:?}",
        description.columns
    );
}

#[tokio::test]
#[ignore = "needs a live MySQL; see the module docs"]
async fn mysql_write_statement_refused_pre_connection() {
    let error = run_query(
        &mysql_config(),
        &FileEnvCredentialSource,
        &query("UPDATE t SET a = 1"),
    )
    .await
    .expect_err("writes are refused");
    assert!(matches!(error, SqlDriverError::InvalidInput { .. }));
}

#[tokio::test]
#[ignore = "needs a live MySQL; see the module docs"]
async fn mysql_auth_failure_never_echoes_the_password() {
    let mut config = mysql_config();
    config.user = "definitely-not-a-user".to_owned();
    let password = std::env::var("B10X_SQL_TEST_MYSQL_PASSWORD").expect("test password set");
    let error = run_query(&config, &FileEnvCredentialSource, &query("SELECT 1"))
        .await
        .expect_err("auth must fail");
    let rendered = error.to_string();
    assert!(!rendered.contains(&password), "{rendered}");
}

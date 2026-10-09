//! MySQL wire fixtures exercise the adapter's MySQL engine through its public
//! library API, the way `protocol.rs` does for PostgreSQL. No database, Docker,
//! environment credential or network outside loopback is needed.
#[path = "mysql/fixture.rs"]
mod fixture;

use async_trait::async_trait;
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{Adapter, Credential, Secret};
use connectors_sql::{Config, Engine, Sql};
use fixture::*;
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc, time::Duration};

struct Password(&'static str);
#[async_trait]
impl Credential for Password {
    async fn resolve(&self) -> Result<Secret> {
        Ok(Secret(self.0.as_bytes().to_vec()))
    }
}

fn config(port: u16, allow_plaintext: bool, ca_file: Option<PathBuf>) -> Config {
    Config {
        engine: Engine::Mysql,
        host: "127.0.0.1".into(),
        port,
        database: "fixture".into(),
        user: "reader".into(),
        allow_plaintext,
        ca_file,
    }
}
fn adapter_with(config: Config, password: &'static str) -> Sql {
    let effective = json!({"service":{"instance":"mysql-fixture","listen":"127.0.0.1:0","service_credential":{"kind":"environment","name":"UNUSED"}},"password":{"kind":"environment","name":"UNUSED"},"adapter":config});
    Sql::new(
        "mysql-fixture",
        config,
        effective,
        Arc::new(Password(password)),
    )
    .unwrap()
}
fn adapter(server: &Server) -> Sql {
    adapter_with(config(server.port, true, None), PASSWORD)
}
async fn invoke(sql: &Sql, operation: &str, input: Value) -> Result<Value> {
    tokio::time::timeout(Duration::from_secs(5), sql.invoke(operation, input))
        .await
        .expect("MySQL fixture hung")
}
fn sanitized(error: &connectors_core::Error) {
    let text = serde_json::to_string(error).unwrap();
    assert!(!text.contains("private-provider-detail"), "{text}");
    assert!(!text.contains(PASSWORD), "{text}");
}

#[tokio::test]
async fn mysql_connect_is_the_credential_check_and_revalidates() {
    let server = start(Script::default()).await;
    let sql = adapter(&server);
    sql.validate_session().await.unwrap();
    sql.validate_session().await.unwrap();
    {
        let log = server.log.lock().unwrap();
        assert_eq!(log.logins.len(), 2, "each validation opens one session");
        for login in &log.logins {
            assert_eq!(login.user, "reader");
            assert_eq!(login.database, "fixture");
            assert_eq!(login.plugin, "caching_sha2_password");
            assert!(login.password_matches, "the scramble proves the password");
            assert!(!login.tls);
        }
        assert!(
            log.statements.is_empty(),
            "validation runs no statement: {:?}",
            log.statements
        );
    }
    let refused = adapter_with(config(server.port, true, None), "wrong-password");
    let error = refused.validate_session().await.unwrap_err();
    assert_eq!(error.code, ErrorCode::Unauthorized);
    sanitized(&error);
}

#[tokio::test]
async fn mysql_typed_values_map_deterministically() {
    let script = Script {
        columns: vec![
            col("signed_big", LONGLONG, 0, BINARY_CHARSET, 0),
            col("unsigned_big", LONGLONG, UNSIGNED, BINARY_CHARSET, 0),
            col("small", LONG, 0, BINARY_CHARSET, 0),
            col("amount", NEWDECIMAL, 0, BINARY_CHARSET, 4),
            col("ratio", DOUBLE, 0, BINARY_CHARSET, 31),
            col("day", DATE, 0, BINARY_CHARSET, 0),
            col("moment", DATETIME, 0, BINARY_CHARSET, 6),
            col("stamp", TIMESTAMP, 0, BINARY_CHARSET, 0),
            col("elapsed", TIME, 0, BINARY_CHARSET, 0),
            col("blob", VAR_STRING, 0, BINARY_CHARSET, 0),
            col("label", VAR_STRING, 0, UTF8MB4, 0),
            col("document", JSON, 0, BINARY_CHARSET, 0),
            col("missing", VAR_STRING, 0, UTF8MB4, 0),
        ],
        rows: vec![vec![
            int8(-9_007_199_254_740_993),
            uint8(u64::MAX),
            int4(42),
            text("12345678901234567890.0123"),
            double(1.5),
            date(2026, 10, 9),
            datetime(2026, 10, 9, [8, 7, 6], 123),
            datetime(2026, 10, 9, [8, 7, 6], 0),
            time(true, 1, [2, 3, 4], 0),
            bytes(&[0, 1, 2, 255]),
            text("snow 雪"),
            text(r#"{"a": 1}"#),
            None,
        ]],
        ..Script::default()
    };
    let server = start(script).await;
    let result = invoke(
        &adapter(&server),
        "query.read",
        json!({"query":"SELECT * FROM typed;","limit":10}),
    )
    .await
    .unwrap();
    assert_eq!(
        result["rows"],
        json!([[
            "-9007199254740993",
            "18446744073709551615",
            "42",
            "12345678901234567890.0123",
            "1.5",
            "2026-10-09",
            "2026-10-09T08:07:06.000123",
            "2026-10-09T08:07:06Z",
            "-26:03:04",
            "AAEC/w==",
            "snow 雪",
            r#"{"a": 1}"#,
            null
        ]])
    );
    let columns = result["columns"].as_array().unwrap();
    let names: Vec<_> = columns.iter().map(|c| c["name"].clone()).collect();
    let types: Vec<_> = columns.iter().map(|c| c["native_type"].clone()).collect();
    assert_eq!(
        names,
        [
            "signed_big",
            "unsigned_big",
            "small",
            "amount",
            "ratio",
            "day",
            "moment",
            "stamp",
            "elapsed",
            "blob",
            "label",
            "document",
            "missing"
        ]
    );
    assert_eq!(
        types,
        [
            "bigint",
            "bigint unsigned",
            "int",
            "decimal",
            "double",
            "date",
            "datetime",
            "timestamp",
            "time",
            "varbinary",
            "varchar",
            "json",
            "varchar"
        ]
    );
    assert_eq!(result["truncated"], false);
    assert_eq!(result["provenance"]["instance"], "mysql-fixture");
    assert_eq!(result["provenance"]["resource"], "fixture");

    // The session is made read-only and bounded before the caller's statement
    // is even prepared, and the statement runs wrapped in a bounded derived table.
    let log = server.log.lock().unwrap();
    assert_eq!(
        log.statements,
        [
            "SET SESSION TRANSACTION READ ONLY",
            "SET SESSION max_execution_time = 10000, SESSION lock_wait_timeout = 2, SESSION innodb_lock_wait_timeout = 2, SESSION time_zone = '+00:00'",
            "SELECT * FROM typed",
            "SELECT * FROM (SELECT * FROM typed\n) AS result (c0,c1,c2,c3,c4,c5,c6,c7,c8,c9,c10,c11,c12) LIMIT 11",
        ]
    );
    assert_eq!(log.executions, [Vec::<Option<String>>::new()]);
}

#[tokio::test]
async fn mysql_truncation_and_parameters_reach_the_wire_without_interpolation() {
    let script = Script {
        columns: vec![col("n", LONG, 0, BINARY_CHARSET, 0)],
        rows: vec![vec![int4(1)], vec![int4(2)], vec![int4(3)]],
        params: 3,
        ..Script::default()
    };
    let server = start(script).await;
    let values = json!(["a'雪\\value", "", null]);
    let result = invoke(
        &adapter(&server),
        "query.read",
        json!({"query":"SELECT n FROM t WHERE a = ? AND b = ? AND c <=> ?","parameters":values,"limit":2}),
    )
    .await
    .unwrap();
    assert_eq!(result["rows"], json!([["1"], ["2"]]));
    assert_eq!(result["truncated"], true);
    {
        let log = server.log.lock().unwrap();
        assert_eq!(
            log.executions,
            [vec![Some("a'雪\\value".into()), Some(String::new()), None]]
        );
        assert!(log.prepares[1].ends_with(" LIMIT 3"));
    }

    // A parameter count the statement does not take is refused before execution.
    let server = start(Script {
        params: 1,
        ..Script::default()
    })
    .await;
    let error = invoke(
        &adapter(&server),
        "query.read",
        json!({"query":"SELECT ?","limit":1}),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert!(server.log.lock().unwrap().executions.is_empty());
}

#[tokio::test]
async fn mysql_write_statements_are_refused() {
    // A statement that returns no rows is refused by the same column guard the
    // PostgreSQL path applies, before it is ever executed.
    let server = start(Script::default()).await;
    let sql = adapter(&server);
    for statement in [
        "INSERT INTO incidents VALUES (1)",
        "UPDATE incidents SET state = 'closed'",
        "DELETE FROM incidents",
        "SET SESSION TRANSACTION READ WRITE",
    ] {
        let error = invoke(&sql, "query.read", json!({"query":statement,"limit":1}))
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::Unsupported, "{statement}");
    }
    assert!(server.log.lock().unwrap().executions.is_empty());

    // A write the server itself refuses in the read-only session is Forbidden.
    let server = start(Script {
        execute_error: Some((1792, "25006")),
        ..Script::default()
    })
    .await;
    let error = invoke(
        &adapter(&server),
        "query.read",
        json!({"query":"SELECT write_through_a_function()","limit":1}),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::Forbidden);
    sanitized(&error);
    assert_eq!(
        server.log.lock().unwrap().statements[0],
        "SET SESSION TRANSACTION READ ONLY"
    );
}

#[tokio::test]
async fn mysql_schema_list_reads_information_schema() {
    let script = Script {
        columns: vec![
            col("table_schema", VAR_STRING, 0, UTF8MB4, 0),
            col("table_name", VAR_STRING, 0, UTF8MB4, 0),
            col("column_name", VAR_STRING, 0, UTF8MB4, 0),
            col("data_type", VAR_STRING, 0, UTF8MB4, 0),
            col("udt_name", VAR_STRING, 0, UTF8MB4, 0),
            col("is_nullable", VAR_STRING, 0, UTF8MB4, 0),
            col("ordinal_position", LONG, UNSIGNED, BINARY_CHARSET, 0),
        ],
        rows: vec![vec![
            text("fixture"),
            text("incidents"),
            text("id"),
            text("bigint"),
            text("bigint unsigned"),
            text("NO"),
            int4(1),
        ]],
        params: 1,
        ..Script::default()
    };
    let server = start(script).await;
    let result = invoke(
        &adapter(&server),
        "schema.list",
        json!({"schema":"fixture","limit":100}),
    )
    .await
    .unwrap();
    assert_eq!(
        result["rows"],
        json!([[
            "fixture",
            "incidents",
            "id",
            "bigint",
            "bigint unsigned",
            "NO",
            "1"
        ]])
    );
    let names: Vec<_> = result["columns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        names,
        [
            "table_schema",
            "table_name",
            "column_name",
            "data_type",
            "udt_name",
            "is_nullable",
            "ordinal_position"
        ]
    );
    let log = server.log.lock().unwrap();
    assert!(
        log.prepares[0].contains("FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = ?"),
        "{}",
        log.prepares[0]
    );
    assert_eq!(log.executions, [vec![Some("fixture".to_owned())]]);
}

/// A MySQL connection is bound to one database, and on MySQL a schema is a
/// database: `schema.list` reads the connected database when no schema is
/// given, and refuses any other before a session is opened.
#[tokio::test]
async fn mysql_schema_list_reads_only_the_connected_database() {
    let script = Script {
        columns: (0..7)
            .map(|_| col("c", VAR_STRING, 0, UTF8MB4, 0))
            .collect(),
        rows: vec![],
        params: 1,
        ..Script::default()
    };
    let server = start(script).await;
    let sql = adapter(&server);
    for input in [
        json!({"schema":"other","limit":10}),
        json!({"schema":"FIXTURE","limit":10}),
        json!({"schema":"fixture ","limit":10}),
    ] {
        let error = invoke(&sql, "schema.list", input.clone())
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput, "{input}");
    }
    assert_eq!(
        server.log.lock().unwrap().sessions,
        0,
        "a refused schema opens no session"
    );
    let result = invoke(&sql, "schema.list", json!({"limit":10}))
        .await
        .unwrap();
    assert_eq!(result["provenance"]["resource"], "fixture");
    let log = server.log.lock().unwrap();
    assert_eq!(log.executions, [vec![Some("fixture".to_owned())]]);
}

/// TIMESTAMP is an instant: the session's time zone is UTC before the
/// caller's statement is prepared, and the cell is written with `Z`. DATETIME
/// records no time zone and stays a naive local date-time.
#[tokio::test]
async fn mysql_timestamp_is_written_as_an_instant_in_utc() {
    let script = Script {
        columns: vec![
            col("stamp", TIMESTAMP, 0, BINARY_CHARSET, 3),
            col("plain_stamp", TIMESTAMP, 0, BINARY_CHARSET, 0),
            col("moment", DATETIME, 0, BINARY_CHARSET, 3),
        ],
        rows: vec![vec![
            datetime(2026, 10, 25, [0, 30, 0], 250_000),
            datetime(2026, 10, 25, [1, 30, 0], 0),
            datetime(2026, 10, 25, [0, 30, 0], 250_000),
        ]],
        ..Script::default()
    };
    let server = start(script).await;
    let result = invoke(
        &adapter(&server),
        "query.read",
        json!({"query":"SELECT stamp, plain_stamp, moment FROM instants","limit":10}),
    )
    .await
    .unwrap();
    assert_eq!(
        result["rows"],
        json!([[
            "2026-10-25T00:30:00.250Z",
            "2026-10-25T01:30:00Z",
            "2026-10-25T00:30:00.250"
        ]])
    );
    let log = server.log.lock().unwrap();
    let zone = log
        .statements
        .iter()
        .position(|s| s.starts_with("SET ") && s.contains("time_zone = '+00:00'"))
        .expect("the session's time zone is set to UTC");
    let prepared = log
        .statements
        .iter()
        .position(|s| s == "SELECT stamp, plain_stamp, moment FROM instants")
        .unwrap();
    assert!(zone < prepared, "{:?}", log.statements);
}

#[tokio::test]
async fn mysql_server_errors_are_classified_and_sanitized() {
    for (code, state, expected, answered) in [
        (1142, "42000", ErrorCode::Forbidden, false),
        (1044, "42000", ErrorCode::Forbidden, false),
        (1064, "42000", ErrorCode::InvalidInput, false),
        (1235, "42000", ErrorCode::Unsupported, false),
        (3024, "HY000", ErrorCode::Timeout, true),
        (1040, "08004", ErrorCode::Capacity, true),
        (2013, "HY000", ErrorCode::Unavailable, false),
    ] {
        let server = start(Script {
            prepare_error: Some((code, state)),
            ..Script::default()
        })
        .await;
        let error = invoke(
            &adapter(&server),
            "query.read",
            json!({"query":"SELECT 1","limit":1}),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code, expected, "MySQL error {code}");
        assert_eq!(error.upstream_answer, answered, "MySQL error {code}");
        sanitized(&error);
    }
}

#[tokio::test]
async fn mysql_plaintext_is_refused_unless_allowed() {
    // The server offers no TLS. Without allow_plaintext the client refuses
    // before it sends a user name or a password scramble.
    let server = start(Script::default()).await;
    let sql = adapter_with(config(server.port, false, None), PASSWORD);
    let error = sql.validate_session().await.unwrap_err();
    assert_eq!(error.code, ErrorCode::Unavailable);
    sanitized(&error);
    let error = invoke(&sql, "query.read", json!({"query":"SELECT 1","limit":1}))
        .await
        .unwrap_err();
    sanitized(&error);
    let log = server.log.lock().unwrap();
    assert_eq!(log.sessions, 2);
    assert!(log.logins.is_empty(), "no credential crossed plaintext");
}

fn tls_material(directory: &std::path::Path) -> (Arc<tokio_rustls::rustls::ServerConfig>, PathBuf) {
    let certificate = rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()]).unwrap();
    let ca = directory.join("ca.pem");
    std::fs::write(&ca, certificate.cert.pem()).unwrap();
    let key = tokio_rustls::rustls::pki_types::PrivateKeyDer::Pkcs8(
        certificate.signing_key.serialize_der().into(),
    );
    let config = tokio_rustls::rustls::ServerConfig::builder_with_provider(Arc::new(
        tokio_rustls::rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(vec![certificate.cert.der().clone()], key)
    .unwrap();
    (Arc::new(config), ca)
}

#[tokio::test]
async fn mysql_tls_trusts_exactly_the_configured_ca() {
    let directory = tempfile::tempdir().unwrap();
    let (tls, ca) = tls_material(directory.path());
    let server = start(Script {
        tls: Some(tls),
        ..Script::default()
    })
    .await;
    // The configured CA replaces the public roots and authenticates the server.
    let sql = adapter_with(config(server.port, false, Some(ca)), PASSWORD);
    sql.validate_session().await.unwrap();
    let result = invoke(
        &sql,
        "query.read",
        json!({"query":"SELECT 42, NULL","limit":1}),
    )
    .await
    .unwrap();
    assert_eq!(result["rows"], json!([["42", null]]));
    // Without it, the public roots do not trust the fixture's certificate.
    let untrusted = adapter_with(config(server.port, false, None), PASSWORD);
    assert!(untrusted.validate_session().await.is_err());
    {
        let log = server.log.lock().unwrap();
        assert_eq!(log.logins.len(), 2);
        assert!(
            log.logins
                .iter()
                .all(|login| login.tls && login.password_matches)
        );
    }
    // An empty CA bundle is refused before any session, as on PostgreSQL.
    let empty = directory.path().join("empty.pem");
    std::fs::write(&empty, "").unwrap();
    let sessions = server.log.lock().unwrap().sessions;
    let error = adapter_with(config(server.port, false, Some(empty)), PASSWORD)
        .validate_session()
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert_eq!(server.log.lock().unwrap().sessions, sessions);
}

async fn stalled(ignore_kill: bool) -> (tokio::task::JoinHandle<Result<Value>>, Server) {
    let started = Arc::new(tokio::sync::Notify::new());
    let server = start(Script {
        stall: Some(started.clone()),
        ignore_kill,
        ..Script::default()
    })
    .await;
    let sql = adapter(&server);
    let call = tokio::spawn(async move {
        sql.invoke("query.read", json!({"query":"SELECT 1","limit":1}))
            .await
    });
    tokio::time::timeout(Duration::from_secs(3), started.notified())
        .await
        .unwrap();
    (call, server)
}
async fn killed(server: &Server, within: Duration) {
    let until = tokio::time::Instant::now() + within;
    while server.log.lock().unwrap().kills.is_empty() {
        assert!(tokio::time::Instant::now() < until, "no KILL QUERY arrived");
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert_eq!(
        server.log.lock().unwrap().kills,
        [format!("KILL QUERY {CONNECTION_ID}")]
    );
}

#[tokio::test]
async fn mysql_deadline_kills_the_running_query() {
    let (call, server) = stalled(false).await;
    let error = tokio::time::timeout(Duration::from_secs(13), call)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Timeout);
    assert!(!error.upstream_answer);
    killed(&server, Duration::from_secs(3)).await;
}

#[tokio::test]
async fn mysql_dropping_the_invocation_still_kills_the_query() {
    let (call, server) = stalled(false).await;
    call.abort();
    assert!(call.await.unwrap_err().is_cancelled());
    killed(&server, Duration::from_secs(3)).await;
}

#[tokio::test]
async fn mysql_unresponsive_server_cannot_extend_the_cleanup_budget() {
    let (call, server) = stalled(true).await;
    call.abort();
    assert!(call.await.unwrap_err().is_cancelled());
    killed(&server, Duration::from_secs(3)).await;
    // The kill went unanswered; the cleanup budget still closes the session.
    let until = tokio::time::Instant::now() + Duration::from_secs(3);
    while server.log.lock().unwrap().abandoned == 0 {
        assert!(
            tokio::time::Instant::now() < until,
            "the stalled session was never closed"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

#[tokio::test]
async fn mysql_a_trailing_line_comment_cannot_swallow_the_wrapper() {
    // `--` and `#` comment to the end of the line, so the derived table closes
    // on the next one: the caller's comment cannot reach the alias list or LIMIT.
    let server = start(Script::default()).await;
    let result = invoke(
        &adapter(&server),
        "query.read",
        json!({"query":"SELECT 42, NULL -- the answer","limit":1}),
    )
    .await
    .unwrap();
    assert_eq!(result["rows"], json!([["42", null]]));
    let log = server.log.lock().unwrap();
    assert_eq!(
        log.prepares[1],
        "SELECT * FROM (SELECT 42, NULL -- the answer\n) AS result (c0,c1) LIMIT 2"
    );
}

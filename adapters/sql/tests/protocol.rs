//! PostgreSQL wire fixtures exercise the adapter through its public library API.
//! No database, Docker, environment credentials or network outside loopback is needed.
use async_trait::async_trait;
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{Adapter, Credential, Secret};
use connectors_sql::{Config, Sql};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

struct Password(Arc<AtomicUsize>);
#[async_trait]
impl Credential for Password {
    async fn resolve(&self) -> Result<Secret> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(Secret(b"fixture-password".to_vec()))
    }
}
fn adapter(port: u16, resolutions: Arc<AtomicUsize>) -> Sql {
    let config = Config {
        engine: connectors_sql::Engine::Postgresql,
        host: "127.0.0.1".into(),
        port,
        database: "fixture".into(),
        user: "reader".into(),
        allow_plaintext: true,
        ca_file: None,
    };
    let effective = json!({"service":{"instance":"sql-fixture","listen":"127.0.0.1:0","service_credential":{"kind":"environment","name":"UNUSED"}},"password":{"kind":"environment","name":"UNUSED"},"adapter":config});
    Sql::new(
        "sql-fixture",
        config,
        effective,
        Arc::new(Password(resolutions)),
    )
    .unwrap()
}
#[derive(Clone)]
struct Scenario {
    rows: Vec<Option<Vec<Option<String>>>>,
    parameters: usize,
    auth_error: Option<&'static str>,
    query_error: Option<&'static str>,
    bound_values: Vec<Option<String>>,
    hang_on_execute: Option<Arc<tokio::sync::Notify>>,
    ignore_cancel: bool,
    /// The columns the caller's statement returns: name and type OID. A
    /// catalogue answer names its own; the default is `answer int4, nullable text`.
    columns: Vec<(&'static str, i32)>,
    /// Every statement text the client prepared, in order.
    prepared: Arc<std::sync::Mutex<Vec<String>>>,
}
impl Default for Scenario {
    fn default() -> Self {
        Self {
            rows: vec![Some(vec![Some("42".into()), None])],
            parameters: 0,
            auth_error: None,
            query_error: None,
            bound_values: vec![],
            hang_on_execute: None,
            ignore_cancel: false,
            columns: vec![("answer", 23), ("nullable", 25)],
            prepared: Arc::default(),
        }
    }
}
/// PostgreSQL type OIDs a catalogue answer uses.
const NAME: i32 = 19;
const TEXT: i32 = 25;
const INT8: i32 = 20;
const INT2: i32 = 21;
async fn send(stream: &mut TcpStream, tag: u8, body: &[u8]) {
    stream.write_u8(tag).await.unwrap();
    stream.write_i32((body.len() + 4) as i32).await.unwrap();
    stream.write_all(body).await.unwrap();
}
async fn receive(stream: &mut TcpStream) -> Option<(u8, Vec<u8>)> {
    let tag = stream.read_u8().await.ok()?;
    let len = stream.read_i32().await.unwrap() as usize;
    assert!((4..65536).contains(&len));
    let mut body = vec![0; len - 4];
    stream.read_exact(&mut body).await.unwrap();
    Some((tag, body))
}
fn error(code: &str) -> Vec<u8> {
    format!("SERROR\0C{code}\0Mprivate-provider-detail fixture-password\0\0").into_bytes()
}
fn description(wrapped: bool, columns: &[(&'static str, i32)]) -> Vec<u8> {
    let fields = if wrapped {
        vec![("array", 1009_i32)]
    } else {
        columns.to_vec()
    };
    let mut body = (fields.len() as i16).to_be_bytes().to_vec();
    for (name, oid) in fields {
        body.extend_from_slice(name.as_bytes());
        body.push(0);
        body.extend(0_i32.to_be_bytes());
        body.extend(0_i16.to_be_bytes());
        body.extend(oid.to_be_bytes());
        body.extend((-1_i16).to_be_bytes());
        body.extend((-1_i32).to_be_bytes());
        body.extend(0_i16.to_be_bytes());
    }
    body
}
fn data(row: &Option<Vec<Option<String>>>) -> Vec<u8> {
    let mut body = 1_i16.to_be_bytes().to_vec();
    if let Some(row) = row {
        let mut array = 1_i32.to_be_bytes().to_vec();
        array.extend(1_i32.to_be_bytes());
        array.extend(25_i32.to_be_bytes());
        array.extend((row.len() as i32).to_be_bytes());
        array.extend(1_i32.to_be_bytes());
        for value in row {
            if let Some(value) = value {
                array.extend((value.len() as i32).to_be_bytes());
                array.extend(value.as_bytes());
            } else {
                array.extend((-1_i32).to_be_bytes());
            }
        }
        body.extend((array.len() as i32).to_be_bytes());
        body.extend(array);
    } else {
        body.extend((-1_i32).to_be_bytes());
    }
    body
}
fn cstring<'a>(body: &mut &'a [u8]) -> &'a str {
    let end = body.iter().position(|b| *b == 0).unwrap();
    let value = std::str::from_utf8(&body[..end]).unwrap();
    *body = &body[end + 1..];
    value
}
fn i16_field(body: &mut &[u8]) -> i16 {
    let (value, tail) = body.split_at(2);
    *body = tail;
    i16::from_be_bytes(value.try_into().unwrap())
}
fn check_bind(mut body: &[u8], expected: &[Option<String>]) {
    cstring(&mut body); // Portal name.
    cstring(&mut body); // Prepared statement name.
    for _ in 0..i16_field(&mut body) {
        assert_eq!(i16_field(&mut body), 0, "parameters use text format");
    }
    assert_eq!(i16_field(&mut body) as usize, expected.len());
    for value in expected {
        let len = i32::from_be_bytes(body[..4].try_into().unwrap());
        body = &body[4..];
        if let Some(value) = value {
            assert_eq!(len as usize, value.len());
            assert_eq!(&body[..len as usize], value.as_bytes());
            body = &body[len as usize..];
        } else {
            assert_eq!(len, -1);
        }
    }
}
async fn fixture(mut scenario: Scenario) -> (Sql, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let sql = adapter(
        listener.local_addr().unwrap().port(),
        Arc::new(AtomicUsize::new(0)),
    );
    let task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let len = stream.read_i32().await.unwrap();
        let mut startup = vec![0; len as usize - 4];
        stream.read_exact(&mut startup).await.unwrap();
        assert!(startup.windows(7).any(|w| w == b"reader\0"));
        send(&mut stream, b'R', &3_i32.to_be_bytes()).await;
        let (tag, password) = receive(&mut stream).await.unwrap();
        assert_eq!(tag, b'p');
        assert_eq!(password, b"fixture-password\0");
        if let Some(code) = scenario.auth_error {
            send(&mut stream, b'E', &error(code)).await;
            return;
        }
        send(&mut stream, b'R', &0_i32.to_be_bytes()).await;
        let mut key = 42_i32.to_be_bytes().to_vec();
        key.extend(1234_i32.to_be_bytes());
        send(&mut stream, b'K', &key).await;
        send(&mut stream, b'Z', b"I").await;
        let mut prepares = 0;
        let mut original = String::new();
        let mut setup = false;
        let mut row = 0;
        let mut failed = false;
        while let Some((tag, body)) = receive(&mut stream).await {
            match tag {
                b'Q' => {
                    let query = String::from_utf8(body).unwrap();
                    if query.starts_with("START") || query.starts_with("BEGIN") {
                        assert!(query.contains("READ ONLY"));
                    } else if query.starts_with("SET") {
                        assert_eq!(
                            query.trim_end_matches('\0'),
                            "SET LOCAL statement_timeout = '10s'; SET LOCAL lock_timeout = '2s'; SET LOCAL search_path = public, pg_catalog"
                        );
                        setup = true;
                    }
                    send(&mut stream, b'C', b"OK\0").await;
                    send(&mut stream, b'Z', b"T").await;
                }
                b'P' => {
                    assert!(
                        setup,
                        "query preparation preceded trusted transaction setup"
                    );
                    let mut body = body.as_slice();
                    cstring(&mut body);
                    let query = cstring(&mut body);
                    prepares += 1;
                    scenario.prepared.lock().unwrap().push(query.to_owned());
                    if prepares == 1 {
                        original = query.into();
                    } else {
                        assert!(query.starts_with(
                            "SELECT CASE WHEN (COALESCE(octet_length(result.c0::text),0)::bigint"
                        ));
                        assert!(query.contains(&format!(
                            ") > {} THEN NULL::text[] ELSE ARRAY[",
                            connectors_core::RESPONSE_LIMIT
                        )));
                        let aliases = (0..scenario.columns.len())
                            .map(|i| format!("c{i}"))
                            .collect::<Vec<_>>();
                        let casts = aliases
                            .iter()
                            .map(|c| format!("result.{c}::text"))
                            .collect::<Vec<_>>()
                            .join(",");
                        assert!(query.contains(&format!("ARRAY[{casts}]")));
                        assert!(query.ends_with(&format!(
                            "FROM ({original}) AS result({})",
                            aliases.join(",")
                        )));
                    }
                    if let Some(code) = scenario.query_error {
                        send(&mut stream, b'E', &error(code)).await;
                        failed = true;
                    } else {
                        send(&mut stream, b'1', b"").await;
                    }
                }
                b'D' if !failed => {
                    let mut params = (scenario.parameters as i16).to_be_bytes().to_vec();
                    for _ in 0..scenario.parameters {
                        params.extend(25_i32.to_be_bytes());
                    }
                    send(&mut stream, b't', &params).await;
                    send(
                        &mut stream,
                        b'T',
                        &description(prepares > 1, &scenario.columns),
                    )
                    .await;
                }
                b'B' => {
                    check_bind(&body, &scenario.bound_values);
                    send(&mut stream, b'2', b"").await;
                }
                b'E' => {
                    assert_eq!(&body[body.len() - 4..], &1_i32.to_be_bytes());
                    if let Some(started) = scenario.hang_on_execute.take() {
                        started.notify_one();
                        let (mut cancel, _) = listener.accept().await.unwrap();
                        let mut message = [0; 16];
                        cancel.read_exact(&mut message).await.unwrap();
                        assert_eq!(&message[..4], &16_i32.to_be_bytes());
                        assert_eq!(&message[4..8], &80877102_i32.to_be_bytes());
                        assert_eq!(&message[8..12], &42_i32.to_be_bytes());
                        assert_eq!(&message[12..], &1234_i32.to_be_bytes());
                        if scenario.ignore_cancel {
                            // Never finish Execute. The cleanup budget must still
                            // close the original connection, rather than leak it.
                            let _ = stream.read_to_end(&mut Vec::new()).await;
                            return;
                        }
                        send(&mut stream, b'E', &error("57014")).await;
                        failed = true;
                        continue;
                    }
                    if let Some(value) = scenario.rows.get(row) {
                        send(&mut stream, b'D', &data(value)).await;
                        send(&mut stream, b's', b"").await;
                        row += 1;
                    } else {
                        send(&mut stream, b'C', b"SELECT 0\0").await;
                    }
                }
                b'S' => {
                    send(&mut stream, b'Z', b"T").await;
                    failed = false;
                }
                b'C' => send(&mut stream, b'3', b"").await,
                b'X' => break,
                _ => {}
            }
        }
    });
    (sql, task)
}
async fn invoke(scenario: Scenario, input: Value) -> Result<Value> {
    let (sql, task) = fixture(scenario).await;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        sql.invoke("query.read", input),
    )
    .await
    .expect("protocol fixture hung");
    task.await.unwrap();
    result
}
#[tokio::test]
async fn parsing_and_limits_refuse_before_credentials_or_connection() {
    let count = Arc::new(AtomicUsize::new(0));
    let sql = adapter(1, count.clone());
    for input in [
        json!({}),
        json!({"query":"SELECT 1","limit":0}),
        json!({"query":"SELECT 1","limit":1001}),
        json!({"query":"x".repeat(8193),"limit":1}),
        json!({"query":"SELECT 1","limit":1,"parameters":[true]}),
        json!({"query":"SELECT 1","limit":1,"parameters":vec!["x";65]}),
        json!({"query":"SELECT 1","limit":1,"unknown":true}),
    ] {
        assert_eq!(
            sql.invoke("query.read", input).await.unwrap_err().code,
            ErrorCode::InvalidInput
        );
    }
    // On PostgreSQL the caller names the schema: an empty one or none at all
    // is refused before any credential or connection.
    for input in [json!({"schema":"","limit":1}), json!({"limit":1})] {
        assert_eq!(
            sql.invoke("schema.list", input).await.unwrap_err().code,
            ErrorCode::InvalidInput
        );
    }
    assert_eq!(
        sql.invoke("missing", json!({})).await.unwrap_err().code,
        ErrorCode::NotFound
    );
    assert_eq!(count.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn rows_nulls_native_types_and_truncation_are_preserved() {
    let result = invoke(
        Scenario::default(),
        json!({"query":"SELECT 42, NULL","limit":1}),
    )
    .await
    .unwrap();
    assert_eq!(result["rows"], json!([["42", null]]));
    assert_eq!(result["columns"][0]["native_type"], "int4");
    assert_eq!(result["truncated"], false);
    let scenario = Scenario {
        rows: vec![Some(vec![Some("1".into()), None]); 2],
        ..Scenario::default()
    };
    let result = invoke(scenario, json!({"query":"SELECT 1, NULL","limit":1}))
        .await
        .unwrap();
    assert_eq!(result["rows"].as_array().unwrap().len(), 1);
    assert_eq!(result["truncated"], true);
    let result = invoke(
        Scenario {
            rows: vec![],
            ..Scenario::default()
        },
        json!({"query":"SELECT 1, NULL WHERE false","limit":1}),
    )
    .await
    .unwrap();
    assert_eq!(result["rows"], json!([]));
    assert_eq!(result["truncated"], false);
}
#[tokio::test]
async fn adversary_only_exact_feature_not_supported_changes_classification() {
    // The accepted correction names one SQLSTATE, not the whole 0A class.
    // Custom neighboring codes exercise the actual provider-error wire path.
    for (state, expected) in [
        ("0A000", ErrorCode::Unsupported),
        ("0A001", ErrorCode::Unavailable),
        ("0A999", ErrorCode::Unavailable),
    ] {
        let error = invoke(
            Scenario {
                query_error: Some(state),
                ..Scenario::default()
            },
            json!({"query":"SELECT 1","limit":1}),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code, expected, "SQLSTATE {state}");
        assert!(!error.upstream_answer, "SQLSTATE {state}");
        let serialized = serde_json::to_string(&error).unwrap();
        assert!(!serialized.contains("private-provider-detail"));
        assert!(!serialized.contains("fixture-password"));
    }
}

#[tokio::test]
async fn authentication_database_errors_and_row_capacity_are_sanitized() {
    for (code, expected) in [
        ("0A000", ErrorCode::Unsupported),
        ("28P01", ErrorCode::Unauthorized),
        ("42501", ErrorCode::Forbidden),
        ("25006", ErrorCode::Forbidden),
        ("57014", ErrorCode::Timeout),
        ("53300", ErrorCode::Capacity),
        ("42601", ErrorCode::InvalidInput),
        ("22003", ErrorCode::InvalidInput),
        ("08006", ErrorCode::Unavailable),
    ] {
        let scenario = if code == "28P01" {
            Scenario {
                auth_error: Some(code),
                ..Scenario::default()
            }
        } else {
            Scenario {
                query_error: Some(code),
                ..Scenario::default()
            }
        };
        let error = invoke(scenario, json!({"query":"SELECT 1","limit":1}))
            .await
            .unwrap_err();
        assert_eq!(error.code, expected);
        assert_eq!(error.upstream_answer, matches!(code, "57014" | "53300"));
        let text = serde_json::to_string(&error).unwrap();
        assert!(!text.contains("private-provider-detail"));
        assert!(!text.contains("fixture-password"));
    }
    let error = invoke(
        Scenario {
            rows: vec![None],
            ..Scenario::default()
        },
        json!({"query":"SELECT 1","limit":1}),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::Capacity);
    let error = invoke(
        Scenario {
            parameters: 1,
            ..Scenario::default()
        },
        json!({"query":"SELECT $1","limit":1}),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput);
}

#[tokio::test]
async fn text_null_and_empty_parameters_reach_bind_without_interpolation() {
    let values = vec![Some("a'雪\\value".into()), Some(String::new()), None];
    let result = invoke(
        Scenario { parameters: 3, bound_values: values.clone(), ..Scenario::default() },
        json!({"query":"SELECT $1::text, $2::text WHERE $3::text IS NULL", "parameters":values, "limit":1}),
    ).await.unwrap();
    assert_eq!(result["rows"], json!([["42", null]]));
}

async fn stalled(
    ignore_cancel: bool,
) -> (
    tokio::task::JoinHandle<Result<Value>>,
    tokio::task::JoinHandle<()>,
) {
    let started = Arc::new(tokio::sync::Notify::new());
    let (sql, server) = fixture(Scenario {
        hang_on_execute: Some(started.clone()),
        ignore_cancel,
        ..Scenario::default()
    })
    .await;
    let call = tokio::spawn(async move {
        sql.invoke("query.read", json!({"query":"SELECT 1", "limit":1}))
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(3), started.notified())
        .await
        .unwrap();
    (call, server)
}

#[tokio::test]
async fn deadline_sends_the_backend_cancel_key_and_drains_the_connection() {
    let (call, server) = stalled(false).await;
    let error = tokio::time::timeout(std::time::Duration::from_secs(13), call)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Timeout);
    tokio::time::timeout(std::time::Duration::from_secs(1), server)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn dropping_the_invocation_still_cancels_the_database() {
    let (call, server) = stalled(false).await;
    call.abort();
    assert!(call.await.unwrap_err().is_cancelled());
    tokio::time::timeout(std::time::Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn unresponsive_database_cannot_extend_the_cleanup_budget() {
    let (call, server) = stalled(true).await;
    call.abort();
    assert!(call.await.unwrap_err().is_cancelled());
    tokio::time::timeout(std::time::Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap();
}

// The catalogue reads on PostgreSQL. Each sends one fixed statement whose
// caller-supplied names are bound parameters, through the same read-only
// transaction, row limit and truncation flag as `query.read`.

async fn catalogue(scenario: Scenario, operation: &str, input: Value) -> Result<Value> {
    let (sql, task) = fixture(scenario).await;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        sql.invoke(operation, input),
    )
    .await
    .expect("protocol fixture hung");
    // Every catalogue read the fixture scripts opens exactly one session; a
    // read that never reaches the server fails here instead of waiting.
    match tokio::time::timeout(std::time::Duration::from_secs(5), task).await {
        Ok(joined) => joined.unwrap(),
        Err(_) => panic!("{operation}: no session reached the fixture: {result:?}"),
    }
    result
}
fn row(cells: &[Option<&str>]) -> Option<Vec<Option<String>>> {
    Some(cells.iter().map(|c| c.map(str::to_owned)).collect())
}
fn names(result: &Value) -> Vec<String> {
    result["columns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap().to_owned())
        .collect()
}
const TABLE_COLUMNS: [(&str, i32); 3] = [
    ("table_name", NAME),
    ("table_kind", TEXT),
    ("row_estimate", INT8),
];
const DESCRIBE_COLUMNS: [(&str, i32); 10] = [
    ("column_name", NAME),
    ("native_type", TEXT),
    ("is_nullable", TEXT),
    ("column_default", TEXT),
    ("ordinal_position", INT2),
    ("primary_key_position", INT8),
    ("foreign_key", NAME),
    ("referenced_schema", NAME),
    ("referenced_table", NAME),
    ("referenced_column", NAME),
];
const INDEX_COLUMNS: [(&str, i32); 6] = [
    ("index_name", NAME),
    ("table_name", NAME),
    ("column_position", INT8),
    ("column_name", TEXT),
    ("is_unique", TEXT),
    ("is_primary", TEXT),
];

#[tokio::test]
async fn postgresql_database_list_reads_connectable_databases_without_parameters() {
    let scenario = Scenario {
        columns: vec![("database_name", NAME)],
        rows: vec![row(&[Some("fixture")]), row(&[Some("postgres")])],
        ..Scenario::default()
    };
    let prepared = scenario.prepared.clone();
    let result = catalogue(scenario, "database.list", json!({"limit":10}))
        .await
        .unwrap();
    assert_eq!(names(&result), ["database_name"]);
    assert_eq!(result["columns"][0]["native_type"], "name");
    assert_eq!(result["rows"], json!([["fixture"], ["postgres"]]));
    assert_eq!(result["truncated"], false);
    assert_eq!(result["provenance"]["resource"], "fixture");
    let original = prepared.lock().unwrap()[0].clone();
    assert!(
        original.contains("FROM pg_catalog.pg_database"),
        "{original}"
    );
    assert!(original.contains("datallowconn"), "{original}");
    assert!(original.contains("NOT d.datistemplate"), "{original}");
    assert!(original.contains("'CONNECT'"), "{original}");
    // Truncated at the bound, and an empty answer is rows, not an error.
    let result = catalogue(
        Scenario {
            columns: vec![("database_name", NAME)],
            rows: vec![row(&[Some("a")]), row(&[Some("b")])],
            ..Scenario::default()
        },
        "database.list",
        json!({"limit":1}),
    )
    .await
    .unwrap();
    assert_eq!(result["rows"], json!([["a"]]));
    assert_eq!(result["truncated"], true);
    let result = catalogue(
        Scenario {
            columns: vec![("database_name", NAME)],
            rows: vec![],
            ..Scenario::default()
        },
        "database.list",
        json!({"limit":1}),
    )
    .await
    .unwrap();
    assert_eq!(result["rows"], json!([]));
    assert_eq!(result["truncated"], false);
}

#[tokio::test]
async fn postgresql_table_list_reports_kind_and_row_estimate_of_one_schema() {
    let hostile = "public'; DROP TABLE incidents; --";
    let scenario = Scenario {
        columns: TABLE_COLUMNS.to_vec(),
        rows: vec![
            row(&[Some("incidents"), Some("table"), Some("1200")]),
            row(&[Some("open_incidents"), Some("view"), None]),
            row(&[Some("unanalysed"), Some("table"), None]),
        ],
        parameters: 1,
        bound_values: vec![Some(hostile.into())],
        ..Scenario::default()
    };
    let prepared = scenario.prepared.clone();
    let result = catalogue(scenario, "table.list", json!({"schema":hostile,"limit":3}))
        .await
        .unwrap();
    assert_eq!(names(&result), ["table_name", "table_kind", "row_estimate"]);
    assert_eq!(
        result["rows"],
        json!([
            ["incidents", "table", "1200"],
            ["open_incidents", "view", null],
            ["unanalysed", "table", null]
        ])
    );
    assert_eq!(result["truncated"], false);
    let original = prepared.lock().unwrap()[0].clone();
    assert!(
        !original.contains("DROP"),
        "the schema is bound, not interpolated: {original}"
    );
    assert!(original.contains("n.nspname = $1"), "{original}");
    assert!(original.contains("reltuples"), "{original}");
    // Truncation at the bound.
    let result = catalogue(
        Scenario {
            columns: TABLE_COLUMNS.to_vec(),
            rows: vec![row(&[Some("a"), Some("table"), Some("0")]); 3],
            parameters: 1,
            bound_values: vec![Some("public".into())],
            ..Scenario::default()
        },
        "table.list",
        json!({"schema":"public","limit":2}),
    )
    .await
    .unwrap();
    assert_eq!(result["rows"].as_array().unwrap().len(), 2);
    assert_eq!(result["truncated"], true);
    // A schema with no visible relation is an empty answer.
    let result = catalogue(
        Scenario {
            columns: TABLE_COLUMNS.to_vec(),
            rows: vec![],
            parameters: 1,
            bound_values: vec![Some("empty".into())],
            ..Scenario::default()
        },
        "table.list",
        json!({"schema":"empty","limit":2}),
    )
    .await
    .unwrap();
    assert_eq!(result["rows"], json!([]));
    assert_eq!(result["truncated"], false);
}

#[tokio::test]
async fn postgresql_table_describe_reports_nullability_defaults_and_keys() {
    let hostile = "incidents\"; SELECT pg_sleep(10); --";
    let scenario = Scenario {
        columns: DESCRIBE_COLUMNS.to_vec(),
        rows: vec![
            row(&[
                Some("id"),
                Some("bigint"),
                Some("NO"),
                Some("nextval('incidents_id_seq'::regclass)"),
                Some("1"),
                Some("1"),
                None,
                None,
                None,
                None,
            ]),
            row(&[
                Some("team_id"),
                Some("integer"),
                Some("YES"),
                None,
                Some("2"),
                None,
                Some("incidents_team_fk"),
                Some("public"),
                Some("teams"),
                Some("id"),
            ]),
        ],
        parameters: 2,
        bound_values: vec![Some("public".into()), Some(hostile.into())],
        ..Scenario::default()
    };
    let prepared = scenario.prepared.clone();
    let result = catalogue(
        scenario,
        "table.describe",
        json!({"schema":"public","table":hostile,"limit":100}),
    )
    .await
    .unwrap();
    assert_eq!(
        names(&result),
        DESCRIBE_COLUMNS.iter().map(|(n, _)| *n).collect::<Vec<_>>()
    );
    assert_eq!(result["rows"][0][2], "NO");
    assert_eq!(result["rows"][0][5], "1");
    assert_eq!(result["rows"][1][8], "teams");
    let original = prepared.lock().unwrap()[0].clone();
    assert!(
        !original.contains("pg_sleep"),
        "the table is bound, not interpolated: {original}"
    );
    assert!(original.contains("n.nspname = $1"), "{original}");
    assert!(original.contains("c.relname = $2"), "{original}");
    assert!(original.contains("contype = 'p'"), "{original}");
    assert!(original.contains("contype = 'f'"), "{original}");
    // Truncation at the bound.
    let result = catalogue(
        Scenario {
            columns: DESCRIBE_COLUMNS.to_vec(),
            rows: vec![
                row(&[
                    Some("c"),
                    Some("text"),
                    Some("YES"),
                    None,
                    Some("1"),
                    None,
                    None,
                    None,
                    None,
                    None
                ]);
                2
            ],
            parameters: 2,
            bound_values: vec![Some("public".into()), Some("wide".into())],
            ..Scenario::default()
        },
        "table.describe",
        json!({"schema":"public","table":"wide","limit":1}),
    )
    .await
    .unwrap();
    assert_eq!(result["rows"].as_array().unwrap().len(), 1);
    assert_eq!(result["truncated"], true);
}

#[tokio::test]
async fn postgresql_table_describe_of_a_missing_table_is_not_found() {
    let error = catalogue(
        Scenario {
            columns: DESCRIBE_COLUMNS.to_vec(),
            rows: vec![],
            parameters: 2,
            bound_values: vec![Some("public".into()), Some("missing".into())],
            ..Scenario::default()
        },
        "table.describe",
        json!({"schema":"public","table":"missing","limit":10}),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::NotFound);
    assert!(!error.upstream_answer);
}

#[tokio::test]
async fn postgresql_index_list_reads_one_table_or_every_table_of_a_schema() {
    let scenario = Scenario {
        columns: INDEX_COLUMNS.to_vec(),
        rows: vec![
            row(&[
                Some("incidents_pkey"),
                Some("incidents"),
                Some("1"),
                Some("id"),
                Some("YES"),
                Some("YES"),
            ]),
            row(&[
                Some("incidents_team_opened"),
                Some("incidents"),
                Some("1"),
                Some("team_id"),
                Some("NO"),
                Some("NO"),
            ]),
            row(&[
                Some("incidents_team_opened"),
                Some("incidents"),
                Some("2"),
                Some("opened_at"),
                Some("NO"),
                Some("NO"),
            ]),
        ],
        parameters: 2,
        bound_values: vec![Some("public".into()), Some("incidents".into())],
        ..Scenario::default()
    };
    let prepared = scenario.prepared.clone();
    let result = catalogue(
        scenario,
        "index.list",
        json!({"schema":"public","table":"incidents","limit":10}),
    )
    .await
    .unwrap();
    assert_eq!(
        names(&result),
        INDEX_COLUMNS.iter().map(|(n, _)| *n).collect::<Vec<_>>()
    );
    assert_eq!(result["rows"][2][3], "opened_at");
    let original = prepared.lock().unwrap()[0].clone();
    assert!(original.contains("FROM pg_catalog.pg_index"), "{original}");
    assert!(original.contains("n.nspname = $1"), "{original}");
    // Without a table the second parameter is bound as NULL: every table of
    // the schema, and the same statement text.
    let scenario = Scenario {
        columns: INDEX_COLUMNS.to_vec(),
        rows: vec![
            row(&[
                Some("a_pkey"),
                Some("a"),
                Some("1"),
                Some("id"),
                Some("YES"),
                Some("YES")
            ]);
            2
        ],
        parameters: 2,
        bound_values: vec![Some("public".into()), None],
        ..Scenario::default()
    };
    let every = scenario.prepared.clone();
    let result = catalogue(scenario, "index.list", json!({"schema":"public","limit":1}))
        .await
        .unwrap();
    assert_eq!(result["truncated"], true);
    assert_eq!(every.lock().unwrap()[0], original);
    // No index is an empty answer.
    let result = catalogue(
        Scenario {
            columns: INDEX_COLUMNS.to_vec(),
            rows: vec![],
            parameters: 2,
            bound_values: vec![Some("public".into()), Some("plain".into())],
            ..Scenario::default()
        },
        "index.list",
        json!({"schema":"public","table":"plain","limit":10}),
    )
    .await
    .unwrap();
    assert_eq!(result["rows"], json!([]));
}

#[tokio::test]
async fn postgresql_catalogue_inputs_are_refused_before_credentials_or_connection() {
    let count = Arc::new(AtomicUsize::new(0));
    let sql = adapter(1, count.clone());
    for (operation, input) in [
        ("database.list", json!({})),
        ("database.list", json!({"limit":0})),
        ("database.list", json!({"limit":1,"schema":"public"})),
        ("table.list", json!({"limit":1})),
        ("table.list", json!({"schema":"","limit":1})),
        ("table.list", json!({"schema":"public","limit":1001})),
        (
            "table.list",
            json!({"schema":"public","table":"t","limit":1}),
        ),
        ("table.describe", json!({"schema":"public","limit":1})),
        ("table.describe", json!({"table":"t","limit":1})),
        (
            "table.describe",
            json!({"schema":"public","table":"","limit":1}),
        ),
        (
            "table.describe",
            json!({"schema":"public","table":"x".repeat(513),"limit":1}),
        ),
        ("index.list", json!({"limit":1})),
        ("index.list", json!({"table":"t","limit":1})),
    ] {
        assert_eq!(
            sql.invoke(operation, input.clone()).await.unwrap_err().code,
            ErrorCode::InvalidInput,
            "{operation} {input}"
        );
    }
    assert_eq!(count.load(Ordering::SeqCst), 0);
}

/// What the PostgreSQL statements hide, pinned on their text; the live cases
/// in `catalogue_adversary.rs` measure the behaviour on a server.
/// - A generated column (`attgenerated <> ''`) has no default: its expression
///   in `pg_attrdef` is not `column_default`, as in `information_schema.columns`.
/// - A foreign key is one declared key: an internal clone into a partition of
///   a referenced partitioned table (its parent constraint is on the same
///   referencing table) is not reported.
/// - A foreign key is reported only when the role holds `USAGE` on the
///   referenced schema, some privilege on the referenced table and a column
///   privilege on every referenced column.
/// - An index is left out when any column it depends on (key, INCLUDE,
///   expression or predicate, `pg_depend`) is one the role holds no column
///   privilege on, the rule that hides the column from `table.describe`. A
///   whole-row reference in an expression or the predicate, and the
///   whole-table dependency (`refobjsubid = 0`), count as every column.
/// A name ending in a space is an ordinary PostgreSQL name and is bound.
#[tokio::test]
async fn postgresql_catalogue_statements_hide_what_the_role_cannot_see() {
    let scenario = Scenario {
        columns: DESCRIBE_COLUMNS.to_vec(),
        rows: vec![row(&[
            Some("b"),
            Some("integer"),
            Some("YES"),
            None,
            Some("2"),
            None,
            None,
            None,
            None,
            None,
        ])],
        parameters: 2,
        bound_values: vec![Some("app".into()), Some("gen ".into())],
        ..Scenario::default()
    };
    let prepared = scenario.prepared.clone();
    catalogue(
        scenario,
        "table.describe",
        json!({"schema":"app","table":"gen ","limit":10}),
    )
    .await
    .unwrap();
    let describe = prepared.lock().unwrap()[0].clone();
    assert!(
        describe.contains("CASE WHEN a.attgenerated <> '' THEN NULL ELSE pg_catalog.pg_get_expr(d.adbin, d.adrelid) END AS column_default"),
        "{describe}"
    );
    assert!(
        describe.contains("pg_catalog.has_schema_privilege(rn.oid, 'USAGE')"),
        "{describe}"
    );
    assert!(
        describe.contains("pg_catalog.has_table_privilege(rc.oid, "),
        "{describe}"
    );
    assert!(
        describe.contains("NOT EXISTS (SELECT 1 FROM pg_catalog.pg_constraint pf WHERE pf.oid = f.conparentid AND pf.conrelid = f.conrelid)"),
        "{describe}"
    );
    assert!(
        describe.contains("NOT EXISTS (SELECT 1 FROM pg_catalog.unnest(f.confkey) AS rk(attnum) WHERE NOT (pg_catalog.pg_has_role(rc.relowner, 'USAGE') OR pg_catalog.has_column_privilege(rc.oid, rk.attnum, 'SELECT, INSERT, UPDATE, REFERENCES')))"),
        "{describe}"
    );
    let scenario = Scenario {
        columns: INDEX_COLUMNS.to_vec(),
        rows: vec![],
        parameters: 2,
        bound_values: vec![Some("app".into()), Some("partial ".into())],
        ..Scenario::default()
    };
    let prepared = scenario.prepared.clone();
    let result = catalogue(
        scenario,
        "index.list",
        json!({"schema":"app","table":"partial ","limit":10}),
    )
    .await
    .unwrap();
    assert_eq!(result["rows"], json!([]));
    let index = prepared.lock().unwrap()[0].clone();
    assert!(index.contains("FROM pg_catalog.pg_depend"), "{index}");
    assert!(
        index.contains("pg_catalog.has_column_privilege(c.oid, "),
        "{index}"
    );
    assert!(index.contains("wt.refobjsubid = 0"), "{index}");
    assert!(
        index.contains("pg_catalog.strpos(COALESCE(i.indexprs::pg_catalog.text, '') || COALESCE(i.indpred::pg_catalog.text, ''), ':varattno 0 ') > 0"),
        "{index}"
    );
}

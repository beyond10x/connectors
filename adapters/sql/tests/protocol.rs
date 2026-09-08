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
        }
    }
}
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
fn description(wrapped: bool) -> Vec<u8> {
    let fields = if wrapped {
        vec![("array", 1009_i32)]
    } else {
        vec![("answer", 23), ("nullable", 25)]
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
                        assert!(query.contains("ARRAY[result.c0::text,result.c1::text]"));
                        assert!(query.ends_with(&format!("FROM ({original}) AS result(c0,c1)")));
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
                    send(&mut stream, b'T', &description(prepares > 1)).await;
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
    assert_eq!(
        sql.invoke("schema.list", json!({"schema":"","limit":1}))
            .await
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
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
async fn authentication_database_errors_and_row_capacity_are_sanitized() {
    for (code, expected) in [
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

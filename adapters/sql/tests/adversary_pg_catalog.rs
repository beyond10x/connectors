//! Adversary cases for story `sql-wrapper-resolves-pg-catalog-first` (CR-002): the
//! PostgreSQL read wrapper and every statement prepared around the caller's statement,
//! run under `search_path = public, pg_catalog`.
//!
//! The first two cases answer on a loopback wire fixture and need no server. The live
//! cases need a disposable PostgreSQL 17 server on loopback, named by
//! `CONNECTORS_PG_ADVERSARY_PORT`, and are ignored otherwise; they were measured on
//! 2026-11-10 against `postgres:17-alpine` with database `fixture` and, as a superuser:
//!
//! ```sql
//! CREATE ROLE reader LOGIN PASSWORD 'reader-pw';
//! CREATE ROLE attacker LOGIN PASSWORD 'attacker-pw';
//! CREATE DATABASE shadow_type;
//! CREATE DATABASE shadow_typeinfo;
//! ALTER DATABASE shadow_type SET track_functions = 'pl';
//! ALTER DATABASE shadow_typeinfo SET track_functions = 'pl';
//! \c shadow_type
//! GRANT CREATE ON SCHEMA public TO attacker;
//! SET ROLE attacker;
//! CREATE FUNCTION public.spy(pg_catalog.text) RETURNS pg_catalog.bool LANGUAGE plpgsql AS $$ BEGIN RETURN true; END $$;
//! CREATE DOMAIN public.text AS pg_catalog.text CHECK (public.spy(VALUE));
//! RESET ROLE;
//! \c shadow_typeinfo
//! GRANT CREATE ON SCHEMA public TO attacker;
//! SET ROLE attacker;
//! CREATE TYPE public.mood AS ENUM ('ok');
//! CREATE FUNCTION public.spy(pg_catalog.oid, pg_catalog.oid) RETURNS pg_catalog.bool LANGUAGE plpgsql AS $$ BEGIN RETURN pg_catalog.oideq($1, $2); END $$;
//! CREATE OPERATOR public.= (LEFTARG = pg_catalog.oid, RIGHTARG = pg_catalog.oid, FUNCTION = public.spy);
//! RESET ROLE;
//! ```
use async_trait::async_trait;
use connectors_core::Result;
use connectors_sdk::{Adapter, Credential, Secret};
use connectors_sql::{Config, Engine, Sql};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

struct Password(&'static str);
#[async_trait]
impl Credential for Password {
    async fn resolve(&self) -> Result<Secret> {
        Ok(Secret(self.0.as_bytes().to_vec()))
    }
}

fn adapter(port: u16, database: &str, password: &'static str) -> Sql {
    let config = Config {
        engine: Engine::Postgresql,
        host: "127.0.0.1".into(),
        port,
        database: database.into(),
        user: "reader".into(),
        allow_plaintext: true,
        ca_file: None,
    };
    let effective = json!({"service":{"instance":"adversary-pg-catalog","listen":"127.0.0.1:0","service_credential":{"kind":"environment","name":"UNUSED"}},"password":{"kind":"environment","name":"UNUSED"},"adapter":config});
    Sql::new(
        "adversary-pg-catalog",
        config,
        effective,
        Arc::new(Password(password)),
    )
    .unwrap()
}

async fn send(stream: &mut TcpStream, tag: u8, body: &[u8]) {
    stream.write_u8(tag).await.unwrap();
    stream.write_i32((body.len() + 4) as i32).await.unwrap();
    stream.write_all(body).await.unwrap();
}
async fn receive(stream: &mut TcpStream) -> Option<(u8, Vec<u8>)> {
    let tag = stream.read_u8().await.ok()?;
    let len = stream.read_i32().await.unwrap() as usize;
    let mut body = vec![0; len - 4];
    stream.read_exact(&mut body).await.unwrap();
    Some((tag, body))
}
fn description(columns: &[(&str, i32)]) -> Vec<u8> {
    let mut body = (columns.len() as i16).to_be_bytes().to_vec();
    for (name, oid) in columns {
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

/// Runs one `query.read` against a wire fixture whose statement returns `int4, text`
/// and no row, and returns every statement text the adapter prepared, in order.
async fn prepared_statements(query: &str) -> Vec<String> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let sql = adapter(
        listener.local_addr().unwrap().port(),
        "fixture",
        "fixture-password",
    );
    let prepared = Arc::new(Mutex::new(Vec::<String>::new()));
    let seen = prepared.clone();
    let task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let len = stream.read_i32().await.unwrap();
        let mut startup = vec![0; len as usize - 4];
        stream.read_exact(&mut startup).await.unwrap();
        send(&mut stream, b'R', &3_i32.to_be_bytes()).await;
        receive(&mut stream).await.unwrap();
        send(&mut stream, b'R', &0_i32.to_be_bytes()).await;
        let mut key = 42_i32.to_be_bytes().to_vec();
        key.extend(1234_i32.to_be_bytes());
        send(&mut stream, b'K', &key).await;
        send(&mut stream, b'Z', b"I").await;
        let mut prepares = 0;
        while let Some((tag, body)) = receive(&mut stream).await {
            match tag {
                b'Q' => {
                    send(&mut stream, b'C', b"OK\0").await;
                    send(&mut stream, b'Z', b"T").await;
                }
                b'P' => {
                    let mut rest = &body[..];
                    let end = rest.iter().position(|b| *b == 0).unwrap();
                    rest = &rest[end + 1..];
                    let end = rest.iter().position(|b| *b == 0).unwrap();
                    seen.lock()
                        .unwrap()
                        .push(String::from_utf8(rest[..end].to_vec()).unwrap());
                    prepares += 1;
                    send(&mut stream, b'1', b"").await;
                }
                b'D' => {
                    send(&mut stream, b't', &0_i16.to_be_bytes()).await;
                    let columns: &[(&str, i32)] = if prepares > 1 {
                        &[("array", 1009)]
                    } else {
                        &[("answer", 23), ("label", 25)]
                    };
                    send(&mut stream, b'T', &description(columns)).await;
                }
                b'B' => send(&mut stream, b'2', b"").await,
                b'E' => send(&mut stream, b'C', b"SELECT 0\0").await,
                b'S' => send(&mut stream, b'Z', b"T").await,
                b'C' => send(&mut stream, b'3', b"").await,
                b'X' => break,
                _ => {}
            }
        }
    });
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        sql.invoke("query.read", json!({"query": query, "limit": 1})),
    )
    .await
    .expect("wire fixture hung");
    task.await.unwrap();
    assert_eq!(result.unwrap()["rows"], json!([]));
    let prepared = prepared.lock().unwrap().clone();
    assert_eq!(
        prepared.len(),
        2,
        "the statement and its wrapper: {prepared:?}"
    );
    prepared
}

/// PostgreSQL's lexer reads letters, digits, `_` and `$` as one identifier, so a
/// keyword written directly after an identifier is not a keyword: `::bigintOPERATOR(`
/// is a type named `bigintoperator` with a modifier, and `pg_catalog.+` inside it is a
/// syntax error (42601), which the adapter answers as `unsupported`.
#[tokio::test]
async fn postgresql_wrapper_writes_each_operator_as_its_own_token() {
    let prepared = prepared_statements("SELECT 42, 'x'").await;
    let wrapper = &prepared[1];
    for (at, _) in wrapper.match_indices("OPERATOR(") {
        let before = wrapper[..at].chars().next_back().unwrap();
        assert!(
            !(before.is_ascii_alphanumeric() || before == '_' || before == '$'),
            "`OPERATOR(` at byte {at} continues the identifier before it, so PostgreSQL \
             cannot parse the wrapper: {wrapper}"
        );
    }
}

/// A type named without its schema resolves through `search_path`, and `public` comes
/// first: a principal with CREATE on `public` makes `public.text` a domain whose CHECK
/// calls its own function, and every `::text` of the wrapper runs it as the reader.
#[tokio::test]
async fn postgresql_wrapper_names_no_type_through_the_search_path() {
    let prepared = prepared_statements("SELECT 42, 'x'").await;
    let wrapper = &prepared[1];
    assert!(
        !wrapper.contains("::text"),
        "the wrapper casts to `text` as `search_path` resolves it, `public` first: {wrapper}"
    );
}

fn live_port() -> u16 {
    std::env::var("CONNECTORS_PG_ADVERSARY_PORT")
        .expect("CONNECTORS_PG_ADVERSARY_PORT names the live server's port")
        .parse()
        .unwrap()
}

async fn live(database: &str, operation: &str, input: Value) -> Result<Value> {
    tokio::time::timeout(
        Duration::from_secs(20),
        adapter(live_port(), database, "reader-pw").invoke(operation, input),
    )
    .await
    .expect("live read hung")
}

/// The calls the server counted of `public.spy`, read by the principal that made it.
async fn spy_calls(database: &str) -> i64 {
    let config = format!(
        "host=127.0.0.1 port={} dbname={database} user=attacker password=attacker-pw",
        live_port()
    );
    let (client, connection) = tokio_postgres::connect(&config, tokio_postgres::NoTls)
        .await
        .unwrap();
    let driver = tokio::spawn(connection);
    let row = client
        .query_one(
            "SELECT COALESCE(pg_catalog.sum(calls), 0)::bigint FROM pg_catalog.pg_stat_user_functions WHERE schemaname = 'public' AND funcname = 'spy'",
            &[],
        )
        .await
        .unwrap();
    drop(client);
    let _ = driver.await;
    row.get(0)
}

/// Reads `spy_calls` until it moves past `before` or three seconds pass; the reader's
/// backend reports its function counts when it exits.
async fn spy_calls_after(database: &str, before: i64) -> i64 {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        let calls = spy_calls(database).await;
        if calls != before || tokio::time::Instant::now() > deadline {
            return calls;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

#[tokio::test]
#[ignore = "needs the PostgreSQL server named by CONNECTORS_PG_ADVERSARY_PORT"]
async fn postgresql_live_read_of_two_columns_returns_its_row() {
    let result = live(
        "fixture",
        "query.read",
        json!({"query": "SELECT 1 AS a, 'two' AS b", "limit": 1}),
    )
    .await;
    assert_eq!(
        result.map(|r| r["rows"].clone()).map_err(|e| e.code),
        Ok(json!([["1", "two"]])),
        "a read of two columns is refused by the server"
    );
}

#[tokio::test]
#[ignore = "needs the PostgreSQL server named by CONNECTORS_PG_ADVERSARY_PORT"]
async fn postgresql_live_catalogue_reads_of_several_columns_answer() {
    for (operation, input) in [
        ("schema.list", json!({"schema": "pg_catalog", "limit": 1})),
        ("table.list", json!({"schema": "pg_catalog", "limit": 1})),
    ] {
        let result = live("fixture", operation, input).await;
        assert!(
            result
                .as_ref()
                .is_ok_and(|r| r["rows"].as_array().is_some_and(|rows| rows.len() == 1)),
            "{operation} does not answer: {:?}",
            result.map_err(|e| e.code)
        );
    }
}

#[tokio::test]
#[ignore = "needs the PostgreSQL server named by CONNECTORS_PG_ADVERSARY_PORT"]
async fn postgresql_live_read_never_runs_a_public_text_domain_check() {
    let before = spy_calls("shadow_type").await;
    let result = live(
        "shadow_type",
        "query.read",
        json!({"query": "SELECT 'x' AS a", "limit": 1}),
    )
    .await;
    let after = spy_calls_after("shadow_type", before).await;
    assert_eq!(
        after - before,
        0,
        "the reader ran the CHECK of `public.text` another principal made; the read answered {:?}",
        result.map_err(|e| e.code)
    );
}

#[tokio::test]
#[ignore = "needs the PostgreSQL server named by CONNECTORS_PG_ADVERSARY_PORT"]
async fn postgresql_live_type_lookup_never_runs_a_public_operator() {
    let before = spy_calls("shadow_typeinfo").await;
    let result = live(
        "shadow_typeinfo",
        "query.read",
        json!({"query": "SELECT 'ok'::public.mood AS m", "limit": 1}),
    )
    .await;
    let after = spy_calls_after("shadow_typeinfo", before).await;
    assert_eq!(
        after - before,
        0,
        "preparing a statement of a non-built-in type ran `public.=` another principal made; the read answered {:?}",
        result.map_err(|e| e.code)
    );
}

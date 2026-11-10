//! Conformance cases for CR-002: nothing the connector itself emits on a PostgreSQL read
//! (the wrapper around the caller's statement, its size guard, the keyset, casts,
//! collations, the catalogue statements and the driver's own type lookup) resolves a name
//! through `public`, which the session puts before `pg_catalog` and which another database
//! principal can write (every role could before PostgreSQL 15, and upgraded clusters keep it).
//!
//! The first cases read the statement text the adapter prepares on a loopback wire fixture.
//! The `live_` cases need a disposable PostgreSQL 17 server on loopback whose port
//! `CONNECTORS_PG_SECURITY_PORT` names and are ignored otherwise. Created by a superuser:
//!
//! ```sql
//! CREATE ROLE reader LOGIN PASSWORD 'reader-pw';
//! CREATE ROLE mallory LOGIN PASSWORD 'mallory-pw';
//! CREATE DATABASE clean; -- and shadow_cast, shadow_size, shadow_collation,
//!                        -- shadow_typeinfo, shadow_catalogue, shadow_wrapper_ops
//! -- in each database:
//! GRANT CREATE ON SCHEMA public TO mallory;
//! CREATE SCHEMA app;
//! GRANT USAGE ON SCHEMA app TO reader;
//! CREATE TYPE app.mood AS ENUM ('calm', 'happy');
//! CREATE TABLE app.labels (id int8 PRIMARY KEY, label pg_catalog.text NOT NULL);
//! INSERT INTO app.labels VALUES (1, 'B'), (2, 'a'), (3, 'b');
//! GRANT SELECT ON app.labels TO reader;
//! ```
//!
//! and then, as `mallory` (no superuser, only CREATE on `public`):
//!
//! ```sql
//! -- shadow_cast
//! CREATE FUNCTION public.shadow_ran(pg_catalog.text) RETURNS boolean LANGUAGE plpgsql
//!   AS $$BEGIN RAISE EXCEPTION 'shadow ran as %', current_user; END$$;
//! CREATE DOMAIN public.text AS pg_catalog.text CHECK (public.shadow_ran(VALUE));
//! -- shadow_size
//! CREATE TYPE public.text AS (v pg_catalog.text);
//! CREATE FUNCTION public.widen(pg_catalog.int4) RETURNS public.text LANGUAGE sql
//!   AS $$ SELECT ROW(pg_catalog.repeat('x'::pg_catalog.text, 20000000))::public.text $$;
//! CREATE CAST (pg_catalog.int4 AS public.text) WITH FUNCTION public.widen(pg_catalog.int4);
//! CREATE FUNCTION public.measure(public.text) RETURNS pg_catalog.text LANGUAGE sql
//!   AS $$ SELECT ''::pg_catalog.text $$;
//! CREATE CAST (public.text AS pg_catalog.text) WITH FUNCTION public.measure(public.text) AS IMPLICIT;
//! -- shadow_collation
//! CREATE COLLATION public."C" (provider = icu, locale = 'und-u-ks-level2', deterministic = false);
//! -- shadow_typeinfo
//! CREATE FUNCTION public.shadow_oideq(pg_catalog.oid, pg_catalog.oid) RETURNS boolean
//!   LANGUAGE plpgsql AS $$BEGIN RAISE EXCEPTION 'shadow ran as %', current_user; END$$;
//! CREATE OPERATOR public.= (LEFTARG = pg_catalog.oid, RIGHTARG = pg_catalog.oid,
//!   FUNCTION = public.shadow_oideq);
//! -- shadow_catalogue
//! CREATE FUNCTION public.shadow_nameeq(pg_catalog.name, pg_catalog.name) RETURNS boolean
//!   LANGUAGE plpgsql AS $$BEGIN RAISE EXCEPTION 'shadow ran as %', current_user; END$$;
//! CREATE OPERATOR public.= (LEFTARG = pg_catalog.name, RIGHTARG = pg_catalog.name,
//!   FUNCTION = public.shadow_nameeq);
//! -- shadow_wrapper_ops
//! CREATE FUNCTION public.octet_length(pg_catalog.text) RETURNS pg_catalog.int4
//!   LANGUAGE plpgsql AS $$BEGIN RAISE EXCEPTION 'shadow ran as %', current_user; END$$;
//! CREATE FUNCTION public.shadow_gt(pg_catalog.int8, pg_catalog.int4) RETURNS boolean
//!   LANGUAGE plpgsql AS $$BEGIN RAISE EXCEPTION 'shadow ran as %', current_user; END$$;
//! CREATE OPERATOR public.> (LEFTARG = pg_catalog.int8, RIGHTARG = pg_catalog.int4,
//!   FUNCTION = public.shadow_gt);
//! ```
//!
//! Every live case first reads the database `clean` (no shadow) and must get the answer it
//! then asserts on the shadowed database, so a red case is the shadow and not the setup.
use async_trait::async_trait;
use connectors_core::Result;
use connectors_sdk::{Adapter, Credential, Secret};
use connectors_sql::{Config, Engine, Sql};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
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
    let effective = json!({"service":{"instance":"sql-security","listen":"127.0.0.1:0","service_credential":{"kind":"environment","name":"UNUSED"}},"password":{"kind":"environment","name":"UNUSED"},"adapter":config});
    Sql::new(
        "sql-security",
        config,
        effective,
        Arc::new(Password(password)),
    )
    .unwrap()
}

fn label_profile(statement: &str) -> connectors_sql::ReadProfile {
    serde_json::from_value(json!({
        "id": "rows-by-label",
        "statement": statement,
        "parameters": ["exclude"],
        "columns": [
            {"name": "row_key", "type": "integer"},
            {"name": "label", "type": "text"}
        ],
        "call_identifier": "row_key",
        "ordering": "label"
    }))
    .unwrap()
}

// ---------------------------------------------------------------------------------------
// Wire fixture: records every statement the adapter prepares, answers no rows.

const TEXT: i32 = 25;
const INT8: i32 = 20;
const TEXT_ARRAY: i32 = 1009;

async fn send(stream: &mut TcpStream, tag: u8, body: &[u8]) {
    stream.write_u8(tag).await.unwrap();
    stream.write_i32((body.len() + 4) as i32).await.unwrap();
    stream.write_all(body).await.unwrap();
}
async fn receive(stream: &mut TcpStream) -> Option<(u8, Vec<u8>)> {
    let tag = stream.read_u8().await.ok()?;
    let len = stream.read_i32().await.ok()? as usize;
    let mut body = vec![0; len - 4];
    stream.read_exact(&mut body).await.ok()?;
    Some((tag, body))
}
fn description(fields: &[(&str, i32)]) -> Vec<u8> {
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
/// The highest `$n` a statement names: the parameter count the server would describe.
fn parameter_count(statement: &str) -> usize {
    let bytes = statement.as_bytes();
    let mut highest = 0;
    for (i, b) in bytes.iter().enumerate() {
        if *b == b'$' {
            let digits = bytes[i + 1..]
                .iter()
                .take_while(|d| d.is_ascii_digit())
                .count();
            if let Ok(n) = statement[i + 1..i + 1 + digits].parse::<usize>() {
                highest = highest.max(n);
            }
        }
    }
    highest
}

/// Runs one operation against a fixture whose first prepared statement returns `columns`;
/// answers every statement the adapter prepared on that connection, in order.
async fn prepared_by(
    profiles: Vec<connectors_sql::ReadProfile>,
    columns: Vec<(&'static str, i32)>,
    operation: &str,
    input: Value,
) -> Vec<String> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let sql = adapter(port, "fixture", "fixture-password")
        .with_read_profiles(profiles)
        .unwrap();
    let prepared = Arc::new(Mutex::new(Vec::<String>::new()));
    let record = prepared.clone();
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
        let mut last = String::new();
        while let Some((tag, body)) = receive(&mut stream).await {
            match tag {
                b'Q' => {
                    send(&mut stream, b'C', b"OK\0").await;
                    send(&mut stream, b'Z', b"T").await;
                }
                b'P' => {
                    let body = &body[body.iter().position(|b| *b == 0).unwrap() + 1..];
                    let end = body.iter().position(|b| *b == 0).unwrap();
                    last = String::from_utf8(body[..end].to_vec()).unwrap();
                    record.lock().unwrap().push(last.clone());
                    send(&mut stream, b'1', b"").await;
                }
                b'D' => {
                    let count = parameter_count(&last);
                    let mut params = (count as i16).to_be_bytes().to_vec();
                    for _ in 0..count {
                        params.extend(TEXT.to_be_bytes());
                    }
                    send(&mut stream, b't', &params).await;
                    let wrapped = record.lock().unwrap().len() > 1;
                    let fields = if wrapped {
                        vec![("array", TEXT_ARRAY)]
                    } else {
                        columns.clone()
                    };
                    send(&mut stream, b'T', &description(&fields)).await;
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
    let _ = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        sql.invoke(operation, input),
    )
    .await
    .expect("wire fixture hung");
    tokio::time::timeout(std::time::Duration::from_secs(5), task)
        .await
        .expect("no session reached the fixture")
        .unwrap();
    prepared.lock().unwrap().clone()
}

/// Every `::` cast in `statement` names a `pg_catalog` type or a type the grammar fixes
/// (`bigint`, `integer`, `smallint`), so no type in a writable schema is resolved.
fn unqualified_casts(statement: &str) -> Vec<String> {
    statement
        .match_indices("::")
        .map(|(i, _)| &statement[i + 2..])
        .filter(|rest| {
            !["pg_catalog.", "bigint", "integer", "smallint"]
                .iter()
                .any(|fixed| rest.starts_with(fixed))
        })
        .map(|rest| rest.chars().take(16).collect())
        .collect()
}
/// Every `COLLATE` in `statement` names a `pg_catalog` collation.
fn unqualified_collations(statement: &str) -> Vec<String> {
    statement
        .match_indices("COLLATE ")
        .map(|(i, _)| &statement[i + 8..])
        .filter(|rest| !rest.starts_with("pg_catalog."))
        .map(|rest| rest.chars().take(16).collect())
        .collect()
}
/// CR-002, the wrapper's casts: the size guard measures `pg_catalog.octet_length(result.cN::text)`
/// and the page sends `ARRAY[result.cN::text]`, with `NULL::text[]` for an oversized row. Under
/// `search_path = public, pg_catalog` the type name `text` is looked up in `public` first, so a
/// `public.text` domain or type (and the casts its owner may declare to and from it) runs as the
/// connector's role inside every read and decides what the guard measures.
#[tokio::test]
async fn postgresql_wrapper_casts_name_pg_catalog_types() {
    let prepared = prepared_by(
        vec![],
        vec![("answer", INT8), ("label", TEXT)],
        "query.read",
        json!({"query":"SELECT 1, 'x'","limit":1}),
    )
    .await;
    assert_eq!(
        prepared.len(),
        2,
        "the statement and its wrapper: {prepared:?}"
    );
    let wrapper = &prepared[1];
    assert_eq!(
        unqualified_casts(wrapper),
        Vec::<String>::new(),
        "the wrapper casts to a type it resolves through `public` first: {wrapper}"
    );
}

/// The size guard of a statement with more than one column sums its cells with
/// `OPERATOR(pg_catalog.+)`. Written straight after `::bigint`, `OPERATOR` lexes as part of
/// the type name (`bigintOPERATOR`), PostgreSQL answers 42601 to the wrapper, and the adapter
/// refuses every read of two or more columns as "not a query" (catalogue reads included).
#[tokio::test]
async fn postgresql_wrapper_size_sum_is_separate_tokens() {
    let prepared = prepared_by(
        vec![],
        vec![("answer", INT8), ("label", TEXT)],
        "query.read",
        json!({"query":"SELECT 1, 'x'","limit":1}),
    )
    .await;
    assert_eq!(
        prepared.len(),
        2,
        "the statement and its wrapper: {prepared:?}"
    );
    let wrapper = &prepared[1];
    let glued = wrapper
        .match_indices("OPERATOR(")
        .filter(|(i, _)| {
            wrapper[..*i]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        })
        .count();
    assert_eq!(
        glued, 0,
        "an OPERATOR( continues the identifier before it: {wrapper}"
    );
}

/// CR-002, the keyset: a text ordering is `x::text COLLATE "C"` (semantics.md, "the returned
/// text, by its bytes"). A collation name is looked up through the search path too, so a
/// `public."C"` created by another principal decides the page order and which rows a
/// continuation skips.
#[tokio::test]
async fn postgresql_text_keyset_names_pg_catalog_collation_and_type() {
    let prepared = prepared_by(
        vec![label_profile(
            "SELECT id, label FROM app.labels WHERE label IS DISTINCT FROM $1",
        )],
        vec![("id", INT8), ("label", TEXT)],
        "profile.read",
        json!({"profile":"rows-by-label","parameters":{"exclude":"none"},"limit":5}),
    )
    .await;
    assert_eq!(
        prepared.len(),
        2,
        "the statement and its wrapper: {prepared:?}"
    );
    let wrapper = &prepared[1];
    assert!(wrapper.contains("ORDER BY"), "a keyset page: {wrapper}");
    assert_eq!(
        unqualified_collations(wrapper),
        Vec::<String>::new(),
        "the keyset collates by a name it resolves through `public` first: {wrapper}"
    );
    assert_eq!(
        unqualified_casts(wrapper),
        Vec::<String>::new(),
        "the keyset casts to a type it resolves through `public` first: {wrapper}"
    );
}

/// CR-002, the catalogue: `schema.list`, `database.list`, `table.list`, `table.describe` and
/// `index.list` run the connector's own fixed statements in the same session. Their functions
/// are `pg_catalog.`-qualified, and so are their casts and collations. Their comparison
/// operators are written bare and resolve through the session's path, which puts `pg_catalog`
/// first; `live_catalogue_does_not_call_a_public_operator` measures that.
#[tokio::test]
async fn postgresql_catalogue_statements_name_pg_catalog_types_and_collations() {
    let mut offending = Vec::new();
    for (operation, input) in [
        ("schema.list", json!({"schema":"app","limit":5})),
        ("database.list", json!({"limit":5})),
        ("table.list", json!({"schema":"app","limit":5})),
        (
            "table.describe",
            json!({"schema":"app","table":"labels","limit":5}),
        ),
        (
            "index.list",
            json!({"schema":"app","table":"labels","limit":5}),
        ),
    ] {
        let prepared = prepared_by(vec![], vec![("name", TEXT)], operation, input).await;
        let statement = prepared
            .first()
            .unwrap_or_else(|| panic!("{operation}: nothing prepared"));
        let names = [
            unqualified_casts(statement),
            unqualified_collations(statement),
        ]
        .concat();
        if !names.is_empty() {
            offending.push(format!("{operation}: {names:?}"));
        }
    }
    assert_eq!(
        offending,
        Vec::<String>::new(),
        "catalogue statements name a type or collation resolved through `public`"
    );
}

// ---------------------------------------------------------------------------------------
// Live cases: a real PostgreSQL 17 with the shadows above.

fn live(database: &str) -> Sql {
    let port: u16 = std::env::var("CONNECTORS_PG_SECURITY_PORT")
        .expect("CONNECTORS_PG_SECURITY_PORT names the live server's port")
        .parse()
        .unwrap();
    adapter(port, database, "reader-pw")
}
async fn read(database: &str, operation: &str, input: Value) -> Result<Value> {
    tokio::time::timeout(
        std::time::Duration::from_secs(20),
        live(database).invoke(operation, input),
    )
    .await
    .expect("server hung")
}
/// Reads `clean` (the control, which must answer `expected`) and then `shadowed`.
async fn same_answer(
    shadowed: &str,
    operation: &str,
    input: Value,
    answer: impl Fn(&Value) -> Value,
    expected: Value,
) {
    let control = read("clean", operation, input.clone())
        .await
        .expect("control read on `clean`");
    assert_eq!(answer(&control), expected, "control read on `clean`");
    match read(shadowed, operation, input).await {
        Ok(result) => assert_eq!(answer(&result), expected, "read on `{shadowed}`"),
        Err(error) => {
            panic!("read on `{shadowed}` failed where `clean` answered {expected}: {error:?}")
        }
    }
}

/// No shadow at all: a two-column read and a catalogue read answer on a plain server.
#[tokio::test]
#[ignore = "needs the PostgreSQL server named by CONNECTORS_PG_SECURITY_PORT"]
async fn live_two_column_and_catalogue_reads_answer_without_any_shadow() {
    let two = read(
        "clean",
        "query.read",
        json!({"query":"SELECT 1 AS a, 'x' AS b","limit":1}),
    )
    .await;
    let tables = read("clean", "table.list", json!({"schema":"app","limit":5})).await;
    assert_eq!(
        (
            two.map(|r| r["rows"].clone()).map_err(|e| e.message),
            tables
                .map(|r| r["rows"][0][0].clone())
                .map_err(|e| e.message)
        ),
        (Ok(json!([["1", "x"]])), Ok(json!("labels")))
    );
}

/// The control for the implementor's change: `public.octet_length(text)` and a
/// `public.>(int8, int4)` that raise are not called by the wrapper.
#[tokio::test]
#[ignore = "needs the PostgreSQL server named by CONNECTORS_PG_SECURITY_PORT"]
async fn live_wrapper_does_not_call_a_public_octet_length_or_operator() {
    same_answer(
        "shadow_wrapper_ops",
        "query.read",
        json!({"query":"SELECT 1 AS answer","limit":1}),
        |r| r["rows"].clone(),
        json!([["1"]]),
    )
    .await;
}

/// A `public.text` domain whose check raises: the wrapper's `::text` runs it as `reader`.
#[tokio::test]
#[ignore = "needs the PostgreSQL server named by CONNECTORS_PG_SECURITY_PORT"]
async fn live_wrapper_cast_does_not_run_a_public_text_domain() {
    same_answer(
        "shadow_cast",
        "query.read",
        json!({"query":"SELECT 1 AS answer","limit":1}),
        |r| r["rows"].clone(),
        json!([["1"]]),
    )
    .await;
}

/// A `public.text` composite with an implicit cast to `pg_catalog.text` that measures every
/// value as empty: the size guard sums 0 while the server sends a 20 000 004-byte cell for
/// `SELECT 1`. The read must answer `1`, or refuse the row by its size, never receive it.
#[tokio::test]
#[ignore = "needs the PostgreSQL server named by CONNECTORS_PG_SECURITY_PORT"]
async fn live_size_guard_measures_what_the_page_sends() {
    same_answer(
        "shadow_size",
        "query.read",
        json!({"query":"SELECT 1 AS answer","limit":1}),
        |r| r["rows"].clone(),
        json!([["1"]]),
    )
    .await;
}

/// A nondeterministic `public."C"`: the text keyset orders case-insensitively and a
/// continuation after `B` skips `b` (and `a`). The contract orders by the returned bytes.
#[tokio::test]
#[ignore = "needs the PostgreSQL server named by CONNECTORS_PG_SECURITY_PORT"]
async fn live_text_keyset_orders_by_bytes_under_a_public_c_collation() {
    // One column, so the page's size guard sums no `+` (see
    // `postgresql_wrapper_size_sum_is_separate_tokens`): the shadow alone decides this case.
    let profile: connectors_sql::ReadProfile = serde_json::from_value(json!({
        "id": "rows-by-label",
        "statement": "SELECT label FROM app.labels WHERE label IS DISTINCT FROM $1",
        "parameters": ["exclude"],
        "columns": [{"name": "label", "type": "text"}],
        "call_identifier": "label",
        "ordering": "label"
    }))
    .unwrap();
    let mut pages = Vec::new();
    for database in ["clean", "shadow_collation"] {
        let port: u16 = std::env::var("CONNECTORS_PG_SECURITY_PORT")
            .unwrap()
            .parse()
            .unwrap();
        let sql = adapter(port, database, "reader-pw")
            .with_read_profiles(vec![profile.clone()])
            .unwrap();
        let mut labels = Vec::new();
        let mut cursor = Value::Null;
        for _ in 0..6 {
            let page = sql
                .invoke(
                    "profile.read",
                    json!({"profile":"rows-by-label","parameters":{"exclude":"none"},"limit":1,"cursor":cursor}),
                )
                .await
                .unwrap_or_else(|e| panic!("{database}: {e:?}"));
            for item in page["items"].as_array().unwrap() {
                labels.push(item["label"].as_str().unwrap().to_owned());
            }
            cursor = page["next_cursor"].clone();
            if cursor.is_null() {
                break;
            }
        }
        pages.push((database, labels));
    }
    assert_eq!(pages[0].1, ["B", "a", "b"], "control read on `clean`");
    assert_eq!(pages[1].1, ["B", "a", "b"], "read on `shadow_collation`");
}

/// A `public.=(oid, oid)` that raises: a caller's statement returning an enum makes the
/// driver look the type up with its own `WHERE t.oid = $1` in the same session.
#[tokio::test]
#[ignore = "needs the PostgreSQL server named by CONNECTORS_PG_SECURITY_PORT"]
async fn live_driver_type_lookup_does_not_call_a_public_operator() {
    same_answer(
        "shadow_typeinfo",
        "query.read",
        json!({"query":"SELECT 'calm'::app.mood AS m","limit":1}),
        |r| r["rows"].clone(),
        json!([["calm"]]),
    )
    .await;
}

/// A `public.=(name, name)` that raises: `table.list` compares `n.nspname = $1`.
#[tokio::test]
#[ignore = "needs the PostgreSQL server named by CONNECTORS_PG_SECURITY_PORT"]
async fn live_catalogue_does_not_call_a_public_operator() {
    same_answer(
        "shadow_catalogue",
        "table.list",
        json!({"schema":"app","limit":5}),
        |r| {
            r["rows"]
                .as_array()
                .unwrap()
                .iter()
                .map(|row| row[0].clone())
                .collect()
        },
        json!(["labels"]),
    )
    .await;
}

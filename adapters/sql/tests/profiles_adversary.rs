//! Adversarial cases for `profile.read` on the MySQL wire fixture: cursors the adapter never
//! issued, cursors carried across a profile change, a shared ordering value at a page
//! boundary, and a cursor longer than the published schemas admit. The rows are the
//! synthetic call table of `tests/mysql/calls.rs`; every number is fictional.
#![cfg(unix)]
#[path = "mysql/calls.rs"]
mod calls;
#[path = "mysql/fixture.rs"]
mod fixture;

use async_trait::async_trait;
use base64::Engine as _;
use connectors_core::ErrorCode;
use connectors_sdk::{Adapter, Credential, Secret};
use connectors_sql::{Config, Engine, ReadProfile, Sql};
use serde_json::{Value, json};
use std::sync::Arc;

struct Password;
#[async_trait]
impl Credential for Password {
    async fn resolve(&self) -> connectors_core::Result<Secret> {
        Ok(Secret(fixture::PASSWORD.as_bytes().to_vec()))
    }
}

fn sql(port: u16, documents: &[Value]) -> Sql {
    let config = Config {
        engine: Engine::Mysql,
        host: "127.0.0.1".into(),
        port,
        database: "fixture".into(),
        user: "reader".into(),
        allow_plaintext: true,
        ca_file: None,
    };
    let effective = json!({"service":{"instance":"fixture-sql","listen":"127.0.0.1:0","service_credential":{"kind":"environment","name":"UNUSED"}},"password":{"kind":"environment","name":"UNUSED"},"adapter":config});
    let profiles: Vec<ReadProfile> = documents
        .iter()
        .map(|d| serde_json::from_value(d.clone()).unwrap())
        .collect();
    Sql::new("fixture-sql", config, effective, Arc::new(Password))
        .unwrap()
        .with_read_profiles(profiles)
        .unwrap()
}
async fn read(sql: &Sql, input: Value) -> connectors_core::Result<Value> {
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        sql.invoke("profile.read", input),
    )
    .await
    .expect("profile.read hung")
}

fn started(row: &calls::CallRow) -> String {
    let s = row.started;
    format!("2026-01-{:02}T{:02}:{:02}:{:02}", s[0], s[1], s[2], s[3])
}
fn in_window(row: &calls::CallRow, values: &[Option<String>]) -> bool {
    let text = |i: usize| values.get(i).cloned().flatten().unwrap_or_default();
    let at = started(row);
    (row.caller == text(0) || row.callee == text(0))
        && at.as_str() >= text(1).as_str()
        && at.as_str() < text(2).as_str()
}
fn cells(row: &calls::CallRow) -> Vec<fixture::Cell> {
    use fixture::*;
    let at = |t: [u8; 4]| datetime(2026, 1, t[0], [t[1], t[2], t[3]], 0);
    vec![
        int8(row.row_key),
        text(row.call_id),
        text(row.direction),
        text(row.caller),
        text(row.callee),
        at(row.started),
        at(row.ended),
        text(row.leg_type),
    ]
}
fn columns() -> Vec<fixture::Col> {
    use fixture::*;
    vec![
        col("id", LONGLONG, 0, BINARY_CHARSET, 0),
        col("call_ref", VAR_STRING, 0, UTF8MB4, 0),
        col("direction", VAR_STRING, 0, UTF8MB4, 0),
        col("from_number", VAR_STRING, 0, UTF8MB4, 0),
        col("to_number", VAR_STRING, 0, UTF8MB4, 0),
        col("started", DATETIME, 0, BINARY_CHARSET, 0),
        col("ended", DATETIME, 0, BINARY_CHARSET, 0),
        col("leg", VAR_STRING, 0, UTF8MB4, 0),
    ]
}

/// The call table behind a server that compares the cursor as MySQL does for the adapter's
/// `CAST(? AS DECIMAL(65,0))`: text that is not a number casts to 0 (with a warning), so
/// every row follows it. It never panics on what a caller sends.
fn integer_keyset_script() -> fixture::Script {
    fixture::Script {
        columns: columns(),
        rows: calls::ROWS.iter().map(cells).collect(),
        params: 3,
        select: Some(Arc::new(|values: &[Option<String>], index: usize| {
            let row = &calls::ROWS[index];
            let after = values
                .get(3)
                .cloned()
                .flatten()
                .map(|cursor| cursor.parse::<i64>().unwrap_or(0));
            in_window(row, values) && after.is_none_or(|a| row.row_key > a)
        })),
        ..fixture::Script::default()
    }
}

fn decode_cursor(token: &str) -> Value {
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(token)
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}
fn encode_cursor(cursor: &Value) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(serde_json::to_vec(cursor).unwrap())
}

/// semantics.md "Read profiles" and `connectors_sql.profiles.ProfileReadOutcome`: a cursor
/// "not issued by this adapter, is `stale_cursor`", refused "before any session is opened".
/// The adapter only ever issues an ordering value its declared type admits (`page` refuses
/// any other), so a cursor whose value is not an integer for an `integer` ordering column
/// was never issued by it. Today it is bound, and on MySQL `CAST('x' AS DECIMAL(65,0))` is
/// 0: the read silently starts over and repeats rows the caller already has.
#[tokio::test]
async fn a_cursor_value_the_adapter_never_issued_is_stale_before_any_session() {
    let server = fixture::start(integer_keyset_script()).await;
    let sql = sql(server.port, &[calls::profile()]);
    let issued = read(&sql, calls::input(None, 2)).await.unwrap()["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut cursor = decode_cursor(&issued);
    assert_eq!(cursor["after"], "12", "{cursor}");
    cursor["after"] = json!("not-a-row-key");
    let forged = encode_cursor(&cursor);
    let sessions = server.log.lock().unwrap().sessions;
    let result = read(&sql, calls::input(Some(&forged), 2)).await;
    let opened = server.log.lock().unwrap().sessions - sessions;
    match result {
        Err(error) => assert_eq!(error.code, ErrorCode::StaleCursor, "{error:?}"),
        Ok(page) => panic!(
            "a cursor this adapter never issued was served ({opened} session(s) opened), row keys {:?}",
            page["items"]
                .as_array()
                .unwrap()
                .iter()
                .map(|i| i["row_key"].clone())
                .collect::<Vec<_>>()
        ),
    }
    assert_eq!(opened, 0, "a stale cursor opened a session");
}

/// semantics.md "Read profiles": "a changed profile is a changed configuration revision",
/// and a cursor "issued for another profile ... is `stale_cursor`". The cursor carries the
/// profile's id and a digest of the parameter values, nothing of the profile itself, so a
/// cursor issued while the profile ordered by `row_key` (an integer) is accepted after the
/// operator re-orders the same id by `started_at` (a timestamp). On MySQL the old value is
/// then `CAST('12' AS DATETIME(6))`, which is NULL: the page is empty and says `complete`
/// while rows remain.
#[tokio::test]
async fn a_cursor_issued_before_the_profile_changed_is_stale() {
    let server = fixture::start(integer_keyset_script()).await;
    let before = sql(server.port, &[calls::profile()]);
    let issued = read(&before, calls::input(None, 2)).await.unwrap()["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();

    let mut changed = calls::profile();
    changed["ordering"] = json!("started_at");
    let after = sql(server.port, &[changed]);
    assert_ne!(
        before.descriptor().revision,
        after.descriptor().revision,
        "the profile change is a revision change"
    );
    let sessions = server.log.lock().unwrap().sessions;
    let result = read(&after, calls::input(Some(&issued), 2)).await;
    let opened = server.log.lock().unwrap().sessions - sessions;
    match result {
        Err(error) => assert_eq!(error.code, ErrorCode::StaleCursor, "{error:?}"),
        Ok(_) => panic!(
            "a cursor issued for the profile before its change was served ({opened} session(s)): \
             bound {:?}",
            server.log.lock().unwrap().executions.last().cloned()
        ),
    }
    assert_eq!(opened, 0, "a stale cursor opened a session");
}

/// The story's acceptance: "every leg of a transferred call, with its type ... across a page
/// boundary of the SQL read". When two legs share the ordering value and a page ends between
/// them, the next page continues after that value and the second leg is never returned,
/// while the read reports `complete`. The adapter received the row past the limit (it asks
/// for `limit + 1`) and could see the shared value at that boundary; it discards the row
/// instead. Refusing the read (`unsupported`) is an acceptable answer; a complete read
/// missing a leg is not.
#[tokio::test]
async fn a_leg_sharing_the_ordering_value_across_a_page_boundary_is_not_silently_dropped() {
    // The transferred call's two legs start in the same second; the profile orders by start.
    let mut rows = calls::ROWS;
    rows[1].started = rows[0].started;
    let shared = rows;
    let script = fixture::Script {
        columns: columns(),
        rows: rows.iter().map(cells).collect(),
        params: 3,
        select: Some(Arc::new(move |values: &[Option<String>], index: usize| {
            let row = &shared[index];
            let after = values.get(3).cloned().flatten();
            in_window(row, values) && after.is_none_or(|a| started(row) > a)
        })),
        ..fixture::Script::default()
    };
    let server = fixture::start(script).await;
    let mut profile = calls::profile();
    profile["ordering"] = json!("started_at");
    let sql = sql(server.port, &[profile]);

    let mut cursor: Option<String> = None;
    let mut legs = Vec::new();
    for _ in 0..20 {
        let page = match read(&sql, calls::input(cursor.as_deref(), 1)).await {
            Ok(page) => page,
            Err(error) => {
                // Refusing a page whose boundary splits a shared ordering value is acceptable.
                assert_eq!(error.code, ErrorCode::Unsupported, "{error:?}");
                return;
            }
        };
        for item in page["items"].as_array().unwrap() {
            if item["call_id"] == calls::TRANSFERRED_CALL {
                legs.push(item["leg_type"].as_str().unwrap().to_owned());
            }
        }
        if page["complete"] == true {
            break;
        }
        cursor = Some(page["next_cursor"].as_str().unwrap().to_owned());
    }
    assert_eq!(
        legs,
        ["customer", "agent"],
        "the read completed without every leg of the transferred call"
    );
}

/// The issued cursor must fit the schema the adapter publishes for it: `next_cursor` and
/// the input's `cursor` admit at most 16,384 characters, while `page` admits an ordering
/// value of up to 8,192 bytes before JSON escaping and base64. An ordering value of 8,000
/// `"` characters is admitted, its cursor is about 21,000 characters, the page breaks its
/// own output schema, and the next call is refused as `invalid_input`: the read cannot go
/// on, and it never completes.
#[tokio::test]
async fn an_issued_cursor_fits_the_published_schemas() {
    let key = "\"".repeat(8000);
    let script = fixture::Script {
        columns: vec![fixture::col(
            "k",
            fixture::VAR_STRING,
            0,
            fixture::UTF8MB4,
            0,
        )],
        rows: vec![vec![fixture::text(&key)]],
        params: 0,
        select: Some(Arc::new(|values: &[Option<String>], _| values.is_empty())),
        ..fixture::Script::default()
    };
    let server = fixture::start(script).await;
    let profile = json!({
        "id": "keys",
        "statement": "SELECT k FROM fixture_keys",
        "parameters": [],
        "columns": [{"name": "key", "type": "text"}],
        "call_identifier": "key",
        "ordering": "key"
    });
    let sql = sql(server.port, &[profile]);
    let schema = sql
        .descriptor()
        .operation("profile.read")
        .unwrap()
        .output_schema
        .clone();
    let first = read(&sql, json!({"profile": "keys", "limit": 1}))
        .await
        .unwrap();
    let cursor = first["next_cursor"].as_str().unwrap().to_owned();
    let length = cursor.len();
    let next = read(
        &sql,
        json!({"profile": "keys", "cursor": cursor, "limit": 1}),
    )
    .await;
    connectors_sdk::validate(&schema, &first).unwrap_or_else(|e| {
        panic!("the page breaks its published output schema (cursor of {length} characters): {e:?}")
    });
    let next = next.unwrap_or_else(|e| panic!("the issued cursor was refused: {e:?}"));
    assert_eq!(next["complete"], true);
}

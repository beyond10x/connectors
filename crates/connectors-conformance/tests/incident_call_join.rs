//! A call by its number, every leg with its type, from two sources (story
//! `incident-call-lookup`): the call table read with the SQL adapter's `profile.read`, and
//! the CDR lines read with the Loki adapter's `logs.profile_lines`, each through its own
//! adapter's `invoke`, then joined the way a deterministic workflow step joins them.
//!
//! Neither source alone answers the question. The call table writes no row for a transfer
//! leg, and a CDR line carries no leg type. The join therefore keys on the field each
//! profile declares as its call identifier, matches a CDR line to a call-table row of the
//! same call by direction, both numbers (masked, as each read answers them) and start
//! second, takes the leg type from the row, and types a CDR leg that the call table has no
//! row for as the transfer leg. There is no composite operation: the pairing is this
//! test's, and adapters never depend on one another (`adapters/README.md`).
//!
//! Fixtures are synthetic: the scripted MySQL server and call table owned by the SQL
//! adapter (`adapters/sql/tests/mysql`), and `tests/fixtures/call_join_cdr_streams.json`,
//! whose CDR lines carry the same call identifiers and numbers. `+999` is not an assigned
//! country code.
#![cfg(unix)]
#[path = "../../../adapters/sql/tests/mysql/calls.rs"]
mod calls;
#[path = "../../../adapters/sql/tests/mysql/fixture.rs"]
mod fixture;

use async_trait::async_trait;
use connectors_host::http::{HttpConfig, ScopedHttp};
use connectors_loki::Loki;
use connectors_sdk::{Adapter, Credential, Secret};
use connectors_sql::{Config, Engine, Sql};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// The type the workflow gives a CDR leg the call table has no row for.
const TRANSFER: &str = "transfer";
/// 2026-01-02T00:00:00Z and one day later: the window of `calls::FROM`..`calls::TO`.
const START_NS: &str = "1767312000000000000";
const END_NS: &str = "1767398400000000000";
const NOW_MS: u64 = 1_767_398_400_000;

struct Password;
#[async_trait]
impl Credential for Password {
    async fn resolve(&self) -> connectors_core::Result<Secret> {
        Ok(Secret(fixture::PASSWORD.as_bytes().to_vec()))
    }
}

fn sql(port: u16) -> Sql {
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
    Sql::new("fixture-sql", config, effective, Arc::new(Password))
        .unwrap()
        .with_read_profiles(vec![serde_json::from_value(calls::profile()).unwrap()])
        .unwrap()
}

/// The CDR line profile an operator might declare: logfmt, the call identifier filterable
/// so a workflow can read one call's lines.
fn loki(base: &str) -> Loki {
    let equal =
        |label: &str, value: &str| json!({"label": label, "operator": "equal", "value": value});
    let configuration = json!({
        "format": "connectors-loki-local/1", "instance": "fixture-loki", "base_url": base,
        "ca_digest": null, "query_scope": {"required_equalities": []},
        "line_profiles": [{
            "name": "fixture-cdr",
            "selector": [equal("service", "edge-proxy"), equal("stream_kind", "cdr")],
            "format": {"kind": "logfmt"},
            "fields": [
                {"name": "direction", "key": "dir", "type": "string"},
                {"name": "caller", "key": "src", "type": "phone_number"},
                {"name": "callee", "key": "dst", "type": "phone_number"},
                {"name": "started_at", "key": "start", "type": "unix_seconds"},
                {"name": "ended_at", "key": "end", "type": "unix_seconds", "optional": true},
                {"name": "call_id", "key": "callid", "type": "string", "filterable": true}
            ],
            "call_identifier": "call_id"
        }]
    });
    let http = ScopedHttp::new_with_ca_bytes(
        &HttpConfig {
            base_url: base.into(),
            credential: None,
            credential_header: "authorization".into(),
            bearer: true,
            allow_plaintext: true,
            ca_file: None,
        },
        None,
        None,
    )
    .unwrap();
    Loki::new("fixture-loki", &configuration, Arc::new(http))
        .unwrap()
        .with_clock(|| NOW_MS)
}

/// A loopback HTTP server answering one connection with `answer`, recording the request
/// target it received.
async fn loki_server(answer: Value) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let record = seen.clone();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut head = Vec::new();
        while !head.ends_with(b"\r\n\r\n") {
            head.push(socket.read_u8().await.unwrap());
        }
        let head = String::from_utf8(head).unwrap();
        record
            .lock()
            .unwrap()
            .push(head.split(' ').nth(1).unwrap().to_owned());
        let body = serde_json::to_vec(&answer).unwrap();
        let head = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        socket.write_all(head.as_bytes()).await.unwrap();
        socket.write_all(&body).await.unwrap();
    });
    (format!("http://{address}/"), seen)
}

async fn invoke(adapter: &impl Adapter, operation: &str, input: Value) -> Value {
    let answer = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        adapter.invoke(operation, input),
    )
    .await
    .unwrap_or_else(|_| panic!("{operation} hung"))
    .unwrap_or_else(|e| panic!("{operation}: {e:?}"));
    // Each answer is the one `operations describe` publishes.
    let descriptor = adapter.descriptor();
    let schema = &descriptor.operation(operation).unwrap().output_schema;
    connectors_sdk::validate(schema, &answer).unwrap_or_else(|e| panic!("{answer}: {e:?}"));
    answer
}

/// Seconds since the Unix epoch of a call-table timestamp, `YYYY-MM-DDTHH:MM:SS`, read as
/// UTC (days from the civil date, proleptic Gregorian).
fn unix_seconds(timestamp: &str) -> i64 {
    let n = |range: std::ops::Range<usize>| timestamp[range].parse::<i64>().unwrap();
    let (year, month, day) = (n(0..4), n(5..7), n(8..10));
    let (hour, minute, second) = (n(11..13), n(14..16), n(17..19));
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    days * 86_400 + hour * 3_600 + minute * 60 + second
}

/// One leg as the join answers it.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Leg {
    started_at: i64,
    leg_type: String,
    direction: String,
    caller: String,
    callee: String,
}

/// The workflow step: every CDR leg of the call, typed by the call-table row it matches,
/// or as the transfer leg when there is none; and every call-table row of the call, so a
/// row with no CDR line is not lost. A CDR line matching two rows is ambiguous and refused.
fn join(call_id: &str, rows: &[Value], row_key: &str, records: &[Value]) -> Vec<Leg> {
    let rows: Vec<&Value> = rows.iter().filter(|r| r[row_key] == call_id).collect();
    let mut used = BTreeSet::new();
    let mut legs = Vec::new();
    for record in records.iter().filter(|r| r["call_id"] == call_id) {
        let line = &record["fields"];
        let started_ns: i64 = line["started_at"].as_str().unwrap().parse().unwrap();
        let started_at = started_ns.div_euclid(1_000_000_000);
        let matching: Vec<usize> = rows
            .iter()
            .enumerate()
            .filter(|(_, row)| {
                row["direction"] == line["direction"]
                    && row["caller"] == line["caller"]
                    && row["callee"] == line["callee"]
                    && unix_seconds(row["started_at"].as_str().unwrap()) == started_at
            })
            .map(|(index, _)| index)
            .collect();
        assert!(matching.len() <= 1, "an ambiguous CDR line: {record}");
        let leg_type = match matching.first() {
            Some(&index) => {
                assert!(used.insert(index), "two CDR lines match one row: {record}");
                rows[index]["leg_type"].as_str().unwrap().to_owned()
            }
            None => TRANSFER.to_owned(),
        };
        legs.push(Leg {
            started_at,
            leg_type,
            direction: line["direction"].as_str().unwrap().to_owned(),
            caller: line["caller"].as_str().unwrap().to_owned(),
            callee: line["callee"].as_str().unwrap().to_owned(),
        });
    }
    for (index, row) in rows.iter().enumerate() {
        if !used.contains(&index) {
            legs.push(Leg {
                started_at: unix_seconds(row["started_at"].as_str().unwrap()),
                leg_type: row["leg_type"].as_str().unwrap().to_owned(),
                direction: row["direction"].as_str().unwrap().to_owned(),
                caller: row["caller"].as_str().unwrap().to_owned(),
                callee: row["callee"].as_str().unwrap().to_owned(),
            });
        }
    }
    legs.sort();
    legs
}

#[tokio::test]
async fn every_leg_of_a_transferred_call_is_typed_by_joining_the_call_table_and_the_cdr_lines() {
    // The call table, read by number. The server returns one row per reply whatever LIMIT
    // says, so the transferred call's two rows arrive on different pages.
    let server = fixture::start(calls::script(Some(1))).await;
    let sql = sql(server.port);
    let mut cursor: Option<String> = None;
    let mut rows = Vec::new();
    let mut page_of = Vec::new();
    let mut call_identifier = None;
    for page_index in 0.. {
        assert!(page_index < 20, "the SQL read does not end");
        let page = invoke(&sql, "profile.read", calls::input(cursor.as_deref(), 5)).await;
        call_identifier.get_or_insert_with(|| page["call_identifier"].clone());
        assert_eq!(page["call_identifier"], call_identifier.clone().unwrap());
        for item in page["items"].as_array().unwrap() {
            rows.push(item.clone());
            page_of.push(page_index);
        }
        if page["complete"] == true {
            break;
        }
        cursor = Some(page["next_cursor"].as_str().unwrap().to_owned());
    }
    assert_eq!(rows.len(), calls::expected().len());
    let row_key = call_identifier.unwrap();
    let row_key = row_key.as_str().unwrap();
    let call = calls::TRANSFERRED_CALL;
    let pages: Vec<usize> = rows
        .iter()
        .zip(&page_of)
        .filter(|(row, _)| row[row_key] == call)
        .map(|(_, page)| *page)
        .collect();
    assert_eq!(pages.len(), 2, "{rows:?}");
    assert_ne!(
        pages[0], pages[1],
        "the call's rows did not cross a page boundary"
    );
    // The call table alone has no transfer leg.
    let table_types: Vec<&str> = rows
        .iter()
        .filter(|row| row[row_key] == call)
        .map(|row| row["leg_type"].as_str().unwrap())
        .collect();
    assert_eq!(table_types, ["customer", "agent"]);

    // The CDR lines of that call, read by its identifier.
    let fixture_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/call_join_cdr_streams.json"
    );
    let answer: Value = serde_json::from_slice(&std::fs::read(fixture_path).unwrap()).unwrap();
    let (base, seen) = loki_server(answer).await;
    let loki = loki(&base);
    let cdr = invoke(
        &loki,
        "logs.profile_lines",
        json!({"profile": "fixture-cdr", "start_unix_ns": START_NS, "end_unix_ns": END_NS,
               "limit": 100, "match": {"call_id": call}}),
    )
    .await;
    assert!(
        seen.lock().unwrap()[0].contains("call-0001"),
        "the CDR read is not narrowed to the call"
    );
    let records = cdr["records"].as_array().unwrap();
    // The CDR lines alone carry no leg type, and only lines of exactly this call are read.
    assert_eq!(records.len(), 3, "{cdr}");
    for record in records {
        assert_eq!(record["call_id"], call);
        assert!(record["fields"].get("leg_type").is_none(), "{record}");
    }
    // The heartbeat line and the lines of call-0002 and call-00010 do not hold the
    // matched call identifier exactly: dropped and counted nowhere (logs §12.2).
    assert_eq!(cdr["skipped"], json!({"unreadable": 0}));

    let legs = join(call, &rows, row_key, records);
    let leg = |started_at: &str, leg_type: &str, direction: &str, caller: &str, callee: &str| Leg {
        started_at: unix_seconds(started_at),
        leg_type: leg_type.into(),
        direction: direction.into(),
        caller: caller.into(),
        callee: callee.into(),
    };
    assert_eq!(
        legs,
        [
            leg(
                "2026-01-02T09:00:00",
                "customer",
                "inbound",
                "********0101",
                "********0900"
            ),
            leg(
                "2026-01-02T09:01:05",
                TRANSFER,
                "outbound",
                "********0900",
                "********0555"
            ),
            leg(
                "2026-01-02T09:01:10",
                "agent",
                "outbound",
                "********0101",
                "********0555"
            ),
        ]
    );
    // Numbers stay masked through both reads and the join.
    for text in [serde_json::to_string(&rows).unwrap(), cdr.to_string()] {
        assert!(!text.contains("+999"), "{text}");
    }
}

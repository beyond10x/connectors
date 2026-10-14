//! A synthetic call table behind the scripted MySQL server, and the read profile an operator
//! might declare over it: every leg row a number appears on, in a time window, ordered by a
//! row key. Fictional material only: the table, its columns, the statement and every number
//! are invented for the fixture (`+999` is not an assigned country code).
//!
//! A test outside this crate drives it by including both files under these module names:
//!
//! ```ignore
//! #[path = "<root>/adapters/sql/tests/mysql/fixture.rs"]
//! mod fixture;
//! #[path = "<root>/adapters/sql/tests/mysql/calls.rs"]
//! mod calls;
//! ```
//!
//! then starting `fixture::start(calls::script(cap))`, building the adapter with
//! `Sql::with_read_profiles(vec![calls::profile()])` and invoking `profile.read` with
//! `calls::input(...)`. Each `CallRow` is also available as plain values (`calls::ROWS`), so a
//! join test can state what it expects without decoding the wire.
#![allow(dead_code)]
use crate::fixture::{self, Cell, Col, Script};
use serde_json::{Value, json};
use std::sync::Arc;

/// The number every transferred-call leg below carries on one side.
pub const NUMBER: &str = "+99900000101";
/// The window the fixture's rows fall in, as the profile's `from` and `to` parameters.
pub const FROM: &str = "2026-01-02T00:00:00";
pub const TO: &str = "2026-01-03T00:00:00";
/// The call that was transferred: an inbound leg and the outbound leg to the transfer
/// target, which keeps the caller's number. The transfer leg between them writes no row in
/// this table.
pub const TRANSFERRED_CALL: &str = "call-0001";

/// One row of the synthetic call table, in the statement's column order.
#[derive(Clone, Copy, Debug)]
pub struct CallRow {
    pub row_key: i64,
    pub call_id: &'static str,
    pub direction: &'static str,
    pub caller: &'static str,
    pub callee: &'static str,
    /// `[day, hour, minute, second]` in January 2026.
    pub started: [u8; 4],
    pub ended: [u8; 4],
    pub leg_type: &'static str,
}

/// Seven rows of four calls for `NUMBER`, one row for another number, and one row for
/// `NUMBER` outside the window. With a row cap below seven, the profile pages.
pub const ROWS: [CallRow; 9] = [
    CallRow {
        row_key: 11,
        call_id: TRANSFERRED_CALL,
        direction: "inbound",
        caller: NUMBER,
        callee: "+99900000900",
        started: [2, 9, 0, 0],
        ended: [2, 9, 4, 30],
        leg_type: "customer",
    },
    CallRow {
        row_key: 12,
        call_id: TRANSFERRED_CALL,
        direction: "outbound",
        caller: NUMBER,
        callee: "+99900000555",
        started: [2, 9, 1, 10],
        ended: [2, 9, 4, 30],
        leg_type: "agent",
    },
    CallRow {
        row_key: 13,
        call_id: "call-0002",
        direction: "inbound",
        caller: "+99900000777",
        callee: "+99900000900",
        started: [2, 10, 0, 0],
        ended: [2, 10, 2, 0],
        leg_type: "customer",
    },
    CallRow {
        row_key: 14,
        call_id: "call-0003",
        direction: "outbound",
        caller: "+99900000900",
        callee: NUMBER,
        started: [2, 11, 0, 0],
        ended: [2, 11, 0, 45],
        leg_type: "customer",
    },
    CallRow {
        row_key: 15,
        call_id: "call-0003",
        direction: "outbound",
        caller: "+99900000900",
        callee: NUMBER,
        started: [2, 11, 0, 50],
        ended: [2, 11, 3, 0],
        leg_type: "retry",
    },
    CallRow {
        row_key: 16,
        call_id: "call-0004",
        direction: "inbound",
        caller: NUMBER,
        callee: "+99900000900",
        started: [2, 12, 0, 0],
        ended: [2, 12, 0, 20],
        leg_type: "customer",
    },
    CallRow {
        row_key: 17,
        call_id: "call-0004",
        direction: "outbound",
        caller: "+99900000900",
        callee: NUMBER,
        started: [2, 12, 5, 0],
        ended: [2, 12, 6, 0],
        leg_type: "callback",
    },
    CallRow {
        row_key: 18,
        call_id: "call-0005",
        direction: "inbound",
        caller: NUMBER,
        callee: "+99900000900",
        started: [2, 13, 0, 0],
        ended: [2, 13, 1, 0],
        leg_type: "customer",
    },
    CallRow {
        row_key: 19,
        call_id: "call-0006",
        direction: "inbound",
        caller: NUMBER,
        callee: "+99900000900",
        started: [5, 9, 0, 0],
        ended: [5, 9, 1, 0],
        leg_type: "customer",
    },
];

/// The rows the profile answers for `NUMBER` in the window, in row-key order.
pub fn expected() -> Vec<CallRow> {
    ROWS.iter()
        .copied()
        .filter(|row| matches(row, NUMBER, FROM, TO))
        .collect()
}

/// The statement's `WHERE`: the number on either side, started inside `[from, to)`.
fn matches(row: &CallRow, number: &str, from: &str, to: &str) -> bool {
    let started = format!(
        "2026-01-{:02}T{:02}:{:02}:{:02}",
        row.started[0], row.started[1], row.started[2], row.started[3]
    );
    (row.caller == number || row.callee == number)
        && started.as_str() >= from
        && started.as_str() < to
}

/// The read profile as an operator would write it in the native file's `read_profiles`.
pub fn profile() -> Value {
    json!({
        "id": "calls-by-number",
        "statement": STATEMENT,
        "parameters": ["number", "from", "to"],
        "columns": [
            {"name": "row_key", "type": "integer"},
            {"name": "call_id", "type": "text"},
            {"name": "direction", "type": "text"},
            {"name": "caller", "type": "phone_number"},
            {"name": "callee", "type": "phone_number"},
            {"name": "started_at", "type": "timestamp"},
            {"name": "ended_at", "type": "timestamp"},
            {"name": "leg_type", "type": "text"}
        ],
        "call_identifier": "call_id",
        "ordering": "row_key"
    })
}
pub const STATEMENT: &str = "SELECT id, call_ref, direction, from_number, to_number, started, ended, leg FROM fixture_calls WHERE ? IN (from_number, to_number) AND started >= ? AND started < ?";

/// The `profile.read` input for `NUMBER` in the window.
pub fn input(cursor: Option<&str>, limit: u16) -> Value {
    let mut input = json!({
        "profile": "calls-by-number",
        "parameters": {"number": NUMBER, "from": FROM, "to": TO},
        "limit": limit
    });
    if let Some(cursor) = cursor {
        input["cursor"] = json!(cursor);
    }
    input
}

fn columns() -> Vec<Col> {
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
fn cells(row: &CallRow) -> Vec<Cell> {
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

/// The server: the profile's statement takes three parameters, and the scripted `WHERE`
/// applies them and, on a continuation, the keyset cursor bound after them (`row_key >`
/// the cursor). `row_cap` is how many rows the server returns whatever `LIMIT` says.
pub fn script(row_cap: Option<usize>) -> Script {
    Script {
        columns: columns(),
        rows: ROWS.iter().map(cells).collect(),
        params: 3,
        row_cap,
        select: Some(Arc::new(|values: &[Option<String>], index: usize| {
            let row = &ROWS[index];
            let text = |i: usize| values.get(i).cloned().flatten().unwrap_or_default();
            let after = values
                .get(3)
                .cloned()
                .flatten()
                .map(|cursor| cursor.parse::<i64>().unwrap());
            matches(row, &text(0), &text(1), &text(2)) && after.is_none_or(|a| row.row_key > a)
        })),
        ..Script::default()
    }
}

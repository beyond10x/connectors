---
format: aep.planning-md/3
id: review-result:adversary-mysql-engine-pass-2
kind: review-result
status: active
title: Adversary pass 2 on the MySQL engine
relations:
- reviews: story:parity-mysql-reads
revision: 1
---
## Report

```
unit: A mysql-engine (adversary pass 2)
verdict: red
cases: executed 43→45, red 2
origin: introduced 3, pre-existing 0, undecided 0
wrote-outside-worktree: the wave scratch adv-a2/ (setup.sql, lock.sql, lock2.sql, lock.out, instants.sql, suite.log), plus Docker container adv-a2-mysql-probe, now removed
needs-coordinator: yes
```

Attacked commit `8bde973f2` on `unit/mysql-engine-20261009b`. The pass 1 fixes hold (float text, the null `ca_file`, the read-only wording). Three new findings; cases in `adapters/sql/tests/mysql_adversary_2.rs`:

- `mysql_schema_list_of_another_database_is_not_labelled_as_the_connected_one` (fixture, red): `assertion left == right failed: schema.list bound [[Some("other")]] and answered rows of [String("other")] under provenance resource "fixture", the connected database`.
- `the_model_and_the_executable_count_the_same_units` (red): `host of 300 characters (600 bytes): model admits true, executable admits false`.
- `live_mysql_distinct_timestamps_are_written_distinctly` (`#[ignore]`, needs `CONNECTORS_MYSQL_ADVERSARY_PORT`; red against live MySQL 8.0.46 with `time_zone = Europe/Berlin`): `TIMESTAMP 2026-10-25 00:30:00 UTC and 01:30:00 UTC are written as [String("2026-10-25T02:30:00"), String("2026-10-25T02:30:00")]`.

Suite after the cases existed: `cargo clippy -p connectors-sql --all-targets -- -D warnings` EXIT=0; `cargo test -p connectors-sql --no-fail-fast` EXIT=101 (`tests/mysql_adversary_2.rs: 0 passed; 2 failed; 1 ignored`; lib 5, main 0, ess_model 3, local_runtime 10 (6 ignored), mysql_adversary 2, mysql_local 3, mysql_protocol 12, protocol 8, doctests 0 all ok). `target/` 4.2G, `/` 24G free.

| file:line | measured | what reaches it | origin | verdict / severity |
|---|---|---|---|---|
| `adapters/sql/src/mysql.rs:520` | `schema.list` on MySQL reads whichever database the caller names, provenance always names the configured one; live, a reader with SELECT on two databases read the second through a connection bound to the first | any caller passing a schema other than the connected database | introduced | NEEDS-CHANGE / warning |
| `adapters/sql/src/mysql.rs:299` (`SESSION_BOUNDS` at :32 sets no `time_zone`) | TIMESTAMP written in the session time zone without offset; on a DST zone two instants an hour apart give the same text | any server whose `time_zone` is a DST zone or `SYSTEM` on local time, the MySQL default | introduced | NEEDS-CHANGE / warning |
| `adapters/sql/spec/ess/domains/connection.yaml:44` | the model bounds `host`, `database`, `user` at 512 characters, `local.rs` at 512 bytes | nothing found in practice (MySQL names ≤ 64/32 characters, DNS names ASCII) | introduced | INFEASIBLE / note |

Fix options handed back: schema.list refuses a schema other than the configured database, or provenance names the database read; TIMESTAMP with session `time_zone = '+00:00'` and a `Z`, or with the offset, and `reads.yaml`/`semantics.md` updated; model and `local.rs` count one unit.

Attacked without a break: `FOR UPDATE` refused (1142 / 1792), `FOR SHARE` holds shared locks at most the 10 s bound; `CLIENT_LOCAL_FILES`/`CLIENT_MULTI_STATEMENTS` unreachable; `mysql_clear_password` and `MysqlOldPassword` refused; a server without SSL refused before any credential; the ring provider install matches `connectors-host/src/http.rs:130,778` and `oauth.rs:667`; nothing formats `Opts` or `Conn`; the descriptor revision differs per engine; the engine/profile binding matches the model.

```findings
- file: adapters/sql/src/mysql.rs
  line: 520
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: schema.list on MySQL lists whichever database the caller names and labels the answer with the connected database as provenance resource, where the brief says it reads information_schema for the connected database
- file: adapters/sql/src/mysql.rs
  line: 299
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: TIMESTAMP is written in the server's session time zone with no offset, so on a DST zone two distinct instants come back as the same text (live MySQL 8.0.46, Europe/Berlin)
- file: adapters/sql/spec/ess/domains/connection.yaml
  line: 44
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the model bounds host, database and user in characters while local.rs bounds them in bytes, so a 300-character non-ASCII value is admitted by the model and refused by the executable
```

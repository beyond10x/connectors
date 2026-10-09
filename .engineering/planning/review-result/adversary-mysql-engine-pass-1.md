---
format: aep.planning-md/3
id: review-result:adversary-mysql-engine-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the MySQL engine
relations:
- reviews: story:parity-mysql-reads
revision: 1
---
## Report

```
unit: A mysql-engine (adversary pass 1), commit 1c0fd572a plus the uncommitted test file adapters/sql/tests/mysql_adversary.rs
verdict: red
cases: executed 40→42, red 2
origin: introduced 3, pre-existing 0, undecided 0
wrote-outside-worktree: 7 paths under the wave scratch adv-a/, plus one Docker container (stopped)
needs-coordinator: no
```

New failing tests in `adapters/sql/tests/mysql_adversary.rs` (red on first run, `cargo test -p connectors-sql --test mysql_adversary`):

- `mysql_float_text_is_the_shortest_text_that_reads_back`: a fixture DOUBLE of 1e300; asserts the cell reads back as 1e300 and is no longer than `1e300`. Output: `column 0: 301 characters where "1e300" (5 characters) reads back as the same value`.
- `the_model_and_the_executable_agree_on_a_null_ca_file`: generates the `LocalConfiguration` schema with the pinned ess 0.56.0 and checks model and executable agree on `"ca_file": null`. Output: `assertion left == right failed: mysql: the executable admits=true, the model's errors=["null is not of type \"string\""]`.

Package gates: `cargo clippy -p connectors-sql --all-targets -- -D warnings` EXIT=0; `cargo test -p connectors-sql --no-fail-fast` EXIT=101 (`tests/mysql_adversary.rs: 0 passed; 2 failed`; lib 4, main 0, ess_model 3, local_runtime 10 (+6 ignored), mysql_local 3, mysql_protocol 12, protocol 8, doctests 0: all ok).

| file:line | measured | what reaches it | origin | verdict / severity |
|---|---|---|---|---|
| `adapters/sql/src/mysql.rs:262` | `Cell::Double(v) => v.to_string()` writes doubles in full positional notation; 1e300 as 301 characters in the fixture; against a live MySQL 8.0.46 `SELECT 1e300` and a stored DOUBLE did the same, and 1e-7 came back as `0.0000001` | any DOUBLE/FLOAT at or above about 1e16, or very small; the PostgreSQL path writes `1e+300` | introduced | NEEDS-CHANGE / warning |
| `adapters/sql/spec/ess/domains/connection.yaml:51` | the model types `ca_file` as a plain string and its generated schema refuses `"ca_file": null`; `local.rs` accepts `Option<PathBuf>`; `tests/ess_model.rs` turns a null change into a removed key and cannot catch it | `tests/mysql_local.rs:55`, `tests/local_runtime.rs:203`, `cli_journey.rs:232` bootstrap with `"ca_file":null` | introduced | NEEDS-CHANGE / warning |
| `docs/local-mysql-cli.md:122-128` (and the MySQL section of `contracts/reads/v1alpha1/semantics.md`) | live MySQL 8.0.46, user with only SELECT and EXECUTE: `query.read "SELECT f6()"` with `f6` a definer-rights function running `SET PERSIST max_connections = 66` succeeded and the value persisted; `SET GLOBAL` the same; table writes, including via `SET SESSION transaction_read_only = 0` inside a function, were refused with 1792 | needs an administrator-written function changing server variables with EXECUTE granted to the configured user; nothing in this repository creates one; the docs claim "only a query can run at all" and that a stored-function write is refused | introduced | CONFIRMED / warning |

Attacked without a break: derived-table wrapper escapes (`--`, `#`, `/*!99999 */`, unclosed `/*`, trailing `;`); `SELECT … INTO @a`, `DO`, `CALL`, `SET`, `EXPLAIN` refused as `unsupported`; multi-statements and `CLIENT_LOCAL_FILES` unreachable (no local-infile handler); TLS refuses a server without SSL before any credential, a configured CA replaces built-in roots, hostname verification on; value mapping on the live server (NULL vs empty, BIT, YEAR, TIME ±838 h, DATETIME(2), zero dates, JSON, ENUM/SET, geometry, varbinary, latin1, unsigned 64-bit, DECIMAL(30,10), invalid UTF-8); `schema.list` order and truncation; errors drop server messages; PostgreSQL configurations without `engine` unchanged, descriptor profile `postgresql-native-text`.

```findings
- file: adapters/sql/src/mysql.rs
  line: 262
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: float_text uses f64 Display, so 1e300 is written as 301 characters, not the shortest text that reads back, which the contract and the ESS model promise
- file: adapters/sql/spec/ess/domains/connection.yaml
  line: 51
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the model refuses "ca_file":null, which the executable and the unit's own tests admit, and the model test's null-means-remove helper hides the disagreement
- file: docs/local-mysql-cli.md
  line: 122
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: on a live MySQL 8.0.46, a reader with only SELECT and EXECUTE ran SET PERSIST through a definer function via query.read, contradicting the claim that only a query can run and that stored-function writes are refused
```

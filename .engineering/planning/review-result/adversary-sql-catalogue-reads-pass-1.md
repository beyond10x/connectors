---
format: aep.planning-md/3
id: review-result:adversary-sql-catalogue-reads-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the SQL catalogue reads
relations:
- reviews: story:parity-mysql-reads
revision: 1
---
## Report

```
unit: B schema-detail (adversary pass 1)
verdict: red
cases: executed 63→64, red 1
origin: introduced 5, pre-existing 0, undecided 0
wrote-outside-worktree: the wave scratch adv-b/ (pg-schema.sql, my-schema.sql, probe1.log, clippy.log, test.log)
needs-coordinator: yes
```

Attacked commit `6d66a0786` with live PostgreSQL 17 and MySQL 8.0.46 in disposable containers (removed). Cases in `adapters/sql/tests/catalogue_adversary.rs`; the setup SQL for both servers is in the file's doc comment.

| case | runs | asserts | result |
|---|---|---|---|
| `mysql_table_describe_of_a_name_mysql_cannot_hold_is_not_unavailable` (:102) | suite, MySQL wire fixture | server answers 3988/HY000 (measured live); the read's error is `NotFound` or `InvalidInput` | red: `a table name MySQL cannot hold is answered as Unavailable` |
| `postgresql_generated_column_is_not_reported_as_a_default` (:133) | ignored, `CONNECTORS_PG_ADVERSARY_PORT` | generated column `b` has `column_default` null | red live: `left: String("(a * 2)") right: Null` |
| `postgresql_index_list_does_not_name_a_column_the_role_cannot_see` (:163) | ignored, PG port | `table.describe` hides `priv`, so `index.list` must not name it | red live: `[["partial_expr","partial","1","(priv + 1)",…],["partial_priv","partial","1","priv",…]]` |
| `mysql_table_describe_of_a_name_with_a_trailing_space_is_not_found` (:209) | ignored, `CONNECTORS_MYSQL_ADVERSARY_PORT` | `orders ` is `NotFound` | red live: answered with the columns of `orders` |
| `postgresql_composite_keys_expressions_and_case_on_a_live_server` (:244) | ignored, PG port | composite foreign keys across schemas, expression index keys, INCLUDE columns left out, mixed case | green live |
| `mysql_table_names_are_case_sensitive_on_a_live_server` (:319) | ignored, MySQL port | `Orders` and `orders` differ; `ORDERS` is `NotFound` | green live |

Suite: `cargo clippy -p connectors-sql --all-targets -- -D warnings` exit 0; `cargo test -p connectors-sql --no-fail-fast` exit 101 (`catalogue_adversary: 0 passed; 1 failed; 5 ignored`; others: ess_model 6, local_runtime 10 (+6 ignored), mysql_adversary 2, mysql_adversary_2 2 (+1 ignored), mysql_local 4, mysql_protocol 20, protocol 14, unit 5); `cargo fmt --package connectors-sql -- --check` exit 0.

| file:line | measured | what reaches it | verdict | origin | severity |
|---|---|---|---|---|---|
| adapters/sql/src/lib.rs:109 | PG `table.describe` reports a stored generated column's expression `(a * 2)` as `column_default`; PG's `information_schema.columns` and MySQL give null | any `GENERATED ALWAYS AS … STORED` column (PG 12+) | NEEDS-CHANGE | introduced | warning |
| adapters/sql/src/lib.rs:113 | PG `index.list` checks table visibility only and names `priv` and `(priv + 1)`, which the role cannot see and `table.describe` hides (semantics.md:168) | a role with column-level grants | NEEDS-CHANGE | introduced | warning |
| adapters/sql/src/mysql.rs:53 (and :57) | MySQL `table.describe` and `index.list` answer `orders ` with `orders` (`utf8mb3_bin` PAD SPACE); MySQL forbids trailing spaces in table names; PostgreSQL answers `not_found` | caller input with trailing whitespace | CONFIRMED | introduced | note |
| adapters/sql/src/mysql.rs:576 | MySQL 3988 (a 4-byte character against `utf8mb3` metadata) is classified by SQLSTATE HY000 as `Unavailable`, inviting a retry that cannot succeed; the classifier predates the unit, the catalogue reads reach it with caller text | a table name with a 4-byte character | CONFIRMED | introduced | note |
| adapters/sql/src/lib.rs:109 (and mysql.rs:53) | `table.describe` names the referenced schema, table and column of a foreign-key target the role cannot see: PG `hidden.secret.s` without USAGE on `hidden`; MySQL a database `database.list` hides (the engine's own `KEY_COLUMN_USAGE` visibility); on PG beyond `information_schema`'s rules | a foreign key into a schema or database the role cannot see | CONFIRMED | introduced | note |

Attacked without a break: PG `ROWS FROM` foreign-key pairing (key order, two keys on the same columns, cross-schema target); `indkey::smallint[]` with ordinality and `pg_get_indexdef` (expression parts, mixed keys, INCLUDE left out) on PG 17; MySQL `TABLE_NAME = COALESCE(?, TABLE_NAME)` case-exact under `utf8mb3_bin` at `lower_case_table_names=0`; mixed-case, quoted and `_`/`%` names exact on both engines; quotes and NUL bound; MySQL scope refusals before a session; PG schema bounds; truncation on all four reads; `database.list` visibility on both; views, materialized views, partitioned tables; null row estimates for views and unanalysed tables; missing table `not_found`; empty table name refused. PG 10 lacks `pg_index.indnkeyatts`; no document claims PG 10 support.

```findings
- file: adapters/sql/src/lib.rs
  line: 109
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: PostgreSQL table.describe reports a stored generated column's generation expression as column_default, where the engine's information_schema and MySQL both report null
- file: adapters/sql/src/lib.rs
  line: 113
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: PostgreSQL index.list checks table-level visibility only and names index columns and expressions over columns the role holds no privilege on, which table.describe hides
- file: adapters/sql/src/mysql.rs
  line: 53
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: MySQL table.describe and index.list match a name with a trailing space to the unpadded table under utf8mb3_bin PAD SPACE, answering a table that does not exist instead of not_found
- file: adapters/sql/src/mysql.rs
  line: 576
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a table name with a supplementary character makes MySQL answer error 3988 HY000, which the catalogue reads report as unavailable rather than not_found or invalid_input
- file: adapters/sql/src/lib.rs
  line: 109
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: table.describe names the referenced schema, table and column of a foreign-key target the role or user cannot see, on PostgreSQL beyond information_schema's rules
```

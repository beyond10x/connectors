---
format: aep.planning-md/3
id: review-result:adversary-sql-catalogue-reads-pass-2
kind: review-result
status: active
title: Adversary pass 2 on the SQL catalogue reads
relations:
- reviews: story:parity-mysql-reads
revision: 1
---
## Report

```
unit: B schema-detail (adversary pass 2), working tree at 23535e8ad plus one untracked test file
verdict: red
cases: executed 66→66, red 3
origin: introduced 3, pre-existing 0, undecided 0
wrote-outside-worktree: the wave scratch adv-b2/ (pg-schema.sql, my-schema.sql, fmt.log, clippy.log, test.log)
needs-coordinator: yes
```

The correction holds: all eight pass-1 live cases pass on real servers. Three new live cases in `adapters/sql/tests/catalogue_adversary_2.rs` (ignored, `CONNECTORS_PG_ADVERSARY_PORT`, setup SQL in the file's doc comment) were red on their first run against a disposable PostgreSQL 17:

- `postgresql_a_foreign_key_into_a_partitioned_table_is_reported_once` (:89): one key into the partitioned `app.part_ref` is answered as six rows for two columns, adding `to_part_x_y_fkey` → `part_ref_lo` and `to_part_x_y_fkey1` → `part_ref_hi` (internal clones, `conparentid <> 0`).
- `postgresql_index_list_leaves_out_a_whole_row_expression_over_a_hidden_column` (:136): `["whole_row","whole","1","(whole.* IS NOT NULL)","NO","NO"]` is listed to a role that sees only `pub` (`refobjsubid = 0` not checked).
- `postgresql_a_foreign_key_does_not_name_a_referenced_column_describe_hides` (:177): `[[x,…,"colchild_fk","other","colpar","secretcol"]]` where `table.describe` of `other.colpar` shows only `a`.

Suite: `cargo fmt --package connectors-sql -- --check` exit 0; `cargo clippy -p connectors-sql --all-targets -- -D warnings` exit 0; `cargo test -p connectors-sql --no-fail-fast` exit 0, 66 passed, 21 ignored (the three need a server).

| file:line | measured | what reaches it | verdict | origin | severity |
|---|---|---|---|---|---|
| adapters/sql/src/lib.rs:113 | a foreign key into a partitioned table is reported once more per referenced partition under undeclared names, against semantics.md:161 | any foreign key into a partitioned table (PostgreSQL 12+), read by a role that can see the partitions | NEEDS-CHANGE | introduced | warning |
| adapters/sql/src/lib.rs:120 | an index over a whole-row reference is listed to a role that cannot see a column, against the widened rule of semantics.md:178 | any index or predicate referencing the whole row, with column-level grants | NEEDS-CHANGE | introduced | note |
| adapters/sql/src/lib.rs:113 | `table.describe` names a referenced column that `table.describe` of that table hides | a foreign key into a column outside the role's column-level grant | CONFIRMED | introduced | note |

Suggested fixes: keep only the declared key (care: plain `conparentid = 0` would also hide a partition's inherited key); treat `refobjsubid = 0` as every column; check the referenced column's privilege or state the exception.

Attacked without a break, live on PostgreSQL 17 and MySQL 8.0: every pass-1 case (generated default null; hidden-schema/table keys left out; key, INCLUDE, expression and predicate index rule per table and per schema; MySQL trailing space and supplementary characters `not_found` on both reads including the `COALESCE(?, TABLE_NAME)` path; case-exact names; cross-schema composite keys; expression keys). By reading: the `catalogue` flag is `#[serde(skip)]`; `query.read` still classifies 3988 as `unavailable`; the ESS `CatalogScope` agrees with the trailing-space check and the order of the schema refusal.

```findings
- file: adapters/sql/src/lib.rs
  line: 113
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: PostgreSQL table.describe reports a foreign key into a partitioned table once more per referenced partition, through the internal clone constraints (conparentid <> 0) under names nobody declared
- file: adapters/sql/src/lib.rs
  line: 120
  category: contract-drift
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: PostgreSQL index.list keeps an index whose expression is a whole-row reference over a hidden column, because pg_depend records it with refobjsubid 0 and the visibility check reads only refobjsubid > 0
- file: adapters/sql/src/lib.rs
  line: 113
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: PostgreSQL table.describe names as referenced_column a column the role holds no privilege on and table.describe of that table hides, against the correction's own rule that the reads never name a hidden column
```

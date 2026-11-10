---
format: aep.planning-md/3
id: review-result:wave-b-sql-pgcat-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on sql-wrapper-resolves-pg-catalog-first (single defect wave)
relations:
- reviews: story:sql-wrapper-resolves-pg-catalog-first
revision: 1
---
```
unit: story:sql-wrapper-resolves-pg-catalog-first, commit d5bfe23d4 plus an uncommitted test file
verdict: red
cases: executed 21→23, red 2 (plus 4 live cases on disposable PostgreSQL 17, red 4)
origin: introduced 2, pre-existing 3, undecided 0
wrote-outside-worktree: <scratch>/units/sql-wrapper-resolves-pg-catalog-first/scratch/adv/ (setup.sql, setup.sh, catalogue-probe.sh, base-type-probe.sh, pull.log, red-offline.log, red-live.log, suite.log, clippy.log)
needs-coordinator: yes
```

The fix breaks every PostgreSQL read that returns two or more columns, `schema.list`, `table.list`,
`table.describe` and `index.list` included; `protocol.rs` stays green because its fake server never
parses SQL.

1. Diff: one untracked file, `adapters/sql/tests/adversary_pg_catalog.rs`; no source changed.
2. Cases, each run alone first, all red:
   - `postgresql_wrapper_writes_each_operator_as_its_own_token`: "`OPERATOR(` at byte 78 continues the identifier before it … `::bigintOPERATOR(pg_catalog.+)COALESCE(`".
   - `postgresql_wrapper_names_no_type_through_the_search_path`: "the wrapper casts to `text` as `search_path` resolves it, `public` first".
   - Live `postgresql_live_read_of_two_columns_returns_its_row`: `Err(Unsupported)`.
   - Live `postgresql_live_catalogue_reads_of_several_columns_answer`: "schema.list does not answer: Err(Unsupported)".
   - Live `postgresql_live_read_never_runs_a_public_text_domain_check`: 2 calls of the shadow function, expected 0.
   - Live `postgresql_live_type_lookup_never_runs_a_public_operator`: 630 calls, expected 0; the read still answered `Ok([["ok"]])`.
3. Suite `cargo test --locked --no-fail-fast -p connectors-sql --test protocol --test adversary_pg_catalog`: adversary_pg_catalog "FAILED. 0 passed; 2 failed; 4 ignored"; protocol "ok. 21 passed"; exit 101. fmt and clippy exit 0.
4. Findings:
   - F1 blocker, introduced: `lib.rs:478` joins the terms with `OPERATOR(pg_catalog.+)` and no spaces; PostgreSQL lexes `bigintOPERATOR` as one type name, a syntax error (42601) returned as `unsupported`. psql on PostgreSQL 17 rejects the new text and accepts the base text. `protocol.rs:251-256` pins the broken text. Fix: join with `" OPERATOR(pg_catalog.+) "`.
   - F2 warning, pre-existing: `result.cN::text` and `NULL::text[]` (`lib.rs:471,476,504`) resolve `text` through `public` first; a `public.text` domain's CHECK function runs as the reader on every read. Fix: `pg_catalog.text`, and `COLLATE pg_catalog."C"` in `profile.rs:125`.
   - F3 warning, pre-existing: preparing the caller's statement (`lib.rs:438`) runs the driver's type lookup (`WHERE t.oid = $1`) under `public, pg_catalog` (`lib.rs:92`), so a `public.=` operator runs for any type the driver does not know (enum, domain, `citext`). Fix only by putting `pg_catalog` first, which changes the order `semantics.md:25` documents.
   - F4 warning, pre-existing (psql only): the fixed catalogue statements (`lib.rs:95-133`) use plain `=`, `<`, `IN`, `<>`; the `table.list` join called `public.=` 415 times as the reader.
   - F5 note, introduced: `semantics.md:297` still shows `WHERE E(…) > E($n)`.
5. Could not break: each qualified name has a pin that catches its mutant; `::bigint` is a keyword type; `ORDER BY` is not looked up through the path.

```findings
[
{"file":"adapters/sql/src/lib.rs","line":478,"category":"mutant","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the size terms are joined with OPERATOR(pg_catalog.+) and no space, so PostgreSQL lexes ::bigintOPERATOR as one type name and every read of two or more columns, catalogue reads included, is refused as unsupported, while protocol.rs:251 pins that text as correct"},
{"file":"adapters/sql/src/lib.rs","line":504,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"the wrapper's ::text and NULL::text[] resolve public.text first, so a domain made there by a principal with CREATE on public runs its CHECK function as the reader on every read"},
{"file":"adapters/sql/src/lib.rs","line":92,"category":"judgement","severity":"warning","verdict":"CONFIRMED","origin":"pre-existing","message":"tokio-postgres prepares its type lookup (t.oid = $1) under search_path public, pg_catalog, so a public.= operator runs as the reader whenever a statement has a type the driver does not know, measured at 630 calls on a read that still answered"},
{"file":"adapters/sql/src/lib.rs","line":108,"category":"judgement","severity":"warning","verdict":"CONFIRMED","origin":"pre-existing","message":"the fixed catalogue statements use unqualified =, <, IN and <> under the public-first path, and the table.list join called a public.= shadow 415 times as the reader"},
{"file":"adapters/sql/contracts/reads/v1alpha1/semantics.md","line":297,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the documented PostgreSQL keyset statement still says WHERE E(x) > E($n) while the wrapper now emits OPERATOR(pg_catalog.>)"}
]
```

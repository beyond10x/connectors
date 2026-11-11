---
format: aep.planning-md/3
id: review-result:wave-b-sql-pgcat-security-1
kind: review-result
status: active
title: Security review 1 on sql-wrapper-resolves-pg-catalog-first (single defect wave)
relations:
- reviews: story:sql-wrapper-resolves-pg-catalog-first
revision: 1
---
```
unit: story:sql-wrapper-resolves-pg-catalog-first (worktree cb-sql-pgcat, commit d5bfe23d4 plus an uncommitted test file)
verdict: red
cases: executed 21→32, red 10
origin: introduced 1, pre-existing 4, undecided 0
wrote-outside-worktree: <scratch>/units/sql-wrapper-resolves-pg-catalog-first/security-{docker-pull.log,setup.sql,setup-db.sql,shadows.sql,probe.sql,readonly.sql,cases.log,cases-2.log,suite.log,clippy.log}; container sec-cb-sql-pgcat-pg (postgres:17-alpine, 127.0.0.1:45455)
needs-coordinator: yes
```

1. Diff: no tracked file changed; untracked `adapters/sql/tests/security_pg_catalog.rs` (597 lines).
2. Cases: 4 wire-fixture cases, red (two-column wrapper glues `OPERATOR(` to `::bigint`; wrapper `::text` casts unqualified; keyset `COLLATE "C"` unqualified; catalogue operators unqualified). 7 live cases on PostgreSQL 17, each reading a shadow-free `clean` database first as a control: green 1 (`live_wrapper_does_not_call_a_public_octet_length_or_operator`, the implementor's fix works); red 6 (no-shadow two-column and catalogue read; `public.text` domain; size guard; `public."C"`; driver type lookup; catalogue). `live_catalogue_does_not_call_a_public_operator` fails on its control (F1); the shadow itself was measured with psql (`ERROR: shadow ran as reader … shadow_nameeq(name,name)`). `live_size_guard…` shows the read failing (`Unavailable`); psql as `reader` measured the bypass: `guard_sum 0 | sent_bytes 20000004` for `SELECT 1`. Alone: `FAILED. 1 passed; 10 failed`, exit 101.
3. Suite (`--test protocol --test security_pg_catalog -- --include-ignored`, `CONNECTORS_PG_SECURITY_PORT=45455`): protocol `ok. 21 passed`; security_pg_catalog `FAILED. 1 passed; 10 failed`, exit 101; clippy and rustfmt exit 0.
4. Findings against d5bfe23d4:
   - F1, introduced, blocks the Acceptance: `lib.rs:478` `.join("OPERATOR(pg_catalog.+)")` emits `::bigintOPERATOR(pg_catalog.+)`; PostgreSQL answers `42601`; every read of two or more columns is refused as not a query. Fix: `.join(" OPERATOR(pg_catalog.+) ")`.
   - F2, pre-existing: `::text` and `NULL::text[]` (`lib.rs:471/476/504`) look up `public.text` first; a `public.text` domain runs the shadow owner's code as `reader`; a `public.text` composite with owner-defined casts makes the guard measure 0 while the server sends 20 MB. The story's own defect still holds in the same statement.
   - F3, pre-existing: `COLLATE "C"` (`profile.rs:125`) looks up `public."C"` first; pages come back `["a","B"]` and `b` is lost, against the byte order `semantics.md:305` promises.
   - F4, pre-existing: tokio-postgres 0.7.18 runs its type lookup `WHERE t.oid = $1` in the connector's session for a user-defined result type; a `public.=(oid,oid)` runs as `reader`. Qualifying the connector's SQL cannot fix it.
   - F5, pre-existing: the catalogue and `schema.list` statements (`lib.rs:95-133`) use bare `=`, `<`, `>`, `<=`, `<>`, `||`, `IN`.
   - One fix for F2-F5: `lib.rs:92` `SET LOCAL search_path = pg_catalog, public`, PostgreSQL's own default order and the only fix for F4; update `semantics.md:25`, `docs/local-postgres-cli.md:149`, `docs/running-services.md:462` and `protocol.rs:209`.
5. Checked, no fault: the qualified `octet_length` and `>` hold live against raising shadows (the base text runs them); READ ONLY holds against shadow code; `::bigint`, `::integer`, `::smallint` are grammar-fixed; `SET` statements look nothing up; plain `ORDER BY` does not use the path.

```findings
[
{"file":"adapters/sql/src/lib.rs","line":478,"category":"acceptance","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"join(\"OPERATOR(pg_catalog.+)\") glues to ::bigint, PostgreSQL answers 42601 and every PostgreSQL read of two or more columns, catalogue reads included, is refused as not a query"},
{"file":"adapters/sql/src/lib.rs","line":471,"category":"integrity","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"the wrapper's ::text and NULL::text[] look up public.text first, so another principal's domain or casts run as the connector role and make the size guard measure 0 while the server sends 20 MB"},
{"file":"adapters/sql/src/profile.rs","line":125,"category":"contract-drift","severity":"warning","verdict":"CONFIRMED","origin":"pre-existing","message":"COLLATE \"C\" looks up public.\"C\" first, so a nondeterministic shadow orders the text keyset case-insensitively and a continuation silently drops rows, against semantics.md's byte order"},
{"file":"adapters/sql/src/lib.rs","line":92,"category":"integrity","severity":"warning","verdict":"CONFIRMED","origin":"pre-existing","message":"search_path = public, pg_catalog makes tokio-postgres's own type lookup (t.oid = $1) call a public =(oid,oid) as the connector role whenever a statement returns a user-defined type"},
{"file":"adapters/sql/src/lib.rs","line":95,"category":"integrity","severity":"warning","verdict":"CONFIRMED","origin":"pre-existing","message":"the catalogue and schema.list statements compare with bare operators that a public =(name,name) shadows, running the shadow owner's code as the connector role"}
]
```

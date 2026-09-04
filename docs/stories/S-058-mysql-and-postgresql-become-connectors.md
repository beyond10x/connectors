---
id: S-058
title: "MySQL and PostgreSQL become connectors"
pillar: Platform
status: done
design: ../design/15-a-zero-configuration-endpoint-plane.md
epic: endpoint-plane
areas: [catalog, integrations, server]
---

# MySQL and PostgreSQL become connectors

## Goal

The platform governs HTTP vendors and Kubernetes but not the databases the product actually
runs on. Give MySQL and PostgreSQL permanent provider identities and a read-only operation
surface first — bounded query (row and byte capped), list-schemas/tables, describe-table —
executed by a SQL protocol driver in the mold of the existing non-HTTP drivers, behind the
same admission seam as every operation. Credentials arrive through platform custody
(connection secret references, Vault), never from the caller; connection configs name a
reference, not a value. Effect posture: everything in this story is `read_only` and rides the
read path; mutations are a later grant-gated story.

## Acceptance

- Providers `mysql` and `postgresql` exist with permanent ids/authorities per the
  adding-a-connector discipline; their read-only operations carry honest risk/effects/
  idempotency and JSON Schemas, and `catalog build && diff && check` stays clean.
- A bounded query against a real database (containerized in tests) returns rows inside the
  result cap with honest truncation, and a write statement (INSERT/UPDATE/DDL) is refused
  before reaching the database, proven by tests.
- No credential value appears in config, catalog, logs, or errors — the custody path is a
  reference; the brand and secret fences stay green.
- `bash scripts/gate.sh` exits 0.

## Progress

- 2026-08-24 — filed from design 15 (Timo: "there should be a generic sql/mysql/postgres
  connector").
- 2026-08-24 — implemented on `impl/S-058`. The closed `sql_v1` driver word joins the whole
  vocabulary chain (connector-spec IR, provider/document/site schemas, catalog, resolve, domain
  `DriverId`/`SqlPlan`, service planning + dispatch slot; integration-platform refuses it like
  SIP). Providers `mysql` (authority `com.mysql.server`) and `postgresql`
  (`org.postgresql.server`) declare four read-only unary operations each — bounded query,
  schemas-list, tables-list, table-describe — as repository-authored native members in the
  b10x mold; SOURCES.toml registers both against official-doc references. The new
  `crates/driver-sql` nested workspace owns the wire: sqlparser-based statement admission
  (single statement, SELECT/SHOW/DESCRIBE class, keyword fence + AST walk) refuses writes
  before credential resolution or any socket; tokio-postgres runs the read as a cursor in a
  `BEGIN READ ONLY` transaction, mysql_async streams inside `START TRANSACTION READ ONLY`;
  row/byte caps live in one shared accumulator with honest truncation causes. Credentials are
  custody references (file/env resolve today; the S-059 `secret_ref{name,namespace}` Kubernetes
  shape is declared behind the same trait and refuses by name until its resolver is composed).
  Proven by 28 unit tests plus 14 `#[ignore]`d live tests run against real postgres:17 and
  mysql:8.4 containers. Deferred: the runtime integration that serves these operations behind a
  Connection (needs S-059 descriptors), and MCP exposure (S-060/S-061).
- 2026-08-24 — merged to main after two independent reviews (both PASS, 0 blocking; the second
  ran a 31-case write-admission attack, all structural writes refused before any socket). One
  minor recorded: admission is a read-statement allowlist, so an admitted SELECT can still call
  side-effecting server functions (pg_read_file, LOAD_FILE) — read-only at the statement level,
  with least-privilege DB accounts as the deployment's second fence, documented in the driver.
  The sql_v1 dispatch slot is present but returns unavailable until custody/discovery compose it
  (S-059 descriptors, S-061 wiring).

## Superseded by

`story:mysql-and-postgresql-become-connectors` in the AEP planning store, at
`.engineering/planning/story/mysql-and-postgresql-become-connectors.md`, now carries this story. Migrated 2026-09-04 by the
`aep-planning:story-migration` skill; this file is kept as the source it was migrated from and is
not the place the rung is recorded any more. The artifact quotes this file's `## Acceptance`
verbatim and names it in its own `## Provenance`.

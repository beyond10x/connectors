---
id: S-058
title: "MySQL and PostgreSQL become connectors"
pillar: Platform
status: ready
priority: 2
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

---
format: aep.planning-md/1
id: story:mysql-and-postgresql-become-connectors
kind: story
status: implemented
title: MySQL and PostgreSQL become connectors
refs:
- provider: legacy
  reference: S-058
relations:
- derived_from: epic:endpoint-plane
scope:
- confidence: cited
  path: crates/catalog
- confidence: cited
  path: crates/server
revision: 6
---
## Acceptance

Verbatim from `docs/stories/S-058-mysql-and-postgresql-become-connectors.md:24`. **read**

- Providers `mysql` and `postgresql` exist with permanent ids/authorities per the
  adding-a-connector discipline; their read-only operations carry honest risk/effects/
  idempotency and JSON Schemas, and `catalog build && diff && check` stays clean.
- A bounded query against a real database (containerized in tests) returns rows inside the
  result cap with honest truncation, and a write statement (INSERT/UPDATE/DDL) is refused
  before reaching the database, proven by tests.
- No credential value appears in config, catalog, logs, or errors — the custody path is a
  reference; the brand and secret fences stay green.
- `bash scripts/gate.sh` exits 0.

## Context

The platform governs HTTP vendors and Kubernetes but not the databases the product actually
runs on. Give MySQL and PostgreSQL permanent provider identities and a read-only operation
surface first — bounded query (row and byte capped), list-schemas/tables, describe-table —
executed by a SQL protocol driver in the mold of the existing non-HTTP drivers, behind the
same admission seam as every operation. Credentials arrive through platform custody
(connection secret references, Vault), never from the caller; connection configs name a
reference, not a value. Effect posture: everything in this story is `read_only` and rides the
read path; mutations are a later grant-gated story.

Source frontmatter: pillar Platform · areas [catalog, integrations, server] · design `../design/15-a-zero-configuration-endpoint-plane.md`. **read**

## Status

`done` in the source. Quoted from `docs/stories/S-058-mysql-and-postgresql-become-connectors.md:5`: `status: done`. **read**

This artifact reached `implemented` with `aep artifact move --evidence test_result=1`. The journal
records that move as resting on an **assertion**, not on a run this migration observed. The flag is
what the CLI provides for evidence that lives outside the store.

What was asserted, and where it came from:

- The source records `status: done` at the line quoted above. **read**
- `bash scripts/gate.sh` was green at commit `a48030b` on 2026-09-04 — exit 0, 136 `test result: ok`
  lines across 11 workspaces. **read**, from `~/.cache/connectors-gate/gate2.log`

No per-story run was attributed to this story. The gate is a repository-wide fact, and reading it as
proof of one story's acceptance would be an inference this record does not make.

## Provenance

Migrated from `docs/stories/S-058-mysql-and-postgresql-become-connectors.md`, which is not deleted and now names this artifact.

- First written 2026-08-24 · last touched 2026-08-24 · 3 revision(s)
- Legacy id `S-058`, recorded as the reference `legacy:S-058`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill

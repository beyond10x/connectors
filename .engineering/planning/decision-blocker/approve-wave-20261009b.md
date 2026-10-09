---
format: aep.planning-md/3
id: decision-blocker:approve-wave-20261009b
kind: decision-blocker
status: cleared
title: Nobody has approved wave 20261009b (MySQL reads in the SQL adapter)
relations:
- blocks: story:parity-mysql-reads
revision: 2
transitions:
- {from: "open", to: "cleared", at: "2026-10-09T18:43:36Z", actor: "human:timo", revision: 2}
---
## Question

Approve wave 20261009b: story:parity-mysql-reads, MySQL query and schema reads in the SQL adapter, delivered on one integration branch (`wave/20261009b`) with one pull request, released as 0.40.0 when it merges. The story's note asks to confirm the engine before building.

## Engine evidence

- Agent session transcripts since 2026-09-09, fluxplane `sql` call lines: the endpoint references are three distinct `aurora-*-ro` names (95 mentions); 23 further lines name `aurora`, 4 name `mysql`, none names `postgres`.
- The fluxplane event store (2026-05-24 to 2026-06-10 only): of 227 `sql.query` events, 151 mention `mysql` and 43 `postgres`.
- Verified: the endpoints are Aurora read replicas. Inferred, not verified: Aurora MySQL; no call line records the wire protocol.

## Units

| unit | surface | operations |
|---|---|---|
| A mysql-engine | `adapters/sql`: an engine choice per connection (`postgresql` default, `mysql`), a MySQL wire binding, a read-only session, typed value mapping; `query.read`, `schema.list` and connect/revalidate on MySQL | `sql.query`, `sql.test` (652 calls) |
| B schema-detail | `adapters/sql`: database listing, primary and foreign keys and nullability, views, index listing, for both engines | `sql.table.show`, `sql.table.list`, `sql.database.list`, `sql.index.list` (51 calls) |

Both units touch `adapters/sql`, so they run in sequence in one wave tree with one `target/`; spec first in the adapter's ESS model.

## Build

One tree building at a time; estimated peak 7G `du`, cap 10G. Builds stop while `/` is under 17G free and resume at 19G.

## Options

| option | what |
|---|---|
| A | units A and B as one wave, release 0.40.0 |
| B | unit A only; B the next wave |
| C | hold until the engine is confirmed by an endpoint's port |

Recommended: A.

## Decided

Option A, 2026-10-09: units A and B in wave 20261009b, one pull request, release 0.40.0. The Aurora evidence is enough to build MySQL; the engine choice per connection keeps PostgreSQL as it is.

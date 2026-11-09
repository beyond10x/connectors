---
format: aep.planning-md/3
id: story:mysql-query-read-provenance
kind: story
status: active
title: MySQL query.read names the database it read
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
- decomposes: epic:zero-open-issues-and-store-defects
scope:
- confidence: inferred
  path: adapters/sql/src/lib.rs
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:58:11Z", actor: "human:timo", revision: 2, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:58:12Z", actor: "human:timo", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
---
## Outcome

On a MySQL connection, `query.read` names the database it actually read, or refuses a statement that reads outside the configured database.

## Found

Unit A of wave 20261009b (correction round 2) bound `schema.list` on MySQL to the configured database. `query.read` still runs `SELECT * FROM other.t` when the configured user is granted `other`. Its answer then carries the configured database as `provenance.resource`. On PostgreSQL a connection cannot leave its database, so the label is true there.

## Options

- Document that grants are the boundary, and that provenance names the connection's database rather than every database a statement touched.
- Refuse qualified names outside the configured database. This needs SQL parsing, which the adapter avoids today.

## Acceptance

- One of the options above, modelled in `adapters/sql/spec/ess/` first.
- A wire-fixture case that pins the chosen behaviour.

---
format: aep.planning-md/3
id: story:mysql-query-read-provenance
kind: story
status: draft
title: MySQL query.read names the database it read
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 1
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

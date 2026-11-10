---
format: aep.planning-md/3
id: story:mysql-query-read-provenance
kind: story
status: implemented
title: MySQL query.read provenance says which database it names
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
- decomposes: epic:zero-open-issues-and-store-defects
scope:
- confidence: cited
  path: adapters/sql/contracts/reads/v1alpha1/semantics.md
- confidence: cited
  path: adapters/sql/spec/ess/domains/reads.yaml
- confidence: cited
  path: adapters/sql/tests/ess_model.rs
- confidence: cited
  path: adapters/sql/tests/query_provenance_adversary.rs
- confidence: cited
  path: docs/local-mysql-cli.md
revision: 14
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:58:11Z", actor: "human:timo", revision: 2, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:58:12Z", actor: "human:timo", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "active", to: "implemented", at: "2026-11-10T06:20:11Z", actor: "human:timo", revision: 14, decided_on: {"recorded":{"test_result":1,"review_outcome":4,"verification":1}}, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
---
## Outcome

On a MySQL connection, `query.read` either names the database it actually read or refuses a statement that reads outside the configured database, or the contract states plainly that the configured user's grants are the boundary and that `provenance.resource` names the connected database, not every database a statement touched.

Restated by the coordinator (wave A, non-interactive) after `review-result:wave-a-mysql-provenance-adversary-1`: the Acceptance admits either option, and the unit took the documented one.

## Found

Unit A of wave 20261009b (correction round 2) bound `schema.list` on MySQL to the configured database. `query.read` still runs `SELECT * FROM other.t` when the configured user is granted `other`. Its answer then carries the configured database as `provenance.resource`. On PostgreSQL a connection cannot leave its database, so the label is true there.

## Options

- Document that grants are the boundary, and that provenance names the connection's database rather than every database a statement touched.
- Refuse qualified names outside the configured database. This needs SQL parsing, which the adapter avoids today.

## Acceptance

- One of the options above, modelled in `adapters/sql/spec/ess/` first.
- A wire-fixture case that pins the chosen behaviour.

## Scope

Landed on, from `git diff a83a03b85...impl/mysql-query-read-provenance` (merged into wave/zero-issues at cdc7c8db4); the earlier scoper lines marked inferred are replaced by this list. Generated reference pages are left out.

- `adapters/sql/contracts/reads/v1alpha1/semantics.md`
- `adapters/sql/spec/ess/domains/reads.yaml`
- `adapters/sql/tests/ess_model.rs`
- `adapters/sql/tests/query_provenance_adversary.rs`
- `docs/local-mysql-cli.md`

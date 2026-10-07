---
format: aep.planning-md/3
id: story:parity-mysql-reads
kind: story
status: draft
title: MySQL query and schema reads through the SQL adapter
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

MySQL query and schema reads through the SQL adapter — parity unit U01 of `docs/fluxplane-plugin-parity.md`.

## Operations

`sql.query`, `sql.table.show`, `sql.table.list`, `sql.test`, `sql.database.list`, `sql.index.list`

704 calls since 2026-09-09 (declared and mapped undeclared names), provider `sql`, planned wave W1.

## Surface

native adapter `adapters/sql` (PostgreSQL only today): a MySQL binding — wire protocol, dialect, typed values.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives (the parity page names the gap per operation).
- The parity page row of each operation moves to covered, with the Connectors operation named.

## Note

Which engine the 647 `sql.query` calls reached is not recorded; the 2026-09-09 baseline saw MySQL/Aurora incident work. Confirm the engine before building.

---
format: aep.planning-md/3
id: story:sql-length-units
kind: story
status: implemented
title: SQL adapter bounds count one unit in model and executable
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
- decomposes: epic:zero-open-issues-and-store-defects
scope:
- confidence: inferred
  path: adapters/sql/src/lib.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:58:26Z", actor: "human:operator", revision: 2, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:58:26Z", actor: "human:operator", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "active", to: "implemented", at: "2026-11-11T06:18:37Z", actor: "human:operator", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

The SQL adapter's model and its executable count every length bound in the same unit.

## Found

Unit A of wave 20261009b fixed `host`, `database` and `user`, which are now counted in characters. Two bounds still disagree:

- The password: the ESS model says `value.count ≤ 8192` (characters), while the host's `EntryField.max_bytes` and the model's comment say bytes. The host field lives in `connectors-host`.
- Query text and parameters: the descriptor's `maxLength: 8192` counts characters, while `adapters/sql/src/lib.rs:223,226` checks `len()` in bytes. This is shared by PostgreSQL and MySQL.

## Acceptance

- Each bound is stated in one unit in the ESS model, and the executable checks that unit.
- A case with multi-byte text at the bound and one past it, on both sides.

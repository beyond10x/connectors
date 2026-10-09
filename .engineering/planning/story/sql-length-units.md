---
format: aep.planning-md/3
id: story:sql-length-units
kind: story
status: draft
title: SQL adapter bounds count one unit in model and executable
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 1
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

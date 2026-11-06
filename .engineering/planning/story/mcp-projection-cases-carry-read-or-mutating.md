---
format: aep.planning-md/3
id: story:mcp-projection-cases-carry-read-or-mutating
kind: story
status: draft
title: MCP projection cases say whether each operation reads or mutates
relations:
- derived_from: review-result:wave-20261105b-u1-adversary
revision: 1
---

## Why

Wave 2026-11-05b split the MCP absent-revision binding: a read tool, or a name the server does not list, binds an absent revision to the advertised one, while a listed mutating tool without a revision is refused `-32602`. `adapters/mcp/contracts/server/v1alpha1/mutation-cases.json` holds the mutating side, since every row there is a mutating call. The 15 visibility rows in `projection-cases.json` have no read or mutating dimension, so a row with a null revision is always checked as a read. The U1 adversary raised this (review-result:wave-20261105b-u1-adversary F5), and the U1 implementor judged the field too large for that unit.

## Outcome

Every row in `projection-cases.json` states whether its operation is a read or mutating, and the capability projection checker (`crates/connectors-build/tests/mcp_inbound_capability_projection.rs`) expects `-32602` for a listed mutating operation without a revision. A mutating row without a revision fails the checker if its expectation is a read's binding.

## Notes

This matters once a mutating tool can be projected. Today readiness refuses any non-Read requirement off V2 (`crates/connectors-host/src/local/runtime.rs:718-719`), and only V3 is projected. Filed as a draft by the 2026-11-05b wave.

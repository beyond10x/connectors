---
format: aep.planning-md/3
id: story:owner-bounded-read-realization-matches-projection
kind: story
status: active
title: The owner's bounded read accepts an implemented realization only where the projection does
relations:
- derived_from: review-result:wave-20261105b-u2-security
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: crates/connectors-host/src/local/owner/governed/bounded.rs
- confidence: inferred
  path: crates/connectors-host/src/local/owner/governed/tests/bounded_reads.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T08:31:13Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-06T08:31:14Z", actor: "human:timo", revision: 3}
---

## Why

The owner's bounded read accepts `realization: implemented` for any operation profile (`crates/connectors-host/src/local/owner/governed/bounded.rs:58`). The MCP projection accepts it only for the `resource` profile and requires inline schemas (`apps/connectors/src/mcp/projection.rs:194`, `:207-208`). A same-uid client sending a `BoundedRead` frame directly reaches that difference. The security review of wave 2026-11-05b raised it (review-result:wave-20261105b-u2-security F3), inferred from code.

## Outcome

The owner's bounded read accepts `realization: implemented` only under the profile and schema conditions the projection requires, and refuses every other combination as `unsupported` before any provider request. A case drives the owner directly with an implemented realization on a non-`resource` profile and asserts a request count of 0.

## Notes

Pre-existing finding, filed as a draft by the 2026-11-05b wave. No wave takes it before every planned wave is delivered. It is not verified at runtime yet.

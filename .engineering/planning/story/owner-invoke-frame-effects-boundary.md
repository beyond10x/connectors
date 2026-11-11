---
format: aep.planning-md/3
id: story:owner-invoke-frame-effects-boundary
kind: story
status: implemented
title: Decide whether the owner's plain Invoke frame holds the effects rule
relations:
- derived_from: review-result:wave-20261105b-u2-adversary
- derived_from: review-result:wave-20261105b-u2-security
- decomposes: epic:zero-open-issues-and-store-defects
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: crates/connectors-host/src/local/owner.rs
- confidence: inferred
  path: crates/connectors-host/src/local/owner/transport.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:59:32Z", actor: "human:timo", revision: 2, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:59:33Z", actor: "human:timo", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "active", to: "implemented", at: "2026-11-11T19:24:47Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---

## Why

The owner's bounded read and, after wave 2026-11-05b, its projected governed read refuse an operation that declares semantic effects or an effect other than `network`. A plain `Invoke` frame from a same-uid client still runs any granted operation without reading its metadata (`crates/connectors-host/src/local/owner/transport.rs:2666-2697`, `crates/connectors-host/src/local/owner.rs:965-972`). Two reviewers of that wave raised this (review-result:wave-20261105b-u2-adversary F2, review-result:wave-20261105b-u2-security F1). The boundary there is the host configuration's grant, which may be intended.

## Outcome

Decide and record whether the plain `Invoke` path is a boundary against a same-uid client. If it is, the owner refuses there what the bounded read refuses, before any provider request. If it is not, the owner's documentation says that the grant alone governs `Invoke`, and that the effects rule binds only the bounded and projected read paths.

## Notes

Pre-existing finding, filed as a draft by the 2026-11-05b wave. No wave takes it before every planned wave is delivered.

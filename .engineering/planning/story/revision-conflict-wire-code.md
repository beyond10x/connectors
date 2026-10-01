---
format: aep.planning-md/3
id: story:revision-conflict-wire-code
kind: story
status: implemented
title: A revision conflict has its own CLI error code
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: apps/connectors-cli-contract/binding.json
- confidence: cited
  path: apps/connectors/src/local.rs
- confidence: cited
  path: apps/connectors/src/local/approval_keys.rs
- confidence: cited
  path: apps/connectors/src/local/connections.rs
- confidence: cited
  path: apps/connectors/tests/revision_conflict.rs
- confidence: cited
  path: apps/connectors/tests/revision_conflict_adversary.rs
- confidence: cited
  path: contracts/cli/v1alpha1/semantics.md
- confidence: cited
  path: crates/connectors-host/src/local/approval_keys.rs
- confidence: cited
  path: crates/connectors-host/src/local/approval_policy.rs
- confidence: cited
  path: crates/connectors-host/src/local/metadata/entity-runtime-definitions.json
- confidence: cited
  path: crates/connectors-host/src/local/owner.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/approval_issuance.rs
- confidence: cited
  path: ess/domains/cli.yaml
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-01T18:10:57Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-01T18:10:57Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-10-01T19:23:38Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":2,"verification":1}}}
---
## Problem
The CLI maps both `Failure::MetadataUnavailable` and `Failure::ConcurrentRevision` to `metadata_unavailable` / `retry_status` (`apps/connectors/src/local.rs:151`, `apps/connectors/src/local/connections.rs:138`), so a revision conflict cannot be told apart from an unreadable store. Raised by the knowledge-ingest consumer on 2026-09-29.

## Acceptance
- A revision conflict has its own wire code and next action.
- A CLI test holds both codes.

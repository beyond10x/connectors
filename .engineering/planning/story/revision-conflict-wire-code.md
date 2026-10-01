---
format: aep.planning-md/3
id: story:revision-conflict-wire-code
kind: story
status: active
title: A revision conflict has its own CLI error code
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: apps/connectors-cli-contract
- confidence: cited
  path: apps/connectors/src/local.rs
- confidence: cited
  path: apps/connectors/src/local/connections.rs
- confidence: inferred
  path: apps/connectors/tests/local_cli.rs
- confidence: inferred
  path: contracts/cli/v1alpha1/semantics.md
- confidence: inferred
  path: crates/connectors-host/src/local/owner.rs
- confidence: inferred
  path: ess/domains/cli.yaml
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-01T18:10:57Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-01T18:10:57Z", actor: "human:timo", revision: 5}
---
## Problem
The CLI maps both `Failure::MetadataUnavailable` and `Failure::ConcurrentRevision` to `metadata_unavailable` / `retry_status` (`apps/connectors/src/local.rs:151`, `apps/connectors/src/local/connections.rs:138`), so a revision conflict cannot be told apart from an unreadable store. Raised by the knowledge-ingest consumer on 2026-09-29.

## Acceptance
- A revision conflict has its own wire code and next action.
- A CLI test holds both codes.

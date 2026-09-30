---
format: aep.planning-md/3
id: story:owner-idle-exit
kind: story
status: implemented
title: An owner with no work exits
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: apps/connectors/tests/local_cli.rs
- confidence: cited
  path: apps/connectors/tests/owner_idle_exit_adversary.rs
- confidence: cited
  path: contracts/cli/v1alpha1/owner.md
- confidence: cited
  path: crates/connectors-host/src/local/owner/maintenance.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/supervisor.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/transport.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T21:31:57Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-09-29T21:31:57Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-09-30T00:49:51Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Problem
Owners never exit when idle: on 2026-09-29 owners from 2026-09-12..14 sandbox runs had run 14-16 days.

## Acceptance
- An owner with no client and no child work for a bounded idle period exits cleanly; the next command starts a new one.
- A test observes the exit and the restart.

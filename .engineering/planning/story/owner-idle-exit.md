---
format: aep.planning-md/3
id: story:owner-idle-exit
kind: story
status: active
title: An owner with no work exits
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: contracts/cli/v1alpha1/owner.md
- confidence: inferred
  path: crates/connectors-host/src/local/owner/maintenance.rs
- confidence: inferred
  path: crates/connectors-host/src/local/owner/supervisor.rs
- confidence: inferred
  path: crates/connectors-host/src/local/owner/transport.rs
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T21:31:57Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-09-29T21:31:57Z", actor: "human:timo", revision: 4}
---
## Problem
Owners never exit when idle: on 2026-09-29 owners from 2026-09-12..14 sandbox runs had run 14-16 days.

## Acceptance
- An owner with no client and no child work for a bounded idle period exits cleanly; the next command starts a new one.
- A test observes the exit and the restart.

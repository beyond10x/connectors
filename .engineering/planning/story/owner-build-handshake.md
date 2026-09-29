---
format: aep.planning-md/3
id: story:owner-build-handshake
kind: story
status: active
title: The CLI refuses or replaces an owner running a different build
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: apps/connectors/src/local.rs
- confidence: inferred
  path: apps/connectors/tests/local_cli.rs
- confidence: inferred
  path: contracts/cli/v1alpha1/fixtures/values.json
- confidence: inferred
  path: contracts/cli/v1alpha1/owner.md
- confidence: inferred
  path: contracts/cli/v1alpha1/semantics.md
- confidence: inferred
  path: crates/connectors-host/src/local/owner.rs
- confidence: inferred
  path: crates/connectors-host/src/local/owner/lifecycle.rs
- confidence: inferred
  path: crates/connectors-host/src/local/owner/transport.rs
- confidence: inferred
  path: crates/connectors-host/tests/local_foundation.rs
- confidence: inferred
  path: ess/domains/cli.yaml
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T11:07:12Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-09-29T11:07:12Z", actor: "human:timo", revision: 4}
---
## Problem
After upgrading `connectors`, the running `__connectors-owner` keeps serving with the old build. Observed 2026-09-29: an owner started 07:43:58 from a 0.15.0 binary (digest dca05368…) served every command after 0.15.1 (60fcf27f…) was installed at 08:52, so the 0.15.1 observation fix never ran.

## Acceptance
- A CLI whose executable digest differs from the running owner's refuses with a named code and next action, or stops that owner and starts one from its own build.
- A test starts an owner from one build and calls it from another.

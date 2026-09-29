---
format: aep.planning-md/3
id: story:owner-build-handshake
kind: story
status: implemented
title: The CLI refuses or replaces an owner running a different build
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: apps/connectors-cli-contract/binding.json
- confidence: cited
  path: apps/connectors/src/local.rs
- confidence: cited
  path: apps/connectors/tests/local_cli.rs
- confidence: cited
  path: apps/connectors/tests/owner_build_security.rs
- confidence: cited
  path: contracts/cli/v1alpha1/fixtures/values.json
- confidence: cited
  path: contracts/cli/v1alpha1/owner.md
- confidence: cited
  path: contracts/cli/v1alpha1/semantics.md
- confidence: cited
  path: crates/connectors-host/src/local/metadata/entity-runtime-definitions.json
- confidence: cited
  path: crates/connectors-host/src/local/owner.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/transport.rs
- confidence: cited
  path: ess/domains/cli.yaml
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T11:07:12Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-09-29T11:07:12Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-09-29T12:47:23Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":2,"review_outcome":4}}}
---
## Problem
After upgrading `connectors`, the running `__connectors-owner` keeps serving with the old build. Observed 2026-09-29: an owner started 07:43:58 from a 0.15.0 binary (digest dca05368…) served every command after 0.15.1 (60fcf27f…) was installed at 08:52, so the 0.15.1 observation fix never ran.

## Acceptance
- A CLI whose executable digest differs from the running owner's refuses with a named code and next action, or stops that owner and starts one from its own build.
- A test starts an owner from one build and calls it from another.

---
format: aep.planning-md/3
id: story:configuration-change-error
kind: story
status: draft
title: A connect under a changed native configuration names the cause
scope:
- confidence: inferred
  path: apps/connectors-cli-contract/binding.json
- confidence: inferred
  path: apps/connectors/src/local.rs
- confidence: inferred
  path: apps/connectors/src/local/connections.rs
- confidence: inferred
  path: contracts/cli/v1alpha1/scenarios.md
- confidence: inferred
  path: contracts/cli/v1alpha1/semantics.md
- confidence: inferred
  path: crates/connectors-host/src/local/owner.rs
- confidence: inferred
  path: crates/connectors-host/src/local/registry.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/lifecycle.rs
- confidence: inferred
  path: crates/connectors-host/src/local/registry/tests.rs
- confidence: cited
  path: docs/local-catalog-provider.md
- confidence: inferred
  path: ess/domains/cli.yaml
revision: 3
---
## Problem
`register()` binds an instance id to one configuration revision (`crates/connectors-host/src/local/registry/lifecycle.rs:287-290`). A connect after any native-config change answers `lifecycle_conflict` / `retry_status`, which invites retries that cannot succeed.

## Acceptance
- The refusal is a distinct code (for example `configuration_changed`) whose next action says to use a new instance id.
- Docs (docs/local-catalog-provider.md) say that a native-config change needs a new instance id.

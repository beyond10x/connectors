---
format: aep.planning-md/3
id: story:upgrade-adapter-identity-mismatch-names-new-connection
kind: story
status: implemented
title: An adapter-reported identity mismatch during an upgrade names a new connection
relations:
- decomposes: epic:connector-probe-20261006
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: apps/connectors/src/local.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/supervisor.rs
- confidence: inferred
  path: crates/connectors-host/src/local/registry/revalidation.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/upgrade_tests.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-06T10:50:42Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1,"verification":1}}}
- {from: "proposed", to: "active", at: "2026-10-06T10:50:43Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"verification":1}}}
- {from: "active", to: "implemented", at: "2026-10-06T10:50:43Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Defect

`story:connection-follows-configuration-upgrade` (PR #108, v0.30.0) answers an adapter-reported
identity mismatch during an upgrade with `next_action: create_connection`: the owner sets
`reconnect` when the upgrade's validation fails (`crates/connectors-host/src/local/owner/supervisor.rs:763-784`,
`error.reconnect = upgrade && reason.is_some()`) and the CLI maps it (`apps/connectors/src/local.rs:132-184`).
No test drives that failure through the supervisor to the CLI answer:
`apps/connectors/src/local.rs:622-642` sets `reconnect` by hand and
`crates/connectors-host/src/local/registry/upgrade_tests.rs:289` covers only the registry. The
implementor's report for PR #108 named this path as uncovered; it was fixed in its last revision
without a test that exercises it end to end.

## Acceptance

- A test sends an adapter `IdentityMismatch` from the validation of an upgrading revalidate through
  the supervisor and asserts the CLI-level answer `identity_mismatch` with `next_action:
  create_connection`, and that nothing is published or invalidated.
- The same failure outside an upgrade answers `repair_connection`, pinned by the same test file.

## Scope

Derived 2026-10-06 by `aep:story-scoper` at connectors `80bee2f`. **Cited** = read from the story
or the tree; **inferred** = a reading that could be wrong.

- **Files:** `crates/connectors-host/src/local/owner/supervisor.rs:763-784`,
  `apps/connectors/src/local.rs:132-184` and its tests at `:590-642`,
  `crates/connectors-host/src/local/registry/upgrade_tests.rs` (`:265`, `:289`) — cited
- **Also likely:** `crates/connectors-host/src/local/registry/revalidation.rs:278-301`
  (`finish_revalidation`), reference only — inferred
- **Confidence:** high
- **Would collide with:** units changing the CLI's next-action table in `apps/connectors/src/local.rs`
  (`story:expired-evidence-invoke-advises-revalidate`, `story:service-failure-carries-upstream-reason`)
  or the supervisor's revalidate task
- **Safety fact:** outside an upgrade `reconnect` stays false (`supervisor.rs:743,782`); during an
  upgrade nothing is invalidated (`revalidation.rs:299`) — traced, unproven

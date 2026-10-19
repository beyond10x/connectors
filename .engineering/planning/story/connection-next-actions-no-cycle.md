---
format: aep.planning-md/3
id: story:connection-next-actions-no-cycle
kind: story
status: draft
title: Every next action a connection command answers is accepted by the state it leads to
summary: 'issue #181: revalidate, reconnect and connect answer each other''s next action in a loop'
refs:
- provider: github
  reference: beyond10x/connectors#181
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:confluence-token-kind-binding
scope:
- confidence: inferred
  path: apps/connectors/src/local.rs
- confidence: cited
  path: apps/connectors/src/local/connections.rs
- confidence: inferred
  path: apps/connectors/tests/connection_next_actions.rs
- confidence: inferred
  path: contracts/cli/v1alpha1/semantics.md
- confidence: inferred
  path: crates/connectors-host/src/local/owner.rs
- confidence: inferred
  path: crates/connectors-host/src/local/registry/lifecycle.rs
- confidence: inferred
  path: crates/connectors-host/src/local/registry/observation.rs
- confidence: inferred
  path: crates/connectors-host/src/local/registry/revalidation.rs
- confidence: inferred
  path: ess/domains/cli.yaml
revision: 4
---
## Outcome

Every next action a connection command answers (`revalidate`, `reconnect`, `connect`) is one the state
it leads to accepts: following the answers ends in a usable connection or in a refusal that names the
failed probe, never in a cycle.

## Why now

https://github.com/beyond10x/connectors/issues/181 (seen on 0.53.1): on `atlassian.basic`, revalidate
answers `create_connection`, reconnect answers `create_connection`, connect answers
`repair_connection`, and the caller loops. The Confluence half (v2 probe 401 "scope does not match",
v1 200) is `story:confluence-token-kind-binding`; the loop itself has no story. Code:
`apps/connectors/src/local/connections.rs:318-335`. Whether 0.54.0 still loops is not known.

## Acceptance

- From fixtures: an Atlassian site whose v2 probe answers 401 "scope does not match" and v1 answers
  200. Starting from a pending connection, following each command's `next_action` reaches a
  connection or a refusal naming the probe within three commands; a test asserts no answer repeats.
- A Jira connection on the same profile reports why it is pending (the issue saw it pending with no
  reason).

## Scope

Derived 2026-10-19 by `story-scoper` on origin/main 0c6cbba7f. Every line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** `apps/connectors/src/local` and `crates/connectors-host/src/local/registry`, where each connection command's failure becomes a `next_action` — cited (story names `connections.rs:318-335`)
- **Files:** `apps/connectors/src/local/connections.rs` (`registry_failure`, 294-357: `BindingChanged` → `create_connection` at 318-322, `InstanceConnected` → `repair_connection` at 330-335) — cited
- **Also likely:** `apps/connectors/src/local.rs` (`connect_failure` 218-227, `owner_failure` 229-300: the `reconnect` flag sets `create_connection` and the `held` flag sets `repair_connection`, at 287-297) — inferred
- **Also likely:** `crates/connectors-host/src/local/registry/revalidation.rs` (`admitted` 51-86 and `changed` 98-113 produce `BindingChanged`) — inferred
- **Also likely:** `crates/connectors-host/src/local/registry/lifecycle.rs` (`admit_allocation` 584-616: `InstanceConnected` when `follow` is Same/Operations/Executable) — inferred
- **Also likely:** `crates/connectors-host/src/local/owner.rs` (`reconnect`/`held` flags 116-130, `saved_binding` 649-660: `Unsupported` → `BindingChanged`) — inferred
- **Also likely (pending reason):** `crates/connectors-host/src/local/registry/observation.rs` (`ObservedConnection` 14, `observe` 299-307, `readiness` 340-370) — inferred
- **Also likely (pending reason):** `ess/domains/cli.yaml` (`ConnectionSummary`/`ConnectionDescription` 838-858 have no reason field) — inferred
- **Documents:** `contracts/cli/v1alpha1/semantics.md` (~1248, 1310-1325) — inferred
- **Tests:** a new `apps/connectors/tests/connection_next_actions.rs` plus registry unit tests beside `crates/connectors-host/src/local/registry/cardinality_tests.rs` and `upgrade_tests.rs` — inferred; fixture pattern `apps/connectors/tests/failed_connect.rs:383-437`
- **Confidence:** medium — the loop's cause is spread over three producers picked from the code, not from a trace; whether 0.54.0 still loops is unknown
- **Would collide with:** registry `revalidation.rs`/`lifecycle.rs`/`observation.rs`, the CLI's `local.rs`/`local/connections.rs`, the `ConnectionSummary` schema in `ess/domains/cli.yaml`, and `story:confluence-token-kind-binding` (adds a reason beside `insufficient_scope`)
- **Safety fact:** the three `next_action`s come from flags every connection command and `operations invoke` read through `owner_failure`, so changing one mapping changes reads, approvals and consumer launches too (`apps/connectors/src/local.rs:283-300`) — unproven

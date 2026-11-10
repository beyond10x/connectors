---
format: aep.planning-md/3
id: story:connection-next-actions-no-cycle
kind: story
status: active
title: Every next action a connection command answers is accepted by the state it leads to
summary: 'issue #181: revalidate, reconnect and connect answer each other''s next action in a loop'
refs:
- provider: github
  reference: beyond10x/connectors#181
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:confluence-token-kind-binding
- decomposes: epic:zero-open-issues-and-store-defects
scope:
- confidence: cited
  path: apps/connectors-cli-contract/binding.json
- confidence: cited
  path: apps/connectors/src/local.rs
- confidence: cited
  path: apps/connectors/src/local/connections.rs
- confidence: cited
  path: apps/connectors/src/local/invoke.rs
- confidence: cited
  path: apps/connectors/tests/connection_next_actions.rs
- confidence: cited
  path: apps/connectors/tests/failed_connect.rs
- confidence: cited
  path: contracts/cli/v1alpha1/semantics.md
- confidence: cited
  path: crates/connectors-build/src/ignored.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/supervisor.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/observation.rs
- confidence: cited
  path: ess/domains/cli.yaml
revision: 21
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:57:56Z", actor: "human:timo", revision: 5, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:57:56Z", actor: "human:timo", revision: 6, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
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

Landed on, from `git diff 5087593c1...impl/connection-next-actions-no-cycle` (merged into wave/zero-issues at 39078c638); the earlier scoper lines marked inferred are replaced by this list. Generated reference pages are left out.

- `apps/connectors-cli-contract/binding.json`
- `apps/connectors/src/local/connections.rs`
- `apps/connectors/src/local/invoke.rs`
- `apps/connectors/src/local.rs`
- `apps/connectors/tests/connection_next_actions.rs`
- `apps/connectors/tests/failed_connect.rs`
- `contracts/cli/v1alpha1/semantics.md`
- `crates/connectors-build/src/ignored.rs`
- `crates/connectors-host/src/local/owner/supervisor.rs`
- `crates/connectors-host/src/local/registry/observation.rs`
- `crates/connectors-host/src/local/registry.rs`
- `ess/domains/cli.yaml`

Decision (coordinator, wave A, non-interactive): the Acceptance's "within three commands" is read as at most three followed `next_action`s after the starting command. The one-connection rule makes a refused upgrade name the revocation first (`revoke_connection`), then `create_connection`, then the connect; `apps/connectors/tests/connection_next_actions.rs` walks from revalidate, reconnect and connect and asserts no answer repeats and the bound holds. A pending connection reports `pending_reason` (`configuration_changed`, `configuration_refused`, `no_credential`, `evidence_expired`).

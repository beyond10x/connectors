---
format: aep.planning-md/3
id: story:connect-refused-while-instance-holds-connection
kind: story
status: implemented
title: Connect is refused while an instance holds a Live or Stale connection, and --connection becomes optional
relations:
- decomposes: epic:connector-vocabulary
- informed_by: decision-blocker:instance-connection-cardinality
- depends_on: story:ess-connector-vocabulary-model
- depends_on: story:cli-connector-instance-selectors
- depends_on: story:connections-reconnect-verb
- depends_on: story:connection-stale-state
- depends_on: story:setup-migrate-adapter-entries
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: apps/connectors-cli-contract
- confidence: inferred
  path: apps/connectors/spec/cli.yaml
- confidence: inferred
  path: apps/connectors/src/local.rs
- confidence: inferred
  path: apps/connectors/src/local/approvals.rs
- confidence: inferred
  path: apps/connectors/src/local/connections.rs
- confidence: inferred
  path: apps/connectors/src/local/operations.rs
- confidence: inferred
  path: apps/connectors/tests
- confidence: inferred
  path: contracts/cli/v1alpha1/semantics.md
- confidence: inferred
  path: crates/connectors-host/src/local/registry/lifecycle.rs
- confidence: inferred
  path: crates/connectors-host/src/local/registry/tests.rs
- confidence: inferred
  path: ess/domains/cli.yaml
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T16:49:14Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-16T16:49:15Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-10-17T01:36:39Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
## Context

`decision-blocker:instance-connection-cardinality` § Decision (operator, 2026-10-16, option 1): at
most one non-revoked connection per instance; revoked connections stay as history. Today
`Registry::begin` allocates a new connection on an instance that already holds live ones, bounded
only by 1000 per `instance_id` (`crates/connectors-host/src/local/registry/lifecycle.rs:20-23`), and
every command that addresses a connection requires `--connection` (`apps/connectors/spec/cli.yaml`:
`approvals prepare|issue`, `connections describe|repair|status|revalidate|launch|revoke`,
`operations invoke`). The ESS guard on `AllocateConnection` is story:ess-connector-vocabulary-model,
decision 11; this story is the host and CLI side.

## Change

- `Registry::begin` refuses an acquisition for an instance that holds a `Live` or `Stale`
  connection. The CLI reports it as `code = lifecycle_conflict`, `stage = admission`,
  `next_action = repair_connection` (the wire name of `connections reconnect` during the window,
  story:connections-reconnect-verb), with `connection` naming the existing connection. Revoked
  connections do not count; the 1000-per-instance bound stays as the bound on revoked history.
- `--connection` becomes optional on every command listed above: absent, the instance's one
  non-revoked connection is selected. An instance with none, or with more than one, refuses an
  absent `--connection` as `invalid_input` at `stage = arguments`, `next_action = none`.
- An instance that already holds more than one non-revoked connection (a migrated alias, decision
  § Decision) keeps all of them: nothing is revoked or rewritten, every one stays addressable with
  `--connection`, and a new connect on it is refused as above until all but one are revoked.
- `ess/domains/cli.yaml`: `connection` becomes `Optional<String>` on the inputs of those commands
  (fields only; the guard itself is the model story's). Regenerate `apps/connectors-cli-contract`.
- `contracts/cli/v1alpha1/semantics.md`: the refusal row and the selector rule.

## Domain relations

- Connection -> instance (ServiceConfiguration), many-to-one, references: `ess/domains/auth_bindings.yaml`,
  entity `connectors.auth_bindings.Connection`, relation `instance` (`via: instance_id`). The bound
  "at most one non-revoked per instance" is the `AllocateConnection` guard of
  story:ess-connector-vocabulary-model (decision 11), decided in
  `decision-blocker:instance-connection-cardinality` § Decision; cited from the model once it
  validates.

## Scope

- `crates/connectors-host/src/local/registry/lifecycle.rs` (`begin`)
- `ess/domains/cli.yaml` (fields), `apps/connectors/spec/cli.yaml`, `apps/connectors-cli-contract/` (generated)
- `apps/connectors/src/local.rs`, `apps/connectors/src/local/{connections,operations,approvals}.rs`
- `contracts/cli/v1alpha1/semantics.md`
- `apps/connectors/tests/` (journey), `crates/connectors-host/src/local/registry/tests.rs`

## Acceptance

A CLI journey test passes only if, on a fixture holding instance A with one connection and a
migrated alias B with two non-revoked connections, `connections connect` is refused with
`lifecycle_conflict` at `admission` and `next_action: repair_connection` on A while its connection
is `Live`, again once it is `Stale`, and on B; `connections describe` on A without `--connection`
returns A's connection; and `connections list` on B still returns both connection ids.

## Notes

- Ordering on shared files: after story:cli-connector-instance-selectors, story:connections-reconnect-verb,
  story:connection-stale-state (`lifecycle.rs`, `connections.rs`) and story:setup-migrate-adapter-entries
  (`cli.yaml`, `local.rs`, `semantics.md`), all by `depends_on`.
- story:cli-text-without-adapter also edits `apps/connectors/spec/cli.yaml` and has no edge to this
  story; that edge (`cli-text-without-adapter depends_on` this story) is for the coordinator to add.
- The docs stories describe `--connection` as it was before this story unless ordered after it.

## Coordinator additions

## Coordinator additions

From the adversary of story:ess-connector-vocabulary-model (2026-10-16):

- A red host test already exists for this story: a second `Registry::begin` on an instance whose
  first connection is Live is admitted today. It was set aside from the ESS unit and is handed to
  this story's implementor as its first failing case (`instance_connection_cardinality_adversary.rs`).
- The model refuses allocation only through the owner-checked input `held_connection_ref`; the host
  never sends it (`crates/connectors-host/src/local/metadata/er.rs:4389`). This story supplies it,
  counting only published Live or Stale connections (a failed connect leaves an unpublished row the
  host cannot revoke, `lifecycle.rs:294-318`, `:378`).

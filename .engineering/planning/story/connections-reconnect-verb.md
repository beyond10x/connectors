---
format: aep.planning-md/3
id: story:connections-reconnect-verb
kind: story
status: implemented
title: connections reconnect replaces connections repair, and instances replaces the adapters command group
relations:
- decomposes: epic:connector-vocabulary
- depends_on: story:cli-connector-instance-selectors
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: apps/connectors-cli-contract
- confidence: inferred
  path: apps/connectors/spec/cli.yaml
- confidence: inferred
  path: apps/connectors/src/local.rs
- confidence: inferred
  path: apps/connectors/src/local/connections.rs
- confidence: inferred
  path: apps/connectors/tests
- confidence: inferred
  path: contracts/cli/v1alpha1/semantics.md
- confidence: inferred
  path: crates/connectors-conformance/tests/cli_surface.rs
- confidence: inferred
  path: ess/domains/cli.yaml
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T15:02:09Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-16T15:02:09Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-10-17T01:36:40Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Context

Step 5 of `migration-plan:connector-vocabulary`, second half, and ADR decision 8. Two command paths
carry the old vocabulary: `connections repair` (`apps/connectors/spec/cli.yaml:118`, types
`connectors.cli.ConnectionRepair*` in `ess/domains/cli.yaml:895-929`) and the `adapters` group
(`adapters list|describe|status|stop`, `apps/connectors/spec/cli.yaml:306-334`).

## Change

- `connections reconnect` replaces `connections repair`; types renamed `ConnectionReconnect*` in
  `ess/domains/cli.yaml`. `connections repair` stays as a path alias (ess-cli/1 command `aliases`)
  and reports its deprecation as the selectors story does: one line on standard error under
  `--output human`, one `deprecations` entry in the JSON result or error envelope under
  `--output json`, with nothing extra on standard error.
- The `adapters` group becomes `instances` (`instances list|describe|status|stop`): the user-facing
  thing these commands address is an instance; the process they observe stays internally an
  adapter (ADR decision 6). `adapters …` stays as an alias with the same deprecation report. This
  naming is this story's call, not the design's; it is reversible until the window closes.
- The wire value `next_action: repair_connection` is kept in this story: it is machine-facing, like
  the JSON `adapter` field, and changes with the removal release (migration plan step 9). Human text
  that names the remedy says `connections reconnect`.
- Regenerate `apps/connectors-cli-contract`; update `contracts/cli/v1alpha1/semantics.md` (table at
  line 89 and the repair prose).

## Domain relations

- Reconnect publishes a new credential on an existing connection, the `publish` transition of
  `connectors.auth_bindings.Connection` (`ess/domains/auth_bindings.yaml`, lifecycle
  `transitions`). No relation changes.

## Scope

- `ess/domains/cli.yaml`
- `apps/connectors/spec/cli.yaml`
- `apps/connectors-cli-contract/` (generated)
- `apps/connectors/src/local.rs`, `apps/connectors/src/local/connections.rs`
- `contracts/cli/v1alpha1/semantics.md`
- `apps/connectors/tests/`, `crates/connectors-conformance/tests/cli_surface.rs`

## Acceptance

A conformance test runs each renamed path (`connections repair` -> `connections reconnect`,
`adapters list|describe|status|stop` -> `instances list|describe|status|stop`) with the same
arguments against one instance under its old and its new name with `--output json`, and finds JSON
results that differ only by one `deprecations` entry for the old name and none for the new one,
with standard error empty on success for both.

## Notes

- `connections repair` already publishes a new credential (its behaviour is unchanged), so equal
  results under both names show that `reconnect` does.
- story:setup-migrate-adapter-entries edits the same CLI files after this story (`depends_on`).

---
format: aep.planning-md/3
id: story:cli-connector-instance-selectors
kind: story
status: implemented
title: Every command selects its target with --connector and --instance; --adapter still works, deprecated
relations:
- decomposes: epic:connector-vocabulary
- depends_on: story:connector-config-dual-read
- informed_by: migration-plan:connector-vocabulary
- depends_on: story:ess-connector-vocabulary-model
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: apps/connectors-cli-contract
- confidence: inferred
  path: apps/connectors/spec/cli.yaml
- confidence: inferred
  path: apps/connectors/src/local
- confidence: inferred
  path: apps/connectors/src/local.rs
- confidence: inferred
  path: apps/connectors/tests
- confidence: inferred
  path: contracts/cli/v1alpha1/semantics.md
- confidence: inferred
  path: crates/connectors-conformance/tests/cli_surface.rs
- confidence: inferred
  path: crates/connectors-host/src/local/config.rs
- confidence: inferred
  path: ess/domains/cli.yaml
revision: 14
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T14:07:00Z", actor: "human:timo", revision: 12, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-10-16T14:07:00Z", actor: "human:timo", revision: 13, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-10-17T01:36:38Z", actor: "human:timo", revision: 14, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
## Context

Step 5 of `migration-plan:connector-vocabulary`, first half. Every grouped command selects its
target with `--adapter <alias>` (`apps/connectors/spec/cli.yaml`, 25 option sources; `ess/domains/cli.yaml`,
`adapter` on every input and on most results), resolved in one place
(`apps/connectors/src/local.rs:494`). The ADR replaces it with `--connector` and `--instance`.

The ESS change (`ess/domains/cli.yaml`) is part of this story, not a story of its own: the binding
in `apps/connectors/spec/cli.yaml` sources every field of those types, and
`cargo run --locked -p connectors-build -- cli --check` refuses a generated contract that is out of
date with them, so the types, the binding, the generated package and the handlers only stay green
together.

## Change

- `ess/domains/cli.yaml`: each input carrying `adapter` gains `connector` and `instance`; `adapter`
  becomes optional with the runtime obligation "exactly one of `adapter` or `connector`+`instance`"
  (stated as a comment, as `ConnectionStatusInput` does today). Each result carrying `adapter` gains
  `connector` and `instance` beside it, as required `String` fields. The header vocabulary comment is
  story:ess-connector-vocabulary-model's; this story changes fields only.
- `apps/connectors/spec/cli.yaml`: `--connector` and `--instance` on every command that has
  `--adapter`; `--adapter` stays. Under `--output human` it prints exactly one deprecation line on
  standard error. Under `--output json` it writes nothing extra to standard error: the result, and
  the error envelope on failure, gain a `deprecations` array with one entry naming `--adapter` and
  its replacement. A consumer that parses all of standard error as one JSON error envelope (the
  llm-gateway `llm-runpod` transport does, `crates/llm-runpod/src/transport/connectors.rs`) keeps
  reading its error codes.
- Regenerate `apps/connectors-cli-contract` (`cargo run --locked -p connectors-build -- cli`).
- `apps/connectors/src/local.rs` and `apps/connectors/src/local/*.rs`: resolve either selector to
  one instance.
- `crates/connectors-host/src/local/config.rs`: `default_instance = "<connector>/<instance>"`, the
  counterpart of `default_adapter`, applies when no selector is given; a file setting both is
  refused as an invalid configuration.
- JSON `adapter` carries the instance's legacy alias where it has one, otherwise
  `<connector>/<instance>`, for the deprecation window (length: UNMAPPED in the migration plan,
  decided in the release plan).
- `contracts/cli/v1alpha1/semantics.md`: selector rules.

## Domain relations

- `ess/domains/cli.yaml` types hold String coordinates and declare no entity relations (the file's
  header comment); this story adds none.
- Instance -> Connector (an instance name is unique within its connector, not globally, so
  `--instance` alone does not select): from story:ess-connector-vocabulary-model
  (`connectors.declarations.ServiceConfiguration` referencing `connectors.declarations.Connector`);
  cited from that model once it validates.

## Scope

- `ess/domains/cli.yaml` (fields)
- `apps/connectors/spec/cli.yaml`
- `apps/connectors-cli-contract/` (generated)
- `apps/connectors/src/local.rs`, `apps/connectors/src/local/{connections,operations,approvals,approval_keys,session,cursor}.rs`
- `crates/connectors-host/src/local/config.rs` (`default_instance`; after story:connector-config-dual-read)
- `contracts/cli/v1alpha1/semantics.md`
- `apps/connectors/tests/`, `crates/connectors-conformance/tests/cli_surface.rs`

## Acceptance

A conformance test runs every command path in `apps/connectors/spec/cli.yaml` that takes
`--adapter` twice against one instance, once with `--connector <c> --instance <i>` and once with
`--adapter <its legacy alias>`, validates every JSON result of both runs against the regenerated
result schema of `apps/connectors-cli-contract` (in which `adapter`, `connector` and `instance` are
required on every result type that carries a selector), and passes only when every result
validates and the two runs differ only by one `deprecations` entry in the second run's JSON, with
the second run's standard error empty on success and exactly one JSON envelope on failure.

## Notes

- Callers in other repositories (`rg -l -F -- --adapter`: 50 files in 9 other repositories,
  `migration-plan:connector-vocabulary` § What persists) move inside the window; that is not a
  story of this repository.
- `--connection` stays as it is; see `decision-blocker:instance-connection-cardinality`.

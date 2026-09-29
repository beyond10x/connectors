---
format: aep.planning-md/3
id: story:configuration-refusal-names-the-entry
kind: story
status: implemented
title: An invalid configuration refusal names the format and the entry
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: apps/connectors-cli-contract/binding.json
- confidence: cited
  path: apps/connectors/src/local.rs
- confidence: cited
  path: apps/connectors/src/local/session.rs
- confidence: cited
  path: apps/connectors/tests/configuration_refusal_adversary.rs
- confidence: cited
  path: apps/connectors/tests/local_cli.rs
- confidence: cited
  path: contracts/cli/v1alpha1/fixtures/values.json
- confidence: cited
  path: contracts/cli/v1alpha1/scenarios.md
- confidence: cited
  path: contracts/cli/v1alpha1/semantics.md
- confidence: cited
  path: crates/connectors-conformance/tests/cli_surface.rs
- confidence: cited
  path: crates/connectors-conformance/tests/configuration_refusal_adversary.rs
- confidence: cited
  path: crates/connectors-host/src/local/config.rs
- confidence: cited
  path: crates/connectors-host/src/local/metadata/entity-runtime-definitions.json
- confidence: cited
  path: crates/connectors-host/tests/configuration_refusal_adversary.rs
- confidence: cited
  path: crates/connectors-host/tests/local_foundation.rs
- confidence: cited
  path: ess/domains/cli.yaml
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T16:41:22Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-09-29T16:41:22Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-09-29T20:39:43Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Source

Split from story:setup-init-writes-current-config-format on 2026-09-29. `config.rs:224` refuses a
`connectors-local/1` entry that carries `private_protocol`, and a `/2` entry that lacks it, as
`invalid_configuration` with nothing naming the rule. Validation runs inside serde `try_from` during
`toml::from_str` (`config.rs:42-52,177`); `Failure` is a payload-free enum (`local/mod.rs:26`); the
CLI error envelope admits no free text (`ess/domains/cli.yaml:267`).

## Acceptance

- A `/1` file whose entry carries `private_protocol`, and a `/2` file whose entry lacks it, are
  refused with structured error data naming the format and the entry's instance id, declared in
  `ess/domains/cli.yaml` and the CLI contract.
- No parser, OS or database text reaches the error.

## Decided for the wave (coordinator, 2026-09-29)

- `local::Failure` (`crates/connectors-host/src/local/mod.rs:24`) stays unchanged, payload-free and
  `Copy`. `Config::load` returns the entry's refusal as a separate typed value that the one CLI load
  site (`apps/connectors/src/local.rs:171`) maps into the error data; no other `Failure` match changes.
- Scope is the CLI's configuration load. The owner's own `Config::load` calls (`owner.rs:161`,
  `owner/transport.rs:667`, `owner/maintenance.rs`) keep `invalid_configuration` unchanged.
- The data carries only the format and the entry's instance id (both written by the operator), as
  optional fields of `connectors.cli.Failure` in `ess/domains/cli.yaml`; no parser text.

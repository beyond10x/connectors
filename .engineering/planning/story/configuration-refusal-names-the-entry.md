---
format: aep.planning-md/3
id: story:configuration-refusal-names-the-entry
kind: story
status: draft
title: An invalid configuration refusal names the format and the entry
relations:
- serves: vision:independent-contract-adapters
revision: 1
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

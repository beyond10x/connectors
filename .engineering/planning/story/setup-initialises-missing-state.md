---
format: aep.planning-md/3
id: story:setup-initialises-missing-state
kind: story
status: draft
title: An existing configuration with no metadata store can be initialised
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Source

Split from story:cli-surface-minor-findings-0-18-0 (D6b). With a configuration and no metadata database, reads answer
`metadata_unavailable` / `retry_status`, and `setup init` refuses with `configuration_exists`; no command initialises
state for an existing configuration (contracts/cli/v1alpha1/semantics.md:204-207).

## Acceptance

- The ESS CLI domain (`ess/domains/cli.yaml`, `apps/connectors/spec/cli.yaml`) declares how state is initialised for
  an existing configuration (a new command or `setup init` behaviour) before any code; the generated contract
  regenerates.
- A test starts from a configuration with no state directory and reaches a working `adapters list` and
  `connections list` through the declared command.

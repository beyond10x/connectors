---
format: aep.planning-md/3
id: story:definition-refusal-keeps-deprecations
kind: story
status: draft
title: A refused definition invoke still reports its deprecated options
summary: 'review run 3 C3 F2: the failure envelope drops deprecations'
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

A command refused by the `--definition` preparation still reports the deprecated options it was given:
the failure envelope carries `deprecations`, and `--output human` prints the `deprecated:` line.

## Why later

Post-0.54.0 review run 3 (lane C3 F2, 2026-10-19): `apps/connectors/src/local.rs:34-37` returns the
`definition::prepare` refusal before `select::deprecations` runs at :38. The contract says the one
envelope a command writes, "success or failure, parse refusals included", carries `deprecations`
(`contracts/cli/v1alpha1/semantics.md:441-442`). Scratch test
`apps/connectors/tests/verify_c3_2.rs` (`operations invoke --adapter forge --definition …`) failed at
line 36: `deprecations` was `null`, expected `"--adapter"` (EXIT=101).

## Acceptance

- `local.rs` computes the deprecations before `definition::prepare` and adds them to its refusal.
- The scratch case is a test in `apps/connectors/tests/invoke_definition.rs` for `--output json` and
  `--output human`.

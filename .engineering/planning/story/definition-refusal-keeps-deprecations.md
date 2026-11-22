---
format: aep.planning-md/3
id: story:definition-refusal-keeps-deprecations
kind: story
status: archived
title: A refused definition invoke still reports its deprecated options
summary: 'review run 3 C3 F2: the failure envelope drops deprecations'
relations:
- serves: vision:independent-contract-adapters
- decomposes: epic:zero-open-issues-and-store-defects
scope:
- confidence: inferred
  path: apps/connectors/src/local.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:58:01Z", actor: "human:operator", revision: 2, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:58:01Z", actor: "human:operator", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "active", to: "archived", at: "2026-11-11T19:57:15Z", actor: "human:operator", revision: 6}
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

## Finding, 2026-11-11 (wave/load-and-flakes)

Not built: the defect can no longer occur. The scratch case used `operations invoke --adapter`,
which 0.60.0 removed (CHANGELOG.md, 0.60.0). Only `invoke` takes `--definition`, and `invoke`
takes no `--adapter`; `select::deprecations` returns `--adapter` only for a plan with an `adapter`
argument, and a renamed path only for the aliases in apps/connectors/spec/cli.yaml (`adapters
list|describe|status|stop`, `connections repair`), none of which is `invoke`. So an
`invoke --definition` line has no deprecation to drop. contracts/cli/v1alpha1/semantics.md
already says a command without `--adapter` refuses it as an unknown option.

Measured on the wave base: `operations invoke --adapter forge --definition f.yaml project.get`
and `invoke --adapter forge --definition f.yaml project.get` both answer `cli_parse`, exit 2;
`invoke --definition missing.yaml project.get` answers `invalid_configuration` with no
deprecations. The existing test `a_definition_is_named_by_its_file_and_takes_no_other_selector`
(apps/connectors/tests/invoke_definition.rs) pins this: 1 passed, exit 0.

Archived as superseded by 0.60.0.

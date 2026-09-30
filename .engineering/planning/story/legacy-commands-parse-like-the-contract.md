---
format: aep.planning-md/3
id: story:legacy-commands-parse-like-the-contract
kind: story
status: active
title: Legacy describe, invoke and serve parse errors follow the CLI error envelope
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: apps/connectors/src/legacy.rs
- confidence: cited
  path: apps/connectors/src/main.rs
- confidence: cited
  path: apps/connectors/tests/compatibility.rs
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T13:03:57Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-09-30T13:03:57Z", actor: "human:timo", revision: 7}
---
## Observed
Found by the black-box CLI surface test of release 0.18.0 on 2026-09-30 (raw output under the tester's sandbox, outside the repository).
`connectors describe <word>` prints clap text echoing the argument (exit 2); global `--output json` before
`describe`/`serve` answers `cli_parse`. Contradicts semantics.md:162-165, :174 and scenarios.md:63 (P08: no argv
echo).
## Acceptance
- Parse errors of the legacy commands never echo argv and use the `cli_parse` envelope; `--output json` is accepted
  before them, or the legacy commands are removed with a CHANGELOG line.

## Also here (moved from story:cli-surface-minor-findings-0-18-0, D9)

Top-level `--help` adds an "Explicit service commands" block advertising the legacy commands
(`apps/connectors/src/local.rs:20,32-34`, checked by `apps/connectors/tests/compatibility.rs:42-45`) and differs from
`apps/connectors-cli-contract/help.txt`. Whatever this story decides for the legacy commands decides that block.

## Decided (coordinator, 2026-09-30)

Route, do not remove: `serve --config` is the only entry to the federation host (semantics.md:180-191,
docs/running-services.md:18). `apps/connectors/src/main.rs:34-41` accepts `--output` (the contract has no `-o`) before the legacy command
word, and `apps/connectors/src/legacy.rs:58-101` turns clap errors into the fixed `cli_parse` envelope with no argv
echo. The extra help block (D9) stays, and `help.txt` or the contract text says so.

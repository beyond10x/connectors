---
format: aep.planning-md/3
id: verification-report:connectors-v0782-release-scope
kind: verification-report
status: accepted
title: Scope of connectors 0.78.2
summary: What 0.78.2 holds, what a consumer reads and what is known open at the release
relations:
- serves: vision:independent-contract-adapters
revision: 2
transitions:
- {from: "draft", to: "accepted", at: "2026-11-24T19:57:57Z", actor: "human:timo", revision: 2}
---
## Scope of connectors 0.78.2

Cut on 2026-11-24 on the operator's word of about 21:01 CET, relayed by the host operator at
21:03 CET: the sessions are paused to save tokens, and a session whose `main` is green and holds
merged work that is not in the last release cuts the next release from it. `main` was
`1f1df688f` (pull request 295), one commit after `v0.78.1`, with every check green.

- story:the-forges-gate-job-compiles-once-and-prints-its-time-per-step, change 1: the gate and
  its step of the ignored tests print the wall time of each step and a closing table
  (`crates/connectors-build/src/steps.rs`). The story stays `active`.
- The record of 0.78.1 in the planning store (the upload story `implemented`, the scope report's
  section on what was verified after the release) and the correcting sentence in the changelog
  entry of 0.78.1.
- Patch version: no command, configuration member, answer member or error code is added,
  removed or changed.

## What a consumer reads

- Nothing changes for an installed binary: between `v0.78.1` and this release only
  `crates/connectors-build` (the repository's build program, in no release archive), the
  development guide, the changelog and planning files differ, and the version.
- A developer of the repository reads `gate: step ran <seconds> s: <name>` and the table
  `gate: time per step` in the output of `connectors-build gate`, and the same lines with the
  prefix `ignored:` on standard error of `connectors-build ignored`.

## Known at the release

- Pull request 296 (change 3 of the same story: the gate's `cargo run` calls name the program
  with `--bin`) is open and not in this release. 15 of its 16 checks are green; the job
  "ignored CLI journeys", which is not required, failed in one test of 137,
  `catalog_cli_background_recovery_waits_for_live_work_and_recovers_unkeyed_attempts`, with
  "background did not request time" after a wait of 8 s. The same job passed on pull request
  295 and on `main`. Whether the failure is one of timing is not established.
- The forge's test step is over its budget of 120 s (638.6 s on pull request 295;
  story:the-gates-test-step-ends-within-120-s-on-the-forge). Twelve draft stories for it are
  written and not committed; four critics asked for a revision.
- The gate job of pull request 295 ran 2,401 s. The story of the gate time holds the rows.
- Not changed from 0.78.1: a write refused at commit answers a code without the engine's reason
  (story:a-refused-local-write-names-its-reason, `draft`), and an approved upload through the
  local CLI does not hold the operation's 2 MiB
  (story:an-approved-binary-write-through-the-local-cli-holds-the-operations-2-mib, `draft`).

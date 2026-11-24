---
format: aep.planning-md/3
id: verification-report:connectors-v0782-release-scope
kind: verification-report
status: accepted
title: Scope of connectors 0.78.2
summary: What 0.78.2 holds, what a consumer reads and what is known open at the release
relations:
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "accepted", at: "2026-11-24T19:57:57Z", actor: "human:timo", revision: 2}
---
## Scope of connectors 0.78.2

Cut on 2026-11-24 on the operator's word of about 21:01 CET, relayed by the host operator at
21:03 CET: the sessions are paused to save tokens, and a session whose `main` is green and holds
merged work that is not in the last release cuts the next release from it. `main` was
`1f1df688f` (pull request 295), one commit after `v0.78.1`, with every check green. The release
pull request (297) then went red in one job that is not required; the fix landed first as pull
request 298, and the release is cut from `main` at `8310e9ed6`, two commits after `v0.78.1`.

- story:the-forges-gate-job-compiles-once-and-prints-its-time-per-step, change 1: the gate and
  its step of the ignored tests print the wall time of each step and a closing table
  (`crates/connectors-build/src/steps.rs`). The story stays `active`.
- Pull request 298, tests only: the journey
  `catalog_cli_background_recovery_waits_for_live_work_and_recovers_unkeyed_attempts` waits 13 s
  instead of 8 s for the owner's background to ask the clock, and names the attempts when the
  wait ends; one new test of the pending scan in `crates/connectors-host`.
- The record of 0.78.1 in the planning store (the upload story `implemented`, the scope report's
  section on what was verified after the release) and the correcting sentence in the changelog
  entry of 0.78.1.
- Patch version: no command, configuration member, answer member or error code is added,
  removed or changed.

## What a consumer reads

- Nothing changes for an installed binary: between `v0.78.1` and this release only
  `crates/connectors-build` (the repository's build program, in no release archive), three test
  files, the development guide, the changelog and planning files differ, and the version.
- A developer of the repository reads `gate: step ran <seconds> s: <name>` and the table
  `gate: time per step` in the output of `connectors-build gate`, and the same lines with the
  prefix `ignored:` on standard error of `connectors-build ignored`.

## Known at the release

- The job "ignored CLI journeys", which is not required, failed on pull request 296 and on the
  first head of this release's pull request 297 in one test, the journey named above,
  with "background did not request time" after a wait of 8 s. The same job passed on pull
  request 295 and on `main` at `1f1df688f`. Read from the code: the owner's background runs a
  pass every 5 s and keeps a scan cursor, so a seeded attempt whose random identifier sorts
  before the cursor is returned by the second pass, up to 10 s after the seed. The two failed
  logs do not show which pass saw which attempt, so the cause is not proven from them. The job
  passed on pull request 298. Its result on the second head of pull request 297 is not known
  when this is written.
- Pull request 296 (change 3 of the gate-time story: the gate's `cargo run` calls name the
  program with `--bin`) is open and not in this release. Its gate job ran 2,094 s, against
  2,401 s on pull request 295.
- The forge's test step is over its budget of 120 s (638.6 s on pull request 295;
  story:the-gates-test-step-ends-within-120-s-on-the-forge). Eighteen draft stories for it are
  written on a branch that is not pushed; four critics asked for a revision of the first twelve,
  and the revised set has not been read by them again.
- Not changed from 0.78.1: a write refused at commit answers a code without the engine's reason
  (story:a-refused-local-write-names-its-reason, `draft`), and an approved upload through the
  local CLI does not hold the operation's 2 MiB
  (story:an-approved-binary-write-through-the-local-cli-holds-the-operations-2-mib, `draft`).

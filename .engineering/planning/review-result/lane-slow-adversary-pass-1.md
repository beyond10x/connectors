---
format: aep.planning-md/3
id: review-result:lane-slow-adversary-pass-1
kind: review-result
status: active
title: 'Adversary read of the lane slow change: three red cases, three findings fixed'
relations:
- reviews: story:the-lane-slow-runs-the-ignored-timing-family
revision: 1
---
Verdict: needs-change, for the first commit of the change of
story:the-lane-slow-runs-the-ignored-timing-family and
story:the-100-run-capacity-stress-keeps-10-runs-in-the-gate. Three findings are fixed in the second
commit of the same pull request, one has a draft story, four are not changed and say why.

Adversary read of the committed change. The adversary added three cases to the tests of
`crates/connectors-build/src/ignored.rs`, uncommitted, and ran them with the 36 committed tests of
`ignored::` on the build host of the shared pool: 39 tests run, 36 passed, 3 failed. It edited no
product line. It ran no test of the timing family, not the 100-round test and no
`connectors-build ignored`.

## Findings

1. Warning, introduced. `ignored.rs`, `time_limit()`. The limit went by the family. The table's
   one `Timing` entry left (`catalog_cli_read_recollects_expired_evidence_without_reentry`, needs
   custody and the CLI) waits out 60,000 ms of evidence and had 600 s before the change; with
   60 s it cannot pass. No job runs it; the guide's command for a host with a keyring does.
   Inferred from the sources; the test was not run.
   Fixed: the 60 s go by the name rule. Every entry of the exact table keeps 600 s. The
   adversary's case is a test now.
2. Warning, introduced. `.github/workflows/slow.yml`, both `paths:` lists. The lane's second
   command shows that a release build has no debug assertions; the files that decide it (the root
   `Cargo.toml`, `.cargo/config.toml`, `rust-toolchain.toml`) did not start the lane.
   Fixed: both lists hold the three paths. The adversary's case is a test now.
3. Note, pre-existing. `ignored.rs`, `execute_test()`. A test ended at its limit is killed with
   its process group; a process it started in a session of its own keeps running. The adversary's
   case (a fixture that starts `setsid sleep 4` under a limit of 1 s) was red. No lane test of
   this change reaches it; the jail tests that later stories add will, inferred from the sources.
   Not changed here: story:a-lane-test-ended-at-its-limit-leaves-a-process-of-another-session-running
   (draft). The case is not in the tree.
4. Warning, introduced. `slow.yml`. A failed lane test says "see log", and the lane had no step
   that prints or uploads the logs, as the journeys job has.
   Fixed: two steps with `if: always()`, and a test that holds them.
5. Note, introduced. `apps/connectors/tests/local_cli.rs`. The assertion that the tally sums to
   the number of runs cannot fail; any code other than `owner_build_mismatch` passes (that part
   is from before the change); nothing asserts that `capacity` was seen. The 10 seeded pauses
   are 4 ms to 272 ms: a CLI that needs longer to start makes all 10 rounds `ok`.
   Not changed: an assertion that `capacity` was seen fails on a slow runner for that same
   reason, and the 100 rounds had no such assertion either. The tally line is printed.
6. Note, introduced. The limit test accepts 1 s to 5 s, so a limit applied four times too long
   stays green. Not changed: the story's acceptance 4 says "within 5 s", and a slow runner needs
   the margin above the 2 s of cleanup.
7. Note, introduced. The assertion "the exact entry is matched first" uses a name the rule does
   not match. Not changed: no entry of the table has a `slow_lane_` name, so there is no case to
   assert on.
8. Note, introduced. `.github/workflows/macos-host.yml` runs the tests of `connectors` that are
   not ignored: macOS ran the 100 rounds before and runs 10 now. The 100 rounds run on Linux
   only. Not changed: the lane `slow` has one Linux runner by the story's "Not in this story".

## Attacked and held

- A module named `slow_lane_x` with a test of another name is unknown; `Slow_lane_x` and
  `slow_lane` are unknown; `slow_lane_` alone is timing (read, and the existing test).
- Doc-tests are not in the inventory; the workspace has no bench (read).
- A lane test that needs the CLI gets its path; one that needs custody fails in the lane (read).
- The gate's ignored step and the journeys job select no `slow_lane_` test and count it neither
  unknown, missing nor failed; `--required` holds for the gate as before (read, and the existing
  test).
- A test that ends just under its limit passes; the limit starts at the spawn of the test binary,
  after the build; the group kill cannot reach the runner (read).
- A result line followed by a hang counts as failed (read).
- The one lane test compiles on the lane's runner; the rule reads the function name, so a renamed
  binary keeps it (read).
- The 100 rounds have a floor of 29.5 s of pauses and sleeps and ran 31.9 s on the forge
  (computed, inferred).
- A change under `docs/` or `.engineering/` alone does not start the lane; the two path lists are
  equal (read).
- No `[profile.release]` in `Cargo.toml`; `.cargo/config.toml` sets `debug` only; the release
  test is the only `release_build_` name of `connectors-sdk` (read).
- Mutants by read, none run: of the rule, the paths, the steps, the selection and the limit, the
  committed tests catch all but the ones named in findings 6 and 7, a `branches:` filter under
  `pull_request`, and a job-level `if:` or `continue-on-error`.

```findings
- file: crates/connectors-build/src/ignored.rs
  line: 900
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: time_limit goes by family, so the table's one Timing entry, whose test waits out 60 000 ms of evidence, is ended at 60 s where it had 600 s; the documented command ignored --family timing --required cannot pass on a host with custody
- file: .github/workflows/slow.yml
  line: 5
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: neither paths list holds Cargo.toml, .cargo/** or rust-toolchain.toml, so a change of the release profile or of the toolchain alone never starts the lane whose second command exists to detect it
- file: crates/connectors-build/src/ignored.rs
  line: 768
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: a test ended at its limit leaves a descendant in its own session running; no lane test of this commit reaches it, the jail tests the story names for later will, since the CLI starts the owner with setsid and the jail clears the tether variable
- file: .github/workflows/slow.yml
  line: 78
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a failed lane test reports see log, and the lane has no step that prints or uploads .local/ignored-suites.logs as the journeys job does
- file: apps/connectors/tests/local_cli.rs
  line: 1738
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the tally-sum assertion cannot fail, any code other than owner_build_mismatch passes, and nothing asserts that capacity was observed in the 10 rounds
- file: crates/connectors-build/src/ignored.rs
  line: 3046
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the limit test accepts 1 s to 5 s, so a limit applied four times too long stays green
- file: crates/connectors-build/src/ignored.rs
  line: 2635
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the exact-entry-first assertion uses a name the rule does not match, so a classify that applies the name rule before the table passes
- file: .github/workflows/macos-host.yml
  line: 60
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the capacity stress ran 100 rounds on macOS in this step and runs 10 now; the 100 rounds run on Linux only
```

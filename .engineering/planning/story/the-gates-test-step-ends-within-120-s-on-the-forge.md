---
format: aep.planning-md/3
id: story:the-gates-test-step-ends-within-120-s-on-the-forge
kind: story
status: draft
title: The gate's test step ends within 120 s on the forge, and fails when it does not
relations:
- serves: vision:independent-contract-adapters
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
revision: 3
---
## Defect

The folder rule "Tests stay fast" gives the test step of a repository's gate 120 s of wall time and
says the step itself fails when it runs over that, locally and in CI. The gate prints the time and
does not fail on it (`crates/connectors-build/src/gate.rs`, `TEST_STEP_BUDGET` and the line
`gate: test step ran … s (budget 120 s)`).

Measured on the forge, pull request 291, job 138270416028, 2026-11-24 08:06 to 08:17 CET:

- `Starting 4317 tests across 240 binaries (223 tests skipped)`
- `Summary [ 620.625s] 4317 tests run: 4316 passed (9 slow), 1 failed, 223 skipped`
- `gate: test step ran 621.8 s (budget 120 s)`

The durations nextest printed sum to 2,475 s, 3.99 times the wall time: the runner ran four tests
at a time. A step of 120 s on that runner holds 480 s of tests.

| band | tests | sum |
|---|---|---|
| 20 s and more | 14 | 412 s |
| 5 s and more (holds the row above) | 81 | 1,118 s |
| 1 s to under 5 s | 406 | 814 s |
| under 1 s | 3,831 | 543 s |

The tests of 5 s and more by binary: `connectors-host` (unit tests) 19 tests 265 s,
`connectors::connection_lease_adversary` 6 tests 143 s, `connectors-catalog-provider::main` 9 tests
77 s, `connectors::connection_lease` 4 tests 68 s, `connectors::owner_slow_start_adversary` 3 tests
66 s, `connectors::local_cli` 4 tests 57 s, `connectors::security_mcp_deadline` 3 tests 49 s,
`connectors::owner_slow_start` 4 tests 47 s, `connectors::adversary_local_mcp_server` 4 tests 41 s.

All test time by binary: `connectors-host` 781 tests 616 s, `connectors-catalog-provider::main`
1,122 tests 364 s (median 0.049 s, 90th percentile 0.582 s), `connectors-catalog::main` 438 tests
135 s.

Six of the ten slowest tests ran 30.2 s to 31.9 s. Inferred, not read from their code: they wait
out a production wait of 30 s.

Second measurement, pull request 292, job 138329455972, 2026-11-24 10:30 to 10:44 CET:

- `Starting 4379 tests across 240 binaries (223 tests skipped)`
- `Summary [ 840.388s] 4379 tests run: 4379 passed (9 slow), 223 skipped`
- `gate: test step ran 842.2 s (budget 120 s)`

The two runs differ by 62 tests and 220 s. Why the second run was slower is not known: the
durations of its tests were not compared with the first run's.

## Change

Decomposed on 2026-11-24 in test-plan:where-the-gates-test-step-spends-its-time, from the
durations of the first run and the sources. It names, for each of the 81 tests of 5 s and
more, the line the time is spent at, and groups them by cause. Eighteen draft stories follow
from it: fourteen change tests, two (N1, N2) build what the others use, and two (N3, N4) are
questions. The first draft of twelve was revised once after four critics read it (acceptance,
design, scope, parallel safety; all four asked for a revision).

"Before" and "after" are seconds summed over the tests of 5 s and more that a story covers,
from the first run; every "after" is an estimate and none was timed.

| story | tests | before | after | seam |
|---|---|---|---|---|
| S1 `story:four-host-tests-no-longer-wait-out-the-30-s-lock-wait` | 4 | 122.0 | 5.5 | `child_lock_wait` and `set_lifecycle_lock_wait` exist; the 30 s test goes to the lane `slow`, a library test replaces it in the gate |
| S2 `story:seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait` | 7 | 120.8 | 36.0 | one debug-only override of the 10 s connect wait |
| S3 `story:eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping` | 11 | 222.9 | 48.0 | open until N3 answers; a clock offset in `connectors-sdk` needs the operator's word |
| S4 `story:four-idle-exit-tests-no-longer-span-several-5-s-recovery-sweeps` | 4 | 67.4 | 9.5 | a debug-only sweep interval |
| S5 `story:seven-mcp-tests-use-a-4-s-request-budget-and-shorter-holds` | 7 | 101.8 | 34.0 | the tests' own values |
| S6 `story:five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts` | 5 | 56.2 | 5.4 | two debug-only HTTP overrides; the SQL timeout by a constructor parameter |
| S7a `story:the-catalog-provider-tests-read-a-committed-lowering-of-the-released-definition` | 4 | 35.9 | 4.7 | a committed lowered file with a digest of its sources; it reaches 40 tests in all, about 94.9 s to 9.7 s |
| S7b `story:the-catalog-crate-tests-ingest-the-released-definition-once-per-process` | 1 | 7.4 | 7.4 | reads S7a's file; it reaches one test, and that test's time does not change |
| S8a `story:eight-adapter-and-catalog-tests-drop-waits-that-prove-nothing` | 7 | 73.0 | 16.6 | the tests' own values; one test to the lane `slow` |
| S8b `story:five-host-unit-tests-shorten-the-waits-they-chose` | 5 | 48.6 | 18.2 | the tests' own values |
| S8c `story:seven-cli-tests-shorten-their-waits-or-a-production-wait` | 6 | 54.0 | 24.5 | three debug-only overrides: print deadline, clock query timeout, client drain bound |
| S9a `story:eighteen-tests-take-a-template-store-or-jail-built-once-per-run` | 12 | 115.0 | 55.9 | a template store and generated SQL schemas built once per run by a setup step |
| S9b `story:six-cli-tests-take-jail-binaries-built-once-per-run` | 6 | 50.7 | 50.6 | jail binaries linked from a product built once per run; the plan estimates no saving |
| S9c `story:the-100-run-capacity-stress-keeps-10-runs-in-the-gate` | 1 | 31.9 | 3.2 | 10 runs in the gate, 100 in the lane `slow` |
| N4 `story:where-do-the-10-s-of-the-identification-retry-loop-test-go` | 1 | 10.2 | 10.2 | a question; no change claimed |
| sum | 81 | 1,117.8 | 329.7 | |

The ids of S7b, S8a, S8c and S9a keep the words of their first titles; the store does not change
an id.

Without a row, because they cover no test of their own:

- N1 `story:the-lane-slow-runs-the-ignored-timing-family`: an ignored test whose function name
  starts with `slow_lane_` runs in the lane `slow` and is ended at 60 s. It owns
  `.github/workflows/slow.yml` and the table of `crates/connectors-build/src/ignored.rs`. With
  its paths the lane starts for nearly every pull request that changes code; the lane has no
  step budget yet.
- N2 `story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build`:
  one helper for every debug-only override, forwarded to the owner by one prefix. It carries the
  one decision beyond the plan: a test on a release build that the override is ignored there.
- N3 `story:where-do-the-connection-lease-tests-clock-and-the-30-s-admission-bound-live`: the
  question S3 waits for.

Edges (`depends_on`) order the stories that write one file; `aep plan artifact waves` puts the
set into seven waves (N1 and S8b first, S3 last). `crates/connectors-host/src/local/owner/transport.rs`
is written by N2, S4, S2, S8c and S3, in that order.

Not claimed by a change, among the 81: 2 tests, 17.6 s (N4's test, and S7b's
`embedded_text_where_the_engine_refuses_it_is_refused`, 7.4 s, which the plan had at 2.0 s
after). Outside the 81 and also unclaimed: 26 provider tests that lower a changed definition,
about 80.5 s, and 9 catalog-crate tests of 1 s to under 5 s, about 22.6 s.

The stories do not reach the budget. Of the 2,474.8 s summed they cover 1,176.8 s (the 81 tests
and the 36 further tests of S7a, 59.0 s) and leave 334.7 s of it; the sum after all of them is
1,632.7 s, 408 s of wall time at four tests at a time, against 120 s. The rest is in tests the
stories do not cover: about 755 s in the band of 1 s to under 5 s and 543 s under 1 s. What
closes that gap is not known. One candidate, not measured: the store of S9a for every test that
builds one, not only for its 12.

Two older drafts overlap the set and are not changed by it:
story:store-tests-end-within-60s-under-load (eight tests of S9a, the stress test of S9c, one
test of S2) and story:test-binary-per-package-rest (it renames the test binaries the acceptance
commands of the set name).

The gate fails when the test step runs over `TEST_STEP_BUDGET` only after the step is under
the budget: before that it makes every pull request red.

## Acceptance

- The forge's "repository gate" job of a pull request prints `gate: test step ran <t> s (budget
  120 s)` with `<t>` at most 120.
- No test in that run is 60 s or longer, and the gate's list of the ten slowest starts under 60 s.
- A gate run whose test step takes longer than 120 s exits non-zero and names the time. A unit
  test of `connectors-build` holds this with an injected duration.

## Limits

- A larger runner, more shards and a higher timeout are not this story's fix (folder rule).
- The 223 skipped tests and the lane `slow` are not measured here.
- The time of the test step on the build server was not measured for this story.

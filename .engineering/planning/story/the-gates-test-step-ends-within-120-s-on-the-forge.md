---
format: aep.planning-md/3
id: story:the-gates-test-step-ends-within-120-s-on-the-forge
kind: story
status: draft
title: The gate's test step ends within 120 s on the forge, and fails when it does not
relations:
- serves: vision:independent-contract-adapters
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
revision: 2
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
more, the line the time is spent at, and groups them by cause. Twelve stories follow from it.
They are not drafted as artifacts yet. "Before" and "after" are seconds summed over the tests a
story covers; every "after" is an estimate and none was timed.

| story | tests | before | after | seam |
|---|---|---|---|---|
| S1 host lock waits | 4 | 122.0 | 5.5 | `child_lock_wait` and `set_lifecycle_lock_wait` exist; one test moves into the library tests |
| S2 owner connect wait | 7 | 120.8 | 36.0 | new debug-only override of the 10 s connect wait |
| S3 lease clock and admission bound | 11 | 222.9 | 48.0 | new debug-only clock offset and admission bound |
| S4 idle exit and sweep | 4 | 67.4 | 9.5 | new debug-only sweep interval |
| S5 MCP budgets and sleeps | 7 | 101.8 | 34.0 | the tests' own values |
| S6 client timeouts | 5 | 56.2 | 5.4 | new test seam for the HTTP and SQL timeouts |
| S7a released definition once, provider side | 66 | 175.4 | 13.4 | a committed lowered file with a digest of its sources |
| S7b memoised ingest, catalog crate | 11 | 31.6 | 5.0 | ingest made once per process |
| S8a waits that prove nothing, adapters and catalog | 8 | 80.0 | 18.6 | the tests' own values; one test to lane `slow` |
| S8b waits in the host unit tests | 5 | 48.6 | 18.2 | the tests' own values |
| S8c waits in the CLI tests | 7 | 64.2 | 34.7 | two new debug-only overrides |
| S9 fixture templates | 18 | 190.6 | 107.7 | a template store and a template jail built once per run |

S1, S6, S7a, S7b, S8a and S8b touch no file of another story. The overlaps of the other six
are listed in the test plan.

The stories do not reach the budget. They cover 1,281.6 s of the 2,474.8 s summed and leave
336.0 s of it; the sum after all of them is 1,529.2 s, 382 s of wall time at four tests at a
time, against 120 s. The rest is in tests the stories do not cover: 650 s in the band of 1 s
to under 5 s and 543 s under 1 s. What closes that gap is not known. One candidate, not
measured: the store and the jail of S9 for every test that builds one, not only for its 18.

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

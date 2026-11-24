---
format: aep.planning-md/3
id: story:the-gates-test-step-ends-within-120-s-on-the-forge
kind: story
status: draft
title: The gate's test step ends within 120 s on the forge, and fails when it does not
relations:
- serves: vision:independent-contract-adapters
revision: 1
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

## Change

Not decomposed yet. The order the numbers give:

1. The 81 tests of 5 s and more. A test that waits out a production constant gets a seam for that
   wait, the way `set_test_query_timeout` (`crates/connectors-host/src/local/clock.rs`) shortens
   the clock query in tests, and one test per constant keeps the production value in lane `slow`.
2. The 1 s to 5 s band, binary by binary, starting with `connectors-host` and
   `connectors-catalog-provider::main`: find what each test pays for (a child process, a store
   opened from nothing, a fixture built per test) and build it once per run, in one of the two
   forms the folder rule accepts.
3. The gate fails when the test step runs over `TEST_STEP_BUDGET`. This is the last step: before
   steps 1 and 2 it makes every pull request red.

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

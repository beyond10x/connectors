---
format: aep.planning-md/3
id: story:seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait
kind: story
status: draft
title: 'Seven CLI tests of a slow owner start no longer wait out the 10 s connect wait: 120.8 s become an estimated 36.0 s'
summary: Owner start delays of 12 s to 34 s exist only to exceed the 10 s connect wait and the 30 s read deadline; one debug-only override of the connect wait lets 1.5 s do
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
- serves: vision:independent-contract-adapters
- depends_on: story:the-lane-slow-runs-the-ignored-timing-family
- depends_on: story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build
- depends_on: story:four-idle-exit-tests-no-longer-span-several-5-s-recovery-sweeps
scope:
- confidence: cited
  path: apps/connectors/tests/cold_owner_start.rs
- confidence: cited
  path: apps/connectors/tests/owner_slow_start.rs
- confidence: cited
  path: apps/connectors/tests/owner_slow_start_adversary.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/transport.rs
revision: 8
---
## Defect

Seven CLI tests start an owner whose start is delayed by 12 s to 34 s, because the delay must be
longer than a production wait: the 10 s connect wait or the 30 s default read deadline. Together
they take 120.8 s of the test step.

Source of the durations: the ones nextest printed in the forge's gate job 138270416028 of pull
request 291, 2026-11-24, as listed in test-plan:where-the-gates-test-step-spends-its-time (group
A). The plan read the sources at 0.77.2. The function names below were read in the tree of 0.78.1;
the lines cited in this story were read again at 0.78.2 and corrected where they had moved.
The plan gives the sum as 120.8 s; its seven rows, printed with one decimal, sum to 120.7 s
(inferred: the plan summed the unrounded durations).

- 35.6 s, binary `connectors::owner_slow_start_adversary`,
  `apps/connectors/tests/owner_slow_start_adversary.rs`,
  `a_read_after_one_that_outlasted_its_deadline_reaches_the_owner_it_left`: two reads, each with a
  34 s owner start delay (line 114) against the 30 s default read deadline.
- 15.5 s, same binary and file,
  `a_revalidation_during_another_commands_slow_owner_start_reaches_that_owner`: owner start delay
  14 s (line 135) against the 10 s connect wait.
- 15.1 s, same binary and file, `owner_stop_during_a_slow_owner_start_stops_that_owner`: owner
  start delay 14 s (line 172); the test waits for the lock and then stops the owner.
- 13.8 s, binary `connectors::owner_slow_start`, `apps/connectors/tests/owner_slow_start.rs`,
  `a_read_waits_for_a_slow_owner_start_within_its_deadline`: `SLOW_START` 12 s (line 28) against
  the 10 s connect wait.
- 13.7 s, binary `connectors::cold_owner_start`, `apps/connectors/tests/cold_owner_start.rs`,
  `a_cold_read_without_a_connection_waits_for_a_slow_owner_start`: slow start 12 s (line 171).
- 13.6 s, binary `connectors::owner_slow_start`, `apps/connectors/tests/owner_slow_start.rs`,
  `a_revalidation_waits_for_a_slow_owner_launch`: `CONNECTORS_TEST_OWNER_LAUNCH_DELAY_MS` 12 s.
- 13.4 s, same binary and file, `a_revalidation_waits_for_a_slow_owner_start`: `SLOW_START` 12 s.

Read at 0.78.2 in `crates/connectors-host/src/local/owner/transport.rs`: `Client::connect` (lines
254 to 262) waits `Instant::now() + Duration::from_secs(10)` (a literal, line 259) for a running
owner and `START_BOUND` (60 s, line 142) for the greeting of an owner it starts. In
`apps/connectors/src/local/invoke.rs`: `READ_DEADLINE_MS` is 30,000 (line 57).

## Change

Each seam, and whether it exists today (grep in the tree of 0.78.1):

1. Exists: the names `START_DELAY` (`CONNECTORS_TEST_OWNER_START_DELAY_MS`) and `LAUNCH_DELAY`
   (`CONNECTORS_TEST_OWNER_LAUNCH_DELAY_MS`) in `transport.rs` (lines 1591 and 1597). The tests set
   them to 12 s, 14 s and 34 s today.
2. Exists: the flag `--deadline-seconds`, 1 to 120 (`READ_DEADLINE_MS` and `MAX_DEADLINE_SECONDS`
   in `apps/connectors/src/local/invoke.rs`).
3. New: one override, through the helper of
   story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build:
   `CONNECTORS_TEST_OVERRIDE_OWNER_CONNECT_MS` replaces the literal 10 s of `Client::connect`. The
   CLI process reads it; the test sets it on the CLI it starts.
4. Values (this story's choice; the plan's example is "a delay of 1.5 s exceeds a wait of 1 s"):
   six tests set the connect wait to 1,000 ms and the start or launch delay to 1,500 ms. The
   delay exceeds the wait by 500 ms, so a CLI that did not wait for the start would fail as it
   does today at 12 s against 10 s.
5. The two-read test names `--deadline-seconds 2` and a start delay of 3,000 ms in place of the
   30 s default and 34 s. The delay exceeds the deadline by 1,000 ms. Inferred from the plan's
   list of seams; the plan does not give these two values.

What the 10 s protects, and why 1,000 ms is safe in a test. The 10 s is the deadline of a command
that names none: within it the command reads the store's authority and reaches a running owner,
so that it ends with `timeout` and does not hang on an owner that never answers
(`contracts/cli/v1alpha1/owner.md` lines 184 to 185: "A command without a deadline of its own
reaches a running owner within 10 seconds"; the comment at `transport.rs` lines 251 to 253). The
six tests need an owner whose start outlasts that deadline, and prove what a command does then:
it waits for the greeting of the owner being started, up to `START_BOUND`. That is a relation,
the start takes longer than the connect wait, and it holds at 1,500 ms against 1,000 ms as at
12 s against 10 s; the code that runs once the deadline has passed is the same at either value.
The value 10 s itself is proved by the lane test below, which sets no override. A release build
ignores the override.

`START_BOUND` (60 s) gets no override. The plan's row names one for it. No test of the seven waits
it out: each delay is below it today (34 s) and after (3 s). Inferred from the delays; if a test
needs it, the pull request adds it through the same helper and says so.

`WriteClient::connect` (`transport.rs` lines 1287 to 1301) has its own literal 10 s (line 1294).
It is out of this story. Its only callers are `apps/connectors/src/local/operations.rs` line 265
(a write) and the test `apps/connectors/tests/owner_build_security.rs` line 139; none of the seven
tests sends a write through it (inferred from their names: reads, revalidations and an owner
stop).

One case at the real 10 s stays, outside the gate:
`slow_lane_a_read_waits_for_a_slow_owner_start_within_its_deadline` in `owner_slow_start.rs`, an
ignored test that sets no override and keeps `SLOW_START` at 12 s. It shares its body with the
gate's test through one helper that takes the delay. The rule of
story:the-lane-slow-runs-the-ignored-timing-family puts it in the timing family by its name. This
story edits neither `ignored.rs` nor `slow.yml`.

Estimate after (the plan; not timed): fixture 3.0 s (the band of the binary `owner_slow_start`:
8.9 s over 3 tests) + delay 1.5 s + 0.5 s = 5 s for each of six tests, and 6 s for the two-read
test. Sum 36.0 s.

## Acceptance

1. The seven tests pass, and their durations in the test step of the forge's "repository gate" job
   of this story's pull request sum to at most 54 s. Before: 120.8 s. Estimate: 36.0 s. Bound: 1.5
   × 36.0 = 54 s. The numbers are read from the durations nextest prints in that job.
2. The margin holds in the sources: in each of the six tests the start or launch delay is
   1,500 ms and the connect wait 1,000 ms; in the two-read test the delay is 3,000 ms and the
   deadline 2 s. Each test still fails when the CLI gives up before the owner started (the
   assertion it has today).
3. `connectors::owner_slow_start::slow_lane_a_read_waits_for_a_slow_owner_start_within_its_deadline`
   passes in the lane `slow` against the real 10 s. The lane's command
   `cargo run --locked --bin connectors-build -- ignored --family timing --required --only-prerequisite provider`
   lists it as `passed`. Alone it runs with
   `cargo test --locked -p connectors --test owner_slow_start -- --ignored --exact slow_lane_a_read_waits_for_a_slow_owner_start_within_its_deadline`.
4. The gate's ignored step still passes: the new ignored test is not unknown to it.

That a release build ignores the override is held once, for the helper, by
story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build.

## Scope

- `crates/connectors-host/src/local/owner/transport.rs`: `Client::connect` (lines 254 to 262).
  Four other stories of this set write this file. The `depends_on` edges put them in one order:
  the helper story, then story:four-idle-exit-tests-no-longer-span-several-5-s-recovery-sweeps,
  then this story, then story:seven-cli-tests-shorten-their-waits-or-a-production-wait, then
  story:eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping.
- `apps/connectors/tests/owner_slow_start.rs`: the three tests above and the lane test.
  `reads_that_name_a_longer_deadline_wait_for_their_turn` of the same file is counted in
  story:six-cli-tests-take-jail-binaries-built-once-per-run, which does not edit the file.
- `apps/connectors/tests/owner_slow_start_adversary.rs`: the three tests above.
- `apps/connectors/tests/cold_owner_start.rs`: the one test above.

## Not in this story

- `reads_that_name_a_longer_deadline_wait_for_their_turn` (6.0 s, group K) and the tests of every
  other group of the test plan.
- `WriteClient::connect` and `START_BOUND`, as said above.
- The lane `slow` itself: story:the-lane-slow-runs-the-ignored-timing-family.
- The gate failing when its test step runs over 120 s:
  story:the-gates-test-step-ends-within-120-s-on-the-forge.
- Not established: every "after" figure on the forge; whether the two-read test reaches 6 s with
  the values of point 5; whether 1,000 ms is enough on a loaded runner for what ends at the same
  deadline before the owner is started, the read of the store's authority (`transport.rs` lines
  278 to 286): a read that takes longer ends the command with `timeout` and fails the test. The
  two delay names are read at `transport.rs` line 1631 (`LAUNCH_DELAY`, in `spawn`, the CLI's
  side) and line 1781 (`START_DELAY`, the owner's side), both under `debug_assertions`.
  story:store-tests-end-within-60s-under-load, a draft outside this set, names
  `a_cold_read_without_a_connection_waits_for_a_slow_owner_start` too.

---
format: aep.planning-md/3
id: story:four-idle-exit-tests-no-longer-span-several-5-s-recovery-sweeps
kind: story
status: draft
title: 'Four idle exit tests no longer span several 5 s recovery sweeps: 67.4 s become an estimated 9.5 s'
summary: The recovery sweep interval is a literal 5 s, so an idle bound that spans three sweeps is 15 s or more; one debug-only override of the interval shortens it
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
- serves: vision:independent-contract-adapters
- depends_on: story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build
scope:
- confidence: cited
  path: apps/connectors/tests/local_cli.rs
- confidence: cited
  path: apps/connectors/tests/owner_idle_exit_adversary.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/maintenance.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/transport.rs
revision: 4
---
## Defect

Four tests of the owner's idle exit choose an idle bound that spans several recovery sweeps, and
the sweep interval is a literal 5 s. Together they take 67.4 s of the test step.

Source of the durations: the ones nextest printed in the forge's gate job 138270416028 of pull
request 291, 2026-11-24, as listed in test-plan:where-the-gates-test-step-spends-its-time (group
G). The plan read the sources at 0.77.2. The function names below were read in the tree of 0.78.1.
The sum is the plan's and equals the sum of its rows.

- 30.9 s, binary `connectors::owner_idle_exit_adversary`,
  `apps/connectors/tests/owner_idle_exit_adversary.rs` (line 125),
  `idle_owners_exit_within_their_bound_when_it_spans_several_recovery_sweeps`: 4 owners with a 30 s
  idle bound, which is 6 sweeps of 5 s; the test polls until all have exited (`limit` 45 s).
- 15.4 s, binary `connectors-host`, `crates/connectors-host/src/local/owner/transport.rs`,
  `an_unsettleable_pending_attempt_does_not_keep_an_owner_alive`: an idle bound of 15 s, which is
  3 sweeps of 5 s.
- 15.2 s, binary `connectors-host`, same file,
  `an_idle_owner_exits_at_a_bound_spanning_several_empty_recovery_sweeps`: an idle bound of 15 s,
  3 sweeps.
- 5.9 s, binary `connectors::local_cli`, `apps/connectors/tests/local_cli.rs` (line 1959),
  `a_connected_client_keeps_an_owner_past_its_idle_bound`: idle bound 1,500 ms; it asserts no exit
  within 4 s, then waits up to 20 s for the idle exit.

Read at 0.78.1 in `crates/connectors-host/src/local/owner/maintenance.rs` (lines 39 to 56):
`Background::start(paths, pool)` starts the recovery thread, which waits
`receiver.recv_timeout(Duration::from_secs(5))` between sweeps (line 48), a literal.
`Background::start` is called at `transport.rs` line 1865 (the owner) and at lines 4811 and 4878
(the test helper `idle_exit`, line 4790, which runs the accept loop in the test's process).

## Change

Each seam, and whether it exists today (grep in the tree of 0.78.1):

1. Exists: `CONNECTORS_TEST_OWNER_IDLE_MS`, read by `idle_bound` in `transport.rs`.
2. Exists: the argument `bound` of the test helper `idle_exit` in the test module of
   `transport.rs`.
3. New: `Background::start` takes the sweep interval as an argument in place of the literal. The
   owner (`transport.rs` line 1865) passes the value of one override, through the helper of
   story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build:
   `CONNECTORS_TEST_OVERRIDE_RECOVERY_SWEEP_MS`, production 5,000 ms. The test helper `idle_exit`
   passes its interval directly and reads no variable.
4. Values (this story's choice; the plan gives the "after" seconds and no values). Sweep interval
   in all four tests: 200 ms. Each idle bound spans at least 3 sweep intervals, as today:
   - the two unit tests: idle bound 1,000 ms = 5 intervals (today 15 s = 3 intervals);
   - `idle_owners_exit_within_their_bound_when_it_spans_several_recovery_sweeps`: idle bound
     2,000 ms = 10 intervals (today 30 s = 6 intervals), `limit` 5 s;
   - `a_connected_client_keeps_an_owner_past_its_idle_bound`: idle bound 1,500 ms = 7.5 intervals;
     the no-exit window is 2 s (the plan), which is longer than the idle bound by 500 ms.

Estimate after (the plan; not timed): 3 + 1.5 + 1.5 + 3.5 = 9.5 s.

## Acceptance

1. The four tests pass, and their durations in the test step of the forge's "repository gate" job
   of this story's pull request sum to at most 15 s. Before: 67.4 s. Estimate: 9.5 s. Bound: 1.5 ×
   9.5 = 14.25, rounded up to 15 s. The numbers are read from the durations nextest prints in that
   job.
2. Several sweeps still pass before the exit: each of the two unit tests and the adversary test
   asserts that its idle bound is at least 3 times its sweep interval, and the two unit tests
   assert that at least 3 sweeps ran before the owner exited (a counter the test reads from the
   helper `idle_exit`).
3. With no variable set, the owner sweeps every 5 s: a unit test of `maintenance.rs` or
   `transport.rs` asserts that the value the owner passes without the variable is 5,000 ms.

That a release build ignores the override is held once, for the helper, by
story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build.

The plan names no test that stays in the lane `slow` for this story. Its column says: "the 600 s
production idle exit is not tested here".

## Scope

- `crates/connectors-host/src/local/owner/maintenance.rs`: `Background::start` (lines 39 to 56).
- `crates/connectors-host/src/local/owner/transport.rs`: the call at line 1865, the test helper
  `idle_exit` (line 4790) and the two tests above. Four other stories of this set write this file.
  The `depends_on` edges put them in one order: the helper story, then this story, then
  story:seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait, then
  story:seven-cli-tests-shorten-their-waits-or-a-production-wait, then
  story:eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping.
- `apps/connectors/tests/owner_idle_exit_adversary.rs`: the one test above.
- `apps/connectors/tests/local_cli.rs`: `a_connected_client_keeps_an_owner_past_its_idle_bound`
  (line 1959). Two other stories edit other tests of this file:
  story:seven-cli-tests-shorten-their-waits-or-a-production-wait
  (`clock_check_below_quorum_refusal`, line 325) and
  story:the-100-run-capacity-stress-keeps-10-runs-in-the-gate (the test at line 1691).

## Not in this story

- A test of the production idle exit of 600 s (`IDLE_EXIT`): the plan says it is not tested here.
- A run at the real 5 s sweep interval in the lane `slow`: the plan names none.
- The tests of every other group of the test plan.
- The gate failing when its test step runs over 120 s:
  story:the-gates-test-step-ends-within-120-s-on-the-forge.
- Not established: every "after" figure on the forge; whether a sweep of 200 ms changes what the
  tests prove about a sweep that finds work (the two unit tests' names say "empty" and
  "unsettleable"; their bodies were read at the lines of the bound only).

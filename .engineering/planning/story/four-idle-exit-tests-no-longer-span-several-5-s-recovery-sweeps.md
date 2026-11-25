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
revision: 9
---
## Defect

Four tests of the owner's idle exit choose an idle bound that spans several recovery sweeps, and
the sweep interval is a literal 5 s. Together they take 67.4 s of the test step.

Source of the durations: the ones nextest printed in the forge's gate job 138270416028 of pull
request 291, 2026-11-24, as listed in test-plan:where-the-gates-test-step-spends-its-time (group
G). The plan read the sources at 0.77.2. The function names below were read in the tree of 0.78.1;
the lines cited in this story were read again at 0.78.2. The sum is the plan's and equals the sum
of its rows.

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

Read at 0.78.2 in `crates/connectors-host/src/local/owner/maintenance.rs` (lines 39 to 63):
`Background::start(paths, pool)` starts the recovery thread, which waits
`receiver.recv_timeout(Duration::from_secs(5))` between sweeps (line 48), a literal.
`Background::start` has three callers in `transport.rs`: the owner (line 1865), the test helper
`idle_exit` (line 4811; the helper starts at line 4790 and runs the accept loop in the test's
process) and the test `a_retirement_relisten_is_not_a_replaced_socket` (line 4878; the test starts
at line 4857).

Nothing counts sweeps today. `Background` holds the stop channel, the thread and `sweeping`, an
`AtomicBool` that `busy()` reads and that only a pass which found something pending sets
(`maintenance.rs` lines 33 to 37, 64 to 69 and 101). `idle_exit` returns `Option<Duration>`, the
time the accept loop took to retire.

## Change

Each seam, and whether it exists today (grep in the tree of 0.78.1):

1. Exists: `CONNECTORS_TEST_OWNER_IDLE_MS`, read by `idle_bound` in `transport.rs`.
2. Exists: the argument `bound` of the test helper `idle_exit` in the test module of
   `transport.rs`.
3. New: `Background::start` takes the sweep interval as an argument in place of the literal. The
   owner (`transport.rs` line 1865) passes `sweep_interval()`, a new function beside `idle_bound`
   (line 1970) that is compiled in every build. It returns the value of one override, through the
   helper of
   story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build:
   `CONNECTORS_TEST_OVERRIDE_RECOVERY_SWEEP_MS`, production 5,000 ms. The test helper `idle_exit`
   passes its interval directly and reads no variable.
   `a_retirement_relisten_is_not_a_replaced_socket` passes the production 5,000 ms; that one
   argument is its only change.
4. New: a count of finished sweeps. `Background` gets a counter that the recovery thread
   increments once after each call of `sweep` (in the loop at `maintenance.rs` lines 47 to 55),
   and a reader for it. The field, the increment and the reader are compiled under `#[cfg(test)]`
   only, after the precedent of `Batch::without_clock` in the same file (lines 16 to 25): no
   production code reads the count. The helper `idle_exit` reads the count in its loop thread
   after `accept` returned and before `recovery.stop()` (`transport.rs` lines 4821 to 4830) and
   returns it beside the time: its return type becomes `Option<(Duration, usize)>`. Its only two
   callers are the two unit tests of this story (lines 4906 and 4919). The count is taken in the
   test's process, so it exists for the two unit tests only; the two tests that start an owner
   process (the adversary test and the `local_cli` test) read no count.
5. Values (this story's choice; the plan gives the "after" seconds and no values). Sweep interval
   in all four tests: 200 ms. Each idle bound spans at least 3 sweep intervals, as today:
   - the two unit tests: idle bound 1,000 ms = 5 intervals (today 15 s = 3 intervals);
   - `idle_owners_exit_within_their_bound_when_it_spans_several_recovery_sweeps`: idle bound
     2,000 ms = 10 intervals (today 30 s = 6 intervals), `limit` 5 s;
   - `a_connected_client_keeps_an_owner_past_its_idle_bound`: idle bound 1,500 ms = 7.5 intervals;
     the no-exit window is 2 s (the plan), which is longer than the idle bound by 500 ms.
6. How the interval reaches an owner process. Both test files start the owner themselves, not
   through the CLI, so the variable is set on the owner's command and the forwarding rule of the
   helper story is not involved:
   - `owner_idle_exit_adversary.rs`: the file's own `start_owner` (lines 68 to 101) sets
     `CONNECTORS_TEST_OVERRIDE_RECOVERY_SWEEP_MS` beside `CONNECTORS_TEST_OWNER_IDLE_MS` (line
     81). Its one caller is the test of this story (line 128).
   - `local_cli.rs`: the shared helper `start_owner_holding_startup` (lines 972 to 1024) sets the
     idle variable only when its argument `idle_ms` is `Some` (lines 1002 to 1004). In that same
     branch it sets `CONNECTORS_TEST_OVERRIDE_RECOVERY_SWEEP_MS` to 200. No signature changes and
     no caller changes. The branch is taken by the two calls that pass `Some(1500)`:
     `a_connected_client_keeps_an_owner_past_its_idle_bound` (line 1963) and
     `an_idle_owner_exits_and_the_next_start_gets_a_new_owner` (line 1930), which so runs with a
     sweep of 200 ms too and is not edited. Every other call passes `None` (through `start_owner`,
     `start_owner_in` or directly) and starts an owner with no variable, as today; among them is
     the 100-run test (line 1694).

What the 5 s protects, and why 200 ms is safe in a test. The interval is the pause between two
passes of the periodic metadata recovery (`contracts/service/local-mutations.md` lines 169 to
174: "It waits five seconds between read-only passes"): it sets how soon the owner looks again
for a pending attempt that no caller came back for, and how often it opens the metadata store to
look. The override changes the pace only. What a pass reads and hands on (`sweep`,
`maintenance.rs` lines 87 to 124) and what counts as work for the idle clock (`working` and
`quiet`, `transport.rs` lines 2045 and 2050) are the same code at 200 ms and at 5 s, and the four
tests assert that passes do not keep an owner alive, which more passes inside one bound test at
least as hard. A release build ignores the override, so no installed owner sweeps faster than the
contract says.

Estimate after (the plan; not timed): 3 + 1.5 + 1.5 + 3.5 = 9.5 s.

## Acceptance

1. The four tests pass, and their durations in the test step of the forge's "repository gate" job
   of this story's pull request sum to at most 15 s. Before: 67.4 s. Estimate: 9.5 s. Bound: 1.5 ×
   9.5 = 14.25, rounded up to 15 s. The numbers are read from the durations nextest prints in that
   job.
2. Several sweeps still pass before the exit: each of the two unit tests and the adversary test
   asserts that its idle bound is at least 3 times its sweep interval, and the two unit tests
   assert that at least 3 sweeps finished before the owner exited. The count is the second value
   `idle_exit` returns, read from the counter of Change point 4. The two tests run with
   `cargo test --locked -p connectors-host --lib -- an_idle_owner_exits_at_a_bound_spanning_several_empty_recovery_sweeps an_unsettleable_pending_attempt_does_not_keep_an_owner_alive`.
3. With no variable set, the owner sweeps every 5 s: the unit test
   `the_owner_sweeps_every_5_s_without_an_override`, in the test module of
   `crates/connectors-host/src/local/owner/transport.rs`, asserts that
   `sweep_interval()` returns 5,000 ms without the variable. And
   `grep -n 'Background::start(' crates/connectors-host/src/local/owner/transport.rs` shows the
   owner's call (line 1865 today) with `sweep_interval()` as its interval argument.

That a release build ignores the override is held once, for the helper, by
story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build.

The plan names no test that stays in the lane `slow` for this story. Its column says: "the 600 s
production idle exit is not tested here".

## Scope

- `crates/connectors-host/src/local/owner/maintenance.rs`: `Background` and `Background::start`
  (lines 33 to 63): the interval argument and the sweep counter.
- `crates/connectors-host/src/local/owner/transport.rs`: the call at line 1865, the new function
  `sweep_interval()` beside `idle_bound` (line 1970) with its unit test, the test helper
  `idle_exit` (line 4790), the two tests above and the one argument of the call at line 4878.
  Four other stories of this set write this file. The `depends_on` edges put them in one order:
  the helper story, then this story, then
  story:seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait, then
  story:seven-cli-tests-shorten-their-waits-or-a-production-wait, then
  story:eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping.
- `apps/connectors/tests/owner_idle_exit_adversary.rs`: the one test above and the file's
  `start_owner` (lines 68 to 101), which has no other caller.
- `apps/connectors/tests/local_cli.rs`, two regions:
  - `a_connected_client_keeps_an_owner_past_its_idle_bound` (line 1959);
  - the shared helper `start_owner_holding_startup` (lines 972 to 1024): one statement inside the
    branch at lines 1002 to 1004. No signature and no caller changes; the helpers `start_owner`,
    `start_owner_with` and `start_owner_in` (lines 935 to 962) are not edited. A call that passes
    `None` starts the same owner as today.

  Two other stories edit other regions of this file:
  story:seven-cli-tests-shorten-their-waits-or-a-production-wait
  (`clock_check_below_quorum_refusal`, lines 325 to 393) and
  story:the-100-run-capacity-stress-keeps-10-runs-in-the-gate (the test at line 1691). Neither
  edits lines 935 to 1024; the 100-run test starts its owner through `start_owner` (line 1694)
  with `None`.

## Not in this story

- A test of the production idle exit of 600 s (`IDLE_EXIT`): the plan says it is not tested here.
- A run at the real 5 s sweep interval in the lane `slow`: the plan names none.
- An edit of `an_idle_owner_exits_and_the_next_start_gets_a_new_owner` in `local_cli.rs`. It runs
  with a sweep of 200 ms through the helper's branch and must still pass; it is in no sum of this
  story.
- The tests of every other group of the test plan.
- The gate failing when its test step runs over 120 s:
  story:the-gates-test-step-ends-within-120-s-on-the-forge.
- Not established: every "after" figure on the forge; whether a sweep of 200 ms changes what the
  tests prove about a sweep that finds work (the two unit tests' names say "empty" and
  "unsettleable"); how long one pass over the unsettleable attempt takes. A recovery batch has a
  work budget of 250 ms (`maintenance.rs` line 137). If a pass of
  `an_unsettleable_pending_attempt_does_not_keep_an_owner_alive` uses all of it, a bound of
  1,000 ms holds two finished passes at an interval of 200 ms, not three (computed: a pass ends
  at 450 ms and at 900 ms), and acceptance 2 fails for that test. The implementor then raises
  that test's bound to at least 1,350 ms (3 × (200 + 250)) and says so in the test.

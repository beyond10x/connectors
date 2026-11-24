---
format: aep.planning-md/3
id: story:seven-cli-tests-shorten-their-waits-or-a-production-wait
kind: story
status: draft
title: 'Six CLI tests shorten their own waits or a production wait: 54.0 s become an estimated 24.5 s'
summary: 'Three tests shorten their own holds and three get a debug-only override of a production wait: the print deadline, the clock query timeout and the client drain bound'
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
- serves: vision:independent-contract-adapters
- depends_on: story:five-host-unit-tests-shorten-the-waits-they-chose
- depends_on: story:seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait
- depends_on: story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build
- informed_by: story:clock-quorum-test-tolerates-a-slow-loopback-source
scope:
- confidence: cited
  path: apps/connectors/tests/catalog_provider_failure.rs
- confidence: cited
  path: apps/connectors/tests/invoke_walks.rs
- confidence: cited
  path: apps/connectors/tests/local_cli.rs
- confidence: cited
  path: apps/connectors/tests/owner_cancel_adversary.rs
- confidence: cited
  path: apps/connectors/tests/owner_tether_adversary.rs
- confidence: cited
  path: crates/connectors-host/src/local/clock.rs
- confidence: cited
  path: crates/connectors-host/src/local/config.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/transport.rs
revision: 7
---
## Defect

Six CLI tests wait on a hold the test chose or on a production wait. Together they take 54.0 s of
the test step.

This story was drafted with seven tests (64.2 s). The seventh,
`adversary2_the_identification_retry_loop_ends_at_its_deadline_without_flooding_the_owner`
(10.2 s), has no seam yet and went to
story:where-do-the-10-s-of-the-identification-retry-loop-test-go. The id of this story keeps
"seven"; an id cannot be changed.

Source of the durations: the ones nextest printed in the forge's gate job 138270416028 of pull
request 291, 2026-11-24, as listed in test-plan:where-the-gates-test-step-spends-its-time (six
rows of group M). The plan read the sources at 0.77.2. The function names and lines below were
read in the tree of 0.78.1. The sum is computed from the plan: 64.2 − 10.2 = 54.0 s. The six rows,
printed with one decimal, sum to 54.2 s (inferred: the plan summed the unrounded durations).

- 11.5 s, binary `connectors::owner_cancel_adversary`,
  `apps/connectors/tests/owner_cancel_adversary.rs`,
  `a_read_cancelled_during_its_launch_reads_no_credential_after_the_cancel`: every later launch is
  held 5 s (the test writes `5000` into the fixture file `/work/start_delay_ms`, line 795), and
  the test sleeps until 7 s after the cancel (line 809).
- 10.9 s, binary `connectors::invoke_walks`, `apps/connectors/tests/invoke_walks.rs` (line 784),
  `each_page_of_a_walk_has_the_named_deadline_to_itself`: `page_ms` 3000 for 3 pages under
  `--deadline-seconds 8`.
- 10.2 s, binary `connectors::catalog_provider_failure`,
  `apps/connectors/tests/catalog_provider_failure.rs` (line 201),
  `a_provider_past_its_deadline_is_named_by_setup_check`: a hanging provider until the 10 s print
  deadline. This is a `pub fn` case listed in the file's `main` and run through `standin::main`,
  not a function with a test attribute.
- 9.0 s, binary `connectors::local_cli`, `apps/connectors/tests/local_cli.rs` (line 325),
  `clock_check_below_quorum_refusal`: a source that answers after 2,500 ms and a silent source
  against the 2 s clock query timeout, over several CLI rows.
- 7.0 s, binary `connectors::owner_cancel_adversary`, its file (line 905),
  `a_queued_read_another_session_cancels_runs_unsent_and_the_running_read_succeeds`: a provider
  read of 3 s (`SLOW`) and sleeps of 700 ms and 1,500 ms.
- 5.6 s, binary `connectors::owner_tether_adversary`,
  `apps/connectors/tests/owner_tether_adversary.rs`,
  `a_lost_owner_waits_for_a_connected_client_and_no_longer_than_the_drain_bound`: it asserts no
  exit within 2 s and an exit within the 5 s client drain bound plus 2 s.

## Change

Three tests shorten values of their own. Each value is this story's choice unless it says "the
plan".

1. `a_read_cancelled_during_its_launch_reads_no_credential_after_the_cancel`: the hold becomes
   1,500 ms (the plan's value; the plan names `CONNECTORS_TEST_OWNER_LAUNCH_DELAY_MS`, which does
   not occur in the file: the hold is the fixture file). The sleep after the cancel becomes
   3,500 ms: the hold plus 2 s, as today (5 s plus 2 s). Estimate after: 8 s (the plan).
2. `each_page_of_a_walk_has_the_named_deadline_to_itself`: `page_ms` 1,200 for 3 pages under
   `--deadline-seconds 3`. The third page cannot answer before 3,600 ms, which is past one cutoff
   of 3 s for the whole walk, as today (9 s against 8 s). A page keeps 1,800 ms for everything
   but the provider. The test's comment says 500 ms is not enough on a loaded host. Estimate
   after: 5 s (the plan).
3. `a_queued_read_another_session_cancels_runs_unsent_and_the_running_read_succeeds`: the sleep
   before the cancel becomes 300 ms and the sleep before the count becomes 500 ms. `SLOW`, a
   value of the file's provider fixture (line 274), stays 3 s. Estimate after: 4 s (the plan).

Three tests wait on a production wait. Each gets one override, three in all, each through the
helper of
story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build:

4. `CONNECTORS_TEST_OVERRIDE_PROVIDER_PRINT_MS`: the print deadline in
   `crates/connectors-host/src/local/config.rs` (line 1505, `derivation_start_failure`, a literal
   of 10 s). `a_provider_past_its_deadline_is_named_by_setup_check` sets 500 ms. Estimate after:
   1.5 s (the plan).
5. `CONNECTORS_TEST_OVERRIDE_CLOCK_QUERY_MS`: the clock query timeout in
   `crates/connectors-host/src/local/clock.rs` (`QUERY_TIMEOUT`, 2 s, line 30).
   `clock_check_below_quorum_refusal` sets 800 ms on the CLI it runs, and its slow source answers
   after 1,300 ms: 500 ms past the timeout, as today (2,500 ms against 2,000 ms). Estimate after:
   4 s (the plan).
6. `CONNECTORS_TEST_OVERRIDE_OWNER_DRAIN_MS`: the client drain bound in
   `crates/connectors-host/src/local/owner/transport.rs` (line 1923, in `accept`, a literal of
   5 s). `a_lost_owner_waits_for_a_connected_client_and_no_longer_than_the_drain_bound` starts its
   owner with 1,000 ms. It asserts no exit within 500 ms (a missing drain exits at about 250 ms,
   the test's comment) and an exit within 2,000 ms (an unbounded drain exits at about 10 s). The
   constant `BOUND` of the file (5 s) stays for the other tests. Estimate after: 2 s (the plan).

The clock query timeout has one source of truth. `query_timeout()` in `clock.rs` (lines 60 to 76)
stays the only reader of the timeout, with two definitions as today:

- under `#[cfg(test)]` (the unit tests of `connectors-host`): the per-thread value of
  `set_test_query_timeout`, as today. It does not read the variable.
- under `#[cfg(not(test))]`: the helper with `CONNECTORS_TEST_OVERRIDE_CLOCK_QUERY_MS` and the
  production value `QUERY_TIMEOUT`. In a release build that is `QUERY_TIMEOUT`.

No build reads both. story:five-host-unit-tests-shorten-the-waits-they-chose uses the first
definition (its `quorum_below_quorum_names_each_source`); this story changes the second and
`depends_on` it.

The implemented story:clock-quorum-test-tolerates-a-slow-loopback-source is about the same test on
a loaded runner: a loopback source that answers at once was counted unreachable. Its acceptance
says the wait the test relies on is stated and held. This story is `informed_by` it. At 800 ms a
prompt source has 800 ms to answer in place of 2 s.

Estimate after (computed from the plan's rows; not timed): 8 + 5 + 4 + 1.5 + 4 + 2 = 24.5 s.

## Acceptance

1. The six tests pass, and their durations in the test step of the forge's "repository gate" job
   of this story's pull request sum to at most 37 s. Before: 54.0 s. Estimate: 24.5 s. Bound: 1.5
   × 24.5 = 36.75, rounded up to 37 s. The numbers are read from the durations nextest prints in
   that job.
2. The margins hold in the sources: the walk's three pages take 3 × 1,200 ms against a deadline
   of 3 s; the slow clock source answers 500 ms past the query timeout; the tether test's windows
   are 500 ms and 2,000 ms around a drain of 1,000 ms.
3. `clock_check_below_quorum_refusal` states the wait it relies on (800 ms for a prompt source)
   beside the value, and passes in 20 runs in a row of this one test, on the forge's runner or on
   a loaded workstation, so that the implemented story's fix is not undone. The pull request's
   description records where the runs were made and the 20 results.
4. With no variable set, the three waits are 10 s, 2 s and 5 s: a unit test per site asserts the
   value the code passes without the variable.

That a release build ignores the three overrides is held once, for the helper, by
story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build.

The plan names nothing that stays in the lane `slow` for this story.

## Scope

- `apps/connectors/tests/owner_cancel_adversary.rs`: the two tests above.
- `apps/connectors/tests/invoke_walks.rs`: the one test above.
- `apps/connectors/tests/catalog_provider_failure.rs`: the one case above.
- `apps/connectors/tests/owner_tether_adversary.rs`: the one test above.
- `apps/connectors/tests/local_cli.rs`: `clock_check_below_quorum_refusal` (line 325). Two other
  stories edit other tests of this file:
  story:four-idle-exit-tests-no-longer-span-several-5-s-recovery-sweeps
  (`a_connected_client_keeps_an_owner_past_its_idle_bound`, line 1959) and
  story:the-100-run-capacity-stress-keeps-10-runs-in-the-gate (the test at line 1691).
- `crates/connectors-host/src/local/config.rs`: the deadline in `derivation_start_failure`.
- `crates/connectors-host/src/local/owner/transport.rs`: the client drain in `accept` (line
  1923). Four other stories of this set write this file. The `depends_on` edges put them in one
  order: the helper story, story:four-idle-exit-tests-no-longer-span-several-5-s-recovery-sweeps,
  story:seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait, then this
  story, then story:eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping.
- `crates/connectors-host/src/local/clock.rs`: the `#[cfg(not(test))]` definition of
  `query_timeout()`. story:five-host-unit-tests-shorten-the-waits-they-chose writes
  `clock/tests.rs` and comes first by the edge.

## Not in this story

- `adversary2_the_identification_retry_loop_ends_at_its_deadline_without_flooding_the_owner`:
  story:where-do-the-10-s-of-the-identification-retry-loop-test-go.
- `quorum_below_quorum_names_each_source` in `clock/tests.rs`:
  story:five-host-unit-tests-shorten-the-waits-they-chose.
- The other rows of group M and the tests of every other group.
- The gate failing when its test step runs over 120 s:
  story:the-gates-test-step-ends-within-120-s-on-the-forge.
- Not established: every "after" figure on the forge. Two of the plan's estimates do not follow
  from the values above: the walk takes 3.6 s plus its fixture (about 5.5 s, inferred from 10.9 −
  9.0 = 1.9 s of fixture today, against the plan's 5 s), and the queued read keeps its 3 s
  provider read (about 5.6 s, inferred from 7.0 − 0.4 − 1.0, against the plan's 4 s). The bound of
  acceptance 1 is kept at the plan's sum; with the two inferred figures the sum is about 26.6 s.
  Whether 800 ms is enough for a prompt clock source on a loaded runner is what acceptance 3
  measures. In which process `derivation_start_failure` runs (the CLI or the owner) was not
  followed; the prefix rule of the helper story covers both.

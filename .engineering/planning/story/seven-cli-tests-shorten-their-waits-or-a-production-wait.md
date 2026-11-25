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
revision: 10
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
read in the tree of 0.78.1 and read again at 0.78.2; three sites were corrected in this revision
(the print deadline, the lines of `query_timeout()` and the helpers of `local_cli.rs`). The sum is
computed from the plan: 64.2 − 10.2 = 54.0 s. The six rows, printed with one decimal, sum to
54.2 s (inferred: the plan summed the unrounded durations).

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
  `clock_check_below_quorum_refusal`: a source whose first answer comes after 2,500 ms, which the
  test's own probe waits out (`answering`, lines 139 to 150), and a silent source that costs the
  2 s clock query timeout once in each of three CLI rows.
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
   `crates/connectors-host/src/local/config.rs` (line 2296, in `Executable::print`, a literal of
   10 s). Read at 0.78.2: `setup check` asks `installed_provider_runs` (line 1597), which prints
   the provider's `--version` (line 1627) and waits at line 2296. The literal of 10 s at line 1505
   (`derivation_start_failure`), which this story named before, is the handshake deadline of a
   derivation's child; `setup check` does not reach it, and it is out of this story.
   `a_provider_past_its_deadline_is_named_by_setup_check` sets 500 ms, and its upper bound (15 s
   today, lines 211 to 215) becomes a value under the production 10 s, so that the case still
   fails when the check waits out 10 s. The case runs the CLI today through `prerequisite` (line
   105) and `Installation::run` (`apps/connectors/tests/support/standin.rs` line 451), which
   clears the environment. It runs `setup check` with a command of its own instead, built in
   `catalog_provider_failure.rs` from `Installation::config()` and the installation's root, with
   the one variable set. `prerequisite` and `support/standin.rs` are not edited. Estimate after:
   1.5 s (the plan).
5. `CONNECTORS_TEST_OVERRIDE_CLOCK_QUERY_MS`: the clock query timeout in
   `crates/connectors-host/src/local/clock.rs` (`QUERY_TIMEOUT`, 2 s, line 30).
   `clock_check_below_quorum_refusal` sets 800 ms on the CLI it runs. What the CLI waits for is
   the silent source, once per row: 3 × 800 ms in place of 3 × 2 s. The slow source's first
   answer comes after 1,300 ms in place of 2,500 ms. The CLI never meets that answer: the test's
   probe (`answering`, lines 139 to 150) takes it before the first row, and every later answer is
   prompt. The delay stays 500 ms past the query timeout, as today (2,500 ms against 2,000 ms),
   so that the test still fails when the probe is removed: the first row would then meet the late
   answer and count the source `unreachable`. The test runs
   the CLI through `clock_unavailable` (`local_cli.rs` lines 153 to 171), which calls `command`
   (lines 40 to 46) and `command_from` (lines 48 to 69); none of them takes an environment. The
   body of `clock_unavailable` moves into a new function beside it that takes the variables to
   set and builds its own command with the arguments of `command_from` (lines 50 to 56).
   `clock_unavailable` keeps its signature and calls the new function with no variable; its two
   other callers (lines 237 and 312) are not edited. `command` and `command_from` are not edited.
   Estimate after: 4 s (the plan).
6. `CONNECTORS_TEST_OVERRIDE_OWNER_DRAIN_MS`: the client drain bound in
   `crates/connectors-host/src/local/owner/transport.rs` (line 1923, in `accept`, a literal of
   5 s). `a_lost_owner_waits_for_a_connected_client_and_no_longer_than_the_drain_bound` starts its
   owner with 1,000 ms. It asserts no exit within 500 ms (a missing drain exits at about 250 ms,
   the test's comment) and an exit within 2,000 ms (an unbounded drain exits at about 10 s). The
   constant `BOUND` of the file (5 s) stays for the other tests. The test starts its owner today
   through the file's `start_owner` (line 71), which sets no variable; a variant of it beside it
   sets the one variable, and `start_owner` keeps its signature and its other callers. Estimate
   after: 2 s (the plan).

What each of the three waits protects, and why the shorter test value is safe:

- The print deadline of 10 s (`config.rs` line 2296; the comment at lines 2247 to 2254: "at most
  4 MiB and ten seconds") ends a command that asks the installed catalog provider to print
  something when the provider hangs: the provider's process group is killed and the failure is
  named `deadline`. The case asserts that name and that the check is not held past the cutoff.
  Both hold at 500 ms as at 10 s, because the provider of the case never answers; no healthy
  provider has to print within the shorter value, since the variable is set on this one CLI run
  only.
- The clock query timeout of 2 s (`clock.rs` line 30: "The one deadline every source shares:
  sends, receives and verification") bounds what one clock acquisition waits for its sources, so
  that a silent source costs the check that time once and is reported `unreachable`. The test
  asserts that report, one request to the silent source per check, and the refusal
  `below_quorum`. These do not depend on the length. What does depend on it is whether a prompt
  source still answers in time: at 800 ms it has 800 ms in place of 2 s, and a source that
  misses it is counted `unreachable` and fails the test. That is the risk
  story:clock-quorum-test-tolerates-a-slow-loopback-source removed; acceptance 4 measures it.
- The client drain bound of 5 s (`transport.rs` line 1923; `contracts/cli/v1alpha1/owner.md`
  lines 71 to 72: "lets the clients already connected finish for at most 5 seconds") gives the
  clients of an owner that lost its socket or its state directory time to finish, and no more
  than that. The test asserts both sides: the owner does not leave at once, and it leaves at the
  bound. At 1,000 ms with windows of 500 ms and 2,000 ms a missing drain (about 250 ms) and an
  unbounded drain (about 10 s) both still fail.

A release build ignores the three overrides.

What a test can read at each site. Two of the three waits are literals in the middle of a
function today, and the third has a definition the unit-test build does not compile. Each gets a
name a unit test in the same file can read:

- `config.rs`: a function `print_deadline()` beside `Executable::print` returns the helper's value
  for `CONNECTORS_TEST_OVERRIDE_PROVIDER_PRINT_MS` with 10 s as the production value; line 2296
  calls it in place of the literal. A `#[cfg(test)]` test in `config.rs`,
  `print_deadline_is_ten_seconds_without_the_override`, asserts 10 s.
- `transport.rs`: a function `drain_bound()` beside `accept` returns the helper's value for
  `CONNECTORS_TEST_OVERRIDE_OWNER_DRAIN_MS` with 5 s; line 1923 calls it. A test in the file's
  `tests` module (line 4320), `drain_bound_is_five_seconds_without_the_override`, asserts 5 s.
- `clock.rs`: a function `overridable_query_timeout()`, compiled in every build, returns the
  helper's value for `CONNECTORS_TEST_OVERRIDE_CLOCK_QUERY_MS` with the constant `QUERY_TIMEOUT`
  (line 30) as the production value. The `#[cfg(not(test))]` `query_timeout()` has that one call
  as its body. The unit-test build does not call the function outside its test, so it carries an
  allowance for dead code in that build. A `#[cfg(test)]` test in `clock.rs`,
  `query_timeout_is_two_seconds_without_the_override`, asserts that it returns 2 s. No unit test
  can call the `#[cfg(not(test))]` `query_timeout()` itself; acceptance 5 reads its one line with
  a `grep`.

The two functions read the variable through the helper only, so in a unit test, where none of the
variables is set, they return the production value.

The clock query timeout has one source of truth. `query_timeout()` in `clock.rs` (lines 69 to 77)
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

A decision beyond the plan: the 20 repeat runs of `clock_check_below_quorum_refusal` (acceptance
4). Neither the parent nor the plan asks for them. This story adds them because it shortens the
wait that the implemented story made safe, and one green run does not show that 800 ms is enough.
The implementor runs them in this story's tree before the pull request is opened; they are not a
step of the gate and not a step of the lane `slow`, so this story edits neither. The parent story
lists them among the decisions beyond the plan.

Estimate after (computed from the plan's rows; not timed): 8 + 5 + 4 + 1.5 + 4 + 2 = 24.5 s.

## Acceptance

1. The six tests pass, and their durations in the test step of the forge's "repository gate" job
   of this story's pull request sum to at most 37 s. Before: 54.0 s. Estimate: 24.5 s. Bound: 1.5
   × 24.5 = 36.75, rounded up to 37 s. The numbers are read from the durations nextest prints in
   that job.
2. The margins hold in the sources: the walk's three pages take 3 × 1,200 ms against a deadline
   of 3 s; the slow clock source's first answer, which the test's probe takes, comes 500 ms past
   the query timeout the test sets; the tether test's windows are 500 ms and 2,000 ms around a
   drain of 1,000 ms.
3. `clock_check_below_quorum_refusal` states the wait it relies on in a comment beside the value:
   a prompt source has 800 ms to answer.
   `grep -n -B3 'CONNECTORS_TEST_OVERRIDE_CLOCK_QUERY_MS' apps/connectors/tests/local_cli.rs`
   shows the variable, the value 800 and that comment.
4. `clock_check_below_quorum_refusal` passes 20 times of 20:
   `cargo nextest run --locked -p connectors --test local_cli --stress-count 20 -E 'test(=clock_check_below_quorum_refusal)'`
   exits 0 in this story's tree. One failed run of the 20 fails the acceptance; the value of
   800 ms is then raised and the 20 runs are repeated. The run leaves a record: the implementor
   records it on this story with `aep plan artifact evidence <this story> --kind test_result
   --source "<the command above, then the summary line nextest printed>"`; the verb has no other
   field for free text. The flag `--stress-count` is in
   `cargo nextest run --help` of cargo-nextest 0.9.143, the version `.github/workflows/rust-gate.yml`
   pins (line 91) and the one installed on the development host.
5. With no variable set, the three waits are 10 s, 2 s and 5 s. Each site has its own check
   (Change, "What a test can read at each site"):
   - print deadline: the unit test `print_deadline_is_ten_seconds_without_the_override` in
     `config.rs` passes; it asserts that `print_deadline()` returns 10 s.
   - drain bound: the unit test `drain_bound_is_five_seconds_without_the_override` in the `tests`
     module of `transport.rs` passes; it asserts that `drain_bound()` returns 5 s.
   - clock query timeout: the unit test `query_timeout_is_two_seconds_without_the_override` in
     `clock.rs` passes; it asserts that `overridable_query_timeout()` returns 2 s. And
     `grep -n -A3 'cfg(not(test))' crates/connectors-host/src/local/clock.rs` shows
     `query_timeout()` for builds that are not the unit-test build with the one call of
     `overridable_query_timeout()` as its body. That one line is read, not run.
6. The clock variable takes effect in the CLI: `clock_check_below_quorum_refusal` takes under 6 s
   in the test step of the gate job of this story's pull request, read from the duration nextest
   prints. The test passes with and without the variable, because the CLI only ever waits for the
   silent source; the duration tells the two apart. With the variable the waits it chose sum to
   3.7 s (1.3 s of probe and 3 × 0.8 s); without it they sum to 7.3 s (1.3 s and 3 × 2 s). The
   test took 9.0 s in the plan's run with waits of 8.5 s.

   The three unit tests run in the test step of the gate with the rest of `connectors-host`; the
   gate sets none of the three variables.

That a release build ignores the three overrides is held once, for the helper, by
story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build.

The plan names nothing that stays in the lane `slow` for this story.

## Scope

- `apps/connectors/tests/owner_cancel_adversary.rs`: the two tests above.
- `apps/connectors/tests/invoke_walks.rs`: the one test above.
- `apps/connectors/tests/catalog_provider_failure.rs`: the one case above (line 201) and the
  command it builds for itself. `prerequisite` (line 105) is not edited.
  `apps/connectors/tests/support/standin.rs` is not written.
- `apps/connectors/tests/owner_tether_adversary.rs`: the one test above (line 162) and a variant
  of the file's `start_owner` (line 71).
- `apps/connectors/tests/local_cli.rs`, two regions:
  - `clock_check_below_quorum_refusal` (lines 325 to 393);
  - `clock_unavailable` (lines 153 to 171) and one new function beside it. `clock_unavailable`
    keeps its signature; its callers at lines 237 and 312 are not edited.

  Not edited: `command` and `command_from` (lines 40 to 69), which every CLI test of the file
  runs through, and the owner helpers `start_owner` to `start_owner_holding_startup` (lines 935
  to 1024). Two other stories edit other regions of this file:
  story:four-idle-exit-tests-no-longer-span-several-5-s-recovery-sweeps
  (`a_connected_client_keeps_an_owner_past_its_idle_bound`, line 1959, and one statement in
  `start_owner_holding_startup`, lines 1002 to 1004) and
  story:the-100-run-capacity-stress-keeps-10-runs-in-the-gate (the test at line 1691).
- `crates/connectors-host/src/local/config.rs`: the deadline in `Executable::print` (line 2296),
  the function `print_deadline()` beside it and its one unit test.
- `crates/connectors-host/src/local/owner/transport.rs`: the client drain in `accept` (line
  1923), the function `drain_bound()` beside it and one unit test in the `tests` module (line
  4320). Four other stories of this set write this file. The `depends_on` edges put them in one
  order: the helper story, story:four-idle-exit-tests-no-longer-span-several-5-s-recovery-sweeps,
  story:seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait, then this
  story, then story:eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping.
- `crates/connectors-host/src/local/clock.rs`: the `#[cfg(not(test))]` definition of
  `query_timeout()` (lines 74 to 77) and one `#[cfg(test)]` unit test of the constant
  `QUERY_TIMEOUT`, in `clock.rs` itself. story:five-host-unit-tests-shorten-the-waits-they-chose
  writes `clock/tests.rs` and comes first by the edge.

## Not in this story

- `adversary2_the_identification_retry_loop_ends_at_its_deadline_without_flooding_the_owner`:
  story:where-do-the-10-s-of-the-identification-retry-loop-test-go.
- `quorum_below_quorum_names_each_source` in `clock/tests.rs`:
  story:five-host-unit-tests-shorten-the-waits-they-chose.
- The 10 s of `derivation_start_failure` (`config.rs` line 1505), which `connect` reaches
  (`apps/connectors/src/local/endpoint.rs` line 430) and `setup check` does not.
- A step of the lane `slow` or of the gate for the 20 repeat runs.
- The other rows of group M and the tests of every other group.
- The gate failing when its test step runs over 120 s:
  story:the-gates-test-step-ends-within-120-s-on-the-forge.
- Not established: every "after" figure on the forge. Two of the plan's estimates do not follow
  from the values above: the walk takes 3.6 s plus its fixture (about 5.5 s, inferred from 10.9 −
  9.0 = 1.9 s of fixture today, against the plan's 5 s), and the queued read keeps its 3 s
  provider read (about 5.6 s, inferred from 7.0 − 0.4 − 1.0, against the plan's 4 s). The bound of
  acceptance 1 is kept at the plan's sum; with the two inferred figures the sum is about 26.6 s.
  Whether 800 ms is enough for a prompt clock source on a loaded runner is what acceptance 4
  measures, on the implementor's host and not on the forge's runner. How nextest 0.9.143 reports
  a run with `--stress-count` was read in its help and not in a run. Whether a plain
  `cargo nextest run` of this test needs the runner configuration the gate adds for the temporary
  directory (`crates/connectors-build/src/gate.rs` lines 529 to 544) was not established; if the
  test is refused `invalid_configuration`, the implementor runs the command with `TMPDIR` set to
  a directory outside `/tmp` and says so. In which process `setup check` asks
  `installed_provider_runs` (`apps/connectors/src/local.rs` line 773: the CLI, or the owner on its
  behalf) was not followed; the prefix rule of the helper story covers both.

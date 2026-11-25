---
format: aep.planning-md/3
id: story:seven-cli-tests-shorten-their-waits-or-a-production-wait
kind: story
status: draft
title: 'Six CLI tests shorten their own waits or a production wait: 54.0 s become an estimated 32.2 s'
summary: Two tests shorten their own holds, one runs its three rows at once, two get a debug-only override of a production wait (the print deadline and the client drain bound), and one stays as it is
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
- serves: vision:independent-contract-adapters
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
  path: crates/connectors-host/src/local/config.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/transport.rs
revision: 17
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
read in the tree of 0.78.1 and read again at 0.78.2 and on main of 2026-11-25. The sum is computed
from the plan: 64.2 − 10.2 = 54.0 s. The six rows, printed with one decimal, sum to 54.2 s
(inferred: the plan summed the unrounded durations).

- 11.5 s, binary `connectors::owner_cancel_adversary`,
  `apps/connectors/tests/owner_cancel_adversary.rs`,
  `a_read_cancelled_during_its_launch_reads_no_credential_after_the_cancel`: every later launch is
  held 5 s (the test writes `5000` into the fixture file `/work/start_delay_ms`, line 795), the
  test sleeps 1,500 ms between the send of the read and its cancel (line 803), and it sleeps
  until 7 s after the cancel (line 809).
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
  2 s clock query timeout once in each of three CLI rows, one row after another.
- 7.0 s, binary `connectors::owner_cancel_adversary`, its file (line 905),
  `a_queued_read_another_session_cancels_runs_unsent_and_the_running_read_succeeds`: a provider
  read of 3 s (`SLOW`) and sleeps of 700 ms and 1,500 ms.
- 5.6 s, binary `connectors::owner_tether_adversary`,
  `apps/connectors/tests/owner_tether_adversary.rs`,
  `a_lost_owner_waits_for_a_connected_client_and_no_longer_than_the_drain_bound`: it asserts no
  exit within 2 s and an exit within the 5 s client drain bound plus 2 s.

## The rule for every value below

A forge runner under load took more than 1.5 s for work that takes 0.2 s on the development host
(pull request 296). On 2026-11-25 a Windows runner missed a clock query deadline of 200 ms in a
unit test of `connectors-host` (pull request 303). So:

- a window in which the test's subject must finish work is at least 2 s wide;
- or the test is built so that a slow machine makes it slower or its check weaker, and not red.

Revision 10 of this story had four windows under 2 s (the launch hold against the cancel, a page
of the walk, the clock query timeout, the drain's exit window). The design critic and the
parallel-safety critic refused it for them. This revision changes the four and drops the clock
override.

## Change

Three tests change values or the order of their own steps. Each value is this story's choice.

1. `a_read_cancelled_during_its_launch_reads_no_credential_after_the_cancel`
   (`owner_cancel_adversary.rs`):
   - the hold becomes 3,000 ms (line 795; 5,000 ms today);
   - the sleep between the send of read 4 and its cancel becomes 1,000 ms (line 803; 1,500 ms
     today). The cancel lands 2 s before the hold ends (3.5 s today);
   - the sleep after the cancel ends 4 s after the cancel (line 809; 7 s today): the 2 s the hold
     still has, plus 2 s (today 3.5 s plus 3.5 s). The 2 s after the hold outlast what a launch
     that was not cancelled does next: it opens the credential and sends the request. That step
     was not timed.

   On a slow machine the read reaches its launch later. The cancel then meets the read before
   its launch, and the last sleep may end before the hold does. Both make the check weaker; the
   two assertions (no request reached the provider, no credential was opened) cannot fail for it.
   The waits the test chose sum to 6.1 s (0.1 + 1.0 + 1.0 + 4.0) in place of 9.6 s. Estimate
   after: 8.0 s (11.5 − 9.6 + 6.1). The comments of the test that name 5 s (lines 768 to 770,
   793 and 808) name the new values.
2. `each_page_of_a_walk_has_the_named_deadline_to_itself` (`invoke_walks.rs`): `page_ms` 1,500
   for 3 pages under `--deadline-seconds 4`. A page keeps 2,500 ms for everything but the
   provider (5 s today). The third page cannot answer before 4,500 ms, which is 500 ms past one
   cutoff of 4 s for the whole walk (today 9 s against 8 s); on a slow machine that distance only
   grows. The doc comment of the test names the new values. Estimate after: 6.4 s (4.5 s of
   pages, and 1.9 s of fixture as today: 10.9 − 9.0).
3. `clock_check_below_quorum_refusal` (`local_cli.rs`, lines 325 to 393): the three rows run at
   the same time, one thread each (`std::thread::scope`), and each row has a silent source of its
   own. The three roots (`clock_root`, lines 95 to 101, which runs `setup init`, a CLI process)
   and the three silent sources are built before the threads start; a thread runs only
   `clock_unavailable` and the row's assertions, so no row spends its 2 s while another starts
   processes for its setup. Nothing else of the test changes: the same four other sources, the same three tables,
   the same assertions, and the assertion "the silent source was asked once per check" reads the
   row's own silent source (0 before, 1 after). The clock query timeout stays the production
   2 s: the test sets no variable, and `crates/connectors-host/src/local/clock.rs` is not
   edited. The slow source's first answer stays 2,500 ms late, 500 ms past the query timeout,
   and the test's probe (`answering`, lines 139 to 150) still takes it before the rows start; if
   the probe is removed, every row meets the late answer and counts the source `unreachable`.
   The test waits 2.5 s for the probe and then one query timeout of 2 s, in place of three.
   Estimate after: 5.0 s (9.0 − 8.5 + 4.5).
   A prompt source keeps the 2 s it has today. What is new: three CLI processes ask it within
   one window, and its one thread answers three requests there. `clock_unavailable`, `command`
   and `command_from` are not edited.

One test is not changed:
`a_queued_read_another_session_cancels_runs_unsent_and_the_running_read_succeeds` keeps its
7.0 s. Revision 10 cut its sleep before the cancel from 700 ms to 300 ms and its sleep before the
count from 1,500 ms to 500 ms. The first gives the owner time to queue the second session's read
behind the running one; whether a cancel that arrives before the read is queued takes the same
path was not established, and if it does not, a slow machine turns the test red. The second is a
negative check (the cancelled read must not be dispatched after the running one ends), and what
it outlasts was not timed. `SLOW` (3 s, line 274) is the read that must still run when the cancel
arrives, and other tests of the file use it. A change of this test waits for those two answers
and is not in this story.

Two tests wait on a production wait. Each gets one override, two in all, each through the helper
of story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build:

4. `CONNECTORS_TEST_OVERRIDE_PROVIDER_PRINT_MS`: the print deadline in
   `crates/connectors-host/src/local/config.rs` (line 2296, in `Executable::print`, a literal of
   10 s). `setup check` asks `installed_provider_runs` (line 1597), which prints the provider's
   `--version` (line 1627) and waits at line 2296. The literal of 10 s at line 1505
   (`derivation_start_failure`) is the handshake deadline of a derivation's child; `setup check`
   does not reach it, and it is out of this story.
   `a_provider_past_its_deadline_is_named_by_setup_check` sets 500 ms, and its upper bound (15 s
   today, lines 211 to 215) becomes 6 s. The case has 5.5 s for everything but the deadline (5 s
   today), and it still fails when the check waits out 10 s, by 4 s. The case runs the CLI today
   through `prerequisite` (line 105) and `Installation::run`
   (`apps/connectors/tests/support/standin.rs` line 451), which clears the environment. It runs
   `setup check` with a command of its own instead, built in `catalog_provider_failure.rs`. The
   command does what `run` does and adds the one variable:
   - the binary is the installation's copy of the CLI, `bin/connectors` under the installation's
     root (`standin.rs` lines 430 to 436), because the provider is found beside the CLI; it is
     not `CARGO_BIN_EXE_connectors`. The field `root` of `Installation` is public (line 414),
     so the case reads the directory from it;
   - `env_clear()`, a null standard input, and the retry on ETXTBSY of `run` (lines 454 to 472):
     the copy is a file the test wrote;
   - the output is read the way `prerequisite` reads it (lines 105 to 117): the entry
     `installed_provider` of `prerequisites`.

   `prerequisite` and `support/standin.rs` are not edited. Estimate after: 1.5 s (the plan).
   `setup check` asks `installed_provider_runs` in the CLI process
   (`apps/connectors/src/local.rs` line 773), so the variable on the case's command is enough,
   and the case's comment says so. The variable shortens every call of `Executable::print` in
   that process, the print of `--print-local-bootstrap` (line 1503) included, and by the helper
   story's prefix rule an owner that this CLI run starts gets it too. The case's installation
   has one provider, and that provider hangs, so no healthy print is cut short. The case's doc
   comment (lines 198 to 199: "ten-second deadline", "near ten seconds") names the 500 ms.
5. `CONNECTORS_TEST_OVERRIDE_OWNER_DRAIN_MS`: the client drain bound in
   `crates/connectors-host/src/local/owner/transport.rs` (line 1923, in `accept`, a literal of
   5 s). `a_lost_owner_waits_for_a_connected_client_and_no_longer_than_the_drain_bound` starts its
   owner with 2,000 ms. After the socket is removed the owner notices within its check interval
   (200 ms: `EVERY`, `crates/connectors-host/src/local/owner/tether.rs` line 14; the contract
   allows at most 250 ms) and leaves one drain later: at about 2,200 ms. The test changes three
   values:
   - the sleep between the client's connect and the removal of the socket becomes 2,000 ms
     (line 169; 300 ms today). In it the owner must accept and count the client. An owner that
     has not counted the client when it notices the loss has nothing to drain and leaves at
     about 200 ms, and the first assertion below is red; 300 ms is under the 2 s of the rule. The
     client is counted for the owner's greeting deadline of 10 s from its connect, so it is still
     counted when the drain ends (about 4.2 s after the connect);
   - no exit within 1,000 ms after the removal (2 s today). An owner without a drain leaves at
     about 200 ms. An owner that has counted the client cannot leave before its drain of 2 s;
   - an exit within 4,200 ms after the removal: the drain, the interval and 2 s (today the drain
     of 5 s and 2 s). An owner with an unbounded drain leaves when the client's greeting
     deadline ends, about 8 s after the removal. An owner that does not read the variable leaves
     between 5.0 s and 5.2 s, at least 0.8 s past the window, so the test also fails when the
     override does not reach the owner. The remaining wait is computed with `saturating_sub`
     (today `BOUND + 2 s − removed.elapsed()`, a subtraction that panics below zero).

   The constant `BOUND` of the file (5 s) stays for the other tests. The test starts its owner
   today through the file's `start_owner` (lines 71 to 97), which sets no variable; a variant of
   it beside it sets the one variable with `Command::env` on the owner's command, never with
   `set_var` in the test process, and `start_owner` keeps its signature and its other callers.
   The doc comment of the test (lines 156 to 161) and the message of its second assertion (line
   184, "bounded at 5 s") name the 2 s of the override. Estimate after: 4.3 s (2.0 s of the
   test's own sleep, 2.2 s, and what the test takes beside its waits today: 5.6 − 5.55).

What each of the two waits protects, and why the shorter test value is safe:

- The print deadline of 10 s (`config.rs` line 2296; the comment at lines 2247 to 2254: "at most
  4 MiB and ten seconds") ends a command that asks the installed catalog provider to print
  something when the provider hangs: the provider's process group is killed and the failure is
  named `deadline`. The case asserts that name and that the check is not held past the cutoff.
  Both hold at 500 ms as at 10 s, because the provider of the case never answers.
- The client drain bound of 5 s (`transport.rs` line 1923; `contracts/cli/v1alpha1/owner.md`
  lines 71 to 72: "lets the clients already connected finish for at most 5 seconds") gives the
  clients of an owner that lost its socket or its state directory time to finish, and no more
  than that. The test asserts both sides: the owner does not leave at once, and it leaves at the
  bound. The client of the test never finishes, so the length of the drain changes only when the
  owner leaves.

A release build ignores the two overrides.

What a test can read at each site. Both waits are literals in the middle of a function today.
Each gets a name a unit test in the same file can read:

- `config.rs`: a function `print_deadline()` beside `Executable::print` returns the helper's value
  for `CONNECTORS_TEST_OVERRIDE_PROVIDER_PRINT_MS` with 10 s as the production value; line 2296
  calls it in place of the literal. A `#[cfg(test)]` test in `config.rs`,
  `print_deadline_is_ten_seconds_without_the_override`, asserts 10 s.
- `transport.rs`: a function `drain_bound()` beside `accept` returns the helper's value for
  `CONNECTORS_TEST_OVERRIDE_OWNER_DRAIN_MS` with 5 s; line 1923 calls it. A test in the file's
  `tests` module (line 4320), `drain_bound_is_five_seconds_without_the_override`, asserts 5 s.

The two functions read the variable through the helper only, so in a unit test, where neither
variable is set, they return the production value.

Decisions beyond the plan:

- The plan gives `clock_check_below_quorum_refusal` an override of the clock query timeout
  (800 ms in revision 10). This story gives it none and runs its rows at once. The reason: at
  800 ms a prompt source that answers late is counted `unreachable` and the test is red, which is
  the failure story:clock-quorum-test-tolerates-a-slow-loopback-source removed from this test;
  20 repeat runs on the development host, which revision 10 asked for, do not show what a forge
  runner does. With the rows at once a slow machine makes the test slower. The 20 repeat runs are
  no longer asked for.
- The two functions `print_deadline()` and `drain_bound()` and their unit tests: the plan names
  the overrides and no function. They exist so that a test holds the production value.
- The plan cuts the two sleeps of the queued-read test. This story leaves the test as it is.

Estimate after (computed, not timed): 8.0 + 6.4 + 5.0 + 1.5 + 4.3 = 25.2 s for the five tests that
change (47.0 s today: 54.0 − 7.0), and 32.2 s for the six with the 7.0 s of the one that stays.
The plan's figure for the six was 24.5 s.

## Acceptance

1. The five tests that change pass, and their durations in the test step of the forge's
   "repository gate" job of this story's pull request sum to at most 38 s. Before: 47.0 s.
   Estimate: 25.2 s. Bound: 1.5 × 25.2 = 37.8, rounded up to 38 s. The numbers are read from the
   durations nextest prints in that job.
2. The margins are in the sources, each with a comment beside the value that names it, and no
   doc comment or assertion message of the five tests names a value the test no longer uses
   (the launch test's 5 s, the provider case's ten seconds, the tether test's 5 s for its own
   drain):
   - the launch test: a hold of 3,000 ms, a sleep of 1,000 ms before the cancel, and a last
     sleep that ends 4 s after the cancel;
   - the walk: `page_ms` 1,500 and `--deadline-seconds 4`, and its doc comment (lines 775 to
     781) with the new values;
   - the clock test: the slow source's first answer 2,500 ms late, and no variable set on the
     CLI: `grep -c 'CONNECTORS_TEST_OVERRIDE' apps/connectors/tests/local_cli.rs` prints 0 on the
     tree of this story unless another story of the set has added one in another test, and then
     no line of `clock_check_below_quorum_refusal` is among the matches;
   - the tether test: a sleep of 2,000 ms before the removal, a drain of 2,000 ms set with
     `Command::env`, windows of 1,000 ms and 4,200 ms, and the remaining wait of the second
     window computed with `saturating_sub`; the diff changes no line of `start_owner` (lines 71
     to 97);
   - the provider case: a deadline of 500 ms and an upper bound of 6 s; its command has
     `env_clear()`, a null standard input and the retry on ETXTBSY; the diff changes no line of
     `prerequisite` (lines 105 to 117).
3. The rows of `clock_check_below_quorum_refusal` run at the same time: the test takes under
   8.5 s in the test step of the gate job of this story's pull request, read from the duration
   nextest prints. Rows one after another cannot end under 8.5 s (2.5 s of probe and 3 × 2 s).
   Read in the diff: the three calls of `clock_root` and the three silent sources come before
   `std::thread::scope`; a thread runs only `clock_unavailable` and the assertions of its row;
   the four other sources, the three tables and the assertions of each row are those of today,
   and each row asserts that its own silent source was asked once.
4. With no variable set, the two waits are 10 s and 5 s:
   - the unit test `print_deadline_is_ten_seconds_without_the_override` in `config.rs` passes; it
     asserts that `print_deadline()` returns 10 s;
   - the unit test `drain_bound_is_five_seconds_without_the_override` in the `tests` module of
     `transport.rs` passes; it asserts that `drain_bound()` returns 5 s.

   Both run in the test step of the gate with the rest of `connectors-host`; the gate sets
   neither variable.
5. Each override reaches the process that waits. Both tests fail when it does not: the provider
   case waits 10 s against its bound of 6 s, and the owner of the tether test leaves no earlier
   than 5.0 s against its window of 4,200 ms. Their pass in the gate job is the check.
6. `git diff --stat origin/main...HEAD -- crates/connectors-host/src/local/clock.rs
   apps/connectors/tests/support/standin.rs` prints nothing on the tree of this story, and the
   diff of `owner_cancel_adversary.rs` touches no line of
   `a_queued_read_another_session_cancels_runs_unsent_and_the_running_read_succeeds`.

That a release build ignores the two overrides is held once, for the helper, by
story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build.

The plan names nothing that stays in the lane `slow` for this story.

## Scope

- `apps/connectors/tests/owner_cancel_adversary.rs`: the launch test (lines 771 to 822). The
  queued-read test (line 905) is not edited.
- `apps/connectors/tests/invoke_walks.rs`: the one test above and its doc comment.
- `apps/connectors/tests/catalog_provider_failure.rs`: the one case above (line 201) and the
  command it builds for itself. `prerequisite` (line 105) is not edited.
  `apps/connectors/tests/support/standin.rs` is not written.
- `apps/connectors/tests/owner_tether_adversary.rs`: the one test above (line 162) and a variant
  of the file's `start_owner` (line 71).
- `apps/connectors/tests/local_cli.rs`, one region: `clock_check_below_quorum_refusal` (lines
  325 to 393).

  Not edited: `clock_unavailable` (lines 153 to 171), `command` and `command_from` (lines 40 to
  69), which every CLI test of the file runs through, and the owner helpers `start_owner` to
  `start_owner_holding_startup` (lines 935 to 1024). Two other stories edit other regions of
  this file: story:four-idle-exit-tests-no-longer-span-several-5-s-recovery-sweeps
  (`a_connected_client_keeps_an_owner_past_its_idle_bound`, line 1959, and one statement in
  `start_owner_holding_startup`, lines 1002 to 1004) and
  story:the-100-run-capacity-stress-keeps-10-runs-in-the-gate (the test at line 1691).
- `crates/connectors-host/src/local/config.rs`: the deadline in `Executable::print` (line 2296),
  the function `print_deadline()` beside it and its one unit test. Two draft stories outside this
  set record the file with no edge to this story: story:definition-files-in-user-config and
  story:openrouter-inference-key-profile. Their regions (the loading of definitions, `migrate`)
  are not `Executable::print`.
- `crates/connectors-host/src/local/owner/transport.rs`: the client drain in `accept` (line
  1923), the function `drain_bound()` beside it and one unit test in the `tests` module (line
  4320). Four other stories of this set write this file. The `depends_on` edges put them in one
  order: the helper story, story:four-idle-exit-tests-no-longer-span-several-5-s-recovery-sweeps,
  story:seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait, then this
  story, then story:eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping. The active
  story:connections-list-answers-through-the-owner writes the file too, with no edge; its region
  (a request kind) is not the drain in `accept`.

`crates/connectors-host/src/local/clock.rs` left the scope after revision 10, and with it the edge
to story:five-host-unit-tests-shorten-the-waits-they-chose, which ordered the two stories' writes
of the clock code.

## Not in this story

- `adversary2_the_identification_retry_loop_ends_at_its_deadline_without_flooding_the_owner`:
  story:where-do-the-10-s-of-the-identification-retry-loop-test-go.
- `a_queued_read_another_session_cancels_runs_unsent_and_the_running_read_succeeds` (see Change).
- An override of the clock query timeout, and any edit of `clock.rs`.
- The 10 s of `derivation_start_failure` (`config.rs` line 1505), which `connect` reaches
  (`apps/connectors/src/local/endpoint.rs` line 430) and `setup check` does not.
- The other rows of group M and the tests of every other group.
- The gate failing when its test step runs over 120 s:
  story:the-gates-test-step-ends-within-120-s-on-the-forge.
- Not established: every "after" figure, on the development host and on the forge; none was
  timed. Whether three CLI processes at once on a loaded runner make a prompt clock source miss
  the 2 s it has in each of them: the source's thread answers three requests in the window where
  it answers one today. Whether a cancel that arrives before the owner has queued its read takes
  the path of a queued read, and how long the owner takes to dispatch the next queued job (both
  for the test this story leaves as it is). Whether a plain
  `cargo nextest run` of the clock test needs the runner configuration the gate adds for the
  temporary directory (`crates/connectors-build/src/gate.rs` lines 529 to 544); if the test is
  refused `invalid_configuration`, the implementor runs the command with `TMPDIR` set to a
  directory outside `/tmp` and says so.

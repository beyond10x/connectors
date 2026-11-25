---
format: aep.planning-md/3
id: story:eight-adapter-and-catalog-tests-drop-waits-that-prove-nothing
kind: story
status: active
title: 'Seven adapter and catalog tests drop waits that prove nothing: 73.0 s become an estimated 16.6 s'
summary: Fixed deadlines, poll timeouts and a host walk per test; five get shorter, one stays and one is removed
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: adapters/catalog/tests/adversary_git_relay.rs
- confidence: cited
  path: adapters/catalog/tests/adversary_one_binary.rs
- confidence: cited
  path: adapters/catalog/tests/local_runtime/cli_journey.rs
- confidence: cited
  path: adapters/catalog/tests/local_runtime/guarded_merge.rs
- confidence: cited
  path: adapters/catalog/tests/security_git_relay.rs
- confidence: cited
  path: adapters/kubernetes/tests/exec_credential_security.rs
- confidence: cited
  path: crates/connectors-catalog/tests/bundle_adversary_2.rs
revision: 16
transitions:
- {from: "draft", to: "proposed", at: "2026-11-25T02:53:29Z", actor: "human:operator", revision: 13}
- {from: "proposed", to: "active", at: "2026-11-25T02:53:30Z", actor: "human:operator", revision: 14}
---
## Defect

Seven tests of the adapters and of the catalog crate wait on values the tests chose, or do host
work per test. Together they take 73.0 s of the test step.

This story was drafted with eight tests (80.0 s). The eighth,
`every_page_is_one_the_model_admits` (7.0 s), went to
story:eighteen-tests-take-a-template-store-or-jail-built-once-per-run, which builds the generated
schemas once per run. The id of this story keeps "eight"; an id cannot be changed.

Source of the durations: the ones nextest printed in the forge's gate job 138270416028 of pull
request 291, 2026-11-24, as listed in test-plan:where-the-gates-test-step-spends-its-time (three
rows of group L, one row of group N, three rows of group M). The plan read the sources at 0.77.2.
The function names and lines below were read in the tree of 0.78.1 and again at 0.78.2. The sum is
computed from the plan: 80.0 − 7.0 = 73.0 s.

Waits that prove nothing more than a shorter one, or nothing under nextest (group L):

- 20.0 s, binary `connectors-catalog::main`, `crates/connectors-catalog/tests/bundle_adversary_2.rs`
  (lines 93 to 100), `a_bundle_file_swapped_for_a_link_after_the_check_is_not_followed`: a race
  loop calls `load` until a fixed deadline of 20 s unless a link is followed. A second thread
  swaps the file until it is stopped.
- 10.0 s, binary `connectors-catalog-provider::main`,
  `adapters/catalog/tests/local_runtime/guarded_merge.rs`,
  `catalog_fault_cleanup_observation_handle_cannot_authorize_signal`: two `poll` timeouts of
  5000 ms in the helper `finish_filtered_owner` (the plan; the helper holds three `libc::poll(…,
  5000)` calls, lines 94, 111 and 123).
- 10.0 s, binary `connectors-catalog-provider::main`,
  `adapters/catalog/tests/adversary_one_binary.rs` (lines 158 to 170),
  `no_suite_sees_the_installation_another_suite_sets`: it polls an environment variable for 10 s.
  Under nextest each test is its own process, so no other suite can write the variable and the test
  cannot fail.

Host work per test (group N):

- 11.6 s, binary `connectors-kubernetes::exec_credential_security`,
  `adapters/kubernetes/tests/exec_credential_security.rs` (lines 468 to 515; the walk is lines 471
  to 501), `exec_plugin_file_owned_by_another_user_is_refused_before_the_digest`: it walks `/etc`,
  `/var/lib`, `/usr/share` and `/usr/include` to depth 3 for a readable file under 1 MiB whose
  owner is neither root nor the test's user and that group and others cannot write. The time
  depends on the host. When the walk finds no such file the test prints `skipped` and passes.

Test-chosen waits and holds (group M), all in binary `connectors-catalog-provider::main`:

- 8.3 s, `adapters/catalog/tests/local_runtime.rs`,
  `a_rate_limited_read_waits_once_for_a_delay_that_fits_and_names_any_other`: a provider
  Retry-After of 1 s taken twice plus a 3 s deadline case. Retry-After is whole seconds.
- 6.7 s, `adapters/catalog/tests/adversary_git_relay.rs`,
  `a_push_the_forge_applies_after_the_caller_hung_up_is_still_logged`: `hold_push_ms` 3000 plus a
  sleep of 3 s.
- 6.4 s, `adapters/catalog/tests/security_git_relay.rs`,
  `a_timed_out_pre_push_check_leaves_no_process_behind`: the check runs
  `(sleep 3; : > /srv/work/survived) … & sleep 8` under `timeout_seconds: 1`; the test then sleeps
  5 s and looks for the file.

## Change

The story adds no override of a production wait and no seam in product code. Each value below is
this story's choice unless it says "the plan".

1. `a_bundle_file_swapped_for_a_link_after_the_check_is_not_followed`: the loop ends after 1,000
   calls of `load` or after 2 s, whichever comes first, in place of 20 s. The test asserts that
   the swapping thread completed at least 100 swaps in that time, so the race was run. Estimate
   after: 2 s (the plan).
2. `catalog_fault_cleanup_observation_handle_cannot_authorize_signal`: `finish_filtered_owner`
   takes the poll timeout as a parameter. This test passes 100 ms for each of the three polls in
   place of 5,000 ms. Estimate after: 0.3 s (the plan; 3 × 100 ms). The helper has a third caller,
   `Cli::shutdown` in `adapters/catalog/tests/local_runtime/cli_journey.rs` (line 271). It passes
   5,000 ms, so nothing changes for the tests that end through it.
3. `no_suite_sees_the_installation_another_suite_sets`: the test is removed, with its comment
   (lines 152 to 157). The reason, read at 0.78.2:
   - The test fails only when another test of the same process sets `CONNECTORS_INSTALLATION`
     while it polls. Its comment says so: "This case is red when a module of `main` sets the
     variable".
   - No module of `main` sets the variable. The one `set_var` under `adapters/catalog/tests/` is
     in `derived_instance_adversary.rs` (line 59), which is its own `[[test]]` target
     (`adapters/catalog/Cargo.toml`) and so its own process.
   - So no filter names "the suites involved": a run of this test with any subset of `main` in one
     process passes for the same reason the whole binary passes. Only a run of the whole binary in
     one process would fail on a later module that sets the variable. That binary has 1,122 tests
     with 363.5 s of summed time in the forge run (computed from the plan: 9 tests of 5 s or more
     with 77.2 s, 75 tests in the band with 166.8 s, 1,038 tests under 1 s with 119.5 s). Inferred,
     not run: such a run is not well under 60 s.
   - The gate and the lane `slow` both run one test per process, so in neither can the test fail.
   What is lost: nothing fails when a later module of `main` sets the variable for its process.
   The file `adversary_one_binary.rs` stays with its other test. Estimate after: 0 s.
4. `exec_plugin_file_owned_by_another_user_is_refused_before_the_digest`: the test tries a list of
   at most 3 candidate paths before the walk and walks only when none of them fits. When the
   variable `GITHUB_ACTIONS` is set and the path came from the walk, the test fails, and the
   failure names the path the walk found. The first candidate is that path: the implementor reads
   it from the failure of the first gate job and adds it to the list. When neither the list nor
   the walk gives a file, the test prints `skipped` and passes, as today. Estimate after: 1 s (the
   plan). Who owns the list afterwards: whoever changes the runner image of the gate job (the key
   `runs-on` of the job `gate` in `.github/workflows/rust-gate.yml`, line 33, `ubuntu-24.04`
   today) updates the list in the same change. The failure message of the test names the list
   with its file, so a red gate job after a change of the image says where to add the path.
5. `a_push_the_forge_applies_after_the_caller_hung_up_is_still_logged`: `hold_push_ms` 3000
   becomes 500, and the sleep of 3 s becomes 700 ms, 200 ms past the hold. Estimate after: 2 s
   (the plan).
6. `a_timed_out_pre_push_check_leaves_no_process_behind`: the check becomes
   `(sleep 2; : > /srv/work/survived) … & sleep 4`; `timeout_seconds` stays 1; the test sleeps 2 s
   in place of 5 s. A survivor would write its file 2 s after the start, 1 s after the timeout;
   the test looks 2 s after the timeout, which is 1 s later. Estimate after: 3 s (the plan).
7. `a_rate_limited_read_waits_once_for_a_delay_that_fits_and_names_any_other`: no seam.
   Retry-After is whole seconds, so the test stays at 8.3 s. The plan lists its file as touched
   and names no change in it.

Estimate after (computed from the plan's rows; not timed): 2 + 0.3 + 0 + 1 + 2 + 3 + 8.3 = 16.6 s.

## Acceptance

1. The six tests that stay pass, and their durations in the test step of the forge's "repository
   gate" job of this story's pull request sum to at most 25 s. The seventh no longer exists.
   Before: 73.0 s for the seven. Estimate: 16.6 s. Bound: 1.5 × 16.6 = 24.9, rounded up to 25 s.
   The numbers are read from the durations nextest prints in that job.
2. The race was run: the bundle test fails when the swapping thread completed fewer than 100
   swaps. The check: the test counts the completed swaps in a variable named `swaps` and asserts
   `swaps >= 100`, and
   `grep -n 'swaps >= 100' crates/connectors-catalog/tests/bundle_adversary_2.rs` prints one line,
   the `assert!` of this test. On the tree before this story the command prints nothing: the test
   counts no swaps today (read at 0.78.2, lines 74 to 91).
3. In the gate job of this story's pull request the kubernetes test passes. `GITHUB_ACTIONS` is
   set in that job, and the test fails there when its path came from the walk. So its pass, with
   the duration of acceptance 1, shows that a candidate of the list fit.
4. `grep -rn no_suite_sees_the_installation_another_suite_sets adapters/ crates/ apps/` prints
   nothing on the tree of this story.
5. `grep -n 'finish_filtered_owner' -r adapters/catalog/tests` shows the definition and three calls,
   each call with a timeout argument: 100 ms twice in `guarded_merge.rs` and 5,000 ms in
   `cli_journey.rs`.

## Scope

- `crates/connectors-catalog/tests/bundle_adversary_2.rs`: the one test above.
- `adapters/catalog/tests/adversary_one_binary.rs`: the one test above is removed.
- `adapters/catalog/tests/local_runtime/guarded_merge.rs`: `finish_filtered_owner` and its two
  callers in this file (lines 171 and 193).
- `adapters/catalog/tests/local_runtime/cli_journey.rs`: the one call at line 271 gets the timeout
  argument.
- `adapters/catalog/tests/adversary_git_relay.rs`: the one test above.
- `adapters/catalog/tests/security_git_relay.rs`: the one test above.
- `adapters/kubernetes/tests/exec_credential_security.rs`: the one test above.

The plan lists `adapters/catalog/tests/local_runtime.rs`. No change is named for it, and it is not
in this story's scope.

No other story of this set writes these seven files.
story:the-catalog-provider-tests-read-a-committed-lowering-of-the-released-definition writes other
files of `adapters/catalog/tests/` (three test files, a new module and `main.rs`).

This story adds no test to the lane `slow` and reads nothing of
story:the-lane-slow-runs-the-ignored-timing-family. Its `depends_on` edge to that story is
removed.

## Not in this story

- `every_page_is_one_the_model_admits` and `adapters/sql/tests/ess_model.rs`:
  story:eighteen-tests-take-a-template-store-or-jail-built-once-per-run.
- `how_long_sixty_four_idle_connections_delay_the_redirect`, the fourth row of group L:
  story:five-host-unit-tests-shorten-the-waits-they-chose.
- A check that replaces the removed test: a test that reads the sources of the modules of `main`
  and fails on a `set_var` of `CONNECTORS_INSTALLATION` would hold the same rule in under a
  second. It is not drafted. The plan asked for a move to the lane, not for a new check.
- A third step of the lane `slow` that runs the whole binary `main` in one process.
- The other rows of group M and the tests of every other group.
- The gate failing when its test step runs over 120 s:
  story:the-gates-test-step-ends-within-120-s-on-the-forge.
- Not established:
  - The candidate paths of the kubernetes test. They depend on the runner's image, which was not
    read. Whether the walk finds a file on the forge's runner at all was not read either: the
    11.6 s say that it walks, not what it finds. If it finds none, the test keeps walking and
    passing as skipped, acceptance 3 shows nothing and the sum of acceptance 1 is 27.2 s, over its
    bound.
  - How many swaps and calls of `load` the bundle test completes in 2 s. The figures 1,000 and
    100 are not measured.
  - How long the whole binary `main` runs in one process. The 363.5 s are a sum over tests that
    each ran in a process of their own.
  - Every "after" figure on the forge.

## As built (2026-11-25)

Where the implementation differs from the Change section, and what each difference keeps.
Durations are from the build host of the shared pool: one run before the change, and the range of
ten runs in a row after it. The forge's are read from the pull request's gate job.

1. `bundle_adversary_2.rs`, the swap race (Change 1). The figures of the Change section do not
   hold. 1,000 calls of `load` take 12 ms to 51 ms, and in 3 of 10 runs the swapping thread had
   completed fewer than 100 swaps by then. A count of swaps does not show that the race ran
   either: the adversary read ran a loop of 1,000 calls and 100 swaps against a `load` that
   checks the file and then reads it by path, the defect this test file was written against. That
   loop missed the defect in 8 of 240 runs, and in 210 of 240 with both threads on one CPU.
   The loop as built counts the calls of `load` during which the swap counter advanced. It ends at
   the first followed link, at 500 such calls, or after 20 s, and the test asserts that the 500
   were reached. Against the defective `load` it caught the defect in 40 of 40 loops on any CPU
   and in 40 of 40 on one CPU; the most overlapped calls before the catch were 207. Against the
   product's `load` it reached 500 in 40 of 40 loops, in 32 ms to 279 ms on any CPU and in
   0.2 s to 5.3 s on one CPU. The defective `load` and those loops are not in the change: a test
   that asserts a catch in every run of a race fails by chance. Before: 20.021 s. After, 20 runs
   in a row: 0.037 s to 0.238 s.
2. The push after a hang-up (Change 5). The 3 s sleep is not a 700 ms sleep. It is a poll for the
   outcome line in the log every 50 ms, for at most 3 s. The sleep starts after the forge applied
   the push, so its length is the time the relay has to write the line. The step took at most
   49 ms. `hold_push_ms` is 500 in place of 3,000: 5 times the 100 ms poll step before the
   hang-up. A late hang-up makes the test weaker, not red. Before: 7.674 s. After: 1.172 s to
   2.532 s.
3. The kubernetes test (Change 4). The list `CANDIDATES` is empty in the first commit of this
   story's pull request, as the Change section says: the first gate job fails at this test. In
   that failing case only, the walk goes on until it has up to 3 fitting files, and the failure
   names them in the order found. They are added in a later commit. A passing run walks no
   further than before. The adversary read notes what this design costs: when the image behind
   the runner's label changes and none of the listed files is left, every gate job fails at this
   test until the list is changed, and the local gate does not show it, because `GITHUB_ACTIONS`
   is not set there.
4. The timed-out check keeps a wait. `sleep 2` and `sleep 4` in the check, and the test sleeps
   2 s in place of 5 s. The check outlasts the 1 s timeout 4 times. The relay kills the check's
   process group once, at the timeout, and a survivor would write 1 s later. The test looks at
   least 1 s after that write. Without the wait the assertion holds for any relay. Before:
   7.245 s. After: 3.580 s to 4.864 s.
5. The guarded merge: both poll timeouts are 100 ms in place of 5,000 ms. Both expire by design
   and both assertions hold for any length. Before: 10.050 s. After: 0.210 s to 0.281 s.
6. `no_suite_sees_the_installation_another_suite_sets` is removed with its comment (10.020 s).
7. The rate-limited read is unchanged: 8.5 s, because `Retry-After` counts whole seconds.

Sum of the six remaining tests on the build host: 14.3 s. Before, for seven: 64.3 s. Ten runs in
a row of the five changed tests: 10 of 10 passed.

Found and not changed by this story:

- On the build host of the pool the kubernetes test prints
  `skipped: no readable file owned by another user`. A run there never reaches its assertion. In
  a user namespace (`unshare -r`) on that host the walk finds a file and the assertion runs: 10
  of 10 passed.
- A check that starts a process in a new session (`setsid`) outlives the relay's refusal: the
  relay kills the check's process group only (`adapters/catalog/src/served/git.rs`). Seen in 10
  of 10 runs of a temporary probe that is not in this change.

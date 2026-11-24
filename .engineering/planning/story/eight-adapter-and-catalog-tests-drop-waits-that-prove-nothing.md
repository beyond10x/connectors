---
format: aep.planning-md/3
id: story:eight-adapter-and-catalog-tests-drop-waits-that-prove-nothing
kind: story
status: draft
title: 'Seven adapter and catalog tests drop waits that prove nothing: 73.0 s become an estimated 16.6 s'
summary: Fixed deadlines, poll timeouts and a host walk per test; five get shorter, one stays and one moves to the lane slow
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
- serves: vision:independent-contract-adapters
- depends_on: story:the-lane-slow-runs-the-ignored-timing-family
scope:
- confidence: cited
  path: adapters/catalog/tests/adversary_git_relay.rs
- confidence: cited
  path: adapters/catalog/tests/adversary_one_binary.rs
- confidence: cited
  path: adapters/catalog/tests/local_runtime.rs
- confidence: cited
  path: adapters/catalog/tests/local_runtime/guarded_merge.rs
- confidence: cited
  path: adapters/catalog/tests/security_git_relay.rs
- confidence: cited
  path: adapters/kubernetes/tests/exec_credential_security.rs
- confidence: cited
  path: crates/connectors-catalog/tests/bundle_adversary_2.rs
revision: 6
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
The function names and lines below were read in the tree of 0.78.1. The sum is computed from the
plan: 80.0 − 7.0 = 73.0 s.

Waits that prove nothing more than a shorter one, or nothing under nextest (group L):

- 20.0 s, binary `connectors-catalog::main`, `crates/connectors-catalog/tests/bundle_adversary_2.rs`
  (lines 93 to 100), `a_bundle_file_swapped_for_a_link_after_the_check_is_not_followed`: a race
  loop calls `load` until a fixed deadline of 20 s unless a link is followed. A second thread
  swaps the file until it is stopped.
- 10.0 s, binary `connectors-catalog-provider::main`,
  `adapters/catalog/tests/local_runtime/guarded_merge.rs`,
  `catalog_fault_cleanup_observation_handle_cannot_authorize_signal`: two `poll` timeouts of
  5000 ms in the helper `finish_filtered_owner` (the plan; the helper holds three `libc::poll(…,
  5000)` calls).
- 10.0 s, binary `connectors-catalog-provider::main`,
  `adapters/catalog/tests/adversary_one_binary.rs`,
  `no_suite_sees_the_installation_another_suite_sets`: it polls an environment variable for 10 s.
  Under nextest each test is its own process, so no other suite can write the variable and the test
  cannot fail.

Host work per test (group N):

- 11.6 s, binary `connectors-kubernetes::exec_credential_security`,
  `adapters/kubernetes/tests/exec_credential_security.rs` (lines 468 to 492),
  `exec_plugin_file_owned_by_another_user_is_refused_before_the_digest`: it walks `/etc`,
  `/var/lib`, `/usr/share` and `/usr/include` to depth 3 for a readable file under 1 MiB whose
  owner is neither root nor the test's user and that group and others cannot write. The time
  depends on the host.

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
   place of 5,000 ms. Estimate after: 0.3 s (the plan; 3 × 100 ms).
3. `no_suite_sees_the_installation_another_suite_sets`: it leaves the gate's test step. It gets
   `#[ignore]` and the name `slow_lane_no_suite_sees_the_installation_another_suite_sets`. The
   rule of story:the-lane-slow-runs-the-ignored-timing-family puts it in the timing family by its
   name. This story edits neither `ignored.rs` nor `slow.yml`. Estimate after in the test step:
   0 s. See "Not established": the lane runs it alone in a process too.
4. `exec_plugin_file_owned_by_another_user_is_refused_before_the_digest`: the test tries a list of
   at most 3 candidate paths before the walk and walks only when none of them fits. The first
   candidate is the path the walk finds on the forge's runner; the test prints the path it used
   and whether it came from the list or from the walk. Estimate after: 1 s (the plan).
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

1. The six tests that stay in the test step pass, and their durations in the test step of the
   forge's "repository gate" job of this story's pull request sum to at most 25 s. The seventh
   does not run in that step. Before: 73.0 s for the seven. Estimate: 16.6 s. Bound: 1.5 × 16.6 =
   24.9, rounded up to 25 s. The numbers are read from the durations nextest prints in that job.
2. The race was run: the bundle test fails when the swapping thread completed fewer than 100
   swaps.
3. The kubernetes test prints `candidate` or `walk` with the path it used. In the gate job of this
   story's pull request it prints `candidate`.
4. `connectors-catalog-provider::main::adversary_one_binary::slow_lane_no_suite_sees_the_installation_another_suite_sets`
   passes in the lane `slow`. The lane's command
   `cargo run --locked -p connectors-build -- ignored --family timing --required --only-prerequisite provider`
   lists it as `passed`. Alone it runs with
   `cargo test --locked -p connectors-catalog-provider --test main -- --ignored --exact adversary_one_binary::slow_lane_no_suite_sees_the_installation_another_suite_sets`.
5. The gate's ignored step still passes: the new ignored test is not unknown to it.

## Scope

- `crates/connectors-catalog/tests/bundle_adversary_2.rs`: the one test above.
- `adapters/catalog/tests/adversary_one_binary.rs`: the one test above.
- `adapters/catalog/tests/local_runtime/guarded_merge.rs`: `finish_filtered_owner` and its callers.
- `adapters/catalog/tests/local_runtime.rs`: listed by the plan; no change is named.
- `adapters/catalog/tests/adversary_git_relay.rs`: the one test above.
- `adapters/catalog/tests/security_git_relay.rs`: the one test above.
- `adapters/kubernetes/tests/exec_credential_security.rs`: the one test above.

No other story of this set writes these seven files.
story:the-catalog-provider-tests-read-a-committed-lowering-of-the-released-definition writes other
files of `adapters/catalog/tests/` (three test files, a new module and `main.rs`).

## Not in this story

- `every_page_is_one_the_model_admits` and `adapters/sql/tests/ess_model.rs`:
  story:eighteen-tests-take-a-template-store-or-jail-built-once-per-run.
- `how_long_sixty_four_idle_connections_delay_the_redirect`, the fourth row of group L:
  story:five-host-unit-tests-shorten-the-waits-they-chose.
- The lane `slow` itself: story:the-lane-slow-runs-the-ignored-timing-family.
- The other rows of group M and the tests of every other group.
- The gate failing when its test step runs over 120 s:
  story:the-gates-test-step-ends-within-120-s-on-the-forge.
- Not established:
  - A run that can fail for `no_suite_sees_the_installation_another_suite_sets`. The plan moves
    it to "the lane `slow` under `cargo test`, where suites share a process". The lane's command
    runs one test per process (`execute_test` in `crates/connectors-build/src/ignored.rs`, line
    794: `--exact`, `--test-threads=1`), as nextest does. So the test cannot fail in the lane
    either. A run of the whole binary in one process is a step the lane does not have; whether it
    gets one, or the test is deleted, is not decided here.
  - The candidate paths of the kubernetes test. They depend on the runner's image, which was not
    read. The pull request takes the first from the path the walk prints in the gate job.
  - How many swaps and calls of `load` the bundle test completes in 2 s. The figures 1,000 and
    100 are not measured.
  - Every "after" figure on the forge.

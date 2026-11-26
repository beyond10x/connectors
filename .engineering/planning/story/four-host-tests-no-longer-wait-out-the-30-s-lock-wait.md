---
format: aep.planning-md/3
id: story:four-host-tests-no-longer-wait-out-the-30-s-lock-wait
kind: story
status: implemented
title: 'Four host tests no longer wait out the 30 s lock wait: 122.0 s become an estimated 5.5 s'
summary: Four tests of connectors-host live through WAIT_BOUND (30 s); existing test seams and one new child action replace the wait, and one run at 30 s stays in the lane slow
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
- serves: vision:independent-contract-adapters
- depends_on: story:the-lane-slow-runs-the-ignored-timing-family
scope:
- confidence: cited
  path: crates/connectors-host/src/local/adversary_w2_tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/approval_policy/tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/metadata.rs
- confidence: cited
  path: crates/connectors-host/tests/local_foundation.rs
revision: 13
transitions:
- {from: "draft", to: "proposed", at: "2026-11-25T03:48:47Z", actor: "human:operator", revision: 8}
- {from: "proposed", to: "active", at: "2026-11-25T03:48:47Z", actor: "human:operator", revision: 9}
- {from: "active", to: "implemented", at: "2026-11-25T12:32:36Z", actor: "human:operator", revision: 13, decided_on: {"recorded":{"test_result":2,"review_outcome":1}}}
---
## Defect

Four tests of `connectors-host` each live through the production lock wait of 30 s. Together they
take 122.0 s of the test step.

Source of the durations: the ones nextest printed in the forge's gate job 138270416028 of pull
request 291, 2026-11-24, as listed in test-plan:where-the-gates-test-step-spends-its-time (group
F). The plan read the sources at 0.77.2. The function names below were read in the tree of 0.78.1.
The plan gives the sum as 122.0 s; its four rows, printed with one decimal, sum to 122.1 s
(inferred: the plan summed the unrounded durations).

- 31.3 s, binary `connectors-host`, `crates/connectors-host/src/local/approval_policy/tests.rs`,
  `child_process_waits_on_the_lock_wait_its_parent_hands_it`: the test sleeps `WAIT_BOUND` + 1 s =
  31 s to outlast the production wait.
- 30.3 s, binary `connectors-host`, same file,
  `held_use_blocks_cross_process_revocation_until_release`: its `blocked` child keeps the production
  lock wait and waits 30 s for a lease the test holds.
- 30.3 s, binary `connectors-host`, `crates/connectors-host/src/local/adversary_w2_tests.rs`,
  `blocked_policy_child_keeps_production_wait_when_the_variable_is_inherited`: it runs the test
  above again in a child process with `CONNECTORS_TEST_LOCK_WAIT_MS` set. The same 30 s again.
- 30.2 s, binary `connectors-host::local_foundation`,
  `crates/connectors-host/tests/local_foundation.rs` (lines 306 to 333),
  `contending_metadata_open_refuses_at_its_bound_and_recovers_after_release`: `Metadata::initialize`
  waits `WAIT_BOUND` against a held flock, and the test asserts `elapsed >= WAIT_BOUND`.

Read at 0.78.1: `WAIT_BOUND` is `Duration::from_secs(30)` in
`crates/connectors-host/src/local/metadata.rs`.

## Change

Each seam, and whether it exists today (grep in the tree of 0.78.1):

1. Exists: `child_lock_wait` in `crates/connectors-host/src/local/metadata.rs` (`#[cfg(test)]`).
   It hands a child process of the test binary a lock wait through `CONNECTORS_TEST_LOCK_WAIT_MS`.
   Read at 0.78.1: the helper `child` in `approval_policy/tests.rs` gives the `blocked` child
   `child_production_lock_wait`, which removes the variable, and gives every other child
   `child_lock_wait` with `CONTENDED_LOCK_WAIT`. Change: the `blocked` child of
   `held_use_blocks_cross_process_revocation_until_release` gets a short wait through
   `child_lock_wait`. Estimate after: 2 s.
2. Exists: `set_lifecycle_lock_wait` in the same file (`#[cfg(test)]`, a `thread_local` value). A
   test under `crates/connectors-host/tests/` is an integration test and cannot reach it. Change:
   a new library test in the module `local::metadata::tests` (`metadata.rs`, line 1547; the module
   calls `set_lifecycle_lock_wait` at lines 1920 and 1942 today) takes over the assertions of
   `contending_metadata_open_refuses_at_its_bound_and_recovers_after_release` with a wait of 1 s:
   `contending_metadata_open_refuses_at_a_short_bound_and_recovers_after_release`. Estimate after:
   1.5 s.
3. The test of `local_foundation.rs` stays in its file with the real 30 s. It gets `#[ignore]` and
   the name `slow_lane_contending_metadata_open_refuses_at_its_bound_and_recovers_after_release`.
   The gate's test step no longer runs it. The rule of
   story:the-lane-slow-runs-the-ignored-timing-family puts it in the timing family by its name.
   This story edits neither `ignored.rs` nor `slow.yml`.
4. New: an action of the child fixture `policy_child` (`approval_policy/tests.rs`) that reports
   its effective lock wait and does not wait. `lifecycle_lock_wait` in `metadata.rs`
   (`#[cfg(test)]`) returns the per-thread value today. Change:
   `child_process_waits_on_the_lock_wait_its_parent_hands_it` and
   `blocked_policy_child_keeps_production_wait_when_the_variable_is_inherited` assert on the
   reported wait. Estimate after: 1 s each.

No override of a production wait is added. All seams are `#[cfg(test)]` items of the library.

Estimate after in the test step, four tests: 2 + 1 + 1 + 1.5 = 5.5 s (the plan; an estimate, not
timed).

## Acceptance

1. Four tests pass in the test step of the forge's "repository gate" job of this story's pull
   request, and their durations sum to at most 9 s: the three tests that keep their names
   (`child_process_waits_on_the_lock_wait_its_parent_hands_it`,
   `held_use_blocks_cross_process_revocation_until_release`,
   `blocked_policy_child_keeps_production_wait_when_the_variable_is_inherited`) and the new
   `local::metadata::tests::contending_metadata_open_refuses_at_a_short_bound_and_recovers_after_release`.
   Before: 122.0 s. Estimate: 5.5 s. Bound: 1.5 × 5.5 = 8.25, rounded up to 9 s. The numbers are
   read from the durations nextest prints in that job.
2. The test at the real `WAIT_BOUND` of 30 s is not one of the four of acceptance 1; it is counted
   here only.
   `connectors-host::local_foundation::slow_lane_contending_metadata_open_refuses_at_its_bound_and_recovers_after_release`
   passes in the lane `slow`. The lane's command
   `cargo run --locked --bin connectors-build -- ignored --family timing --required --only-prerequisite provider`
   lists it as `passed`. Alone it runs with
   `cargo test --locked -p connectors-host --test local_foundation -- --ignored --exact slow_lane_contending_metadata_open_refuses_at_its_bound_and_recovers_after_release`.
3. The gate's ignored step still passes: the new ignored test is not unknown to it.

## Scope

- `crates/connectors-host/src/local/metadata.rs`: the lock wait helpers (`child_lock_wait`,
  `child_production_lock_wait`, `set_lifecycle_lock_wait`, `lifecycle_lock_wait`) and the new test
  in `mod tests`.
- `crates/connectors-host/src/local/approval_policy/tests.rs`: `child`, `policy_child` and the two
  tests.
- `crates/connectors-host/src/local/adversary_w2_tests.rs`: the one test.
- `crates/connectors-host/tests/local_foundation.rs`: the one test gets `#[ignore]` and its new
  name.

No other story of this set writes these four files.
story:eighteen-tests-take-a-template-store-or-jail-built-once-per-run, which planned to read
`metadata.rs` as an input of a digest, is archived and was not built. The stories that took its
place name none of the four files in their scope.

## Not in this story

- The tests of every other group of the test plan.
- The lane `slow` itself: story:the-lane-slow-runs-the-ignored-timing-family.
- The gate failing when its test step runs over 120 s:
  story:the-gates-test-step-ends-within-120-s-on-the-forge.
- `CONTENDED_LOCK_WAIT` (120 s) and the other callers of `set_lifecycle_lock_wait`.
- Not established: every "after" figure on the forge. The plan says "one run at the real 30 s wait
  that proves WAIT_BOUND itself" and names no test; the choice of the test of `local_foundation.rs`
  is this story's.

## As built (2026-11-25)

Where the implementation differs from the Change and Acceptance sections. Durations are from the
build host of the shared pool: one run before the change, one after, and the range of ten runs in
a row after it (10 of 10 passed).

1. The helper `child` of `approval_policy/tests.rs` takes the wait as a third argument; `None`
   means production's wait.
2. Change 4 is two actions of `policy_child`, not one. `lock-wait` reports the child's effective
   wait and takes no lock. `production-child` reports its own wait and starts a `lock-wait` child
   with production's wait.
3. `blocked_policy_child_keeps_production_wait_when_the_variable_is_inherited` no longer runs
   `held_use_blocks_cross_process_revocation_until_release` in a child. It starts `policy_child`
   handed 600 s through `child_lock_wait` and asserts that this child reports 600 s and that the
   child it starts reports 30 s. Before: 30.720 s. After: 0.009 s to 0.010 s. Not shown by a
   mutation: on Unix two things hold the 30 s of the inner child, the removed variable and the
   binding of the variable to a process id; removing one alone would leave the test green.
4. `child_production_lock_wait` has one caller now, that reporting child. No child of a test that
   takes a real lock keeps production's wait.
5. `child_process_waits_on_the_lock_wait_its_parent_hands_it` asserts that a child handed 31 s
   reports 31 s, and keeps its `waits` child, which is admitted after the release. The hold
   before the release is 200 ms in place of 31 s; no assertion depends on its length. Before:
   31.579 s. After: 0.531 s to 0.676 s.
6. `held_use_blocks_cross_process_revocation_until_release`: the `blocked` child waits 1 s
   (`BLOCKED_LOCK_WAIT`) in place of 30 s. The test holds the lease until that child has exited,
   so a slow machine makes the test slower and not red. Before: 30.716 s. After: 1.359 s to
   1.454 s.
7. The new `local::metadata::tests::contending_metadata_open_refuses_at_a_short_bound_and_recovers_after_release`
   waits 1 s, and the refusal must arrive within 11 s. It is `#[cfg(unix)]`, because it uses
   `flock`. After: 1.098 s to 1.132 s. The test it takes over took 30.181 s.
8. `metadata.rs`: no helper changed its behaviour. One comment of `child_lock_wait` is reworded.
9. `slow_lane_contending_metadata_open_refuses_at_its_bound_and_recovers_after_release` was
   compiled and listed, and not run: the operator allows no run of a test of the timing family
   outside the forge's lane (2026-11-25).
10. Second commit, after the adversary read (review-result:four-host-lock-adversary-pass-1):
    `held_use_blocks_cross_process_revocation_until_release` times its `blocked` child and asserts
    that it ended before `WAIT_BOUND / 2` (15 s). A lease that waits production's 30 s keeps the
    child 30 s and turns the test red. With the hold of 200 ms of item 5 no other test showed that
    the policy lease uses the wait handed to the child.
11. Second commit: `child_process_waits_on_the_lock_wait_its_parent_hands_it` asserts, before the
    lease is released, that its `waits` child is still running and that the revision is still 1.
    The child was handed 120 s and ends only by getting the lease.
12. Second commit: a fifth test in the gate, Unix only,
    `a_wait_handed_to_a_parent_is_not_its_childs_but_that_of_a_process_replacing_the_parent`
    (`adversary_w2_tests.rs`, through `/bin/sh`): a child of a shell that was handed 600 s reports
    30 s, and a process that replaces the shell reports 600 s. It is not one of the four tests of
    acceptance 1.
    In one run on the build host after the second commit the five tests passed in 2.5 s of wall
    time (0.02, 0.03, 1.2, 1.4 and 2.5 s); 10 of 10 repeated runs passed in 28.6 s; `cargo clippy
    --locked -p connectors-host --all-targets -- -D warnings` ended 0.

Sum of the four tests of acceptance 1 on the build host: 5.1 s after the second commit (0.02, 1.2,
1.4 and 2.5 s), 3.3 s after the first. Before, with the test that is now in the lane: 123.2 s. The 52 tests of the touched modules passed in one run; the slowest took 3.9 s.

Open until the jobs of the pull request: acceptance 1 (at most 9 s in the gate job), acceptance 2
(the lane lists the `slow_lane_` test as `passed`), acceptance 3 (the gate's ignored step). The
pull request opens after story:the-lane-slow-runs-the-ignored-timing-family is on main: without
that story's rule the new ignored test is unknown to the gate's ignored step.

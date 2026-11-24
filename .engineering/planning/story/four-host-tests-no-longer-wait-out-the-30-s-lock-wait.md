---
format: aep.planning-md/3
id: story:four-host-tests-no-longer-wait-out-the-30-s-lock-wait
kind: story
status: draft
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
revision: 6
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
   `cargo run --locked -p connectors-build -- ignored --family timing --required --only-prerequisite provider`
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
story:eighteen-tests-take-a-template-store-or-jail-built-once-per-run reads `metadata.rs` as an
input of a digest and does not write it.

## Not in this story

- The tests of every other group of the test plan.
- The lane `slow` itself: story:the-lane-slow-runs-the-ignored-timing-family.
- The gate failing when its test step runs over 120 s:
  story:the-gates-test-step-ends-within-120-s-on-the-forge.
- `CONTENDED_LOCK_WAIT` (120 s) and the other callers of `set_lifecycle_lock_wait`.
- Not established: every "after" figure on the forge. The plan says "one run at the real 30 s wait
  that proves WAIT_BOUND itself" and names no test; the choice of the test of `local_foundation.rs`
  is this story's.

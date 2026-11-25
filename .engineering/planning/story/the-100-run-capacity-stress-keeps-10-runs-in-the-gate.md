---
format: aep.planning-md/3
id: story:the-100-run-capacity-stress-keeps-10-runs-in-the-gate
kind: story
status: active
title: 'The 100-run capacity stress keeps 10 runs in the gate: 31.9 s become an estimated 3.2 s'
summary: One CLI test runs the CLI 100 times in the test step; 10 runs stay in the gate and the 100 run in the lane slow
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
- serves: vision:independent-contract-adapters
- depends_on: story:the-lane-slow-runs-the-ignored-timing-family
scope:
- confidence: cited
  path: apps/connectors/tests/local_cli.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-25T04:03:29Z", actor: "human:operator", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-25T04:03:29Z", actor: "human:operator", revision: 6}
---
## Defect

One CLI test runs the CLI 100 times in the test step and takes 31.9 s:
`adversary2_a_same_build_owner_at_capacity_is_never_another_build_over_100_runs`, binary
`connectors::local_cli`, `apps/connectors/tests/local_cli.rs` (line 1691 at 0.78.1).

Source of the duration: the one nextest printed in the forge's gate job 138270416028 of pull
request 291, 2026-11-24, as listed in test-plan:where-the-gates-test-step-spends-its-time (one row
of group K). The plan read the source at 0.77.2.

Read at 0.78.1: the test starts one owner, then loops `for _ in 0..100`. Each round holds 32
sockets, starts `instances status --adapter forge`, sleeps a seeded pause of up to 300 ms, releases
one socket, waits for the command, and sleeps 150 ms. It prints
`adversary2 tally over 100 runs: …` and asserts that no run reported `owner_build_mismatch`.

## Change

The test's own value; no seam in product code.

1. The body moves into one helper that takes the number of runs.
2. In the test step the test runs 10 rounds, under the name
   `adversary2_a_same_build_owner_at_capacity_is_never_another_build_over_10_runs`.
3. The 100 rounds stay as an ignored test named
   `slow_lane_adversary2_a_same_build_owner_at_capacity_is_never_another_build_over_100_runs`. The
   rule of story:the-lane-slow-runs-the-ignored-timing-family puts it in the timing family by its
   name. This story edits neither `ignored.rs` nor `slow.yml`.

This story goes in one pull request with story:the-lane-slow-runs-the-ignored-timing-family. Its
`depends_on` means "not before that story", not "in a later pull request": the lane's first
command refuses an empty selection, and the test of point 3 is the first one it selects.

Estimate after in the test step (the plan; not timed): 3.2 s.

## Acceptance

1. The test of 10 runs passes in the test step of the forge's "repository gate" job of this
   story's pull request in at most 5 s. Before: 31.9 s. Estimate: 3.2 s. Bound: 1.5 × 3.2 = 4.8,
   rounded up to 5 s.
2. Kept runs in the gate: the test asserts that its tally sums to 10, and its line reads
   `adversary2 tally over 10 runs`.
3. Kept runs in the lane `slow`:
   `connectors::local_cli::slow_lane_adversary2_a_same_build_owner_at_capacity_is_never_another_build_over_100_runs`
   asserts that its tally sums to 100. The lane's command
   `cargo run --locked --bin connectors-build -- ignored --family timing --required --only-prerequisite provider`
   lists it as `passed`. Alone it runs with
   `cargo test --locked -p connectors --test local_cli -- --ignored --exact slow_lane_adversary2_a_same_build_owner_at_capacity_is_never_another_build_over_100_runs`.
4. The gate's ignored step still passes: the new ignored test is not unknown to it.

## Scope

- `apps/connectors/tests/local_cli.rs`: the one test (lines 1684 to 1727). Two other stories of
  this set edit other tests of this file:
  story:four-idle-exit-tests-no-longer-span-several-5-s-recovery-sweeps
  (`a_connected_client_keeps_an_owner_past_its_idle_bound`, line 1959) and
  story:seven-cli-tests-shorten-their-waits-or-a-production-wait
  (`clock_check_below_quorum_refusal`, line 325).

## Not in this story

- The other tests of `local_cli.rs`.
- The gate failing when its test step runs over 120 s:
  story:the-gates-test-step-ends-within-120-s-on-the-forge.
- Not established: how long the 100 runs take in the lane. They took 31.9 s in the gate job
  above. story:store-tests-end-within-60s-under-load, a draft outside this set, reports this test
  killed at 60 s on a loaded workstation; the lane ends a test at 60 s.

## As built (2026-11-25)

1. The helper takes the number of runs and asserts that the tally sums to it. Both tests call it:
   10 in the gate, 100 under `#[ignore = "lane slow"]`. The file's other ignored test has a bare
   `#[ignore]`.
2. The test of 100 runs was not run before or after the change: the operator allows no run of it
   outside the forge's lane (2026-11-25). The "before" stays the forge's 31.9 s.
3. The test of 10 runs on the build host of the shared pool: 3.5 s in one run, and 3.4 s to
   3.8 s in ten runs in a row, 10 of 10 passed. It printed
   `adversary2 tally over 10 runs: {"\"capacity\"": 6, "\"ok:not_running\"": 4}`.
4. `cargo test --locked -p connectors --test local_cli -- --ignored --list --format terse` lists
   `slow_lane_adversary2_a_same_build_owner_at_capacity_is_never_another_build_over_100_runs`.

Open until the jobs of the pull request: acceptance 1 (at most 5 s in the gate job), acceptance 3
(the lane's report lists the test of 100 runs as `passed`), acceptance 4 (the gate's ignored
step).

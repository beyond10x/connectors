---
format: aep.planning-md/3
id: story:eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping
kind: story
status: draft
title: 'Eleven connection lease tests stop sleeping through lifetimes and the 30 s admission bound: 222.9 s become an estimated 48.0 s'
summary: Evidence lifetimes and the 30 s admission bound are proved on the owner's wall clock; the seam is open until the question story answers
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
- serves: vision:independent-contract-adapters
- depends_on: story:the-lane-slow-runs-the-ignored-timing-family
- depends_on: story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build
- depends_on: story:where-do-the-connection-lease-tests-clock-and-the-30-s-admission-bound-live
- depends_on: story:seven-cli-tests-shorten-their-waits-or-a-production-wait
scope:
- confidence: cited
  path: apps/connectors/src/local/connections.rs
- confidence: cited
  path: apps/connectors/tests/adversary_d218_u1.rs
- confidence: cited
  path: apps/connectors/tests/connection_lease.rs
- confidence: cited
  path: apps/connectors/tests/connection_lease_adversary.rs
- confidence: cited
  path: apps/connectors/tests/support/mounted.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/supervisor.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/transport.rs
- confidence: cited
  path: crates/connectors-sdk/src/lib.rs
revision: 7
---
## Defect

Eleven CLI tests of the connection lease prove evidence lifetimes and the 30 s admission bound on
the owner's wall clock. Seven sleep through a lifetime (group B, 131.5 s). Four run probes of 16 s
to 18 s under the 30 s admission bound (group C, 91.4 s). Together they take 222.9 s of the test
step.

Source of the durations: the ones nextest printed in the forge's gate job 138270416028 of pull
request 291, 2026-11-24, as listed in test-plan:where-the-gates-test-step-spends-its-time (groups B
and C). The plan read the sources at 0.77.2. The function names below were read in the tree of
0.78.1. The sums are the plan's and equal the sums of its rows.

Group B, the test sleeps the lifetime:

- 47.2 s, binary `connectors::connection_lease_adversary`,
  `apps/connectors/tests/connection_lease_adversary.rs`,
  `adversary_a_hanging_early_recollection_does_not_cost_a_read_its_current_evidence`: sleeps 31 s
  (half of a 60 s evidence lifetime plus 1 s), then the knob `probe_ms` at 40 s.
- 26.4 s, binary `connectors::connection_lease`, `apps/connectors/tests/connection_lease.rs`,
  `a_revalidated_connection_serves_reads_across_three_evidence_lifetimes`: a loop of reads every
  250 ms over 3 × 8 s + 1 s = 25 s of wall clock.
- 13.5 s, binary `connectors::connection_lease_adversary`, its file,
  `adversary_a_read_on_current_evidence_is_served_when_its_early_recollection_fails_transiently`:
  sleeps 11 s (half of a 20 s lifetime plus 1 s).
- 11.8 s, binary `connectors::connection_lease`, its file,
  `a_read_at_10_5_s_of_a_20_s_lifetime_recollects_once`: waits until 10.5 s of a 20 s lifetime, up
  to 6 attempts.
- 11.7 s, binary `connectors::adversary_d218_u1`, `apps/connectors/tests/adversary_d218_u1.rs`,
  `a_lapsed_connection_is_selected_and_rechecked_before_the_call`: a 10 s evidence window, polled
  every 250 ms until the state is pending.
- 10.7 s, binary `connectors::connection_lease`, its file,
  `a_read_at_9_5_s_of_a_20_s_lifetime_probes_nothing`: waits until 9.5 s of a 20 s lifetime.
- 10.2 s, binary `connectors::connection_lease_adversary`, its file,
  `adversary_a_read_in_its_evidences_last_two_seconds_still_recollects_first`: lifetime 8 s, sleeps
  until 1.6 s before `valid_until`, then `invoke_ms` at 2.5 s.

Group C, probes under the 30 s admission bound:

- 33.3 s, binary `connectors::connection_lease_adversary`, its file,
  `adversary_two_revalidations_with_a_slow_probe_both_answer_ready_within_the_bound`: `probe_ms`
  16 s run alone, then two together (2 × 16 s in series).
- 20.5 s, same binary and file,
  `adversary_a_revalidation_during_a_reads_recollection_joins_it_within_the_bound`: `probe_ms` 18 s
  while a read's recollection is in flight.
- 19.1 s, binary `connectors::connection_lease`, its file,
  `concurrent_revalidations_both_answer_ready_within_the_admission_bound`: probe 18 s, two
  revalidations; it asserts 2 × 18 s > the 30 s bound.
- 18.5 s, binary `connectors::connection_lease_adversary`, its file,
  `adversary_a_revalidation_joining_a_failed_read_recollection_runs_no_probe_of_its_own`:
  `unavailable_after_ms` 16 s under a 1 s lifetime.

## Change

The design of this story is open until
story:where-do-the-connection-lease-tests-clock-and-the-30-s-admission-bound-live answers. This
story `depends_on` it and is revised from its answer before it is proposed. What is known, read at
0.78.1:

1. Exists: the knobs `evidence_ms`, `probe_ms`, `invoke_ms` and `unavailable_after_ms` of the
   adapter fixture in `apps/connectors/tests/support/mounted.rs` (lines 185 to 230). The fixture
   computes `valid_until_ms` as its own `connectors_sdk::now_ms()` plus `evidence_ms` (line 213).
2. Route 1, group C: an override of the 30 s admission bound, through the helper of
   story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build, with a
   shorter `probe_ms`. Where production sets the bound: the owner refuses a revalidation whose
   deadline is more than 30,000 ms ahead at
   `crates/connectors-host/src/local/owner/transport.rs` line 2817; lines 2884, 3000 and 3038 of
   the same file, `apps/connectors/src/local/connections.rs` lines 61 and 319 and
   `crates/connectors-host/src/local/owner/supervisor.rs` lines 1355 and 1600 compute the same
   30,000 ms. Which of them the four tests wait on is question 1 of the question story. The two
   candidates the plan named (`DEADLINE_MS` in `access_token.rs`, `EXECUTABLE_LEASE_MS` in
   `owner.rs`) do not name revalidation.
3. Route 2, group B: shorter lifetimes by the fixture's knobs alone, with no change of a clock.
   Question 2 of the question story. A lifetime under about 3 s is not an option (the plan): a
   comment in `a_revalidated_connection_serves_reads_across_three_evidence_lifetimes` says one read
   costs about 3 s on a loaded host.
4. Route 3, only where route 2 fails: a debug-only offset on `connectors_sdk::now_ms()`
   (`crates/connectors-sdk/src/lib.rs` line 356). The CLI, the owner and the adapter fixture are
   three processes, and deadlines built from `Instant` do not move with it. This route changes the
   clock of the crate the adapters build against. It needs the operator's word before this story
   takes it.
5. An override variable carries the prefix `CONNECTORS_TEST_OVERRIDE_`, so the owner spawn in
   `transport.rs` (lines 1663 to 1698) forwards it by the rule of the helper story. This story
   does not edit the spawn.

Outside the gate two tests keep the real waits, as ignored tests that share their body with the
gate's test through one helper:

- `slow_lane_adversary_a_hanging_early_recollection_does_not_cost_a_read_its_current_evidence`:
  the real 60 s evidence lifetime. It is the only one of the eleven with a 60 s lifetime.
- `slow_lane_concurrent_revalidations_both_answer_ready_within_the_admission_bound`: the real 30 s
  admission bound.

The plan says "one 60 s lifetime run, one 30 s admission run" and names no test; the choice is
this story's. The rule of story:the-lane-slow-runs-the-ignored-timing-family puts both in the
timing family by their names. This story edits neither `ignored.rs` nor `slow.yml`.

Estimate after (the plan; not timed): group B 32.0 s, about 4 s each (the same binary's band: 2.9 s
per test + 1 s), 7 s for `a_lapsed_connection_is_selected_and_rechecked_before_the_call` because
its window must outlast a connect, 5 s for
`adversary_a_read_in_its_evidences_last_two_seconds_still_recollects_first`. Group C 16.0 s, 4 s
each. Sum 48.0 s.

## Acceptance

1. The eleven tests pass, and their durations in the test step of the forge's "repository gate"
   job of this story's pull request sum to at most 72 s. Before: 222.9 s. Estimate: 48.0 s. Bound:
   1.5 × 48.0 = 72 s. The numbers are read from the durations nextest prints in that job.
2. `connectors::connection_lease_adversary::slow_lane_adversary_a_hanging_early_recollection_does_not_cost_a_read_its_current_evidence`
   and
   `connectors::connection_lease::slow_lane_concurrent_revalidations_both_answer_ready_within_the_admission_bound`
   pass in the lane `slow`. The lane's command
   `cargo run --locked -p connectors-build -- ignored --family timing --required --only-prerequisite provider`
   lists both as `passed`. Alone they run with
   `cargo test --locked -p connectors --test connection_lease_adversary -- --ignored --exact slow_lane_adversary_a_hanging_early_recollection_does_not_cost_a_read_its_current_evidence`
   and
   `cargo test --locked -p connectors --test connection_lease -- --ignored --exact slow_lane_concurrent_revalidations_both_answer_ready_within_the_admission_bound`.
3. The gate's ignored step still passes: the two new ignored tests are not unknown to it.

That a release build ignores an override is held once, for the helper, by
story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build.

## Scope

- `apps/connectors/tests/connection_lease.rs`: the four tests above and one lane test.
- `apps/connectors/tests/connection_lease_adversary.rs`: the six tests above and one lane test.
- `apps/connectors/tests/adversary_d218_u1.rs`: the one test above.
- `apps/connectors/tests/support/mounted.rs`: the knobs of the adapter fixture (lines 185 to 230).
  story:six-cli-tests-take-jail-binaries-built-once-per-run touches another region of this file:
  `Container::new` (lines 264 to 292).
- `crates/connectors-host/src/local/owner/transport.rs`: the admission bound at line 2817 and the
  30,000 ms at lines 2884, 3000 and 3038. Four other stories of this set write this file. The
  `depends_on` edges put this story last: the helper story,
  story:four-idle-exit-tests-no-longer-span-several-5-s-recovery-sweeps,
  story:seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait,
  story:seven-cli-tests-shorten-their-waits-or-a-production-wait, then this story.
- `crates/connectors-host/src/local/owner/supervisor.rs`: lines 1355 and 1600.
- `apps/connectors/src/local/connections.rs`: lines 61 and 319.
- `crates/connectors-sdk/src/lib.rs`: `now_ms()` (line 356), under route 3 only. The helper story
  writes the same file (its helper and tests) and comes first by the edge.

Which of the last four files this story writes is settled by the answer of the question story.

## Not in this story

- The answer to the three questions:
  story:where-do-the-connection-lease-tests-clock-and-the-30-s-admission-bound-live.
- The lane `slow` itself: story:the-lane-slow-runs-the-ignored-timing-family.
- The tests of every other group of the test plan.
- The gate failing when its test step runs over 120 s:
  story:the-gates-test-step-ends-within-120-s-on-the-forge.
- Not established: where production sets the 30 s admission bound that the four tests prove; every
  "after" figure on the forge; whether the lane test of the 60 s lifetime fits the lane's bound of
  60 s for one test (the same body took 47.2 s in the gate job above).

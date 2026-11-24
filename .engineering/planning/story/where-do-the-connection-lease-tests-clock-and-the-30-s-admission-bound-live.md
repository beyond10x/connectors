---
format: aep.planning-md/3
id: story:where-do-the-connection-lease-tests-clock-and-the-30-s-admission-bound-live
kind: story
status: draft
title: Where do the connection lease tests' clock and the 30 s admission bound live?
summary: 'A question, not a change: which 30,000 ms site the lease tests wait on, whether the fixture''s knobs suffice, and what a clock offset would need'
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
- serves: vision:independent-contract-adapters
revision: 1
---
## Defect

story:eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping cannot name its seam. The
test plan did not find where production sets the 30 s admission bound, and it asks for a clock
offset without naming a function. This story is a question. It changes no product code. The store
has no kind for a spike, so it is a story with one acceptance: a written answer.

Read at 0.78.1, not run:

- `crates/connectors-host/src/local/owner/transport.rs` line 2817: the owner refuses a
  revalidation whose `deadline_ms` is more than 30,000 ms after `connectors_sdk::now_ms()`
  (`InvalidInput`). Lines 2884, 3000 and 3038 bound an `Ensure` task and two `Validate` tasks by
  `connectors_sdk::now_ms() + 30_000`.
- `apps/connectors/src/local/connections.rs` line 61: `execute` computes
  `connectors_sdk::now_ms() + 30_000` and hands it to `client.revalidate`; `launch` (line 319)
  computes the same.
- `crates/connectors-host/src/local/owner/supervisor.rs` line 1355: the `Task::Validate` arm
  bounds a validation by `job.deadline.min(connectors_sdk::now_ms() + 30_000)`. Line 1600: a
  recollection expires at `bound.min(now.saturating_add(30_000))`. Line 142:
  `EARLY_READ_MARGIN_MS` is 2,000.
- `grep 30_000` finds more sites in the same three files that were not followed for this draft:
  `transport.rs` lines 3096 (`Request::Launch`), 3176, 3385, 3429 and 3457, `supervisor.rs` lines
  2537 and 3263.
- `apps/connectors/tests/support/mounted.rs` line 213: the adapter fixture computes
  `valid_until_ms` as its own `connectors_sdk::now_ms()` plus the knob `evidence_ms` (default
  50,000).
- `crates/connectors-sdk/src/lib.rs` line 356: `pub fn now_ms()`. The parallel-safety critic
  counted about 191 callers. The CLI, the owner and the adapter fixture are three processes.
  Deadlines in `transport.rs` that are built from `Instant` do not move with an offset on `now_ms()`.

## Change

None in product code. A probe used for the run of question 1 is not committed.

Questions, in this order:

1. Of the sites above that compute 30,000 ms, which decide the outcome of the four tests of
   group C (two probes of 16 s to 18 s that must fit, or not fit, under 30 s)? Proven by one run of
   `concurrent_revalidations_both_answer_ready_within_the_admission_bound` with the site changed
   or logged.
2. Can the seven tests of group B get a short evidence lifetime by a parameter of the fixture
   alone (the knob `evidence_ms` and the others in `mounted.rs`), with no change of a clock? What
   is the shortest lifetime at which each test still shows what it shows? A comment in
   `a_revalidated_connection_serves_reads_across_three_evidence_lifetimes` says one read costs
   about 3 s on a loaded host.
3. Only if question 2 fails for a test: what would a debug-only offset on
   `connectors_sdk::now_ms()` need? The callers that must see it, the three processes,
   `Registry::with_system_clock` (called in `supervisor.rs` lines 1364 and 1477 and in other
   files), and the `Instant` deadlines that do not move. An offset in the SDK needs the operator's
   word before any story takes it.

The story is bounded by these three questions. It has no time box.

## Acceptance

1. A written answer to each question is added to this story under a heading `## Answer`: for
   question 1 the `file:line` of each deciding site and the output of the run; for question 2 per
   test the knob values and whether it passed; for question 3 either "not needed" or the list of
   what an offset needs.
   story:eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping is then revised from it
   and names its seam.

## Scope

No file of the repository is written. Recorded scope: none. The files read are the five named
above.

## Not in this story

- Any change of the eleven tests or of a production wait:
  story:eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping.
- The helper for debug-only overrides:
  story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build.
- The run of question 1 follows the folder rules: one test, bound to 60 s by the runner; it is one
  test of 19.1 s on the forge.

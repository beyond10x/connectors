---
format: aep.planning-md/3
id: story:five-host-unit-tests-shorten-the-waits-they-chose
kind: story
status: active
title: 'Five host unit tests shorten the waits they chose: 48.6 s become an estimated 18.2 s'
summary: Budgets, lifetimes and late answers of 2 s to 10 s that are the tests' own values, and one unasserted second measurement
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: crates/connectors-host/src/local/clock/tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/oauth_adversary_pass2_tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/refresh/tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/protected/tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/runtime/process/write_tests.rs
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-25T01:32:07Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-25T01:32:07Z", actor: "human:timo", revision: 5}
---
## Defect

Five unit tests of `connectors-host` wait on values the tests chose themselves. Together they take
48.6 s of the test step.

Source of the durations: the ones nextest printed in the forge's gate job 138270416028 of pull
request 291, 2026-11-24, as listed in test-plan:where-the-gates-test-step-spends-its-time (four
rows of group M, one row of group L). The plan read the sources at 0.77.2. The function names
below were read in the tree of 0.78.1. The plan gives the sum as 48.6 s; its five rows, printed
with one decimal, sum to 48.5 s (inferred: the plan summed the unrounded durations).

All five are in binary `connectors-host`, under `crates/connectors-host/src/local/`:

- 13.4 s, `runtime/process/write_tests.rs`,
  `drop_eof_and_original_deadline_destroy_pending_without_a_write`: five modes; the expiry and
  preflight modes sleep a budget of 2 s, a stall of 2.5 s, then a doubled budget of 4 s.
- 11.9 s, `owner/refresh/tests.rs`,
  `a_stored_response_is_published_by_an_owner_restarting_after_the_evidence_lifetime`: sleeps
  `LIFETIME_MS` (10 s) + 1 s.
- 10.4 s, `oauth_adversary_pass2_tests.rs`,
  `how_long_sixty_four_idle_connections_delay_the_redirect`: two `redirect_delay` runs of up to 8 s;
  only the first is asserted.
- 6.6 s, `protected/tests.rs`,
  `controlling_terminal_hides_input_and_restores_echo_after_sigint`: three modes; the mode `late`
  answers 5.5 s after the prompt.
- 6.2 s, `clock/tests.rs`, `quorum_below_quorum_names_each_source`: a silent source costs the 2 s
  clock query timeout per row.

## Change

Every seam exists today (grep in the tree of 0.78.1). The story adds no seam and no override of a
production wait.

1. `drop_eof_and_original_deadline_destroy_pending_without_a_write`: the budget is the test's own
   (`expiring`, which starts at `Duration::from_secs(2)`, and `EXPIRING_CAP`). Change: start at 1 s
   and stall 1.2 s. Estimate after: 6 s.
2. `a_stored_response_is_published_by_an_owner_restarting_after_the_evidence_lifetime`:
   `LIFETIME_MS` is the test's own constant (10,000). Change: 1,000. Estimate after: 3 s.
3. `how_long_sixty_four_idle_connections_delay_the_redirect`: drop the second `redirect_delay` run
   (`held`), which no assertion reads. Estimate after: 5.2 s.
4. `controlling_terminal_hides_input_and_restores_echo_after_sigint`: the deadline handed to the
   child is the test's (the plan). Change: a shorter deadline and a shorter `late` answer. Estimate
   after: 3 s.
5. `quorum_below_quorum_names_each_source`: `set_test_query_timeout` in
   `crates/connectors-host/src/local/clock.rs` (`#[cfg(test)]`). Read at 0.78.1: other tests of
   `clock/tests.rs` call it; this test does not. Change: the test calls it. Estimate after: 1 s.

Estimate after (the plan; not timed): 6 + 3 + 5.2 + 3 + 1 = 18.2 s.

## Acceptance

1. The five tests pass, and their durations in the test step of the forge's "repository gate" job
   of this story's pull request sum to at most 28 s. Before: 48.6 s. Estimate: 18.2 s. Bound: 1.5
   × 18.2 = 27.3, rounded up to 28 s. The numbers are read from the durations nextest prints in
   that job.

The plan names nothing that stays in the lane `slow` for this story.

## Scope

- `crates/connectors-host/src/local/runtime/process/write_tests.rs`: `expiring` and the one test.
- `crates/connectors-host/src/local/owner/refresh/tests.rs`: `LIFETIME_MS` and the one test (near
  line 995 at 0.78.1). story:eighteen-tests-take-a-template-store-or-jail-built-once-per-run
  touches another region of this file: `Connected::build` (lines 314 to 323), where the store of a
  refresh test is initialised.
- `crates/connectors-host/src/local/oauth_adversary_pass2_tests.rs`: the one test.
- `crates/connectors-host/src/local/protected/tests.rs`: the one test.
- `crates/connectors-host/src/local/clock/tests.rs`: the one test.

No other story of this set touches the other four files. `clock.rs` itself is not changed here.
story:seven-cli-tests-shorten-their-waits-or-a-production-wait changes `query_timeout()` in
`clock.rs` for builds that are not the unit-test build, and it `depends_on` this story. The
`#[cfg(test)]` definition that this story's test reaches through `set_test_query_timeout` stays as
it is.

## Not in this story

- The 103 tests of `connectors-host` in the band of 1 s to under 5 s (189.5 s, section 3 of the
  plan), among them `oauth_adversary_pass2_tests::a_head_completing_at_the_idle_limit` and
  `owner::refresh::tests::a_read_recollection_during_the_exchange_does_not_burn_the_rotated_token`.
- The other rows of groups M and L and the tests of every other group.
- The gate failing when its test step runs over 120 s:
  story:the-gates-test-step-ends-within-120-s-on-the-forge.
- Not checked by the plan: every "after" figure on the forge.

## As built (2026-11-25)

Values chosen at implementation, where they differ from the plan's, and what each keeps. Durations
are local, on the workstation: one run before the change, and the range of ten runs in a row after
it. The forge's are read from the pull request's gate job.

1. `drop_eof_and_original_deadline_destroy_pending_without_a_write`: `expiring` takes the first
   budget as a parameter and this test passes 1 s. Its stalled mode is a new fixture mode,
   `briefly-stalled-preflight`, that sleeps 1.2 s: longer than the first budget, shorter than the
   second (2 s). A runner that spends the second budget before the preparation answers gets a
   discarded attempt and a doubled budget, as before: slower, not red. 13.7 s became 7.2 to 9.2 s;
   the plan's estimate was 6 s. The test stays over 5 s: its stalled mode needs one failed attempt
   of 1 s, then a budget of 2 s that the test sleeps through.
   Two things stay as they were, after review-result:five-host-waits-adversary-pass-1 (findings 2
   and 3). The mode `stalled-preflight` still sleeps 2.5 s, and `adversary_w2_tests.rs`, its other
   user, is not changed: that test has no retry and its 2 s deadline has to cover the child's
   start on a loaded runner. `native_outcomes_and_lost_replies_never_repeat_a_send` passes 2 s to
   `expiring`, as before: its two expiring modes need the child to record `send` inside the budget
   after the preparation answered, and `expiring` retries only a failed preparation.
2. `a_stored_response_is_published_by_an_owner_restarting_after_the_evidence_lifetime`:
   `LIFETIME_MS` is 2,000, not the plan's 1,000, and the test sleeps the lifetime plus 500 ms. The
   lifetime bounds two spans on a loaded runner. One is from collecting evidence to validating it
   inside one refresh or one recovery (`owner/refresh.rs`, `valid_until_ms`). The other was the
   test's own: recovery dates the evidence from inside `recover()`, and the test read `Ready` with
   the clock of a later call, after two more store reads (finding 1 of the same review: 257 to
   470 ms locally against 2 s, where it was 10 s). The test now reads the state as of the instant
   `recover()` returned (`described_at`), so the store reads after it do not count. Probed once,
   not committed: with 2.2 s slept after that instant the test passes. 12.6 s became 3.4 to 5.9 s.
3. `how_long_sixty_four_idle_connections_delay_the_redirect`: the second measurement is dropped
   as planned; `redirect_delay` keeps its `refresh` parameter. 10.5 s became 2.0 to 2.2 s.
4. `controlling_terminal_hides_input_and_restores_echo_after_sigint`: the mode `late` answers
   1.5 s after the prompt (was 5.5 s). The deadline handed to the child stays 120 s: it is a
   ceiling for a slow machine and costs no time. The 1.5 s stays above the 1 s slice the read polls
   in (`protected.rs`), so the mode still crosses one slice. What the mode no longer shows: that a
   fixture which assumes 5 s of its own fails. The fixture has had no deadline of its own since it
   reads `CONNECTORS_TERMINAL_FIXTURE_DEADLINE_MS`. 6.7 s became 1.6 to 2.7 s.
5. `quorum_below_quorum_names_each_source`: the query deadline is 1 s, not the 500 ms of the
   second-attempt tests. The three answering sources are loopback fixtures and must answer inside
   the deadline; story:clock-quorum-test-tolerates-a-slow-loopback-source records a runner on
   which one answered later than 2 s. 6.3 s became 3.2 to 3.4 s; the plan's estimate was 1 s.

Sum of the five, local: 49.8 s before; 20.7 s after (the mean of the ten runs; one single run
gave 21.5 s). Ten runs in a row of the five tests and the two neighbours named in point 1: 10 of
10 passed.

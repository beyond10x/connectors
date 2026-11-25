---
format: aep.planning-md/3
id: story:five-host-unit-tests-shorten-the-waits-they-chose
kind: story
status: draft
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
revision: 3
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

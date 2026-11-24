---
format: aep.planning-md/3
id: story:seven-mcp-tests-use-a-4-s-request-budget-and-shorter-holds
kind: story
status: draft
title: 'Seven MCP tests use a 4 s request budget and shorter holds: 101.8 s become an estimated 34.0 s'
summary: Three tests chose a request budget of 20 s and four sleep or hold the store for 4 s to 9 s; all values are the tests' own
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- informed_by: test-plan:where-the-gates-test-step-spends-its-time
- serves: vision:independent-contract-adapters
- depends_on: story:the-lane-slow-runs-the-ignored-timing-family
scope:
- confidence: cited
  path: apps/connectors/tests/adversary_local_mcp_server.rs
- confidence: cited
  path: apps/connectors/tests/mcp_tool_call_shapes_adversary.rs
- confidence: cited
  path: apps/connectors/tests/security_mcp_deadline.rs
revision: 5
---
## Defect

Seven MCP tests wait on values the tests chose themselves. Three use a request budget of 20 s
(group D, 60.5 s). Four sleep or hold the store for 4 s to 9 s (group E, 41.3 s). Together they
take 101.8 s of the test step.

Source of the durations: the ones nextest printed in the forge's gate job 138270416028 of pull
request 291, 2026-11-24, as listed in test-plan:where-the-gates-test-step-spends-its-time (groups D
and E). The plan read the sources at 0.77.2. The function names and lines below were read in the
tree of 0.78.1. The sums are the plan's and equal the sums of its rows.

Group D, a request budget of 20 s:

- 21.9 s, binary `connectors::security_mcp_deadline`,
  `apps/connectors/tests/security_mcp_deadline.rs`,
  `security_control_a_late_reply_to_a_paused_reader_keeps_the_session`: `BUDGET` is 20,000 ms
  (line 730); the reply is released at `BUDGET` + 600 ms (line 846).
- 21.7 s, same binary and file,
  `security_an_on_time_reply_written_across_its_deadline_keeps_the_session`: the same `BUDGET`; the
  reader is paused until `BUDGET` + 500 ms (line 871).
- 16.9 s, binary `connectors::mcp_tool_call_shapes_adversary`,
  `apps/connectors/tests/mcp_tool_call_shapes_adversary.rs`,
  `a_walk_that_outlasts_the_request_budget_answers_the_records_taken`: `REQUEST_MS` is 20,000
  (line 43); pages with `delay_ms` 4000 (line 423), so 5 pages fit one budget.

Group E, sleeps and holds, all in binary `connectors::adversary_local_mcp_server`,
`apps/connectors/tests/adversary_local_mcp_server.rs`:

- 16.5 s, `adversary_a_store_replaced_under_the_owner_ends_the_session` (line 881): after the
  store is replaced it polls every 20 ms, up to 5 s, for the session to end. It has no sleep of
  its own.
- 10.6 s, `adversary_a_lost_session_leaves_no_provider_read_open` (line 1117): sleeps until
  9,000 ms after dispatch to see that the stalled provider read closed.
- 8.2 s, `adversary_stdin_closed_while_the_store_is_held_exits_successfully` (line 1264): a release
  thread sleeps 7 s and the test joins it, though its assertion is "under 3 s".
- 6.0 s, `adversary_a_long_read_with_the_store_held_is_answered_and_the_session_stays_ready`
  (line 1171): the provider answers in 3,000 ms and the test sleeps 4 s with the store held.

## Change

Every seam exists today (grep in the tree of 0.78.1). The story adds no seam and no override.

Group D:

1. `BUDGET` in `security_mcp_deadline.rs`: 20,000 ms becomes 4,000 ms (the plan's value). The two
   holds are written relative to `BUDGET` and keep their margins: the late reply is released at
   4,000 + 600 = 4,600 ms, the reader is paused until 4,000 + 500 = 4,500 ms. The margins of
   600 ms and 500 ms are today's and do not change.
2. `REQUEST_MS` in `mcp_tool_call_shapes_adversary.rs`: 20,000 becomes 4,000. `delay_ms` per page:
   4000 becomes 800, so 5 pages fit one budget as today (4,000 / 800 = 5; today 20,000 / 4,000 =
   5). The value 800 is computed from that ratio; the plan marks as inferred that this file's
   configuration feeds the budget, and the configuration the file writes holds
   `"request_milliseconds":REQUEST_MS` (line 299).
3. One case with the budget of 20 s stays, outside the gate:
   `slow_lane_security_an_on_time_reply_written_across_its_deadline_keeps_the_session` in
   `security_mcp_deadline.rs`, an ignored test that shares its body with the gate's test through
   one helper that takes the budget. The plan says "one 20 s budget case" and does not say which
   of the three; the choice is this story's. The rule of
   story:the-lane-slow-runs-the-ignored-timing-family puts it in the timing family by its name.
   This story edits neither `ignored.rs` nor `slow.yml`.

Group E (`LEASE_MS` is 1,900 in `apps/connectors/src/mcp/session.rs` line 16):

4. `adversary_stdin_closed_while_the_store_is_held_exits_successfully`: the release thread no
   longer sleeps 7 s. It releases the store when the test tells it to, on a channel, after the
   assertion "under 3 s" has run. The hold then lasts as long as the exit plus the assertion.
5. `adversary_a_long_read_with_the_store_held_is_answered_and_the_session_stays_ready`: the
   provider answers in 2,400 ms in place of 3,000 ms, and the store is held 2,900 ms in place of
   4,000 ms. The read stays longer than one lease (2,400 > 1,900) and the hold stays longer than
   the read by 500 ms. Values are this story's choice.
6. `adversary_a_lost_session_leaves_no_provider_read_open`: in place of the sleep until 9,000 ms
   after dispatch, the test waits for the session's end and then 500 ms more.
7. `adversary_a_store_replaced_under_the_owner_ends_the_session`: no value of its own to shorten
   was found. See "Not established".

Estimate after (the plan; not timed): group D 18.0 s, 6 s each (fixture 3 s + budget 4 s, less the
overlap, rounded up). Group E 16.0 s: 6 + 3 + 4 + 3 in the order of the rows above (fixture 2.1 s,
the band median of the binary, plus 1 to 3 s). Sum 34.0 s.

## Acceptance

1. The seven tests pass, and their durations in the test step of the forge's "repository gate" job
   of this story's pull request sum to at most 51 s. Before: 101.8 s. Estimate: 34.0 s. Bound: 1.5
   × 34.0 = 51 s. The numbers are read from the durations nextest prints in that job.
2. The margins hold in the sources: the late reply is released 600 ms after the budget, the reader
   is paused until 500 ms after it, and the walk's page delay is one fifth of the budget.
3. `connectors::security_mcp_deadline::slow_lane_security_an_on_time_reply_written_across_its_deadline_keeps_the_session`
   passes in the lane `slow` with a budget of 20,000 ms. The lane's command
   `cargo run --locked -p connectors-build -- ignored --family timing --required --only-prerequisite provider`
   lists it as `passed`. Alone it runs with
   `cargo test --locked -p connectors --test security_mcp_deadline -- --ignored --exact slow_lane_security_an_on_time_reply_written_across_its_deadline_keeps_the_session`.
4. The gate's ignored step still passes: the new ignored test is not unknown to it.

## Scope

- `apps/connectors/tests/security_mcp_deadline.rs`: `BUDGET`, the two tests above and the lane
  test. `security_requests_past_their_deadline_each_get_one_reply_on_their_own_id` of the same
  file is counted in story:six-cli-tests-take-jail-binaries-built-once-per-run, which does not
  edit the file.
- `apps/connectors/tests/mcp_tool_call_shapes_adversary.rs`: `REQUEST_MS` and the one test above.
- `apps/connectors/tests/adversary_local_mcp_server.rs`: the four tests above.

No other story of this set writes these three files.

## Not in this story

- The 14 tests of `connectors::adversary_local_mcp_server` and the 15 of
  `connectors::local_mcp_server` in the band of 1 s to under 5 s (section 3 of the plan), and the
  tests of every other group.
- `security_requests_past_their_deadline_each_get_one_reply_on_their_own_id` (5.7 s, group K).
- The lane `slow` itself: story:the-lane-slow-runs-the-ignored-timing-family.
- The gate failing when its test step runs over 120 s:
  story:the-gates-test-step-ends-within-120-s-on-the-forge.
- Not established: every "after" figure on the forge. Three of the four estimates of group E are
  the plan's and do not follow from the values above:
  - the long read: fixture 2.1 s + hold 2.9 s is about 5 s, against the plan's 3 s;
  - the lost session: the test's comment says the session ends about 5 s after stdin closed, so
    the plan's 3 s holds only if that end comes sooner;
  - the replaced store: where its 16.5 s go was not found in the test (it has no sleep of its
    own), so the plan's 6 s has no value behind it.
  The bound of acceptance 1 is kept at the plan's sum. If the three tests take what the values
  above give, the sum is about 40 s (inferred: 18 + 6 + 7 + 4 + 5), still under 51 s.

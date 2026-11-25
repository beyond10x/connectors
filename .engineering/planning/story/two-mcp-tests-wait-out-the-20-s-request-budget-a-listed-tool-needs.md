---
format: aep.planning-md/3
id: story:two-mcp-tests-wait-out-the-20-s-request-budget-a-listed-tool-needs
kind: story
status: draft
title: Two MCP tests wait out the 20 s request budget a listed tool needs
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- depends_on: story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build
revision: 2
---
## Defect

Two MCP tests of package `connectors` wait on a request budget of 20 s and cannot be shortened by
a value of their own:

- 22.9 s, `apps/connectors/tests/security_mcp_deadline.rs`,
  `security_an_on_time_reply_written_across_its_deadline_keeps_the_session`: its scenario is a
  `tools/list` reply of several 12 KB tools that is blocked in a 4096-byte pipe when the deadline
  passes. The reader is paused until 500 ms after the budget.
- 17.7 s, `apps/connectors/tests/mcp_tool_call_shapes_adversary.rs`,
  `a_walk_that_outlasts_the_request_budget_answers_the_records_taken`: the walk's pages stop
  `WALK_REPLY_RESERVE` (5 s, `apps/connectors/src/mcp/projection.rs`) before the budget ends.

Both durations are from one run on the build host of the shared pool, 2026-11-25, in the change of
story:seven-mcp-tests-use-a-4-s-request-budget-and-shorter-holds. That story planned a budget of
4 s for them. It does not work: `limits` in `apps/connectors/src/mcp/projection.rs` lists an
implemented read only when the configuration's `request_milliseconds` is at least the read's
`execution_ms`, and the function accepts 20,000 ms alone for `execution_ms`. Run with a budget of
4 s on the build host:

- the walk test failed: `tools/call` answered `not_found`;
- the on-time test passed and printed "scenario not built": the reply held no tool, was 215 bytes
  long and did not fill the pipe;
- `max_pages_written_as_an_integral_number_is_the_integer_the_schema_admits`, which shares the
  walk test's configuration, failed with `not_found`.

A second bound holds the walk test: its reply reserve of 5 s is longer than a budget of 4 s.

## Change

The operator's decision (2026-11-25): both tests stay in the gate at 20 s until this story is
built, and the way is the first of the three below, the debug-only override. It gets its critics
before implementation, and it is off in a release build, shown by a test on a release build. The
points of the change are open until the story is planned. The three ways that were named when the
defect was found:

1. A debug-only override of the 20 s bound, read through `connectors_sdk::debug_override_ms`
   (story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build),
   so that a debug build lists a tool at a shorter budget and a release build does not. The walk's
   reply reserve needs the same. This is product code under `debug_assertions`.
2. Both tests move to the lane `slow` as `slow_lane_` tests. Each ends within the lane's 60 s.
   The two checks then run in no required job.
3. The on-time scenario is rebuilt at a short budget from replies that need no listed tool. The
   walk test stays as it is.

## Acceptance

Open until the story is planned. Two statements are fixed:

- The two tests keep what they assert today, and a run shows each of them red for the defect it
  guards after the change.
- A test on a release build shows that the override is off there: a release build lists no tool
  at a request budget under 20 s.

## Scope

- `apps/connectors/tests/security_mcp_deadline.rs`: the on-time test.
- `apps/connectors/tests/mcp_tool_call_shapes_adversary.rs`: the walk test and `REQUEST_MS`.
- For way 1 also `apps/connectors/src/mcp/projection.rs`.

## Not in this story

- The four tests that story:seven-mcp-tests-use-a-4-s-request-budget-and-shorter-holds shortened.
- `adversary_a_store_replaced_under_the_owner_ends_the_session` (16.5 s in the forge's gate job
  138270416028), which no story of the set changes.

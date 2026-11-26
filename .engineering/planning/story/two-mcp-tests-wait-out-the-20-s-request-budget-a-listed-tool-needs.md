---
format: aep.planning-md/3
id: story:two-mcp-tests-wait-out-the-20-s-request-budget-a-listed-tool-needs
kind: story
status: active
title: Two MCP tests wait out the 20 s request budget a listed tool needs
summary: Two MCP tests wait 21.7 s and 16.9 s on the forge because a read is listed only at a request budget of 20 s; two debug-only overrides read through the merged helper let both run at 4 s
relations:
- decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge
- depends_on: story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build
- depends_on: story:seven-mcp-tests-use-a-4-s-request-budget-and-shorter-holds
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: apps/connectors/src/mcp/projection.rs
- confidence: cited
  path: apps/connectors/tests/mcp_tool_call_shapes_adversary.rs
- confidence: cited
  path: apps/connectors/tests/security_mcp_deadline.rs
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-11-25T14:07:37Z", actor: "human:operator", revision: 9, decided_on: {"recorded":{"review_outcome":11}}}
- {from: "proposed", to: "active", at: "2026-11-25T16:33:13Z", actor: "human:operator", revision: 10, decided_on: {"recorded":{"review_outcome":11}}}
---
## Defect

Two MCP tests of package `connectors` wait on a request budget of 20 s and cannot be shortened by
a value of their own. Durations from story:seven-mcp-tests-use-a-4-s-request-budget-and-shorter-holds
("As built", one run on the build host after its second commit; its section "Defect" for the
forge's gate job):

- `apps/connectors/tests/security_mcp_deadline.rs`,
  `security_an_on_time_reply_written_across_its_deadline_keeps_the_session`: 22.3 s on the build
  host, 21.7 s on the forge. Its scenario is a `tools/list` reply of several 12 KB tools that is
  blocked in a 4096-byte pipe when the deadline passes. The reader is paused until 500 ms after
  the budget.
- `apps/connectors/tests/mcp_tool_call_shapes_adversary.rs`,
  `a_walk_that_outlasts_the_request_budget_answers_the_records_taken`: 17.0 s on the build host,
  16.9 s on the forge. Each page takes 4 s, and the walk's pages stop `WALK_REPLY_RESERVE` (5 s,
  `apps/connectors/src/mcp/projection.rs`) before the budget ends.

That story planned a budget of 4 s for them. It does not work: `Snapshot::limits` in
`apps/connectors/src/mcp/projection.rs` lists a read only when the configuration's
`request_milliseconds` is at least the read's `execution_ms` (20,000 for an implemented read,
40,000 for a generic one). Run with a budget of 4 s on the build host, in that story's change:

- the walk test failed: `tools/call` answered `not_found`;
- the on-time test passed and printed "scenario not built": the reply held no tool, was 215 bytes
  long and did not fill the pipe;
- `max_pages_written_as_an_integral_number_is_the_integer_the_schema_admits`, which shares the
  walk test's configuration, failed with `not_found`.

A second bound holds the walk test: its reply reserve of 5 s is longer than a budget of 4 s.

## Change

The operator's decision (2026-11-25): both tests stay in the gate at 20 s until this story is
built; the way is a debug-only override; it gets its critics before implementation; it is off in
a release build, shown by a test on a release build.

1. Two waits of `apps/connectors/src/mcp/projection.rs` are read through
   `connectors_sdk::debug_override_ms`
   (story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build):
   - the least `request_milliseconds` at which a read is listed: the right-hand side of the one
     comparison `config["request_milliseconds"] < expected.0` in `Snapshot::limits`. Variable
     `CONNECTORS_TEST_OVERRIDE_MCP_LISTED_REQUEST_MS`, production value `expected.0`;
   - the walk's reply reserve: `WALK_REPLY_RESERVE` becomes a function read by `walk_deadline`.
     Variable `CONNECTORS_TEST_OVERRIDE_MCP_WALK_RESERVE_MS`, production value 5,000.
2. `expected.0` itself does not change. It stays the value compared with the definition's
   `execution_ms` and the execution time of the read's `ReadBudget`, so the owner's own check of
   20,000 ms (`crates/connectors-host/src/local/owner/governed/bounded.rs`) and the fixtures'
   `"execution_ms":20000` stay as they are. With the first variable set, a read is listed and
   called under a request budget shorter than its execution time; the request's deadline ends
   it first.
3. Both names start with `CONNECTORS_TEST_OVERRIDE_` and are read only through the helper. The
   MCP server process reads them, so the list of variables forwarded to the owner does not
   change. The scan `crates/connectors-build/tests/debug_override_reads.rs` holds that no file
   under `src/` reads such a name directly.
4. The two test files start the server after `.env_clear()`. Each sets the variables on that
   command:
   - `security_mcp_deadline.rs`: the on-time case runs at a budget of 4,000 ms with
     `…_LISTED_REQUEST_MS=4000`. Both it and the control start the server through `paused`,
     which calls `spawn`, which builds the command with `Served::server`. `paused` takes the
     variables as a parameter and hands them to `spawn`, which sets them on that command. The
     on-time test passes the one variable. The control keeps its `LATE_BUDGET` and passes
     none. `Served::server` does not change, so its other callers (`Peer::start` and the tests
     that use it) get no variable.
   - `mcp_tool_call_shapes_adversary.rs`: `REQUEST_MS` becomes 4,000; the walk test's page
     delay becomes 1,000 ms, so that the walk outlasts the budget after two or three pages.
     `served` takes the budget as an argument and `Peer::start` takes the variables. Every
     test that exists in the file today passes `REQUEST_MS`, `…_LISTED_REQUEST_MS=4000` and
     `…_WALK_RESERVE_MS=1000`. The new test of acceptance 5 passes 19,999 and no variable.
5. The on-time test fails when its scenario is not built. Today it prints "scenario not built"
   and passes when `began_on_time` is false. If the forge shows it red for that reason, the
   budget of the case is raised; the assertion stays.

The release-build proof is the helper's: in a build without `debug_assertions` the helper
returns the production value and reads no variable, whatever the name.
`release_build_ignores_a_debug_override` of `connectors-sdk` shows it, and the lane `slow` runs
it in a release build. This story adds no release-build test of package `connectors`: none of
`.github/workflows/slow.yml`, `crates/connectors-build/src/ignored.rs` and
`docs/development.md` changes. A session's reading of the decision, for the critics to judge:
one release test of the helper covers every wait read through it, as the helper's story says.

## Acceptance

1. Time: in the test step of the forge's "repository gate" job of the pull request, each of the
   two tests ends within 9 s (before: 21.7 s and 16.9 s).
2. The on-time scenario is built: at the budget of 4,000 ms the test asserts `began_on_time`
   and fails without it. The test passes in the same job.
3. The on-time test is red-capable: with a mutant of product code in which the server ends the
   session when a reply's write ends after the request's deadline (not committed), the test
   fails. "As built" names the file and line of the mutant and the failed assertion.
4. The walk test is red-capable: with a mutant in which a page refused after the first ends
   the call with the refusal and no records (not committed), the test fails. "As built" names
   the file and line of the mutant and the failed assertion.
5. Production is unchanged without the variables: a new test
   `a_request_budget_under_the_reads_execution_time_lists_no_tool` in
   `mcp_tool_call_shapes_adversary.rs` starts the server with `request_milliseconds` 19,999 and
   no override variable, and `tools/list` holds no tool. It passes before and after the change.
6. Every other test of the two files passes in the same job, among them
   `max_pages_written_as_an_integral_number_is_the_integer_the_schema_admits` and
   `security_control_a_late_reply_to_a_paused_reader_keeps_the_session`.
7. The scan `debug_override_reads` of `connectors-build` passes in the same job, and the lane
   `slow` of the pull request lists `release_build_ignores_a_debug_override` as passed.

## Scope

- `apps/connectors/src/mcp/projection.rs`: the comparison in `Snapshot::limits`,
  `WALK_REPLY_RESERVE`, `walk_deadline`.
- `apps/connectors/tests/security_mcp_deadline.rs`: `BUDGET`, the on-time test, `paused` and
  `spawn`, and the control's call of `paused`. `Served::server` is read and not changed.
- `apps/connectors/tests/mcp_tool_call_shapes_adversary.rs`: `REQUEST_MS`, `served()`, which
  writes the budget into the configuration and takes it as an argument for the new test,
  `Peer::start`, the walk test's page delay, the new test.

No other file changes: no unit test is added under `apps/connectors/src/mcp/`, and
`crates/connectors-host/src/local/owner/transport.rs` is not touched, because both names carry
the forwarded prefix and the server process reads them.

Other stories on these files:

- story:seven-mcp-tests-use-a-4-s-request-budget-and-shorter-holds writes both test files; this
  story depends on it and starts after it is merged.
- story:mcp-outbound-stdio-runtime (draft) carries an inferred scope `apps/connectors/src`,
  which holds `projection.rs`. It is not planned beside this story.
- story:test-binary-per-package-rest (draft, no scope) would move both test files under one
  test binary. It is not planned beside this story.

## Not in this story

- The other two ways named when the defect was found, moving both tests to the lane `slow` and
  rebuilding the on-time scenario from replies that need no listed tool: not built, no story.
- A release-build test of package `connectors`: see "Change"; no story.
- The four tests that story:seven-mcp-tests-use-a-4-s-request-budget-and-shorter-holds
  changed. Its "As built" gives one run each: three got shorter
  (`security_control_a_late_reply_to_a_paused_reader_keeps_the_session` 6.3 s,
  `adversary_stdin_closed_while_the_store_is_held_exits_successfully` 1.8 s,
  `adversary_a_lost_session_leaves_no_provider_read_open` 4.5 s) and
  `adversary_a_long_read_with_the_store_held_is_answered_and_the_session_stays_ready` stayed
  at 6.7 s. The control and the long-read test stay over 5 s; no story.
- `adversary_a_store_replaced_under_the_owner_ends_the_session` (16.5 s in the forge's gate job
  138270416028), which no story of the set changes.

## As built

Commit 8c225885e on the branch `story/two-mcp-tests-listed-budget`, not pushed. It changes 3
files, 121 lines added and 49 removed: `apps/connectors/src/mcp/projection.rs`,
`apps/connectors/tests/security_mcp_deadline.rs` and
`apps/connectors/tests/mcp_tool_call_shapes_adversary.rs`.

Product: `listed_request_ms(execution_ms)` and `walk_reply_reserve()` in `projection.rs` read
`CONNECTORS_TEST_OVERRIDE_MCP_LISTED_REQUEST_MS` and
`CONNECTORS_TEST_OVERRIDE_MCP_WALK_RESERVE_MS` through `connectors_sdk::debug_override_ms`,
with the read's `execution_ms` and 5,000 ms as the defaults.

Two departures from the story's item 4, both decided from runs on a loaded host:

- `max_pages_written_as_an_integral_number_is_the_integer_the_schema_admits` keeps
  production's 20,000 ms and passes no variable. The test waits out no budget. Under a short
  budget its server's start, which is bounded by the request time, missed its bound.
- The walk test runs under a request budget of 6,000 ms with a reply reserve of 1,000 ms,
  not 4,000 ms. At 4,000 ms the server's start took 4.2 to 4.6 s in runs at a load average
  over 100, or the first page was refused `timeout`.

Seen red before the product change: with the test changes alone, 4 tests failed.

Acceptance 3, one run with a mutant that was not committed: `LATE_WRITE_GRACE` in
`apps/connectors/src/mcp/stdio.rs:43` set to `Duration::ZERO`.
`security_an_on_time_reply_written_across_its_deadline_keeps_the_session` failed at
`security_mcp_deadline.rs:928`: "a reply that began inside its deadline ended the session
when its write crossed the deadline", with 4,096 bytes of the reply written after 4.500 s.
The control and `security_requests_past_their_deadline_each_get_one_reply_on_their_own_id`
failed in that run too, so the mutant is wider than the one case.

Acceptance 4, one run with a mutant that was not committed: in `projection.rs:519-525` the
arms of `match (answered, refused)` changed to `(Some(envelope), None)`,
`(_, Some(Ok(envelope)))` and `(_, Some(Err(rpc)))`, so a refused page wins over the records
taken. `a_walk_that_outlasts_the_request_budget_answers_the_records_taken` failed at
`mcp_tool_call_shapes_adversary.rs:442`: the reply was the error `timeout` with no records.
The other 5 tests of the binary passed.

Both mutants were reverted; the diff of the tree was 3 files, 121 added and 49 removed
before and after.

One run of the two binaries with the change, on the workstation at a load average of 20 to
23: 10 of 10 tests passed, the run took 14.924 s. Per test: the walk test 10.868 s (on main,
one run: 19.898 s), the on-time test 9.915 s (on main: 25.235 s), the past-deadline test
10.515 s, the control 9.940 s, the max_pages test 5.811 s,
`a_request_budget_under_the_reads_execution_time_lists_no_tool` 4.398 s. The two tests of
acceptance 1 were over its 9 s in this run; the forge's figures decide that acceptance and
are added with the pull request.

Six earlier runs with the change failed in part, each with 2 to 5 failing tests of the 10:
one on the build node and five on the workstation. Three of the five have their load average
in the log: 28 to 55 (2 failed, the walk test still at 4,000 ms), 104 to 108 (3 failed) and
88 to 133 (2 failed, the walk test at 6,000 ms); two have none (5 failed each). The failures
were the server's start bound (`stdio.rs:94-98`), the first page of the walk, and
`security_requests_past_their_deadline_each_get_one_reply_on_their_own_id` (not changed by
this story, "mcp_unavailable" under its start bound of 2 s, failed in all six). The walk test alone passed in
24.011 s at a load average of 113 to 168. Not measured: the same tests on main at that load.

Clippy with `-D warnings` over all targets of `connectors`, the format check and the three
scan tests `debug_override_reads` of `connectors-build` pass locally. Acceptances 1, 2, 6
and 7 are read from the forge's jobs of the pull request.

---
format: aep.planning-md/3
id: review-result:wave4-drafts-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, round 1, three drafts of wave 4
relations:
- reviews: story:two-mcp-tests-wait-out-the-20-s-request-budget-a-listed-tool-needs
- reviews: story:the-sixteen-ess-model-tests-read-schemas-generated-once-per-run
- reviews: story:a-provider-budget-that-ends-in-an-unanswered-tls-handshake-is-marked-as-the-providers-timeout
revision: 1
---
Verdict: needs-revision for each of the three drafts. Read on the branch of pull request
309 at 7f6d60b4e (the MCP draft) and on the branch of the four stories at 7e0bb9401 (the
other two).

Verdicts: two-mcp `needs-revision`, sixteen-ess-model `needs-revision`, provider-budget `needs-revision`.
No build or test was run by the critic.

## story:two-mcp-tests-wait-out-the-20-s-request-budget-a-listed-tool-needs (tree mcp-budget)

1. blocker, story line 60: the acceptance names no duration bound and joins "keep what they
   assert" with "red for the defect it guards" without naming a mutant. Change: "Each test ends
   within N s in the gate's test step (before: …), N set before implementation." plus one
   statement naming a mutant of product code per test (as the seven-story's As built 8 and 9).
2. blocker, `apps/connectors/tests/security_mcp_deadline.rs:895-900`: the on-time test returns
   green when `seen.began_on_time` is false ("scenario not built", story line 33). Change: "At the
   shortened budget the on-time test's `began_on_time` is true: the test fails instead of printing
   'scenario not built'."
3. blocker, story line 63: the release-build statement names no test, no variable, no job. The lane
   `slow` runs release tests only as `cargo nextest run --release --locked -p connectors-sdk
   --no-tests fail release_build_`; a release test in package `connectors` runs in no job. Change:
   name `release_build_<name>` in <package>, the variable `CONNECTORS_TEST_OVERRIDE_<NAME>_MS`, and
   the lane command or forge job that runs it.
4. warning, story lines 25-26: 22.9 s and 17.7 s are not in the cited seven-story, which lists
   22.5 s then 22.3 s (on-time) and 17.5 s then 17.0 s (walk). Use those or name the run.

Verified in code by the critic: `limits` accepts 20,000 ms alone for an implemented read
(`apps/connectors/src/mcp/projection.rs:274, 300, 311`); `WALK_REPLY_RESERVE` 5 s
(`projection.rs:664`); test names at `security_mcp_deadline.rs:889`,
`mcp_tool_call_shapes_adversary.rs:417`, `:450`, `adversary_local_mcp_server.rs:902`;
`debug_override_ms` at `crates/connectors-sdk/src/lib.rs:504`; `BUDGET` 20,000 ms
(`security_mcp_deadline.rs:730`), `REQUEST_MS` 20,000 (`mcp_tool_call_shapes_adversary.rs:43`).

## story:the-sixteen-ess-model-tests-read-schemas-generated-once-per-run (tree four-stories)

1. blocker, story lines 66-68: acceptance 1 leaves how the once-only generation is observed "settled
   before implementation"; no counting wrapper of `ess` exists in the tree. `resolve` accepts
   `CONNECTORS_ESS` (`crates/connectors-spec/src/toolchain.rs:74-79`), so a wrapper is feasible.
   Change: pick one form, e.g. a test that points `CONNECTORS_ESS` at a script that appends one line
   per start; after two runs of the binary the file holds 1 line; it fails on the code as it is (16).
2. blocker, lines 69-70: acceptance 2 "Shown once; the changed byte is not committed." names no
   place a reviewer reads it. Change: the As built records command, changed byte and line count, or
   a test that copies the 4 files to a temporary directory.
3. blocker, lines 73-76: acceptance 4 has no bound or margin; the forge baseline with the calls
   removed is not measured (line 44); at most 1.41 s is inside the noise of two forge jobs. Change:
   drop it, or "the 17 durations sum to at most N s in the pull request's gate job". Title should
   say "measured on 5 of 16 tests".
4. warning, lines 71-72: acceptance 3 joins three outcomes. Split; make "no assertion changes"
   checkable: "the diff removes no line containing `assert`".

Verified in code: 17 tests in `adapters/sql/tests/ess_model.rs`, 16 call `schema` (line 24;
not `query_text_and_parameters_are_bounded_in_characters`, line 1719); 4 files under
`adapters/sql/spec/ess`; `toolchain::pin()`; `CARGO_TARGET_TMPDIR` used in other test files;
2.18 s = 3.424 − 1.242.

Out of lane note: the `ess_model` binary sits beside the rule "one integration-test binary per
crate" (design).

## story:a-provider-budget-that-ends-in-an-unanswered-tls-handshake-is-marked-as-the-providers-timeout (tree four-stories)

1. blocker, story lines 70-71: acceptance 2 names no test; no existing test sets a provider budget
   on a stalled body (`provider_timeout_adversary.rs` has no `with_provider_budget`; the budget test
   at `crates/connectors-host/src/http.rs:2135` asserts `Timeout` and never reads `upstream_answer`).
   Change: a new named test in `provider_timeout_adversary.rs`: peer sends status and headers and
   then stalls, budget 300 ms, asserts `Timeout` and `upstream_answer = true`; passes before and
   after.
2. warning, line 74: acceptance 4 does not name where the 5 s is read. Change: the nextest output of
   the forge's "repository gate" job; state that no new test is a `slow_lane_` test.
3. warning, lines 66-69: acceptance 1 names neither its test nor the commit it is red on; the red run
   was on the five-tests tree, the base commit run is not measured (line 51). Change: "fails on the
   head of `story/client-timeouts` merged (the commit this story branches from) with
   `upstream_answer = true`, shown by running the new test before the change"; name the test.

Verified in code: `provider_error` marks every timeout that is not `is_connect()` as `answered()`
(`http.rs:1144-1156`); `send` applies `request.timeout(remaining)` (`http.rs:586-591`);
`with_provider_budget` called at `adapters/catalog/src/local.rs:45, 1248, 2458`; production
timeouts 15 s and 5 s (`http.rs:81, 83`); semantics.md §6 at line 1952, sentence at line 2392;
`a_tls_handshake_that_never_completes_sent_no_request_and_is_not_marked`
(`provider_timeout_adversary.rs:191`); `a_body_that_stalls_after_the_provider_answered_is_the_providers_timeout`
(line 120).

Out of lane note: whether `upgrade` has the same mark (scope).

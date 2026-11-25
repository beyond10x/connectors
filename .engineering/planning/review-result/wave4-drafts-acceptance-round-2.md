---
format: aep.planning-md/3
id: review-result:wave4-drafts-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic, round 2, three drafts of wave 4
relations:
- reviews: story:two-mcp-tests-wait-out-the-20-s-request-budget-a-listed-tool-needs
- reviews: story:the-sixteen-ess-model-tests-read-schemas-generated-once-per-run
- reviews: story:a-provider-budget-that-ends-in-an-unanswered-tls-handshake-is-marked-as-the-providers-timeout
revision: 1
---
Verdicts: the MCP draft `approve`, the provider-budget draft `approve`, the ess_model draft
`needs-revision`. Read at commit 7ac4c2086. No build or test was run.

## Round-1 findings

All 11 are resolved.

- MCP draft: 1 (bound of 9 s, two named mutants), 2 (`began_on_time` asserted; the early
  `return` at `apps/connectors/tests/security_mcp_deadline.rs:895-900` is a change to make),
  3 (the helper's release test `release_build_ignores_a_debug_override`,
  `crates/connectors-sdk/src/lib.rs:569`, run by `.github/workflows/slow.yml:113`), 4 (the
  durations now match the seven-tests story).
- ess_model draft: 1 (`generations.log` counted over 4 child processes), 2 ("As built" records
  the line count), 3 (the acceptance on time is dropped), 4 (three statements).
- Provider-budget draft: 1 (a named new test), 2 (the nextest output of the "repository gate"
  job), 3 (the test, the base commit and the failing message are named).

## New findings

ess_model draft:

1. blocker: no acceptance observes that the 16 existing tests read the shared directory.
   Acceptance 1 uses its own empty directory and children; acceptances 4 and 5 hold for a helper
   nobody calls. Today `adapters/sql/tests/ess_model.rs:24-37` holds the one `Command::new(ess)`.
   Change: add a diff property: in `adapters/sql/tests/ess_model.rs` the executable `ess` is
   started in one place, inside the helper's generation branch, and no test passes
   `tempfile::tempdir()` to `schema`.
2. warning: the child test is a third new test and runs in every ordinary run; acceptance 7
   bounds two tests, and no line says what the child does without its directory variable.
   Change: "The child test returns at once when the variable that names its directory is absent;
   the three new tests end within 5 s each."

MCP draft and provider-budget draft: no finding.

## Facts checked, as the stories say

- MCP draft: the one `request_milliseconds` comparison (`apps/connectors/src/mcp/projection.rs:311`);
  `WALK_REPLY_RESERVE` and `walk_deadline` (`projection.rs:664-672`); `LATE_WRITE_GRACE`
  (`apps/connectors/src/mcp/stdio.rs:43, 71`), so the on-time mutant can be built; the walk mutant
  acts on the match at `projection.rs:518-524`.
- Provider-budget draft: `connector_layer` is not feature-gated (reqwest 0.13.5 `client.rs:2475`);
  `connect_timeout` stays outermost (`connect.rs:141-147`); `is_timeout` accepts an `io::Error` of
  kind `TimedOut` (`error.rs:140-141`); `is_connect` follows hyper-util's (`error.rs:162`);
  tower-layer and tower-service are 0.3.3 in `Cargo.lock`; `Failure` derives `PartialEq` and
  `Debug`. The "Inferred, not run" paragraph is honest and acceptance 1 decides it.

Out of lane: the MCP draft's scope omits `served()`, which writes the budget at
`apps/connectors/tests/mcp_tool_call_shapes_adversary.rs:299`; acceptance 5 needs it to take a
budget.

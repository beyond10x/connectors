---
format: aep.planning-md/3
id: review-result:wave4-drafts-design-round-1
kind: review-result
status: active
title: Design critic, round 1, three drafts of wave 4
relations:
- reviews: story:two-mcp-tests-wait-out-the-20-s-request-budget-a-listed-tool-needs
- reviews: story:the-sixteen-ess-model-tests-read-schemas-generated-once-per-run
- reviews: story:a-provider-budget-that-ends-in-an-unanswered-tls-handshake-is-marked-as-the-providers-timeout
revision: 1
---
Verdict: needs-revision for each of the three drafts. Read on the branch of pull request
309 at 7f6d60b4e (the MCP draft) and on the branch of the four stories at 7e0bb9401 (the
other two).

Verdicts: all three `needs-revision`. No cycle. No build or test run by the critic.

## story:two-mcp-tests-wait-out-the-20-s-request-budget-a-listed-tool-needs (tree mcp-budget)

1. blocker, frontmatter lines 7-10: no frontmatter `scope:` (only a body section, lines 66-70) and
   no edge to `story:seven-mcp-tests-use-a-4-s-request-budget-and-shorter-holds`, which is active,
   declares both test files and leaves `LATE_BUDGET` beside `BUDGET`
   (`apps/connectors/tests/security_mcp_deadline.rs:730-736`). `waves` prints it as unassessed.
   Change: add `scope:` for the three paths and `depends_on` to the seven-mcp story.
2. blocker, lines 48-51: way 1 does not say which comparison the override replaces. `limits` uses
   `expected.0` (20,000) against the definition's `execution_ms`
   (`apps/connectors/src/mcp/projection.rs:300`), against the configuration's
   `request_milliseconds` (`:311`), and as the read's `ReadBudget` (`:437`, `:751`). The override
   may only replace the right-hand operand at `:311`; `expected.0` stays. `WALK_REPLY_RESERVE`
   (`:664`, used in `walk_deadline` at `:671`) is a second variable and becomes a function, like
   `idle_bound`.
3. warning, lines 50-51: say that the names carry the prefix `CONNECTORS_TEST_OVERRIDE_` (no edit
   of the forwarded list: the override is read in the MCP server process, `stdio.rs:90`,
   `projection.rs`; the spawn forwards any prefixed name,
   `crates/connectors-host/src/local/owner/transport.rs:1603`), are read only through
   `debug_override_ms` (guard scan `crates/connectors-build/tests/debug_override_reads.rs:20-36`),
   and are set on the `server()` command after `.env_clear()` in both test files
   (`security_mcp_deadline.rs:434`, `mcp_tool_call_shapes_adversary.rs:322`).

Out of lane: the Change still lists three ways although the operator chose one.

## story:the-sixteen-ess-model-tests-read-schemas-generated-once-per-run (tree four-stories)

4. blocker, line 6 and Change 1 lines 49-54: the digest directory is filled lazily without a lock;
   tests that start together on a cold directory each run `ess generate`; the forge is cold on
   every checkout (`.config/nextest.toml:1-9`: no thread cap, no setup script). Acceptance 1
   counts only the warm second run. Change: single-flight creation (atomic `create_dir` of a lock
   sibling, others wait for the rename) and count `ess` starts on the cold run; or restate the
   claim.
5. warning, lines 24-25 and acceptance 4 lines 73-76: the figures read state the nagle story
   produces (`every_page_is_one_the_model_admits` 7.0 s to about 0.6 s; `ess_model.rs:8` includes
   `mysql/fixture.rs` by `#[path]`). Add `depends_on:
   story:the-mysql-test-fixture-sends-its-packets-without-the-nagle-hold`.

Verified: 16 of 17 tests call `schema`; 4 source files; `connectors_spec::toolchain::version()`
gives the pin without a process (`toolchain.rs:34`); `CARGO_TARGET_TMPDIR` precedent
(`crates/connectors-build/tests/adversary_docs_audit_literal.rs:14`); no other story declares
`ess_model.rs`. Not a finding: `mysql_adversary_2.rs:145` holds a second copy of `schema`, called
once.

## story:a-provider-budget-that-ends-in-an-unanswered-tls-handshake-is-marked-as-the-providers-timeout (tree four-stories)

6. blocker, lines 82-83: "This story starts after it is merged" but the only edge is `informed_by`
   (line 9); both stories write the same two files. Add `depends_on:
   story:five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts`.
7. blocker, lines 78-79: three places set the budget's timeout: `send`
   (`crates/connectors-host/src/http.rs:588-590`), the read POST (`http.rs:1275-1283`), `upgrade`
   (`http.rs:814`). `upgrade` decides `NotSent` versus `Lost` with `is_connect()`
   (`http.rs:831-835`); a handshake cut by the budget there is `Lost(provider_error(..))`, marked
   answered; it races the outer `timeout_at(until, …)` (`http.rs:818-824`), which yields an
   unmarked `Lost` Timeout. Form 2 cannot change `upgrade`'s `is_connect()` branch. Change: say
   the scope covers all three sites, or that `upgrade` stays out with the reason; state what a
   handshake cut by the budget returns in `upgrade`.
8. warning, lines 57-61: form 1 assumes a per-request connect bound; reqwest 0.13.5 has
   `connect_timeout` on `ClientBuilder` only (`async_impl/client.rs:1475`), `RequestBuilder` has
   only `timeout` (`async_impl/request.rs:294`); the client is built once (`http.rs:253-259`) and
   cloned by `with_provider_budget` (`http.rs:174`). Name the client rebuild or connector layer,
   or drop the form.
9. warning, lines 48-49: the fixed call reports `timeout` at `stage = admission`
   (`runtime.rs:524`, `Failure::Timeout`) instead of `dispatch` (`runtime.rs:507`,
   `Failure::ProviderTimeout`); `next_action` stays `retry_explicitly`; `upstream_answer` is an
   internal `Error` field; `Refusal::from_provider` (`runtime.rs:445-450`) keeps a reason only
   when it is set. Add one sentence to Change and one acceptance line on `stage`.

The chosen behaviour agrees with `contracts/cli/v1alpha1/semantics.md:2392-2393`, the
client-connect path (`http.rs:1148-1150`) and the pre-dispatch `budget_expired`
(`http.rs:662-667`).

Not established: whether nextest on the forge runs the 16 tests concurrently; whether reqwest
reports a request timeout during connect as a non-connect timeout (the draft marks it inferred).

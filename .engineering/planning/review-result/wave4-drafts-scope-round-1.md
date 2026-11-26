---
format: aep.planning-md/3
id: review-result:wave4-drafts-scope-round-1
kind: review-result
status: active
title: Scope critic, round 1, three drafts of wave 4
relations:
- reviews: story:two-mcp-tests-wait-out-the-20-s-request-budget-a-listed-tool-needs
- reviews: story:the-sixteen-ess-model-tests-read-schemas-generated-once-per-run
- reviews: story:a-provider-budget-that-ends-in-an-unanswered-tls-handshake-is-marked-as-the-providers-timeout
revision: 1
---
Verdict: needs-revision for each of the three drafts. Read on the branch of pull request
309 at 7f6d60b4e (the MCP draft) and on the branch of the four stories at 7e0bb9401 (the
other two).

Verdicts: all three `needs-revision`. No build or test run by the critic.

## story:two-mcp-tests-wait-out-the-20-s-request-budget-a-listed-tool-needs (tree mcp-budget)

1. blocker, story lines 17, 21: 22.9 s and 17.7 s are not in the cited seven-mcp story; its As built
   (lines 224-227) gives 22.5 s before and 22.3 s after (on-time), 17.5 s before and 17.0 s after
   (walk); forge 21.7 s and 16.9 s.
2. blocker, lines 40-55, 66-70: the owner repeats the 20 s check:
   `crates/connectors-host/src/local/owner/governed/bounded.rs:70,74-75` accepts an implemented
   read only if `budget.limits() == (20_000, 15_000, …)` and `limits["execution_ms"] == 20_000`;
   the MCP side passes the value from `Snapshot::limits` into the owner's `ReadBudget`
   (`projection.rs:437-438,453`). An override of `expected.0` (`projection.rs:274`) makes the owner
   refuse `Unsupported` and needs the fixture metadata `"execution_ms":20000`
   (`mcp_tool_call_shapes_adversary.rs:278`, `security_mcp_deadline.rs:380`) to follow. An override
   of only the `request_milliseconds >= expected.0` comparison (`projection.rs:311`) leaves
   `bounded.rs` untouched. The Change must name which.
3. blocker, lines 63-64: a release-build test in package `connectors` runs in no job:
   `.github/workflows/slow.yml:113` and `docs/development.md:362` run `cargo nextest run --release …
   -p connectors-sdk … release_build_`; `crates/connectors-build/src/ignored.rs:2825` asserts
   `pair("-p","connectors-sdk")`, `ignored.rs:2835-2841` the test names. Either list those three
   files in Scope or state that `release_build_ignores_a_debug_override`
   (`crates/connectors-sdk/src/lib.rs:569`) is the release test the decision asks for.
4. warning, lines 1-11: no frontmatter `scope:`.
5. warning, lines 72-76: "Not in this story" excludes neither way 2 nor way 3 and does not name
   `max_pages_written_as_an_integral_number_is_the_integer_the_schema_admits`, which shares
   `REQUEST_MS` (`mcp_tool_call_shapes_adversary.rs:43,299,450`).

Verified: 4096-byte pipe and 12,000-byte tool (`security_mcp_deadline.rs:36,771`); "scenario not
built" (`:897`); both test files `.env_clear()` the CLI child (`mcp_tool_call_shapes_adversary.rs:322`,
`security_mcp_deadline.rs:434`).

## story:the-sixteen-ess-model-tests-read-schemas-generated-once-per-run (tree four-stories)

1. warning, "Not in this story" last bullet: the other callers of `ess generate` get no story id and
   no "no story": `adapters/sql/tests/mysql_adversary.rs:94`, `adapters/sql/tests/mysql_adversary_2.rs:154`
   (both on `spec/ess`), and the ess_model tests of `adapters/loki`, `adapters/grafana`,
   `adapters/prometheus`. Name the files and a destination ("no story" or
   `story:where-do-the-1298-s-of-the-tests-under-5-s-go`).

Verified: 2.18 = 3.424 − 1.242; 1.41 = 16 × 0.088; 16 calling tests at `ess_model.rs:338,396,433,
569,626,785,979,1082,1275,1304,1368,1417,1492,1554,1642,1695`; 4 source files, `system.yaml` imports
nothing outside; `sha2`, `hex`, `tempfile`, `connectors-spec` are dev-dependencies already
(`adapters/sql/Cargo.toml`), no `Cargo.toml` edit; `connectors_spec::toolchain::version()` is public.

## story:a-provider-budget-that-ends-in-an-unanswered-tls-handshake-is-marked-as-the-providers-timeout (tree four-stories)

1. blocker, "Not in this story" and "Scope": the upgrade path has no destination. Two places set the
   budget's timeout: `send` (`crates/connectors-host/src/http.rs:589`), `upgrade` (`http.rs:814`).
   `upgrade` wraps `request.send()` in `tokio::time::timeout_at(until, …)` with the same `until`
   (`http.rs:818`); that arm returns `Upgraded::Lost(Timeout)` without `answered()`
   (`http.rs:819-824`); the marked path is the `Ok(Err(error))` arm (`http.rs:831-832`), reached only
   if reqwest's timer wins the race. An unanswered handshake returns `Lost`, not `NotSent`. State
   excluded by name (with this reading and a destination) or covered.
2. warning, Defect last paragraph: a budget that ends before the TCP connection opens is the
   contract's "A connection that never opened" (`contracts/cli/v1alpha1/semantics.md:2392`) and
   goes through the same `provider_error` branch (`http.rs:1145-1152`); it is neither claimed nor
   excluded. Add an acceptance row or an exclusion with a destination.

Verified: the existing handshake test sets the connect override, not a provider budget
(`provider_timeout_adversary.rs:189-203`); mapping `Timeout if upstream_answer => ProviderTimeout`
at `crates/connectors-host/src/local/runtime.rs:507`; the contract text already says what the draft
asks for, so no `spec/` or contract file changes.

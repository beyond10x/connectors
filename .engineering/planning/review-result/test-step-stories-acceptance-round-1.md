---
format: aep.planning-md/3
id: review-result:test-step-stories-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, round 1, the test-step story set
relations:
- reviews: story:the-gates-test-step-ends-within-120-s-on-the-forge
revision: 1
---
Recorded by the session that ran the panel, 2026-11-24. The critic read the first draft of twelve
stories that decompose story:the-gates-test-step-ends-within-120-s-on-the-forge. The set was revised once after this
report (eighteen stories); the report below is the critic's text about the first draft, with
the path of the worktree shortened to `<tree>`.

verdict: needs-revision

needs-revision (set)

Per story:
- four-host-tests-no-longer-wait-out-the-30-s-lock-wait: needs-revision
- seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait: needs-revision
- eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping: needs-revision
- four-idle-exit-tests-no-longer-span-several-5-s-recovery-sweeps: needs-revision
- seven-mcp-tests-use-a-4-s-request-budget-and-shorter-holds: needs-revision
- five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts: needs-revision
- the-catalog-provider-tests-read-a-committed-lowering-of-the-released-definition: needs-revision
- the-catalog-crate-tests-ingest-the-released-definition-once-per-process: needs-revision
- eight-adapter-and-catalog-tests-drop-waits-that-prove-nothing: needs-revision
- five-host-unit-tests-shorten-the-waits-they-chose: approve
- seven-cli-tests-shorten-their-waits-or-a-production-wait: needs-revision
- eighteen-tests-take-a-template-store-or-jail-built-once-per-run: needs-revision

The same two defects recur across the set. Each affected story is listed below, so each can be revised on its own.

1. Slow-lane acceptance names no test and no trigger. `slow.yml` runs only on `ess/**`, `Cargo.lock` and a few build files, and has no test step. A story's pull request never starts the lane for `crates/connectors-host/**`, `apps/**` or `adapters/**`. "Passes in the lane `slow`" cannot be observed until the story names the test, the command and the path filter.
2. Acceptance 3 ("a build without `debug_assertions` ignores it") names no job. A debug-profile gate cannot show what a release build does.

Findings, most severe first:

1. four-host — "One test still runs against the real WAIT_BOUND of 30 s ... the pull request names it. Inferred candidate: contending_metadata_open_refuses_at_its_bound_and_recovers_after_release". That test is moved to the library tests with a short wait and is summed in acceptance 1 at 9 s or less, so it cannot also run at 30 s. Fix: name a separate test and its `cargo test` command, and add its path to the `slow.yml` triggers.
2. eighteen — "Bound: the estimate itself, rounded up to 108 s". The estimate has no margin, and 11 of its rows are an unmeasured 50 % assumption. The story's own figures disagree: 107.7 s against 95.3 s for 50 % of 190.6 s. Fix: state the margin and where the 108 comes from, or measure first.
3. eighteen — "Before the template is built, the cost of `Metadata::initialize` ... is measured apart, and the three figures are written into this story." Ordering and a record in a story body cannot be observed. Fix: replace it with an observable such as a nextest setup-script log line.
4. eighteen — the title claims a template "built once per run", but no acceptance observes it. Name an observation such as one setup-script run per nextest invocation and tests that copy and never write the template. The "keeps 10 runs in the test step" statement is in Change only. Acceptance 2 could pass with the 100 runs removed and nothing else changed.
5. eleven — "one run under the real 30 s admission bound still passes in the lane slow". The story says production's 30 s is not located: `DEADLINE_MS`, `EXECUTABLE_LEASE_MS`, `execute` and the `Task::Validate` arm are all candidates. Acceptance can pass with the override on a bound production does not use. Fix: name the constant, and the observed refusal at exactly that bound.
6. eleven — "the pull request names both" (the 60 s lifetime run and the 30 s admission run). This is the unnamed-test pattern from finding 1. It also states no state for the clock offset: which of the three processes sees it, and the value the test advances.
7. four-idle — acceptance 1 and 2 never say the shortened tests still span several sweeps. That is stated only in Change point 4 ("an idle bound that still spans several sweeps"). Fix: state that each test's bound is at least 3 sweep intervals and the values it sets. Acceptance 2 also lacks the job from defect 2.
8. catalog-crate — "sum to at most 8 s". The story admits the sub-step of `lower` is unmeasured and that a once-per-process cache saves time only in tests that lower more than once. The 0.3 s per test is unexplained. Nothing observes that ingest happens once, and nothing says a variant lowering is unaffected by the cache. The baseline of 11 tests does not match the 13 test attributes. Fix: measure first, then bound; add an assertion on ingest count and on variant results.
9. catalog-provider — acceptance 2 and 3 name no path for the fixture, no home for the fast check, no list of digest inputs, and no job that runs the check. Acceptance 3 passes if the digest omits the lowering sources. Acceptance 3 is also two statements, the digest and the check. Acceptance 1 compares 66 tests before against 67 attribute-counted tests after.
10. eight — acceptance 1 does not say how `no_suite_sees_the_installation_another_suite_sets` stays out of the gate. Acceptance 2 names no `cargo test` command or trigger path.
11. eight — Change 1 "cap the loop by attempts" gives no number and no source. A cap too low stops hitting the race, so the test passes and proves nothing. Fix: state the attempts, and say the link swap must be observed to occur.
12. eight — Change 4 "a short list of candidate files tried before the walk" and Change 7 "shorter sleeps" have no stated check that the refusal and the survivor probe are still exercised.
13. seven-cli-shorten — acceptance 1 counts test 7 (`adversary2_the_identification_retry_loop...`) at 10.2 s with no seam, so the bound passes with it unchanged. Acceptance 2 does not say what is observed for "the wait is the production value (10 s, 2 s, 5 s)". Nothing runs at those production values in a lane.
14. seven-mcp — "Change: 4,000 ms" for the budget, but no value is stated for the group E holds ("multiples of `LEASE_MS`"). The reply margin of 0.6 s and 0.5 s past `BUDGET` is not carried into the acceptance. Acceptance 2 does not say which of the three group D tests runs at 20 s.
15. five-tests — "The plan names no run at the 5 s connect timeout." Acceptance 2 therefore proves no production value for the connect timeout. The override form is unstated. The "one run at the production 15 s and 10 s" tests are unnamed.
16. seven-cli-slow-start — acceptance 2 ("one case at the real 10 s ... the pull request names it") is unnamed. No acceptance says the delay (1.5 s) exceeds the override wait (1 s). The two-read test's short `--deadline-seconds` is only "inferred". Acceptance 3 lacks the job from defect 2.

five-host: each change names the test, a value and a bound source. The only gap is that preserved assertions are implicit, which is not a finding.

What I read: 12 of 12 ids, as story files under `.engineering/planning/story/` (`sed` from `## Defect`; frontmatter of the first four only), plus `.github/workflows/slow.yml`, the acceptance procedure and the critic rubric. I did not run `aep plan artifact kinds` or `lifecycle`, and did not open the test plan.

Could not establish:
- Line numbers: I cite by sentence, with no `path:line`, so the findings block carries no `line`.
- Whether the forge's nextest runs the shortened tests with the margins assumed (the "after" figures are unmeasured estimates).
- Out of my lane: eight stories edit the same `slow.yml`, `.config/nextest.toml` is shared (the eight and eighteen stories), and `transport.rs` is shared across three stories. Those are for the design and parallel-safety critics.

```findings
- {file: .engineering/planning/story/four-host-tests-no-longer-wait-out-the-30-s-lock-wait.md, category: acceptance, severity: blocker, verdict: needs-revision, origin: introduced, message: "acceptance 2 names no real-30-s test; its inferred candidate is also summed in acceptance 1 at 9 s or less, and slow.yml has no trigger path for crates/connectors-host"}
- {file: .engineering/planning/story/eighteen-tests-take-a-template-store-or-jail-built-once-per-run.md, category: acceptance, severity: blocker, verdict: needs-revision, origin: introduced, message: "acceptance 2 bound is the estimate with no margin, resting on an unmeasured 50 % assumption, and 107.7 s disagrees with 95.3 s"}
- {file: .engineering/planning/story/eighteen-tests-take-a-template-store-or-jail-built-once-per-run.md, category: acceptance, severity: blocker, verdict: needs-revision, origin: introduced, message: "acceptance 1 (measure before building, figures written into the story) is not an observable state"}
- {file: .engineering/planning/story/eighteen-tests-take-a-template-store-or-jail-built-once-per-run.md, category: acceptance, severity: blocker, verdict: needs-revision, origin: introduced, message: "no acceptance observes the template built once per run, unmodified by tests, or the gate keeping 10 runs"}
- {file: .engineering/planning/story/eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping.md, category: acceptance, severity: blocker, verdict: needs-revision, origin: introduced, message: "the 30 s admission bound the override targets is not located, so acceptance can pass on a bound production does not use"}
- {file: .engineering/planning/story/eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping.md, category: acceptance, severity: warning, verdict: needs-revision, origin: introduced, message: "acceptance 2 leaves both slow-lane tests to the pull request and states no clock-offset state or trigger path"}
- {file: .engineering/planning/story/four-idle-exit-tests-no-longer-span-several-5-s-recovery-sweeps.md, category: acceptance, severity: blocker, verdict: needs-revision, origin: introduced, message: "acceptance never states that each shortened test's idle bound still spans several sweep intervals; acceptance 2 names no job"}
- {file: .engineering/planning/story/the-catalog-crate-tests-ingest-the-released-definition-once-per-process.md, category: acceptance, severity: blocker, verdict: needs-revision, origin: introduced, message: "8 s bound rests on an unmeasured, unexplained 0.3 s per test; no observation that ingest happens once or that variant lowerings are unaffected; 11 vs 13 test baseline"}
- {file: .engineering/planning/story/the-catalog-provider-tests-read-a-committed-lowering-of-the-released-definition.md, category: acceptance, severity: blocker, verdict: needs-revision, origin: introduced, message: "acceptance 2 and 3 name no fixture path, no home or job for the fast check, no digest inputs; acceptance 3 is two statements; 66 vs 67 test baseline"}
- {file: .engineering/planning/story/eight-adapter-and-catalog-tests-drop-waits-that-prove-nothing.md, category: acceptance, severity: warning, verdict: needs-revision, origin: introduced, message: "acceptance names no mechanism keeping the moved test out of the gate, no cargo test command or trigger path for the slow lane"}
- {file: .engineering/planning/story/eight-adapter-and-catalog-tests-drop-waits-that-prove-nothing.md, category: acceptance, severity: warning, verdict: needs-revision, origin: introduced, message: "attempt cap, candidate-file list and shorter sleeps have no stated value or check that the race, refusal and survivor probe are still exercised"}
- {file: .engineering/planning/story/seven-cli-tests-shorten-their-waits-or-a-production-wait.md, category: acceptance, severity: warning, verdict: needs-revision, origin: introduced, message: "acceptance 1 counts the identification-retry test at 10.2 s with no seam; acceptance 2 names no observation of the production value; no lane run at 10/2/5 s"}
- {file: .engineering/planning/story/seven-mcp-tests-use-a-4-s-request-budget-and-shorter-holds.md, category: acceptance, severity: warning, verdict: needs-revision, origin: introduced, message: "group E hold values are not stated and the 0.5-0.6 s reply margin is not in the acceptance; acceptance 2 does not name the 20 s test"}
- {file: .engineering/planning/story/five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts.md, category: acceptance, severity: warning, verdict: needs-revision, origin: introduced, message: "no production-value run of the 5 s connect timeout; the override form is unstated; acceptance 2 tests unnamed; acceptance 3 names no job"}
- {file: .engineering/planning/story/seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait.md, category: acceptance, severity: warning, verdict: needs-revision, origin: introduced, message: "acceptance 2 test unnamed; no acceptance says the owner delay exceeds the override wait; acceptance 3 names no job"}
```

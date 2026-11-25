---
format: aep.planning-md/3
id: review-result:test-step-stories-scope-round-1
kind: review-result
status: active
title: Scope critic, round 1, the test-step story set
relations:
- reviews: story:the-gates-test-step-ends-within-120-s-on-the-forge
revision: 1
---
Recorded by the session that ran the panel, 2026-11-24. The critic read the first draft of twelve
stories that decompose story:the-gates-test-step-ends-within-120-s-on-the-forge. The set was revised once after this
report (eighteen stories); the report below is the critic's text about the first draft, with
the path of the worktree shortened to `<tree>`.

verdict: needs-revision

needs-revision

Coverage of the 81 tests is clean. The findings are one seam that cannot deliver its estimate, five acceptance items that add work nobody asked for, and one story that exceeds the parent's own table.

Findings, most severe first:

1. story:the-catalog-crate-tests-ingest-the-released-definition-once-per-process — its seam is an in-process memo of the ingest, which the folder rule says is not an accepted form under nextest, so most of the 31.6 s to 5.0 s claim cannot be delivered. The story's own "Open" section concedes it. A per-process memo saves time only in a test that lowers more than once, so the ten single-lowering tests gain nothing and the 0.3 s per test is unexplained.
   - Quote 1: "A `OnceLock` or other in-process global is not this form under cargo-nextest, which runs each test in its own process" (`<home>/beyond10x/AGENTS.md:36`).
   - Quote 2: "The ingest of the released definition is made once per process and reused by `lower`" (`.engineering/planning/story/the-catalog-crate-tests-ingest-the-released-definition-once-per-process.md:51`).
   - Revise so the claim narrows to the one multi-lowering test, or the seam becomes a cache on disk or committed file.

2. Five stories add an acceptance neither the parent nor the plan asked for, a test that a build without `debug_assertions` ignores the new override. Each admits "Which job builds the tests without `debug_assertions` is not in the plan", so implementing it may need a new release-profile test job that counts against the 120 s step. The plan says only "debug-only".
   - story:seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait — acceptance 3, line 91.
   - story:eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping — acceptance 3, line 130.
   - story:four-idle-exit-tests-no-longer-span-several-5-s-recovery-sweeps — acceptance 2, line 73.
   - story:five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts — acceptance 3, line 101. The plan says "new test seam", not "debug-only".
   - story:seven-cli-tests-shorten-their-waits-or-a-production-wait — acceptance 2, line 116.
   - Revise by dropping the acceptance or stating in the body that it is a decision beyond the plan.

3. story:seven-cli-tests-shorten-their-waits-or-a-production-wait — it adds three overrides where the parent's table says two. The story notes the disagreement at line 105 and resolves it by taking the third. Plan row 47 (`catalog_provider_failure.rs:201`) says only "needs a new seam". Making it debug-only is the story's choice.
   - Quote: "| S8c waits in the CLI tests | 7 | 64.2 | 34.7 | two new debug-only overrides |" (parent, `.engineering/planning/story/the-gates-test-step-ends-within-120-s-on-the-forge.md:77`).
   - Revise by recording the third as a decision, or correcting the parent's row.

Answers to your checks:

- **81 tests, claimed once each.** Per story: S1 4, S2 7, S3 11 (B 7 plus C 4), S4 4, S5 7 (D 3 plus E 4), S6 5, S7a 4, S7b 1, S8a 8, S8b 5, S8c 7, S9 18. The total is 81. No test is claimed twice and none is missing.
  - Group M's 14 split 3 (S8a), 4 (S8b) and 7 (S8c).
  - Group L's 4 split 3 (S8a) and 1 (S8b).
  - The extra band tests are claimed only by S7a (62) and S7b (10).
  - The tests shared by file are split by test, not by file: `local_cli.rs` rows 4, 46, 56 and 77 go to S9, S8c, S8c and S4. The same holds for `security_mcp_deadline.rs` (S5 and S9) and `owner_slow_start.rs` (S2 and S9).

- **Numbers.** Every before and after matches the plan's section 2 and section 4 rows. The afters sum to 336.0 s.
  - Row sums differ from the plan's group figures by 0.1 to 0.2 s in S1, S2, S8b and S8c. The stories say so.
  - S9's 107.7 s is 56.5 % of 190.6 s, not the "50 %" the plan states. The story notes this.

- **What nothing in the set claims.** These are left to the parent, and the parent states them:
  - the test step at 120 s or less;
  - the gate failing over 120 s, with its `connectors-build` unit test (every story says "Not in this story");
  - the order "fails only after the step is under the budget".
  - The set reaches about 382 s, and the parent says "What closes that gap is not known" (parent line 86). No story takes the remaining 1,049 s.
  - "No test of 60 s or longer" and "ten slowest under 60 s" need no story. `.config/nextest.toml:5` has `terminate-after = 2` on a 30 s period, which kills a test at 60 s. `crates/connectors-build/src/gate.rs:543` prints the ten slowest.
  - The second measurement (pull request 292, 842 s, 62 more tests) is not analysed anywhere. The parent does not promise it.

- **Limits, "slow" and "skipped".**
  - The parent's Limits read "A larger runner, more shards and a higher timeout are not this story's fix (folder rule)" (line 102) and "The 223 skipped tests and the lane `slow` are not measured here" (line 103). No sentence says nothing may be skipped, and none forbids the lane `slow`.
  - A test moved to the lane `slow` is inside the parent's own table. Row S8a reads "one test to lane `slow`" (line 75), so `no_suite_sees_the_installation_another_suite_sets` is covered.
  - S9's cut of `adversary2_a_same_build_owner_at_capacity_is_never_another_build_over_100_runs` from 100 runs to 10 is in plan sections 2 and 4 but not in the parent's table. It is not forbidden. S9 acceptance 3 keeps the 100 runs in the lane `slow`.
  - The "one real-wait run stays in `slow`" items in S1, S2, S3, S5, S6 and S7a are plan section 4's column, so they are traceable.
  - No story adds a larger runner, a shard or a higher timeout.

Out of my lane:
- Parallel safety: parent line 80 says "S1, S6, S7a, S7b, S8a and S8b touch no file of another story", but S1, S6, S7a and S8a each list `.github/workflows/slow.yml`, which eight stories edit. S8a and S9 may also both touch `.config/nextest.toml`.
- Acceptance: S9's bound is the estimate itself (108 s), where the other stories use 1.5 times the estimate.

What I read: the parent, the plan (256 lines) and the 12 stories in full. I also ran `aep plan artifact show` and `grep` on `gate.rs`, `.config/nextest.toml` and `AGENTS.md`. I did not run `aep plan artifact graph` to the end (its output was truncated) and did not check for older stories that claim part of the parent.

What I could not establish: whether the gate's test step is nextest only, which decides whether `adversary_one_binary:159` loses coverage in the gate.

```findings
- file: .engineering/planning/story/the-catalog-crate-tests-ingest-the-released-definition-once-per-process.md
  line: 51
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the seam is an in-process memo of the ingest, which the folder rule says is not an accepted form under nextest, so the claim of 31.6 s to 5.0 s for eleven tests reaches beyond what the seam can deliver"
- file: .engineering/planning/story/seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait.md
  line: 91
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 3 asks for a test that a build without debug_assertions ignores the override, which neither the parent nor the plan asked for, and the story says no job builds that way"
- file: .engineering/planning/story/eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping.md
  line: 130
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 3 asks for a test that a build without debug_assertions ignores the clock offset and the bound override, which neither the parent nor the plan asked for"
- file: .engineering/planning/story/four-idle-exit-tests-no-longer-span-several-5-s-recovery-sweeps.md
  line: 73
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 2 asks for a test that a build without debug_assertions ignores the sweep interval, which neither the parent nor the plan asked for"
- file: .engineering/planning/story/five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts.md
  line: 101
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 3 asks for a test per override that a build without debug_assertions ignores it, and the plan names only a test seam and not a debug-only one"
- file: .engineering/planning/story/seven-cli-tests-shorten-their-waits-or-a-production-wait.md
  line: 116
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 2 asks for a release-build test per override, and the story adds three overrides where the parent's table row says 'two new debug-only overrides'"
```

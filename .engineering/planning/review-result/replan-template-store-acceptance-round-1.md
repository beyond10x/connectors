---
format: aep.planning-md/3
id: review-result:replan-template-store-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, round 1, the re-plan of the template-store story
relations:
- reviews: story:the-gates-test-step-ends-within-120-s-on-the-forge
revision: 1
---
Verdict: needs-revision. Read at commit 198fc306b. S9d is the registry draft, S9e the MySQL
fixture draft, N6 the question story.

## Findings

1. S9d, acceptance 1. Blocking. "At most 1" cannot tell a process-wide value from a narrower
   cache, and the stated reason for allowing 1 does not hold.
   - The test calls `initialize` before `take_costs()`. `initialize` reaches
     `provision_with_lock`, which calls `registry()` (`er.rs` line 2377). The first build always
     falls before the count starts. Every correct implementation counts 0 after `take_costs()`.
   - A cache per store path, or a `thread_local!` cache built during `initialize`, also counts 0
     and passes. Only a cache per handle or per simulated process fails.
   - The draft says the statement "holds the change on its own". It does not hold "once per
     process".
   - Settles it: call `registry()` once before `take_costs()`, then assert exactly 0; add a count
     taken on a fresh thread, which must also be 0 (a `thread_local!` cache counts 1 there).
   - The seam placement is ambiguous. If `timed` wraps the `get_or_init` call, correct code
     counts every call and fails. If it wraps only the build closure, it counts builds. State the
     closure.
2. S9d, acceptance 1. Note. Which interface makes "5 persists that each change a record" is not
   named.
   - `persist(er, connection, mode, staged)` (`er.rs` line 4163) needs a SQLite connection.
   - "`fresh_opens()` rose by 3" depends on the 5 persists reusing the pooled handle.
     `simulate_process` only sets a thread-local (line 3000), so nothing there resets a
     `OnceLock`. That is correct and wanted.
   - The new test should take `own_processes()` (line 3064), as the other fresh-open tests do.
   - The red run needs a patch that does not exist today: `registry()` has no `timed` seam now.
     The statement does not say where the red run's output is recorded.
   - Existence: `take_costs` (3208), `timed` (3187/3194), `simulate_process` (3000),
     `fresh_opens` (2959) and `note_fresh_open` (2990) exist as described. At least 8
     `registry()` calls today is plausible (5 in `persist_batch`, 3 in `open`).
3. S9d, acceptance 4 and its derivation. Blocking. The bound is not one derivable number.
   - Statement 4 says "at most 92 s". The derivation says that when the base sum is under
     108.0 s the bound is "that sum minus 16.0 s". With a base of 100 s a reader cannot tell
     whether 92 or 84 is the bound.
   - For a base of 108.0 s or more no rule is stated. A base of 130 s with the estimated saving
     lands near 98 s and would go red against 92.
   - "The newest gate job of its base commit" may not exist: the gate's scope job can skip the
     code build on a planning-only base commit.
   - Arithmetic holds. Small drifts: the R+P column sums to 35.981, not 35.979; test 3's saving
     is 1.54, not 1.55.
4. S9d and S9e, same defect. Blocking. "A job over the bound is not run again until it passes"
   contradicts itself. The house rule is that a red job is not re-run as a remedy. State whether
   the pull request may merge with the time statement red.
5. S9d, acceptance 2 and 3. Note. Both are checkable from the diff. The new test lives in
   `mod tests` of `er.rs`, the product file: rephrase as "outside `mod tests` of `er.rs` the diff
   touches no test code". Statement 3 follows from statement 2 and cannot fail alone.
6. S9e, acceptance 1. Blocking. The decision rule is incomplete.
   - The 3.0 s limit is not derived from a figure in the draft.
   - Nothing requires the three base-commit runs to stay high. Add a floor for each base run.
   - The `strace` runs and the 10 ms counts have no pass or fail role.
   - One run at 3.0 s or more refutes even when the other two are under 2 s; a run at 5 s is a
     gain and is discarded. State that this is intended.
   - Where the six durations are recorded is not stated.
   - Recomputed and correct: 91.4 %, 0.79 s, 0.60 s, 0.9 s, 83 test functions.
7. S9e, acceptance 2 and 3. Note. Statement 2 joins two outcomes. Statement 3 follows from "the
   pull request changes none of them". Statement 4's margin is wide enough.
8. N6, acceptance. Blocking in part.
   - All twelve items carry a "To measure first" line.
   - Statement 1 accepts any one-sentence "dropped"; no threshold says when an item deserves a
     story.
   - Statements 2 and 3 cover only answers that attribute a cost to this repository or to the
     dependency. Items 10 and 12 can end in neither.
   - Statement 4 bundles three rules; items 9 and 12 have no ordering rule.
   - Statement 1 says every item gets an answer; statement 4 says item 10 is taken up only when
     the fixture story is refuted twice. Say that an unmet item 10 gets a "dropped" entry.
   - Scope says no file is written, but the `## Answer` section lives in the story file.
   - 8.07 − 5.61 = 2.46, written 2.45.
9. Parent arithmetic. No figure is wrong: 81 tests, 1,117.8 before, 361.2 after, 368.6, 268.6,
   375.0, 373.6, 1,671.6, 417.9; the old 352.9, 1,650.9 and 412.7 also check.

## Checked and found correct

- The line citations in `er.rs` match the tree; the file is unchanged between d5cf22480 and
  f9f263a2.
- The five non-test callers of `registry()` are as the draft says.
- `model_digest` already caches per process.
- N6's item numbers match the references in S9d and S9e.

## Not checked

- The 43-case count, the `ess generate` timings and the measurement figures themselves.
- The forge job 138270416028 (no network).
- The dependency's source for the clone claim.

## For another critic

- N6's ordering rule (measure after S9d merges) has no `depends_on` edge to S9d.

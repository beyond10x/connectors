---
format: aep.planning-md/3
id: review-result:replan-template-store-acceptance-round-3
kind: review-result
status: active
title: Acceptance critic, round 3, the re-plan of the template-store story
relations:
- reviews: story:the-gates-test-step-ends-within-120-s-on-the-forge
revision: 1
---
Verdict: **approve**

Read at commit cb5e856e8 (`git rev-parse HEAD` printed it; `git status --short` was empty). Reviewed: N7, S9d, S9e, N6 and N5, the five stories in this lens, all opened and read in full.

## Round-2 findings of the acceptance role

1. N7 red run: resolved. The Defect now states the tolerance as +2/−1 lines. The red run inserts 10 lines at line 1 of `er.rs`, and acceptance 1 says 3 lines are the least that fail. The window was re-derived against `cli_spec_mapping_adversary.rs` lines 181 to 199 and `er.rs` lines 5093 to 5097. It stays green at +1, +2 and −1 and goes red at +3 and −2. With +10 the window shows old lines 5085 to 5087, which do not hold `ConnectionListCursor`.
2. S9d acceptance 2 against `record_writer_for_tests`: resolved. The text now reads "the only test-support code changed outside it is `record_writer_for_tests` (point 4)", and `er.rs` line 1169 is that function.
3. N6 unit of the 1.0 s for items 9 and 10: resolved. Item 9 is summed over the 16 tests of `ess_model.rs`. Item 10 is the duration of `every_page_is_one_the_model_admits`.
4. N6 item 13 count and statistic: resolved. One figure per test and run is defined, with the leader or original not counted and the range stated. The aggregate is the median of three runs per commit. A base median of 0 means the test does not meet today, and a story is due anyway.
5. N5 question 5: resolved. The set is fixed by the base job, tests in only one job are listed by name, no job is re-run to make a spread pair, and with no pair the difference is reported without a verdict.
6. Round-2 notes 4, 5, 6 and 10 are also applied:
   - the new thread calls `own.namespace().enter()`;
   - the N7 arm checks are exact text;
   - the nextest command for the emissions test is named;
   - item 12 gets one entry per timer over 1.0 s of own time.

## Blocking findings

None.

## Notes

1. N6 item 13, third test: "where its two threads meet" is left to be read before its counter is placed. The draft says so openly; it is not an acceptance gap.
2. S9d acceptance 1: `State::suppress` is an upsert. The draft's phrase "each change a record" does not matter for the red run, because a persist with no change still pays both builds. The "at least 8 builds and 13 parses" bound holds either way.
3. S9d acceptance 2 joins a diff property to a statement about the eleven tests. It is checkable as written.
4. N7 acceptance 4: the failure must name the file and line of the comment. That is a message requirement on the new test, and it is stated.
5. S9e: the figure "jobs running in the pool beside each duration" is a record and no part of the decision, so it is not an acceptance gap.

## Checked and found correct

- N7 arms against `er.rs`:
  - Line 5095 trimmed is `("connectors.cli.ConnectionListCursor", "Active") => {`, and line 5096 holds `ExpireConnectionListCursor`.
  - The quoted string `"connectors.cli.ConnectionListCursor"` occurs on one line of the `removal_command` body only, since the Expire name differs after the dot.
  - In `update_command`, the `PublishBinding` string occurs once in the body (line 4934), and line 4933 is exactly the pinned pattern line.
  - `fn removal_command(` and `fn update_command(` each occur once at column 0 (lines 5093 and 4910).
  - The bodies end at the first `}` in column 0.
- The nextest filter `test(/^cli_spec_mapping_adversary::/)` is valid. The module is declared in `tests/main.rs`, and the pass2 module is excluded by the colons.
- `the_committed_metadata_emissions_are_current` (`metadata_conformance.rs` line 966) only compares digests, so it runs fast with no synthesis. `inputs.json` lists 27 files under `ess/domains/`, and the commits c97cb5660 and df1f2c5b1 each changed `cli.yaml` and 4 lines of `inputs.json`.
- S9d seams:
  - `timed` is a bare closure call outside `cfg(test)` and a per-thread counter in tests.
  - `take_costs`, `fresh_opens`, `simulate_process`, `own_processes`, `OwnProcesses::namespace` and `ProcessNamespace::enter` exist at the cited places. `enter` is documented as not inherited by a new thread.
  - After points 2 and 4, `definition_bundle()` has no caller outside the build and the nine test-module calls (6119, 6460, 6539, 6784, 6877, 7078, 7534, 7591, 8288). `registry()` has the eight test-module calls cited.
  - A fresh open reaches `registry()` at line 2484 and a persist at line 4334, which supports the red-run counts "by reading".
- The 43 cases of the first test recompute: relocated has 5 entries and PROFILES has 5, giving 1 + 10 + 4 + 1 + 5 + 20 + 2.
- N6 item 13: the line citations (`issue.rs` 299 to 345, `Barrier` 311, sleep 319, `registry/tests.rs` 805, `mutations/tests.rs` 497, `lock_within` 1465 and the 5 ms sleep) match. The three count definitions can be placed with a temporary counter, for example at `issuance` for the first test and the loop of `lock_within` for the second.
- Statements 1 and 2 of N6 are internally consistent. A base median of 0 and a head median of 0 or below half of the base cannot both fall into "dropped".
- S9e's second probe now says six new runs and that the base runs are not reused.

## Not checked

- No build and no test was run, so the counts "at least 8 builds and 13 parses", the 43 cases at run time and the red runs are readings of the source, as the drafts say.
- The run-5 figures that were unchanged this round (items 9 and 10 and the table) were not rechecked.
- The forge job figures and the clone cost could not be checked.

## For another critic

- The wording "outside `mod tests`" and the 2-line tolerance of N7's Defect, which depends on S9d's line shift, are design and parallel-safety matters. The edge to N7 holds either way.

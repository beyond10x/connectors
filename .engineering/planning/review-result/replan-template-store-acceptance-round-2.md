---
format: aep.planning-md/3
id: review-result:replan-template-store-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic, round 2, the re-plan of the template-store story
relations:
- reviews: story:the-gates-test-step-ends-within-120-s-on-the-forge
revision: 1
---
Verdict: needs-revision. Read at commit 42f599466. N7 is the citation draft, S9d the registry
draft, S9e the MySQL fixture draft, N6 the question story of the re-plan, N5 the question story
of the tests under 5 s.

## Round-1 findings

1. S9d acceptance 1, count and seam: resolved.
2. Interface and red-run record: resolved. The helpers `take_costs`, `timed`,
   `simulate_process`, `fresh_opens` and `own_processes` exist as described.
3. S9d bound: resolved. One formula, "base minus 16.0 s"; it covers a base commit whose gate
   job has no test step.
4. A red job is not re-run: resolved in S9d and S9e.
5. S9d statements 2 and 3: resolved in part (new finding 2).
6. S9e statement 1: resolved; one gap (new finding 7).
7. S9e statements 2 and 3: unchanged and harmless.
8. N6 acceptance: resolved in part (new findings 8 to 10).
9. Parent arithmetic: resolved.

## New findings

1. Blocking. N7 acceptance 1 and the Defect bullet "adds or removes one line above line 5095"
   name a red run that does not go red. The test takes lines 5095 to 5097 and requires
   `ConnectionListCursor` in them. Line 5095 is the arm and line 5096 is
   `"connectors.cli.ExpireConnectionListCursor"`. With one line inserted the window shows old
   lines 5094 to 5096 and still passes; with one line deleted it shows old 5096 to 5098 and
   passes. The text leaves the window at a shift of 3 lines or more. Settles it: insert at
   least 3 lines (for example 10) and correct the Defect sentence; the reason for S9d's edge
   reads "a shift of 3 lines or more".
2. Blocking. S9d acceptance 2 says "outside `mod tests` of that file the pull request changes
   no test code". Point 4 changes `record_writer_for_tests` (line 1169), which is
   `#[cfg(any(test, feature = "test-support"))]` code outside `mod tests`. Settles it: "no
   assertion, loop or trial count of the eleven tests changes, and `mod tests` gains only the
   new test", or name `record_writer_for_tests` as the one allowed change.
3. Note. S9d point 5: "four in the test module (lines 6119, 6460, 6539 and 6784)" is wrong.
   The test module calls `definition_bundle()` at nine places (also 6877, 7078, 7534, 7591,
   8288). The conclusion holds: the build is the only caller outside the test module.
4. Note. S9d acceptance 1: the fresh thread acts as the default process id and does not
   inherit the namespace (`ProcessNamespace::enter`, `er.rs` near line 3044). Say that the
   thread calls `own.namespace().enter()`.
5. Note. N7 point 3. For `update_command` the comment says no Revoked row change maps to
   `PublishBinding`; the test only requires that the string occurs. For `removal_command` "an
   arm with both" has no definition at text level, and `ExpireConnectionListCursor` holds
   `ConnectionListCursor` as a substring. State the checks as: the line that names
   `"connectors.cli.ConnectionListCursor"` also names `"Active"`; the arm that yields
   `PublishBinding` does not name `Revoked`.
6. Note. N7 acceptance 2 has no named run for "fails when the finder returns the lines of
   another function": show it once in the pull request's body with a changed finder.
   Acceptance 4 does not name the command that runs
   `the_committed_metadata_emissions_are_current` (a unit test in `metadata_conformance.rs`).
7. Note. S9e statement 1, the second probe: "the same six runs" does not say whether the three
   base runs are used again or made again.
8. Blocking. N6 statement 1: the 1.0 s for a story is "summed over the eleven tests". Items 9
   and 10 are in the SQL test, which is not among the eleven. Name the unit for them: the
   saving in that test's own duration.
9. Blocking. N6 item 13: "overlap" is not defined (does the leader count; one caller or two),
   and neither is the statistic over three runs. State the count and one aggregate, for example
   the median of the three runs, and say what a base that already counts 0 means.
10. Note. N6 statement 1: "exactly one entry of one kind" does not fit item 12, which holds
    several timers with different owners. Allow one entry per timer over 1.0 s of own time.
11. Blocking in part. N5 question 5. The two forge jobs have different test sets and "under
    5 s" is not tied to one job: fix the set by the base job's durations and name the tests
    present in only one job. A pair of gate jobs of main with no code change between them may
    not exist, and no job is re-run to make one: add "no pair exists, so the spread is not
    known and the difference is reported without a verdict".

## Checked and found correct

- N7: `cli.yaml` line 2218 and `auth_bindings.yaml` line 521 cite what the draft says; the test
  spans lines 181 to 199; `removal_command` (5093, ends 5106) and `update_command` (4910, ends
  5062); the `slow` job's step exists and is triggered by `ess/**`; `inputs.json` lines 52 and
  56; the module is declared in `tests/main.rs` line 11.
- S9d: `model_digest` does not call `definition_bundle()`; the callers of `registry()` match;
  13 parses and 8 builds of the red run follow from the call sites; the bound's arithmetic.
- S9e: citations and arithmetic.
- N6: the figures recompute; the line citations of item 13 match the source.
- Parent: 22 drafts (15 + 3 + 4); 11 + 1 = 12 unconfirmed tests.

## Not checked

- The 43-case count and the nextest filter (no build run); the forge job; the clone cost.

## For another critic

- Whether S9d's edits shift lines above 5095 by 3 or more is a design matter; the edge to N7
  holds either way.

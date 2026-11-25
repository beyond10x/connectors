---
format: aep.planning-md/3
id: review-result:lane-story-acceptance-reread-after-d24
kind: review-result
status: active
title: 'Acceptance re-read of the lane story after D24: approve at revision 13'
relations:
- reviews: story:the-lane-slow-runs-the-ignored-timing-family
revision: 1
---
Verdict: approve, at revision 13 of story:the-lane-slow-runs-the-ignored-timing-family, after
needs-revision at revision 10.

Acceptance critic, two reads on 2026-11-25. The story went back to revision by its own acceptance
5 of revision 9: none of the eight `Timing` entries without a prerequisite ended within 60 s. The
operator decided that the eight move to `Deferred` and the lane runs `slow_lane_` tests only (D24,
option A). Read-only; no build and no test.

## First read, revision 10: needs-revision

1. Acceptance 5, the `grep -c` on the eight test names: it named no file and no pattern, and on
   `crates/connectors-build/src/ignored.rs` such a count is 16 before any comment exists, because
   the names are in the match arms and in a test list.
   Changed: acceptance 7 counts the lines that hold the id
   story:eight-timing-tests-have-no-job-and-none-ends-within-60-s in that file and expects 8;
   Change 7 lets no other line hold the id.
2. Acceptance 5 held three outcomes (classification, report status, comment).
   Changed: acceptance 5 holds the classification and the `evaluate()` outcome in one unit test;
   the comment is acceptance 7.
3. Acceptance 3 named no fields of the report and did not tie the check to the one pull request
   that holds both stories.
   Changed: it names `result.unknown`, `result.failed`, `result.selected`, `result.executed`, the
   outcome's `name` in the report's form and its `status`, and opens with that pull request.
4. The risk that the 100-run test runs over 60 s in the lane was not stated.
   Changed: a bullet of "Not in this story" states the outcome, that acceptance 3 then fails, and
   that the fix is fewer rounds in the other story.

Not blocking, changed too: the summary said one step; the sum of the five times.

## Second read, revision 13: approve

Nothing blocking. Notes: the count of acceptance 7 passes with eight lines even if one test were
named twice; the story leaves names and figures to the diff. The new paragraph of
story:the-100-run-capacity-stress-keeps-10-runs-in-the-gate leaves its four acceptance statements
checkable.

## Checked and right

- Under the lane's options a `Deferred` test has `selected` false and the status `not-selected`.
- `--required` refuses an empty selection (`Report::accepted`).
- The table holds eight `Timing` entries without a prerequisite and a ninth with two.
- No `slow_lane_` test exists in the workspace before the 100-run story.
- The draft story's figures match the lane story's section "Measured".

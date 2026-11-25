---
format: aep.planning-md/3
id: review-result:lane-story-design-reread-after-d24
kind: review-result
status: active
title: 'Design re-read of the lane story after D24: approve at revision 13'
relations:
- reviews: story:the-lane-slow-runs-the-ignored-timing-family
revision: 1
---
Verdict: approve, at revision 13 of story:the-lane-slow-runs-the-ignored-timing-family, after
needs-revision at revision 12.

Design critic, two reads on 2026-11-25, after the operator's decision that the eight `Timing`
entries without a prerequisite move to `Deferred` and the lane runs `slow_lane_` tests only (D24,
option A). Read-only; no build and no test.

## First read, revision 12: needs-revision

1. Blocker. The lane story's acceptance 3 reads the test that
   story:the-100-run-capacity-stress-keeps-10-runs-in-the-gate adds, and that story `depends_on`
   the lane story. The edge back would close a cycle. The 100-run story did not say that the two
   go in one pull request.
   Changed: the 100-run story says it and says what its `depends_on` means; the lane story says
   why no edge runs back.
2. Warning. The draft story:eight-timing-tests-have-no-job-and-none-ends-within-60-s read what the
   lane story writes and had only `informed_by` to it.
   Changed: it has `depends_on` the lane story.
3. Warning. Change 6 did not cover the guide's paragraph on the `deferred` family, which says that
   each deferred test's ignore reason names a limitation or a follow-up story.
   Changed: Change 6 names the paragraph and the sentence it gains.
4. Warning. "Nothing was run for this draft." contradicted the section "Measured".
   Changed: removed, with the clause about the eight on a runner of the forge.
5. Warning. The lasting coupling was not stated: from the merge on, the lane fails when no
   `slow_lane_` test exists.
   Changed: "Decision beyond the plan" ends with it.
6. Warning. Change 7 did not keep the eight apart from the `Deferred` arm whose comment names
   story:main-failing-ci-journeys.
   Changed: the eight get match arms of their own under their own comment.

## Second read, revision 13: approve

All six closed; no contradiction in the new sentences; `aep plan artifact validate` prints
`valid`.

## Checked and right

- No cycle among the declared edges of the lane story, the 100-run story, the draft and the eight
  stories that `depends_on` the lane story.
- No other story of the set lists `ignored.rs`, `slow.yml` or `docs/development.md` in its scope.
- No job selects the family `Deferred`.
- Story ids in comments of `ignored.rs` exist already; the comment of acceptance 7 is the same
  form.
- The eight stories that move a test into the lane use the name rule and the same command; none
  cites an old `Timing` entry.

Left to parallel safety: two drafts outside the set also touch `ignored.rs`
(story:main-failing-ci-journeys, story:mounted-custody-kind-secret-volume).

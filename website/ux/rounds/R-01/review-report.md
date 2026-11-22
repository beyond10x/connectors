# R-01 review of the report

Reviewer: report reviewer (`references/review-agents.md` § 2), fresh agent, 2026-11-22. Inputs:
`report.md`, `round.json`, `shots/` (by name), `decision.md`, `../trend.md`, `ux/findings/`, the raw
runs in `website/.cache/ux/results/measure/` (layout 51, axe 60, screen-only 49, keyboard 6 files),
and the tools `tests/ux/*.ts`, `tests/ux/core/*.ts`, `ux/adapter.ts`, `playwright.config.ts`. No
recordings, no questionnaire, no last round, no round README and no operator notes exist for R-01.
Nothing was run.

**Verdict: not fit to send as is.** The gate numbers recompute exactly. The task numbers do not mean
what they say: 4 of the 5 screen-only failures at gating sizes or worst values come from defects in
the bot (T1 to T5), not from the site.

**Default action:** send gates 1, 2 and 5 with the caveats in § 4. Do not take any R-01 task value
into `ux/baseline.json`. Fix T1 to T5, rebuild `build/` from the working tree, and run the
screen-only, keyboard and frames part again in one run without `UX_KEEP`. Then write the report
again. (`build/` has been gone since about 09:03, so a new build is needed anyway.)

## 1. Are the numbers right and meaningful?

**Recomputed from the raw JSON** (`recompute.js` in the session scratch dir): every gate cell of
every viewport and theme, all 7 task-worst rows, the 49 per-viewport cells, and the trend row
(gate 1 0, gate 2 0, gate 3 3, gate 4 1, gate 5 236, 38/49 done, 98 actions). All match
`round.json` and `report.md`: **0 differences**.

What the numbers mean:

| number | reads as | is |
|---|---|---|
| gate 3 = 1 at 1366x768 light and dark | completion hard to see | `exit-status-older-binary` `completion_miss` from the measured `goalProbe`, which checked the target's vertical middle (T1). The target is a 10-line paragraph (`docs/guides/upgrade-the-cli.md` lines 20 to 29). Its first line was in view, its middle was below the viewport, and `elementFromPoint` returned null, so the probe answered "covered". Artifact. |
| gate 4 = 1 at 1366x768 light | the reader fails a task | the same T1 artifact: the bot scrolled past the paragraph, went back twice and ended on `/docs/concepts/local-runtime` after 30 actions |
| gate 4 = 0 at 1366x768 dark | passes | a stale record (T5): `exit-status-older-binary@1366x768@dark.json` was written at 07:35 by the screen-only.ts that was replaced at 07:49. All other screen-only records are from 08:30 to 08:42. The dark frame of the same task was not rewritten at 08:49 either, so with the current code the dark run probably did not finish. The "✗30 / 6" cell compares two versions of the bot. |
| gate 3 = 1 at 1280x720 light | a false completion | real as a bot event: `onboarding-gitlab` ended on `/connectors/` with `false_complete` 1. The run still counts as done (T4) and gives the anchor task its worst value, 27 actions, against 2 at 1366, 1024 and 1440. Probable cause: the stale page read in T2 (the step after the click on GitLab shows 262 words, the same as `/docs` before it). |
| gate 4: 10 failures at 768x1024 and 390x844 | 5 of 7 tasks fail on tablet and phone | the bot gets stuck on its own drawer (T3). It opens the drawer again on the target page and then turns the wheel on a page the drawer has locked. The reading load stays the same for 22 to 27 steps (for example `install-command@390x844`: 180 words, 24 times). |
| gate 5 = 77 / 78 / 81 (trend 236) | 236 serious accessibility problems | 229 are `link-in-text-block`, the known loss UXF-002. The other 7 are `scrollable-region-focusable`, measured on a build from 05:23 that does not have the working tree's `src/css/tables.css` fix (07:04, UXF-004). Each node is counted again in every viewport and theme. |
| gate 2 = 0 at the gating sizes | always-visible holds | true only with 338 held violations (sidebar at page end, UXF-003, on 168 of 171 pages in light and 170 in dark) and with search absent (UXF-001) left out of every count |
| "pages 171" | gates on 171 pages | layout only. axe ran on 14 template pages at 1366x768 and 1280x720 and on 4 pages at each measured size. |
| "worst over the gating viewports and both themes" | 4 combinations | 3: 1280x720 dark has no axe, screen-only or keyboard run. Keyboard ran only at 1366x768 light. |
| `screen_only.time_to_act_steps.max` 0 for status-pod-exec | acted at once | the bot scrolled 13 to 21 screens before the row (T7) |
| onboarding-gitlab 2 vs 3 actions, install-command 3 vs 4 | a difference between theme or size | T2 noise: the extra action is a look taken on the previous page's content. With a count tolerance of 0, the ratchet will flap on it. |
| `keyboard.presses` "–" for exit-code-walk | not run | the run timed out at 60 s (UXF-007 says twice), so the rotating card fails the keyboard part of gate 5. The report does not say so. |

R-01 is the first round: no baseline, no last round, no trend to compare. The trend row is not
comparable with the next round: its gate 3, 4 and 5 sums leave out 1280x720 dark, and "38/49" counts
a false completion as done.

**The `goalProbe` change after the measure** (`ys`, probes.ts:191). The probe now tests two points on
the target's first line (top + 2 px and the middle of the first 40 px) at the same x, in place of
the vertical middle. The `visible` test already requires those points to be in the viewport, so
`elementFromPoint` can no longer return null for an element that is off screen. Expected effect on
R-01: `exit-status-older-binary` at 1366x768 finishes with `completion_miss` 0. That would make
gates 3 and 4 0 at 1366x768, and the trend's gate 3 1 and gate 4 0. This has not been measured. It
also changes the meaning in two places:
- A first line under the sticky navbar now counts as covered, where the middle test could pass it.
- A paragraph covered below its first line now passes.

Neither the onboarding-gitlab false completion nor the drawer failures depend on the change. The
change sits in the tool source while `round.json` still holds data from the old probe, and nothing
in the round records that (T6).

## 2. Findings the report misses

- **The rotating card fails the keyboard part of gate 5:** `exit-code-walk` timed out with keyboard
  alone (no record; UXF-007). The report shows "–".
- **`scrollable-region-focusable` is wider than UXF-004 says.** UXF-004 names only
  `/docs/reference/contracts/cli` at 1366x768. The raw data also has 3 nodes on
  `/docs/reference/cli` at 1280x720 and 1 on `/docs/connectors/gitlab` at 390x844. All are
  `.b10x-table-wrap > table`, the shape the working-tree fix targets. That fix has not been measured.
- **The worst moments are all held or measured-only:** 3 sidebar groups held under UXF-003, and 3
  gate-1 groups at 1024x768 and 390x844. None counts against a gate. The report does not say this, so
  a reader takes them as gate failures.
- **The contact sheets show stale frames:** the frames of the `exit-status-older-binary` task date
  from 07:45 and 07:47, before the bot change. Neither theme reached the goal at 08:4x, so the frame
  shows an end state that the measured code never reached.
- **The round is not deterministic in places.** The same route at the same size gives different
  results per theme:
  - Gate 2 at `/docs/reference/contracts/model/domains/connectors-feed` and `connectors-refresh`:
    no sidebar violation in light, one in dark.
  - The sidebar rect on `connectors-oauth2`: [0,-461,300,234] in light, [0,-310,300,384] in dark.
  - axe `link-in-text-block` on `/docs`: 15 at 1366x768 light, 16 in every other run.
- **The site's results came from a build older than the working tree:**
  - `build/` was dated 05:23. `tables.css` changed at 07:04, before the layout sweep (07:08).
  - `adapter.ts` and `measure.spec.ts` changed at 07:49, between the layout and axe part
    (07:08 to 07:42) and the screen-only part (08:30 to 08:42).
  - The report says "from the built site" and does not name which build.
- **No R-01 run supports a site-side screen-only defect on tablet or phone.** The phone data only
  shows this: every task opens the drawer, picks an item, and opens the drawer again.

## 3. Tool defects

- **T1 `goalProbe` tested the vertical middle (fixed after the measure, not re-measured).**
  `tests/ux/core/probes.ts` goalProbe. Input:
  `screen-only/exit-status-older-binary@1366x768@{light,dark}.json`, `completion_miss` 1 in both
  themes, on the paragraph in `docs/guides/upgrade-the-cli.md:20-29`.
- **T2 `settle()` does not wait for the client-side route to render.**
  `tests/ux/core/fixture.ts` settle, used after `waitForURL` in `tests/ux/core/screen-only.ts:118-119`.
  In 8 of 70 page-changing clicks, the next step reads the same word count as the page before.
  Examples: `onboarding-gitlab@1366x768@dark` steps 0 and 1 (195 and 195 words, one extra look);
  `onboarding-gitlab@1280x720@light` steps 2 and 3 (262 and 262 words); `install-command@1280x720`
  and `@1440x900`.
- **T3 the bot reopens the navigation drawer and scrolls a locked page.**
  `tests/ux/core/screen-only.ts:128-135`: the opener is tried on every new page and never closed.
  `navScrollers` in `ux/adapter.ts` has no drawer menu. Input: all 10 failures at
  `*@768x1024@light` and `*@390x844@light`, whose reading load stays flat for 22 to 27 steps.
- **T4 a goal found off the task page counts as done.** `tests/ux/core/screen-only.ts:98-101` sets
  `done = true` together with `false_complete`. Input: `onboarding-gitlab@1280x720@light.json`,
  where `endPath` is `/connectors/`, `done` is true and there are 27 actions. The task's worst value,
  gate 4 and the trend's "38/49" all count it as done.
- **T5 old results are merged silently.** `tests/ux/setup.ts`: with `UX_KEEP=1`, a test killed at
  60 s leaves the previous version's record in place. `.cache/ux/frames/` is never cleared.
  `round-report.spec.ts` shows a missing record as "–". Input: the screen-only record
  `exit-status-older-binary@1366x768@dark.json` (07:35), the frames `{light,dark}--task_exit-status-older-binary.png`
  (07:45 and 07:47), and the missing `keyboard/exit-code-walk.json`.
- **T6 no provenance.** `tests/ux/round-report.spec.ts` writes no build revision, build time or tool
  digest to `round.json`, and no per-record time or tool version. Input: build 05:23 against
  `tables.css` 07:04, the tools changed at 07:49 and 08:57, and the data from 07:08 to 08:44.
- **T7 `time_to_act_steps.max` drops the last run of looks.** `tests/ux/core/screen-only.ts:115,167,195`:
  `lookRuns` grows only on a click or a back. Input: `status-pod-exec@*` (12 to 21 scrolls, value 0)
  and `exit-code-walk@*` (value 0).
- **T8 the labels say more than was measured.** `tests/ux/round-report.spec.ts:97-128`:
  - "pages" counts layout pages only.
  - "both themes" in the Tasks heading and the trend header, while 1280x720 dark ran no axe, bot or
    keyboard.
  - Keyboard ran at 1366x768 light only.
  - The worst moments do not mark held or measured-only.
  - The gate 5 count mixes in the known loss without a column of its own.
- **T9 gate 2 and axe depend on timing.** `tests/ux/sweep.ts` (the light "end" state is measured
  before the toggle, the dark one after it). Input: the `connectors-feed`, `connectors-refresh` and
  `connectors-oauth2` sidebar results and `/docs` `link-in-text-block` 15 against 16 (§ 2). With a
  count tolerance of 0, the ratchet will not be stable.
- **T10 `layout_shift.score` includes the extra direct visit.** `tests/ux/core/screen-only.ts:177-200`
  reads the shift sum after `page.goto(target)`, and the shift is carried through sessionStorage.
  The key is not the skill's `layout_shift.per_minute`.

## 4. Caveats to send with the round

1. Gates 1, 2 and 5 at the gating sizes are as measured.
   - Gates 1 and 2 are 0 only with UXF-001 (search) and UXF-003 (sidebar at the end of a page) held.
   - Gate 5 is 229 known-loss nodes (UXF-002) plus 7 `scrollable-region-focusable` nodes. Those 7
     come from a build without the working tree's `tables.css` fix.
2. The task metrics, gates 3 and 4, and the per-viewport table are not fit for a baseline. Of the
   12 failures and false completions:
   - 10 come from the drawer (T3);
   - 1 from the probe (T1);
   - 1 from a stale page read and done-counting (T2, T4).
   - 1366x768 dark `exit-status-older-binary` is a stale record (T5).
3. 1280x720 dark was swept for gates 1 and 2 only. Axe, the screen-only user and the keyboard ran in
   both themes only at 1366x768. The trend sums 3 of the 4 gating combinations.
4. The rotating card `exit-code-walk` timed out with keyboard alone (UXF-007).
5. The data came from a build of 05:23 and from two versions of the tools. `goalProbe` changed after
   the measure. `build/` has since been deleted, so the measured site cannot be inspected again.
6. No recordings, no questionnaire, no L4 or L5, and no person took part. The heuristic review's
   evidence is screenshots only.

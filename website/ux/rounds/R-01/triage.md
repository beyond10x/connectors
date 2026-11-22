# R-01 triage

Rule: skill § 3.3, `priority = severity weight × frequency × confidence ÷ cost`.

## Caveats

- Calls: R-01 has no recorded sessions. Calls are the 49 screen-only scenarios: 7 tasks at 6 viewports in
  light, plus 1366x768 in dark. A page-level finding counts the scenarios whose route passes its
  page. Search and the link-in-text-block rule are on every page, so their frequency is 1.
- Confidence is 2 only where a metric and the heuristic review agree (UXF-013). Elsewhere it is 1.
- The host's load average was 86 to 102 on 20 CPUs during the whole measure. Timings are 5 to 10
  times slower than on an idle host. Two measure tests hit the 60 s cap: keyboard `exit-code-walk`
  and screen-only `exit-status-older-binary` at 1366x768 light.
- Bot defects, not site findings, fixed or noted before the gate:
  - The goal probe read a tall paragraph as "covered" when its middle was below the viewport. That
    made `exit-status-older-binary` miss its goal at 1366x768 (completion_miss 1). The probe now
    checks the first line only (`tests/ux/core/probes.ts`). It was fixed after the measure, so
    R-01's round.json still holds the old value.
  - At 768 and 390 the bot opens the mobile menu again on the goal page. The open drawer locks the
    page scroll, and the bot scrolls in place up to the 30-action cap. Those ✗ cells are measured
    only and are not site findings. Fix it in the bot in R-02.
  - `onboarding-gitlab` at 1280x720 light timed out on the GitLab page and later finished on `/`
    (false_complete 1). The other 6 runs took 2 to 4 actions. Treated as a load flake; the gate
    run checks it again.
- The report reviewer (`review-report.md`) recomputed every gate and task number from the raw JSON;
  all of them match. Its verdict is "not fit to send as is" because of bot defects T1 to T10. T1
  (the goal probe checked the middle of the element) is fixed. T2 (`settle()` does not wait for a
  client-side route to render) is the likely cause of the `onboarding-gitlab` 1280 flake. T3 is the
  mobile drawer. T2 to T10 go into R-02's change set ahead of site work. `ux/baseline.json` is to
  be written from a gate run with the fixed probe, not from R-01's round.json. The final gate run
  did not finish, so no baseline exists yet.
- `layout_shift.score` 0.112 for `onboarding-gitlab` at 1024x768 in one run. Not repeated
  anywhere else. Watch it in R-02.

## Did the last change sets move their metrics?

| round | change | target metric | moved |
|---|---|---|---|
| none | R-01 is the first round | | |

## Findings

| id | severity | owner | frequency | confidence | cost | priority |
|---|---|---|---|---|---|---|
| UXF-002 link-in-text-block | S2 (3) | docs-system | 1 | 1 | S (1) | 3.00 |
| UXF-001 no search | S2 (3) | docs-system | 1 | 1 | L (4) | 0.75 |
| UXF-004 table scroll box unreachable by keyboard | S2 (3) | site | 7/49 (exit-code-walk goal page) | 1 | S (1) | 0.43 |
| UXF-007 table of contents after the article for keyboard | S2 (3) | docs-system | 7/49 (exit-code-walk; the cost shows in every long page) | 1 | M (2) | 0.21 |
| UXF-003 sidebar slides up at page end | S3 (1) | docs-system | 1 | 1 | M (2) | 0.50 |
| UXF-005 status page long | S3 (1) | generator | 7/49 | 1 | M (2) | 0.07 |
| UXF-006 CLI reference long | S3 (1) | generator | 7/49 | 1 | M (2) | 0.07 |
| UXF-008 terminal title clipped | S3 (1) | docs-system | 14/49 (landing page at 1024 and 390) | 1 | S (1) | 0.29 |
| UXF-009 doc header source runs past | S3 (1) | docs-system | 14/49 (gitlab, slack at 390) | 1 | S (1) | 0.29 |
| UXF-010 inline code runs past | S3 (1) | docs-system | 7/49 (contracts/cli at 1024) | 1 | S (1) | 0.14 |
| UXF-016 one section, three names | S3 (1) | site (config, sidebars) | 1 (navbar on every page) | 1 | S (1) | 1.00 |
| UXF-011 primary button on the fold at 1366 | S3 (1) | docs-system (landing) | 49/49 start on `/` | 1 | S (1) | 1.00 |
| UXF-012 key data below 14 px | S3 (1) | docs-system | 1 | 1 | S (1) | 1.00 |
| UXF-013 exit statuses mid-paragraph | S3 (1) | generator, prose | 14/49 | 2 (reading_load + review) | M (2) | 0.29 |
| UXF-014 CLI reference names no exit status | S3 (1) | generator | 7/49 | 1 | S (1) | 0.14 |
| UXF-015 empty Meaning column | S3 (1) | generator | 7/49 | 1 | S (1) | 0.14 |
| UXF-017 contract status in free text | S3 (1) | generator | 7/49 | 1 | M (2) | 0.07 |

UXF-011 to 017 come from the heuristic review (`review-heuristics.md`: 5 pass, 9 partial, 2
fail). It arrived after the change set had been built and gated. UXF-016 is the only one owned by
the site; it heads R-02's change set (priority 1.00). Renaming navigation labels is also a wording
decision, so R-02 asks it in decision.md first.

No S1 finding. The 9 docs-system findings (UXF-001, 002, 003, 007, 008, 009, 010, 011, 012) go
to Docs System and are not built here. The generator findings (UXF-005, 006, 013, 014, 015, 017)
go to `connectors-docs`.
The site's Markdown prose belongs to another agent.

## Change set (1 of at most 5)

1. **UXF-004**: `src/css/tables.css` no longer makes a table inside `.b10x-table-wrap` the scroller.
   The wrapper scrolls, takes `tabindex=0` and names the region. Target metric: gate 5
   `scrollable-region-focusable` nodes on `/docs/reference/contracts/cli`, 2 → 0 at both gating
   viewports. Regression asset:
   `tests/ux/a11y.spec.ts :: UXF-004: wide tables scroll in a box the keyboard reaches`. Red on
   the R-01 build (exit 1, 1 and 2 nodes); green on the fixed build at both gating viewports (5.4 s and 15.1 s).

## Gate after the change set: budget defect

The final gate run on the fixed build (load 93 to 102 on 20 CPUs; global timeout lifted on the
command line for this one measuring run only, the committed config keeps 120 s) did not finish.
The slot pool killed it at 10 min 01 s, after 44 tests passed and 4 failed. The 4 failures are the
1366x768 layout chunks, each killed at the 60 s test cap. Screen-only reached 1 of 21 tests.
`ux/baseline.json` was not written. Ten slowest: layout 1366 contracts/cli chunk 66.0 s ✘; the
other three 1366 layout chunks 60.0 s ✘ each; the four 1280 layout chunks 55.2, 51.2, 44.2 and
42.2 s; keyboard onboarding-gitlab 37.5 s; break gate 5 37.3 s. That is a budget defect of the
step: it ran over 120 s, and the layout chunks hit the 60 s cap. It gets a story ahead of R-02.

Design decision open: decision.md (site-local search ahead of Docs System; default no).

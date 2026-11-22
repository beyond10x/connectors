# R-01 report

Written by `npm run ux:round` (`UX_ROUND=R-01`) from the built site; deterministic, no model, no network. Numbers: `round.json`.

## Gates (violations; bar = 0)

| viewport | theme | gated | pages | 1 no overflow | 2 always visible | 3 completion | 4 screen-only | 5 axe serious+critical nodes | axe moderate/minor |
|---|---|---|---|---|---|---|---|---|---|
| 1366x768 | light | yes | 171 | 0 | 0 (+168 held) | 1 | 1 | 77 (link-in-text-block, scrollable-region-focusable) | 0 |
| 1366x768 | dark | yes | 171 | 0 | 0 (+170 held) | 1 | 0 | 78 (link-in-text-block, scrollable-region-focusable) | 0 |
| 1280x720 | light | yes | 15 | 0 | 0 (+14 held) | 1 | 0 | 81 (link-in-text-block, scrollable-region-focusable) | 0 |
| 1280x720 | dark | yes | 15 | 0 | 0 (+14 held) | – | – | – | – |
| 1440x900 | light | measured | 15 | 0 | 0 (+14 held) | 0 | 0 | 33 (link-in-text-block) | 0 |
| 1440x900 | dark | measured | 15 | 0 | 0 (+14 held) | – | – | – | – |
| 1024x768 | light | measured | 15 | 2 | 0 (+14 held) | 0 | 0 | 33 (link-in-text-block) | 0 |
| 1024x768 | dark | measured | 15 | 2 | 0 (+14 held) | – | – | – | – |
| 768x1024 | light | measured | 15 | 0 | 0 | 0 | 5 | 34 (link-in-text-block) | 0 |
| 768x1024 | dark | measured | 15 | 0 | 0 | – | – | – | – |
| 390x844 | light | measured | 15 | 3 | 0 | 0 | 5 | 33 (link-in-text-block, scrollable-region-focusable) | 0 |
| 390x844 | dark | measured | 15 | 3 | 0 | – | – | – | – |

## Tasks (worst over the gating viewports and both themes; lower is better)

| task | card | screen_only.actions | screen_only.looks | screen_only.steers | screen_only.time_to_act_steps.max | glance_cost.median | glance_cost.p95 | reading_load.max | reading_load.median | layout_shift.score | keyboard.presses |
|---|---|---|---|---|---|---|---|---|---|---|---|
| install-command | anchor | 4 | 2 | 0 | 1 | 2 | 3 | 257 | 195 | 0 | 7 |
| onboarding-gitlab | anchor | 27 | 23 | 2 | 8 | 2 | 3 | 426 | 208 | 0 | 61 |
| exit-code-walk | rotating | 6 | 3 | 0 | 0 | 2 | 2 | 505 | 290 | 0.005 | – |
| onboarding-slack | extra | 2 | 0 | 0 | 0 | 2 | 2 | 221 | 195 | 0 | 30 |
| exit-status-older-binary | extra | 30 | 23 | 3 | 5 | 2 | 3 | 344 | 225 | 0.028 | 56 |
| cli-invoke | extra | 13 | 10 | 0 | 3 | 2 | 3 | 248 | 221 | 0.028 | 72 |
| status-pod-exec | extra | 16 | 15 | 0 | 0 | 2 | 2 | 403 | 297 | 0 | 23 |

## Screen-only user per viewport (actions; ✗ = target not reached)

| task | 1366x768 | 1280x720 | 1440x900 | 1024x768 | 768x1024 | 390x844 |
|---|---|---|---|---|---|---|
| install-command | 3 / 3 | 4 / – | 4 / – | 3 / – | ✗30 / – | ✗30 / – |
| onboarding-gitlab | 2 / 3 | 27 / – | 2 / – | 2 / – | 4 / – | 4 / – |
| exit-code-walk | 5 / 5 | 6 / – | 5 / – | 7 / – | ✗30 / – | ✗30 / – |
| onboarding-slack | 2 / 2 | 2 / – | 2 / – | 2 / – | 4 / – | 5 / – |
| exit-status-older-binary | ✗30 / 6 | 5 / – | 3 / – | 5 / – | ✗30 / – | ✗30 / – |
| cli-invoke | 11 / 11 | 13 / – | 11 / – | 12 / – | ✗30 / – | ✗30 / – |
| status-pod-exec | 14 / 14 | 16 / – | 12 / – | 22 / – | ✗30 / – | ✗30 / – |

Cells are light / dark; – = not run. Gate 5 counts include the known loss link-in-text-block (UXF-002); held = gate 2 sidebar at page end (UXF-003). Search is absent on every page (UXF-001, held).

## Worst moments

- gate 2, 392×: `sidebar` outside the viewport [0,-310,300,337]; covered at centre — first at `/docs` 1280x720 light (end): [shots/worst-01-gate2-sidebar-1280x720-light.png](shots/worst-01-gate2-sidebar-1280x720-light.png)
- gate 2, 29×: `sidebar` outside the viewport [0,-273,300,421] — first at `/docs/reference/contracts/model/domains/connectors-tool_contracts` 1366x768 light (end): [shots/worst-02-gate2-sidebar-1366x768-light.png](shots/worst-02-gate2-sidebar-1366x768-light.png)
- gate 1, 4×: `clipped div.b10x-hero-art.b10x-hero-art--terminal > figure.b10x-terminal.b10x-terminal--tilt > div.b10x-terminal__bar > span.b10x-terminal__title` scrollWidth 259 > clientWidth 209, overflow hidden, ellipsis without its full value — first at `/` 1024x768 light (top): [shots/worst-03-gate1-clipped-1024x768-light.png](shots/worst-03-gate1-clipped-1024x768-light.png)
- gate 1, 4×: `runs-past article > div.theme-doc-markdown.markdown > header.b10x-doc-header > p` span.b10x-doc-header__source past the box (scrollWidth 313 > clientWidth 302) — first at `/docs/connectors/gitlab` 390x844 light (top): [shots/worst-04-gate1-runs-past-390x844-light.png](shots/worst-04-gate1-runs-past-390x844-light.png)
- gate 1, 2×: `runs-past div > article > div.theme-doc-markdown.markdown > p` code past the box (scrollWidth 515 > clientWidth 511) — first at `/docs/reference/contracts/cli` 1024x768 light (top): [shots/worst-05-gate1-runs-past-1024x768-light.png](shots/worst-05-gate1-runs-past-1024x768-light.png)
- gate 2, 1×: `sidebar` outside the viewport [0,-461,300,234]; covered at bottom-left, bottom-right — first at `/docs/reference/contracts/model/domains/connectors-oauth2` 1366x768 light (end): [shots/worst-06-gate2-sidebar-1366x768-light.png](shots/worst-06-gate2-sidebar-1366x768-light.png)

Contact sheets: [light](shots/contact-sheet-light.png), [dark](shots/contact-sheet-dark.png).

## Not run this round

- L4 replayed real sessions and L5 discoverability probe: no paid API and no key in this round.
- L6 recorded live rounds and the questionnaire: no person took part; no questionnaire scores.
- L0 engine scenarios and gate 6: the site has no guidance cue.


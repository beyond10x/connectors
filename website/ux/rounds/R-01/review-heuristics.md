# R-01 heuristic review

5 pass, 9 partial, 2 fail

Inputs: `report.md`, `shots/` (both contact sheets, worst moments), the 42 frames in
`.cache/ux/frames/R-01/` (21 per theme, 1366x768), the screen-only traces
(`.cache/ux/results/measure/screen-only/`, 49 runs), keyboard, layout and axe results, the task
cards in `ux/adapter.ts`, `references/heuristics.md`. No recordings exist in this round, so no
evidence cites a recording second; traces are cited as `<task>@<viewport>@<theme>`. Frames are cited
as `frames/<file>` (under `.cache/ux/frames/R-01/`), traces as `so/<file>` (under
`.cache/ux/results/measure/screen-only/`). New shots: `shots/review-*.png`, cropped from the frames.

| # | Heuristic | Verdict | Evidence |
|---|---|---|---|
| 1 | One next thing | partial | Landing at 1366x768: the hero's "Get started" row starts at y ≈ 745, labels unreadable (`shots/review-landing-cta-fold-1366x768-light.png`); every `install-command` run takes the navbar link instead (glance 2, `so/install-command@*`). Docs pages end on one "Next" card (`shots/worst-01-gate2-sidebar-1280x720-light.png`); exercises name "Next guided step" (`frames/light--page_docs_examples_contract-exercises.png`). → UXF-011 |
| 2 | State in words | pass | Status rows say "● shipped" in words (`frames/light--task_status-pod-exec.png`); follow-a-request says "Ready to start", "Available today" (`frames/light--page_docs_examples_follow-a-request.png`); exercises "0 / 8" (`frames/light--page_docs_examples_contract-exercises.png`). |
| 3 | Nothing required hidden | fail | Search absent on 171 of 171 pages (UXF-001, `shots/decision-no.png`); sidebar slides under the navbar at page end (UXF-003, `shots/worst-01…`, `worst-06…`); wide tables scroll unfocusably (UXF-004, axe on `/docs/reference/cli`, `/docs/reference/contracts/cli`, `/docs/connectors/gitlab`@390); the second `install -m 0755` line of the install block is cut at the right edge (`shots/review-install-block-cut-1366x768-light.png`); terminal title clipped (UXF-008). |
| 4 | Stable layout | partial | `layout_shift.score` 0–0.028 at the gating viewports (`so/cli-invoke@1366x768@*`, `so/exit-status-older-binary@1366x768@*` 0.028); 0.112 on `so/onboarding-gitlab@1024x768@light`, 0.075 on `/docs` at 768x1024. The traces do not record the shifting element; no frame shows a jump. Not filed: cause not observed. |
| 5 | Provenance of every value | pass | Doc headers list their sources (`frames/light--page_docs_connectors.png`, `shots/worst-04…`); contract callout "Rendered from contracts/cli/v1alpha1/semantics.md by connectors-docs" (`frames/light--page_docs_reference_contracts_cli.png`); Status "As of 2026-10-16 … connectors-docs generates this page" (`frames/light--page_docs_status.png`); footer "THIS BUILD docs-system a3fae6c" (`shots/worst-01…`). |
| 6 | Errors cheap | pass | Every steer in the traces is one `back` (`so/exit-status-older-binary@1366x768@light`, `so/onboarding-gitlab@1280x720@light`); the sidebar with the active item is on every docs frame. |
| 7 | Words the user can say | partial | Navbar "Contracts" opens pages the sidebar and breadcrumb call "Specification" (`frames/light--page_docs_reference_contracts_cli.png`) → UXF-016; global options have no meaning and placeholders `<global_state>` (`shots/review-cli-global-options-meaning-empty-1366x768-light.png`) → UXF-015. Sidebar labels elsewhere are plain ("Upgrade the CLI", "Status"). |
| 8 | The reader's time respected | partial | `reading_load.max` 505 on exit-code-walk, answer mid-paragraph (`shots/review-walk-exit-in-prose-1366x768-light.png`) → UXF-013; status row 12–22 actions (UXF-005, `so/status-pod-exec@*`); CLI reference 11–13 actions (UXF-006, `so/cli-invoke@*`). The two onboarding tasks take 2 actions (`so/onboarding-slack@*`). |
| 9 | Keyboard and pointer parity | partial | Six keyboard runs reach their target with focus visible, at 7–72 presses against 2–14 screen-only actions (`.cache/ux/results/measure/keyboard/*.json`); exit-code-walk has no keyboard result (timed out, UXF-007); table scroll boxes take no focus (UXF-004). |
| 10 | Recognition over recall | partial | Every connector is named in the sidebar, so the onboarding tasks need no recall (`frames/light--task_onboarding-slack.png`). Exit statuses need the reader to know a second CLI page exists under Specification: the CLI reference names none and links none (`frames/light--page_docs_reference_cli.png`) → UXF-014. |
| 11 | Calm by default | pass | No banner, toast or alert in any of the 42 frames, either theme; admonitions only carry content (`frames/light--page_docs_reference_contracts_cli.png`); layout shift ≤ 0.028 at the gating viewports. Motion over time not observed (no recordings). |
| 12 | One state vocabulary | fail | Status page: shipped, unreleased, decided, planned (`shots/review-status-vocabulary-1366x768-light.png`); contract pages: "Specified; partial production implementation", "full contract acceptance pending" and about ten other phrasings (`shots/review-contract-status-wording-1366x768-light.png`); examples: "Available today" / "Specified architecture". → UXF-017 |
| 13 | A wrong cue recovered in one action (navigation, search) | partial | No search to jump back on track (UXF-001). `so/exit-status-older-binary@1366x768@light`: 3 steers, 30 actions, target not reached; `so/onboarding-gitlab@1280x720@light`: 2 steers, 27 actions. A reader at the CLI reference has no link to the page with exit statuses (UXF-014). Single wrong pages are undone in one `back`. |
| 14 | Unmistakable end state | pass | Every task frame shows the H1, the breadcrumb, the active sidebar item and the navbar underline of the target (`frames/light--task_*.png`, `frames/dark--task_*.png`). The bot's `false_complete` in `so/onboarding-gitlab@1280x720@light` ends on `/`, which the frames do not support as a UI cause. |
| 15 | Readable in every theme; key data at 14 px | partial | No colour-contrast hit in 58 axe runs, both themes (`.cache/ux/results/measure/axe/`); both contact sheets read alike. Key data below 14 px: code blocks 13.1 px, table code 12.7 px, table of contents 12.8 px with code near 11 px (`shots/review-toc-small-code-1366x768-light.png`) → UXF-012; links told by colour alone (UXF-002). |
| 16 | Focus follows the work (navigation, search) | partial | The table of contents marks the current section (`frames/light--task_exit-code-walk.png`, "Walking pages"); the sidebar scrolls to the active item (`frames/light--task_cli-invoke.png`). The sidebar leaves the viewport at page end (UXF-003); the table of contents comes after the article in Tab order (UXF-007). |

Not judged: below 997 px, 5 of 7 tasks end ✗30. Each trace reopens the drawer on the target page and
scrolls with a constant word count (`so/install-command@390x844@light`: 27 scrolls at 180 words;
`so/cli-invoke@768x1024@light`: 27 at 234). No frame of that state exists, so whether the page or
the bot holds the drawer open is not observed.

## Findings filed

| id | title | severity | class | sessions | cost |
|---|---|---|---|---|---|
| UXF-011 | The landing page's primary button sits on the fold at 1366x768 | S3 | layout | all 7 tasks (start `/`), gating viewports, both themes | S |
| UXF-012 | Key data is set below 14 px: code blocks, code in tables, the table of contents | S3 | layout | install-command, cli-invoke, exit-code-walk; all viewports | S |
| UXF-013 | Exit statuses are stated mid-sentence in long paragraphs | S3 | copy | exit-code-walk, exit-status-older-binary | M |
| UXF-014 | The CLI reference names no exit status and does not link to the Local CLI contract | S3 | affordance | exit-code-walk, cli-invoke | S |
| UXF-015 | The CLI reference's global options have an empty Meaning column | S3 | copy | cli-invoke | S |
| UXF-016 | One section, three names: Contracts, Specification, Contract reference | S3 | copy | exit-code-walk | S |
| UXF-017 | Contract pages state their status in free text, not in the Status page's words | S3 | copy | status-pod-exec, exit-code-walk | M |

Cited, not refiled: UXF-001, 002, 003, 004, 005, 006, 007, 008.

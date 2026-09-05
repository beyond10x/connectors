---
format: aep.planning-md/1
id: story:handbook-clarity-and-usability
kind: story
status: implemented
title: The handbook follows one flow and remains readable on small screens
relations:
- derived_from: epic:public-surface
- informed_by: story:visual-architecture-handbook
scope:
- confidence: cited
  path: README.md
- confidence: cited
  path: docs/architecture
- confidence: cited
  path: docs/guides/connect-slack.md
revision: 9
---
## Outcome

Implement the accepted second handbook pass: explain one hosted Slack companion mention-to-reply flow across the existing chapters, distinguish current implementation from ESS declarations, and improve shared diagram/table rendering and search titles. The operator approved content plus shared Website fixes and publication.

## Grounding

The first handbook is story:visual-architecture-handbook. On a 390-pixel viewport its system SVG shrinks from a 958-pixel viewBox to 358 pixels; the three-column ownership table clips its last column under overflow:hidden. Docs System already has a keyboard-scrollable Diagram component, but it imports the Website Mermaid theme, so direct reuse from that theme would recurse.

## Implementation

- Connectors: revise README.md and docs/architecture/*.md; retain routes and historical design citations; use one clearly illustrative hosted companion Connection/event/reference throughout.
- Docs System: extract an additive DiagramFrame component, keep Diagram compatible, preserve intrinsic diagram dimensions and accessible native content, and test existing consumers. Add a shared semantic scrollable table if Website has no reusable owner.
- Website: wrap the original Mermaid renderer with DiagramFrame without nesting existing Diagram frames; retain source markers; expose wide Markdown tables to keyboard/touch scrolling; deduplicate identical page/project search titles.
- Atlas: govern the immutable Docs System/Website runtime promotion, generated callers and deterministic snapshots; preserve primary checkout work.

## Acceptance

Repository gates, existing consumer rendering, 320/390-pixel mobile and desktop at light/dark and 200% zoom, keyboard panning, all table columns, chapter ordering, search/source links, and exact live revision verification pass. Public Connector APIs, runtime behavior, ESS formats, and Entity Runtime adoption are unchanged.

## Evidence

The complete Connectors gate passes on 0.6.4: catalog (65 providers, 68 artifacts), portable Markdown links, story consistency, ESS validation, and exact committed clap-outline comparison. Hosted approval claims were checked against hosted/approval.rs and hosted/enforcement.rs; event E1 is not hosted approval A1. The published and deployed handbook source is f109b2226d5ac0ec8fc6f600772bab2a17d1b0d4. Later concurrent GitLab startup-recovery work is preserved; its public handbook bytes are identical.

Docs System's full gate passes at 1c8c31697e87235dda8bec9467264b22a7fa0c95. Website runtime 815fad1b977992d01695f6b5c79495c02576212b passes its production and browser gates. All seven handbook pages pass 56 combinations of desktop, 320/390px, 200% reflow, and both themes. Actual touch panning, keyboard scrolling, final table columns, the existing DependencyGraph, and deduplicated search titles pass. The browser regression caught and verified the fix for an initially zero-width SVG.

Website 61b246c933f37f5ecc872eeb14cbe66a515e8452 publishes the refreshed remote source lock and Atlas-owned snapshot. Its full local production gate and GitHub gate 33962723274 pass: 347 routes, 1,299 files, 22,756 references, and 348 indexed pages. Atlas's managed-root reconciliation and built-portal checks pass under the published controls 9f3b42f6d990d849be918936039d7dd5567653c8 and ADR 0035.

Publication run https://github.com/beyond10x/atlas/actions/runs/33962528580 and root Pages run https://github.com/beyond10x/beyond10x.github.io/actions/runs/33962766792 succeeded. Live root provenance at https://beyond10x.github.io/.well-known/b10x-docs.json binds Website runtime 815fad1b977992d01695f6b5c79495c02576212b, Connectors f109b2226d5ac0ec8fc6f600772bab2a17d1b0d4, and source-set digest a2f960242eca58f22aa1905ba780ff8d8870b87365e3c7c8b5400dfd1943603f. The handbook is live at https://beyond10x.github.io/docs/connectors/architecture/.

The root-only and complete live atlas docs verify-pages fences pass. All 24 project facades use the intended runtime. Five facades with legacy alias files required the existing migration_website_sha input to preserve their bytes; the corrective runs 33963190650, 33963191874, 33963193241, 33963194646, and 33963196454 succeeded before the final live fence passed. No publication format or application behavior changed.

The required unscoped local Atlas fences were also run. Stale primary Docs System/Website checkouts and the unrelated Widgets AGENTS.md without a Serves section remain separate local workspace issues; they do not change the passing managed-source, artifact, and live-delivery evidence above. Broad local caller gates caused unnecessary disk pressure. Reviewed disposable task build outputs were cleared, and completed published worktrees are retired through the worktree CLI with exact reviewed ids.

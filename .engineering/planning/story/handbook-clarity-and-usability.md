---
format: aep.planning-md/1
id: story:handbook-clarity-and-usability
kind: story
status: active
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
revision: 7
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

The complete Connectors gate passes on 0.6.4: catalog (65 providers, 68 artifacts), portable Markdown links, story consistency, ESS validation, and exact committed clap-outline comparison. Hosted approval claims were checked against hosted/approval.rs and hosted/enforcement.rs; event E1 is not hosted approval A1.

Docs System's full gate passes at 1c8c31697e87235dda8bec9467264b22a7fa0c95. Website's production gate and GitHub gate 33959605114 pass at runtime 815fad1b977992d01695f6b5c79495c02576212b. Browser checks pass for all seven handbook pages in 56 combinations: desktop, 320/390px, 200% reflow, and both themes. Actual touch panning, keyboard scrolling, final table columns, the existing DependencyGraph, and deduplicated search titles pass. The browser regression caught and verified the fix for an initially zero-width SVG.

Atlas's managed-root reconciliation and built-portal checks pass. Runtime/caller promotion and exact live verification are in progress under Atlas ADR 0035.

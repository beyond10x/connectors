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
revision: 6
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

Connectors gate: green (catalog: 65 providers, 68 artifacts; portable Markdown links; story index; ESS system valid; committed clap tree matches specification generation). The hosted Slack example was checked against hosted/approval.rs and hosted/enforcement.rs: hosted writes need a Grant and an issued one-time ApprovalRecord. An event reference is not hosted approval; the local runtime claim journal is a separate mechanism. The Slack guide now states that boundary.

Docs System gate passes for the additive frame/table exports and native accessibility tests. Website integration, browser review, and publication remain in progress.

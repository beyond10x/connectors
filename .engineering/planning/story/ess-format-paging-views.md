---
format: aep.planning-md/3
id: story:ess-format-paging-views
kind: story
status: draft
title: Move the spec to an ESS format with view paging and model list cursors
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Source

ESS design review of the CLI domain (2026-09-30): offset paging on a view (`paging:`) is refused at the spec's
`ess/15` (ESS-VIEW-009) and validates and synthesizes at `ess/16`. Cursor staleness by registry epoch needs
`when_related:` (ess/18), which the Entity Runtime lowering still refuses (`RelatedGuardUnsupported`).

## Acceptance

- Decide the target format; move the spec, regenerate, and keep conformance green.
- The adapter, operation and connection lists are modelled with paging so conformance checks page boundaries.

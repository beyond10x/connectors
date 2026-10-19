---
format: aep.planning-md/3
id: story:ess-format-paging-views
kind: story
status: draft
title: Move the spec to an ESS format with view paging and model list cursors
relations:
- serves: vision:independent-contract-adapters
revision: 2
---
## Source

ESS design review of the CLI domain (2026-09-30): offset paging on a view (`paging:`) is refused at the spec's
`ess/15` (ESS-VIEW-009) and validates and synthesizes at `ess/16`. Cursor staleness by registry epoch needs
`when_related:` (ess/18), which the Entity Runtime lowering still refuses (`RelatedGuardUnsupported`).

## Acceptance

- Decide the target format; move the spec, regenerate, and keep conformance green.
- The adapter, operation and connection lists are modelled with paging so conformance checks page boundaries.

## Half done

The format half is done: `ess/system.yaml` line 1 declares `format: ess/18`, which accepts
view `paging:` (`ess/16`) and `when_related:` (`ess/18`). What remains is the second acceptance
line: no `paging:` appears under `ess/` yet, so the adapter, operation and connection lists are
not modelled with paging and conformance does not check page boundaries. The cursor staleness
guard still waits on the Entity Runtime lowering, which refuses `when_related:`
(`ess/domains/cli.yaml` lines 1500-1508, whose "this specification is ess/15" is stale). The
story narrows to modelling the list paging.

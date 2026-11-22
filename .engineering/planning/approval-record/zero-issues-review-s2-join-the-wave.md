---
format: aep.planning-md/3
id: approval-record:zero-issues-review-s2-join-the-wave
kind: approval-record
status: draft
title: Four S2 code review findings join the single defect wave with no operator present
tags:
- non-interactive
relations:
- decides: story:mcp-tool-deadline-keeps-the-server
- decides: story:sql-wrapper-resolves-pg-catalog-first
- decides: story:reconnect-retained-bound-matches-refresh
- decides: story:read-use-bound-counts-live-reads
revision: 1
---
## Which stop

Adding four stories to the running single defect wave of `epic:zero-open-issues-and-store-defects`
(`approval-record:zero-issues-single-defect-wave`) after its stage-1 proposal was approved.

## Why it is bypassed: non-interactive

The coordinator is a sub-agent dispatch with no operator turn. The operator's order, relayed by the
caller on 2026-11-10, verbatim: "connectors -> ok good - lets at least fix all bugs/defects and
dispatch them as another single wave". The caller, 2026-11-10: add the four S2 findings of the
operator-ordered code review at `23b9f1bc4` to the current single wave.

## What was decided

- The four S2 findings, filed as stories, join the wave: `story:mcp-tool-deadline-keeps-the-server`
  (CR-001), `story:sql-wrapper-resolves-pg-catalog-first` (CR-002),
  `story:reconnect-retained-bound-matches-refresh` (CR-003), `story:read-use-bound-counts-live-reads`
  (CR-004).
- Each goes red first; one that cannot be made red is recorded with that evidence and not changed.
- CR-001 and CR-002 get an adversary and a security review.
- The 21 new S3 findings stay drafts in the backlog, outside this wave.

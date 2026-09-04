---
format: aep.planning-md/1
id: epic:catalog-day-one
kind: epic
status: active
title: Catalog day one
revision: 3
---
## Provenance

Read from the `epic: catalog-day-one` key in 4 file(s) under `docs/stories/`. No source document
exists for this epic itself; the grouping is a frontmatter value and nothing else. Migrated
2026-09-04 by the `aep-planning:story-migration` skill. **read**

## Stories

- `S-001` `story:the-document-carries-the-callers-contract` — done — The document carries the caller's contract, so nothing at runtime parses source
- `S-002` `story:effects-are-read-never-derived` — blocked — Per-operation effects are read from the document, never derived
- `S-003` `story:the-lockfile-gets-a-verifier` — done — `catalog check` verifies every addressable hash and refuses unverifiable claims
- `S-015` `story:retire-the-quirks-umbrella` — done — Retire the `quirks` umbrella — pagination, rate limits and error envelopes are ordinary facts

## Status

`active`, derived from its stories (blocked: 1, done: 3). **inferred** — no source document states a status
for this epic, so the rung follows the work underneath it: all done is `implemented`, any work
started is `active`, nothing started is `draft`.

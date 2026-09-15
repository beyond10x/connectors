---
format: aep.planning-md/1
id: epic:catalog-day-one
kind: epic
status: archived
title: Catalog day one
revision: 5
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

## Disposition

Archived 2026-09-15 on the acceptance of `docs/design/21-clean-room-rewrite.md` (org-state review
decision sheet 2026-09-15, items 1 and 15; atlas ADR 0051, 2026-09-15).

**Absorbed by the successor lineage.** Design 21 §2 keeps the catalog pipeline as K1 — "vendor/authored
spec + patch rules → canonical document → pack → lock, byte-deterministic, provenance-pinned" — and the
successor already implements it: `connectors_v2/adapters/catalog` compiles a bundle from a pinned OpenAPI
document and serves GitLab through it (`connectors_v2/README.md` § *Where the project stands*). Design 21
§4 rules this store out of the rewrite besides: "Planning/story sediment … Not migrated; fresh store in
the new repo seeded from the extraction epic only."

Its 4 `docs/stories/S-*` records keep their own statuses (3 `done`, 1 `blocked`). This repository's `AGENTS.md` states no
convention for stories under an archived epic, so none was moved. Archiving keeps the record.

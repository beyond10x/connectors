---
format: aep.planning-md/1
id: epic:sources
kind: epic
status: active
title: Sources
revision: 4
---
## Provenance

Read from the `epic: sources` key in 2 file(s) under `docs/stories/`. No source document
exists for this epic itself; the grouping is a frontmatter value and nothing else. Migrated
2026-09-04 by the `aep-planning:story-migration` skill. **read**

## Stories

- `S-016` `story:sources-are-processed-by-code` — ready — Sources are processed by code: the index is validated, checksummed and refreshed by the tool
- `S-017` `story:mint-source-entries-from-the-mined-catalogs` — backlog — Mint source entries from the mined competitor catalogs

## Status

`active`, derived from its stories (backlog: 1, ready: 1). **inferred** — no source document states a status
for this epic, so the rung follows the work underneath it: all done is `implemented`, any work
started is `active`, nothing started is `draft`.

## Disposition

**Left `active` on 2026-09-15 — needs the operator.** `docs/design/21-clean-room-rewrite.md` was accepted
that day (decision sheet items 1 and 15) and six sibling epics archived under it; this one does not
classify cleanly and was not moved.

Design 21 §2 pulls in two directions here. The *data* survives untouched — "Data that survives verbatim,
no rewrite: `specs/` (19 vendors + provenance), `providers/*.toml`, `SOURCES.toml`" — which argues the
source-index tooling of S-016 (validated, checksummed and refreshed by the tool) is carried over rather
than re-derived. But the successor's catalog takes a different road entirely, compiling a bundle from one
pinned OpenAPI document (`connectors_v2/README.md`), so it is not clear the index tooling has a consumer.
S-017 (mint source entries from the mined competitor catalogs) is growth of a 19-vendor corpus whose
successor currently ships one provider bundle.

What settles it: whether the successor ingests `SOURCES.toml` and `specs/` as design 21 §2 assumes, or
compiles only from pinned vendor documents.

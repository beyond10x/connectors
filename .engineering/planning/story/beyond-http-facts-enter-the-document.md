---
format: aep.planning-md/1
id: story:beyond-http-facts-enter-the-document
kind: story
status: implemented
title: Beyond-HTTP facts enter the connector document orthogonally
refs:
- provider: legacy
  reference: S-023
relations:
- derived_from: epic:beyond-http
scope:
- confidence: cited
  path: crates/catalog
- confidence: cited
  path: crates/catalog-build
- confidence: cited
  path: crates/connector-resolve
- confidence: cited
  path: crates/connector-spec
revision: 6
---
## Acceptance

Verbatim from `docs/stories/S-023-beyond-http-facts-enter-the-document.md:20`. **read**

- [x] The schema and IR type each axis independently and reject unknown driver/capability values.
- [x] Ordinary HTTP documents lower byte-identically unless a reviewed migration says otherwise.
- [x] Exposure to models remains distinct from operation direction and direct-byte interaction.
- [x] Catalog build, reader, resolver, lock, site projection, and fixtures change in the single
      coordinated schema wave.
- [x] The substrate first-party translation has an explicit mapping/overlay fixture; no field is
      called mechanically identical when its vocabulary differs.

## Context

Represent interaction shape, closed protocol driver, placement requirements, implementation form,
and capability predicates as independent document facts without restoring a generic runtime enum.

Source frontmatter: pillar Catalog · areas [catalog, catalog-build, connector-spec, connector-resolve] · design `docs/design/03-beyond-http.md`. **read**

Source `note:` field, quoted: “ADR 0010 delivery item 1; this is schema-wave work and never runs in parallel with another document-schema author”

## Status

`done` in the source. Quoted from `docs/stories/S-023-beyond-http-facts-enter-the-document.md:5`: `status: done`. **read**

This artifact reached `implemented` with `aep artifact move --evidence test_result=1`. The journal
records that move as resting on an **assertion**, not on a run this migration observed. The flag is
what the CLI provides for evidence that lives outside the store.

What was asserted, and where it came from:

- The source records `status: done` at the line quoted above. **read**
- `bash scripts/gate.sh` was green at commit `a48030b` on 2026-09-04 — exit 0, 136 `test result: ok`
  lines across 11 workspaces. **read**, from `~/.cache/connectors-gate/gate2.log`

No per-story run was attributed to this story. The gate is a repository-wide fact, and reading it as
proof of one story's acceptance would be an inference this record does not make.

## Provenance

Migrated from `docs/stories/S-023-beyond-http-facts-enter-the-document.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-08-13 · 2 revision(s)
- Legacy id `S-023`, recorded as the reference `legacy:S-023`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill

---
format: aep.planning-md/1
id: story:orphaned-inputs-are-removed-or-re-owned
kind: story
status: implemented
title: Orphaned inputs are removed or re-owned
refs:
- provider: legacy
  reference: S-022
relations:
- derived_from: epic:post-m1
scope:
- confidence: cited
  path: specs
- confidence: cited
  path: web
revision: 6
---
## Acceptance

Verbatim from `docs/stories/S-022-orphaned-inputs-are-removed-or-re-owned.md:37`. **read**

- [x] **anthropic** remains under `specs/` and covered by the OpenAPI 3.1/YAML ingest tests. Its
      provenance says `origin = "repository-authored"`, pins its bytes, records the predecessor
      authorship and exact official grounding references, and states `coverage =
      "partial-ingest-fixture"`; it cannot be mistaken for vendor-published bytes.
- [x] **anthropic's full connector** is reproducible from a sufficiently complete authored spec plus
      reviewed overlays into the canonical connector document. Until then,
      `providers/anthropic.toml` is named migration debt and the partial excerpt is not allowed to
      claim provider-level coverage.
- [x] **`specs/flux/core-v1.json`** is deleted together with every claim that something is emitted
      from it (`web/README.md`'s `public/v1/` paragraph), or a named consumer is filed as its own
      story before it stays. A vendored input with no consumer is a file that still validates and
      describes nothing.
- [x] **`migration/`** is decided against its live successor: the wave/fixture rule restated by
      [S-010](../../../docs/stories/S-010-m5-flux-re-points-and-the-gitlab-plugin-retires.md) is what actually governs
      plugin retirement now. Either the ratchet's records move under that story's evidence with an
      owner and a checker, or the directory goes. It does not stay as an unchecked schema plus two
      fixtures.
- [x] `SOURCES.toml`'s "known defects" header shrinks by exactly the entries closed, and the
      predecessor-import entry's `consumers` list stops naming paths that no longer exist. The index
      never describes a tree that is not there.
- [x] After the sweep, orphan checking passes **in both directions** by construction: every file under
      `specs/` is covered by a `[[source]]` entry, and every entry's files exist. Landing this before
      S-016 means that check's first run is a green one; landing it after means S-016 ships with three
      declared exceptions, which is the state this story exists to prevent.
- [x] Each disposition is its own reviewable commit with the reason in the body — a deleted 157 KB
      vendored document, a re-owned authored spec and a deleted migration ratchet are decisions,
      not cleanup noise.

## Context

Close the three orphans the M1 import left behind, so every file this repository derives from has a
consumer, a refresh path and a provenance record — and so
[S-016](../../../docs/stories/S-016-sources-are-processed-by-code.md)'s `catalog sources check` can arrive **green** rather
than arriving with three permanent exceptions.

Source frontmatter: pillar Catalog · areas [specs, migration, web]. **read**

Source `note:` field, quoted: “three M1 classification defects: the Flux core input and migration ratchet had lost their consumers; the Anthropic YAML was a live repository-authored source spec whose origin and deliberately partial coverage were not recorded”

## Status

`done` in the source. Quoted from `docs/stories/S-022-orphaned-inputs-are-removed-or-re-owned.md:5`: `status: done`. **read**

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

Migrated from `docs/stories/S-022-orphaned-inputs-are-removed-or-re-owned.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-08-13 · 7 revision(s)
- Legacy id `S-022`, recorded as the reference `legacy:S-022`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill

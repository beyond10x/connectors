---
format: aep.planning-md/1
id: story:the-explorer-works-against-the-new-site-json
kind: story
status: implemented
title: The web explorer works against the site JSON M1 actually emits
refs:
- provider: legacy
  reference: S-018
relations:
- derived_from: epic:post-m1
scope:
- confidence: cited
  path: crates/catalog-build
revision: 6
---
## Acceptance

Verbatim from `docs/stories/S-018-the-explorer-works-against-the-new-site-json.md:21`. **read**

- [x] The dormant `web/` application, its Node manifests, tests, schemas, and brand copies are
      removed together.
- [x] `catalog-build` no longer constructs or writes `web/public/catalog.json`; the site-only model,
      renderer, status projection, and surface helpers are removed from the build graph.
- [x] JSON governance, source ownership, ignore rules, architecture fences, and local gate guidance
      no longer name the removed projection.
- [x] Catalog browsing is served from the hosted, authenticated Catalog protocol and presented by
      DevCenter, while canonical documents and `catalog.pack` remain the reviewed build artifacts.

## Context

Source frontmatter: pillar Catalog · areas [catalog-build] · priority 7. **read**

Source `note:` field, quoted: “Resolved by retiring the dormant repository explorer and its duplicate site projection after the hosted catalog contract and DevCenter became the supported browsing path”

## Status

`done` in the source. Quoted from `docs/stories/S-018-the-explorer-works-against-the-new-site-json.md:5`: `status: done`. **read**

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

Migrated from `docs/stories/S-018-the-explorer-works-against-the-new-site-json.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-09-02 · 4 revision(s)
- Legacy id `S-018`, recorded as the reference `legacy:S-018`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill

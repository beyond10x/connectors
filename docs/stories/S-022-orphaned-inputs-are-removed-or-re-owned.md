---
id: S-022
title: "Orphaned inputs are removed or re-owned"
pillar: Catalog
status: done
priority:
design:
epic: post-m1
areas: [specs, migration, web]
note: "three M1 classification defects: the Flux core input and migration ratchet had lost their consumers; the Anthropic YAML was a live repository-authored source spec whose origin and deliberately partial coverage were not recorded"
---

# Orphaned inputs are removed or re-owned

## Goal

Close the three orphans the M1 import left behind, so every file this repository derives from has a
consumer, a refresh path and a provenance record — and so
[S-016](S-016-sources-are-processed-by-code.md)'s `catalog sources check` can arrive **green** rather
than arriving with three permanent exceptions.

## The three, as SOURCES.toml records them

| Orphan | State |
|---|---|
| `specs/flux/core-v1.json` (157 KB) | its consumer — the core-catalogue projection — was deliberately dropped in M1. Nothing reads it. `web/README.md` still tells a reader that `public/v1/` is emitted from it. |
| `migration/` (README, `conformance-v1.schema.json`, `native-plugins.toml`, two fixtures) | the predecessor's C-505 native-plugin ratchet. Its verb (`migration-check`) was dropped in M1, so nothing reads or validates any of it; the only reference left is the `consumers` list of the predecessor-import source entry. |
| `specs/anthropic/2023-06-01-excerpt.yaml` | a live OpenAPI 3.1/YAML ingest source, authored in predecessor commit `ffd3613`, but lacking the provenance needed to distinguish B10x-authored bytes from an official Anthropic artifact. |

The third is the one that is not merely tidy-up. The source model had conflated "authoritative"
with "vendor-published". When no official machine-readable specification exists, the repository
authors the missing spec from cited official documentation and marks that ownership explicitly.
History proves this Anthropic excerpt was authored deliberately to exercise ingest. Its correct
disposition is therefore under `specs/` with authored provenance—not deletion, an invented vendor
origin, or a move outside the source layer.

## Acceptance

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
      [S-010](S-010-m5-flux-re-points-and-the-gitlab-plugin-retires.md) is what actually governs
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

## Progress
- 2026-08-13 — closed. The two repository-authored OpenAPI 3.1/YAML documents now cover every
  endpoint, parameter, response and authentication fact used by the eleven shipped Models/Admin
  reads, with one exact official reference on every operation and shared official authentication
  and versioning references in provenance. `providers/anthropic.toml` is now only identity,
  credential/config declarations and reviewed overlays; the real loader reproduces all eleven
  operations and records source provenance for each. Optional pagination/filter parameters remain
  visible in source and are omitted explicitly. Directory↔provenance and patch↔published coverage
  are tested in both directions.
- 2026-08-13 — removed the undocumented Console and consumer-subscription OAuth endpoints,
  services, configuration and third-party-inferred acquisition facts from connector authority.
  Anthropic API and Admin API access remains; Claude/Claude Code credentials belong to the harness
  adapter boundary fixed by architecture ADR 0014.
- Earlier in this story Anthropic's provider was hand-curated and contained no `[[spec]]` ingest,
  while the excerpt
  was not unused: `openapi_ingest.rs` depended on it for OpenAPI 3.1, YAML integer response keys and
  deterministic-ingest coverage. It is restored under `specs/` as a repository-authored,
  hash-pinned source grounded in Anthropic's official API, versioning, Messages and Models
  references. Its provenance is honest about partial coverage and does not claim refreshable vendor
  bytes or full provider derivation.
- The Flux core catalog input and its complete explorer surface were deleted together. M1 had
  already removed the projection that could generate this data; retaining static routes, types,
  tests, or publication claims would have described a catalog this repository no longer emits.
- The native-plugin ratchet was deleted rather than re-owned. Its five predecessor implementation
  waves conflict with the six readiness-ordered retirement waves; it had no checker and carried
  only synthetic fixtures, with no captured conformance or
  publication evidence. S-010 owns the live wave-1 proof over the operation inventory Flux actually
  serves, so retaining this second inventory would create two authorities for the same cutover.

## Notes

- `web/` work overlaps [S-018](S-018-the-explorer-works-against-the-new-site-json.md): the core
  explorer's deletion and the `core-v1.json` deletion are the same decision seen from two sides
  (the view and its input). Sequence them together or let S-018 land first and delete the input here.
- Read `migration/README.md` before deciding its fate: the ratchet's design point — no authored
  `present`/`published`/`retired` booleans, everything derived from a supplied checkout and paired
  observations — is a good idea retained by the current cutover story, so the question is
  where it lives, not whether it was right.
- Anthropic now belongs inside S-021's fully spec-derived provider count: two authored specs plus
  reviewed overlays reproduce its canonical document. `exact-shipped-surface` is intentionally not
  a claim of full Anthropic API coverage. A later curated Messages/inference expansion for the
  agent layer's direct-provider adapter remains possible from official API documentation; only
  Claude Code and consumer-subscription credential borrowing is outside this connector boundary.

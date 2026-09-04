---
id: S-017
title: "Mint source entries from the mined competitor catalogs"
pillar: Catalog
status: backlog
design:
epic: sources
areas: [catalog-build, docs]
note: "builds on S-016. The vendored reference corpus is a source-reference database: Nango's providers.yaml carries docs URLs, auth endpoints and base URLs for ~950 vendors; a spec directory (APIs.guru) carries the actual OpenAPI locations."
---

## Goal

`catalog sources mint <vendor>` turns the vendored competitor catalogs into a lookup instead
of a hunt: it consults the locally vendored reference artifacts — Nango `providers.yaml`,
Airbyte declarative connectors, Apideck's connector API, and a vendored spec directory
(APIs.guru or equivalent) — and proposes a ready-to-review `[[source]]` entry plus the
provider-file reference skeleton for that vendor: the spec upstream URL, the docs URL, and the
auth endpoints as cross-checks. It **proposes references to fetch, never content** — the actual
bytes still arrive only through `sources refresh` + the declared scrub, so the
sourced-never-authored rule holds by construction.

## Acceptance

- `mint` reads **only** locally vendored reference artifacts (each itself a `SOURCES.toml`
  entry with its own refresh) — no live network at mint time; the mining corpus stays fresh
  through the ordinary refresh path, hermetic-build discipline preserved.
- A spec directory is added to the vendored reference corpus (APIs.guru's machine-readable
  index or equivalent) so spec-URL lookup is actually answerable, with provenance like every
  other reference artifact.
- Output goes to **stdout only** (the scaffold rule: mint never writes or overwrites a file),
  with `TODO` holes for every judgment field.
- **Per-field citation**: every proposed value names which reference artifact it came from.
- **Disagreements are printed, never silently resolved** — e.g. Nango's `token_url` vs the
  spec's `servers` entry vs Apideck's connector metadata; the human resolves, the tool reports.
- A vendor found in no reference artifact yields an honest empty proposal naming what was
  searched — not a guess. Model-memory answers remain fabrication (AGENTS.md rule).

## Why now

Eight of 55 providers use spec ingest today, each spec URL hunted by hand. Scaling the catalog
(and the decision-0024 plugin-retirement waves, which need specs for aws, kubernetes and the
observability stack) means the corpus we already vendored should do the hunting — with the
citations preserved, because a minted reference is only as good as where it came from.

## Superseded by

`story:mint-source-entries-from-the-mined-catalogs` in the AEP planning store, at
`.engineering/planning/story/mint-source-entries-from-the-mined-catalogs.md`, now carries this story. Migrated 2026-09-04 by the
`aep-planning:story-migration` skill; this file is kept as the source it was migrated from and is
not the place the rung is recorded any more. The artifact quotes this file's `## Acceptance`
verbatim and names it in its own `## Provenance`.

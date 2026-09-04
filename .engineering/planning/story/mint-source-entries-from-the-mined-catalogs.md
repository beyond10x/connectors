---
format: aep.planning-md/1
id: story:mint-source-entries-from-the-mined-catalogs
kind: story
status: draft
title: Mint source entries from the mined competitor catalogs
refs:
- provider: legacy
  reference: S-017
relations:
- derived_from: epic:sources
revision: 1
---
## Acceptance

Verbatim from `docs/stories/S-017-mint-source-entries-from-the-mined-catalogs.md:23`. **read**

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

## Context

`catalog sources mint <vendor>` turns the vendored competitor catalogs into a lookup instead
of a hunt: it consults the locally vendored reference artifacts — Nango `providers.yaml`,
Airbyte declarative connectors, Apideck's connector API, and a vendored spec directory
(APIs.guru or equivalent) — and proposes a ready-to-review `[[source]]` entry plus the
provider-file reference skeleton for that vendor: the spec upstream URL, the docs URL, and the
auth endpoints as cross-checks. It **proposes references to fetch, never content** — the actual
bytes still arrive only through `sources refresh` + the declared scrub, so the
sourced-never-authored rule holds by construction.

Source frontmatter: pillar Catalog · areas [catalog-build, docs]. **read**

Source `note:` field, quoted: “builds on S-016. The vendored reference corpus is a source-reference database: Nango's providers.yaml carries docs URLs, auth endpoints and base URLs for ~950 vendors; a spec directory (APIs.guru) carries the actual OpenAPI locations.”

## Status

`backlog` in the source. Quoted from `docs/stories/S-017-mint-source-entries-from-the-mined-catalogs.md:5`: `status: backlog`. **read**

## Provenance

Migrated from `docs/stories/S-017-mint-source-entries-from-the-mined-catalogs.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-08-13 · 2 revision(s)
- Legacy id `S-017`, recorded as the reference `legacy:S-017`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill

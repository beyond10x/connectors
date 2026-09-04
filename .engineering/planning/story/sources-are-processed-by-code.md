---
format: aep.planning-md/1
id: story:sources-are-processed-by-code
kind: story
status: proposed
title: 'Sources are processed by code: the index is validated, checksummed and refreshed by the tool'
refs:
- provider: legacy
  reference: S-016
relations:
- derived_from: epic:sources
scope:
- confidence: cited
  path: crates/catalog-build
- confidence: cited
  path: crates/connector-spec
revision: 3
---
## Acceptance

Verbatim from `docs/stories/S-016-sources-are-processed-by-code.md:23`. **read**

- The `[[source]]` entry schema is owned by code and refused-by-name on unknown fields: `id`,
  `kind` (`openapi-spec | provider-catalog | reference-artifact | predecessor-import`),
  `origin` (`vendor | repository-authored`), `upstream` (an exact fetchable location for vendor
  bytes), `references` (exact authoritative documentation for repository-authored specs), the
  refresh method (native, a declared legacy script under `scripts/`, or `authored-review`), the
  declared scrub, coverage, pins (`sha256`, retrieval/review date, or a pointer to the per-vendor
  detail record), and `consumers`.
- `catalog sources check` — validates the index schema; verifies **every** recorded checksum
  against the bytes on disk; detects orphans in both directions (an entry whose files are
  missing, a vendored file no entry covers); exits non-zero on any drift. It runs inside the
  catalog invariant suite, so a drifted pin or an unindexed source fails the build.
- `catalog sources refresh <id> | --all` — for vendor origins, fetches per the entry (invoking the
  declared legacy `scripts/vendor-*.*` where one exists), applies the declared scrub, recomputes
  checksums, and rewrites the pins. For repository-authored origins, enters an authored-review
  workflow that checks source coverage against the cited references and re-pins reviewed bytes;
  it never implies that those bytes came from the vendor.
- `catalog sources diff <id>` — for vendor origins, fetches upstream to a scratch location and
  reports whether the pinned bytes drifted, **mutating nothing**. For authored origins, reports
  reference and coverage changes that require review. This is the cheap probe that cadence
  automation (`b10x-bot`) can run without changing the authored spec.
- The source model records the projection from each spec and its reviewed overlays to the canonical
  connector document; a reproducibility check rebuilds our format from those inputs and refuses
  drift. A partial spec may serve an ingest test, but cannot claim full provider reproducibility.
- Scale is a stated requirement: check/refresh/diff over hundreds of entries with zero
  per-entry human action.
- The AGENTS.md "Refreshing a source" instruction references only these verbs plus the
  catalog-diff review; no manual fetch/compare step survives in any document.

## Context

`SOURCES.toml` — the single index of everything this repository derives from (official and
repository-authored specs, mined reference artifacts, the predecessor import) — is a
**machine-processed manifest**.
A `catalog sources` verb family owns it end to end: schema validation, checksum verification,
orphan detection, drift probing, and refresh execution. No human or agent ever fetches an
upstream or compares a checksum by hand; the whole refresh discipline collapses to
*refresh → build → review the canonical-document diff*.

Source frontmatter: pillar Catalog · areas [catalog-build, connector-spec] · priority 5. **read**

Source `note:` field, quoted: “SOURCES.toml must never be prose + a manual runbook. At 8 ingest providers hand-comparison survives; at catalog scale (Nango: 957 providers) it cannot. Code owns the index.”

## Status

`ready` in the source. Quoted from `docs/stories/S-016-sources-are-processed-by-code.md:5`: `status: ready`. **read**

## Provenance

Migrated from `docs/stories/S-016-sources-are-processed-by-code.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-08-13 · 5 revision(s)
- Legacy id `S-016`, recorded as the reference `legacy:S-016`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill

---
id: S-016
title: "Sources are processed by code: the index is validated, checksummed and refreshed by the tool"
pillar: Catalog
status: ready
priority: 5
design:
epic: sources
areas: [catalog-build, connector-spec]
note: "SOURCES.toml must never be prose + a manual runbook. At 8 ingest providers hand-comparison survives; at catalog scale (Nango: 957 providers) it cannot. Code owns the index."
---

## Goal

`SOURCES.toml` — the single index of everything this repository derives from (official and
repository-authored specs, mined reference artifacts, the predecessor import) — is a
**machine-processed manifest**.
A `catalog sources` verb family owns it end to end: schema validation, checksum verification,
orphan detection, drift probing, and refresh execution. No human or agent ever fetches an
upstream or compares a checksum by hand; the whole refresh discipline collapses to
*refresh → build → review the canonical-document diff*.

## Acceptance

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

## Why now

The index was one instruction away from being born as documentation with a six-step manual
runbook. The predecessor's per-vendor vendoring scripts remain as declared fetch backends until
subsumed. Bootstrapping the initial `SOURCES.toml` content happens right after M1 (from the M1
report's import inventory); this story makes it live machinery instead of drifting prose —
the same derive-then-test-the-derivation rule every other artifact here follows.

## Superseded by

`story:sources-are-processed-by-code` in the AEP planning store, at
`.engineering/planning/story/sources-are-processed-by-code.md`, now carries this story. Migrated 2026-09-04 by the
`aep-planning:story-migration` skill; this file is kept as the source it was migrated from and is
not the place the rung is recorded any more. The artifact quotes this file's `## Acceptance`
verbatim and names it in its own `## Provenance`.

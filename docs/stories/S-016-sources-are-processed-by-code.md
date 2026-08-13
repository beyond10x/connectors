---
id: S-016
title: "Sources are processed by code: the index is validated, checksummed and refreshed by the tool"
pillar: Catalog
status: ready
priority: 5
design:
epic: catalog-day-one
areas: [catalog-build, connector-spec]
note: "SOURCES.toml must never be prose + a manual runbook. At 8 ingest providers hand-comparison survives; at catalog scale (Nango: 957 providers) it cannot. Code owns the index."
---

## Goal

`SOURCES.toml` — the single index of everything this repository derives from (vendored vendor
specs, mined reference artifacts, the predecessor import) — is a **machine-processed manifest**.
A `connectors sources` verb family owns it end to end: schema validation, checksum verification,
orphan detection, drift probing, and refresh execution. No human or agent ever fetches an
upstream or compares a checksum by hand; the whole refresh discipline collapses to
*refresh → build → review the canonical-document diff*.

## Acceptance

- The `[[source]]` entry schema is owned by code and refused-by-name on unknown fields: `id`,
  `kind` (`openapi-spec | provider-catalog | reference-artifact | predecessor-import`),
  `upstream` (an exact fetchable location, not a repo homepage), the fetch method (native, or a
  declared legacy script under `scripts/`), the declared scrub, pins (`sha256`, retrieval date,
  or a pointer to the per-vendor detail record), and `consumers`.
- `connectors sources check` — validates the index schema; verifies **every** recorded checksum
  against the bytes on disk; detects orphans in both directions (an entry whose files are
  missing, a vendored file no entry covers); exits non-zero on any drift. It runs inside the
  catalog invariant suite, so a drifted pin or an unindexed source fails the build.
- `connectors sources refresh <id> | --all` — fetches per the entry (invoking the declared
  legacy `scripts/vendor-*.sh` where one exists), applies the declared scrub, recomputes
  checksums, and rewrites the pins in the index and the detail record. Nothing under `specs/`
  is ever hand-edited.
- `connectors sources diff <id>` — fetches upstream to a scratch location and reports whether
  the pinned bytes drifted, **mutating nothing**: the cheap "did upstream change?" probe that
  cadence automation (selfdirect-bot) can run and act on only when the answer is yes.
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

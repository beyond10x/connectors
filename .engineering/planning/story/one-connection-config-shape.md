---
format: aep.planning-md/1
id: story:one-connection-config-shape
kind: story
status: draft
title: One connection config shape
refs:
- provider: legacy
  reference: S-051
scope:
- confidence: cited
  path: crates/connectors-config/src/lib.rs
- confidence: cited
  path: crates/connectors-config/src/personal.rs
- confidence: cited
  path: crates/integration-platform/src/tests.rs
revision: 4
---
## Acceptance

Verbatim from `docs/stories/S-051-one-connection-config-shape.md:19`. **read**

- connectors-config exposes a single connection config shape consumed by both call sites, or a
  doc comment on each type states the deliberate distinction and a test pins their divergence
  as intentional.
- Existing personal config files parse unchanged; no serialized field name moves.

## Context

Since S-048 retired `approval_evidence_ref`, `ConnectionConfig` and `B10xConnectionConfig`
in connectors-config are structurally identical (S-048 implementation notes, 2026-08-23). Two
names for one shape invite drift: a field added to one silently diverges the other. Merge them
into one connection config shape — or document why the b10x-provider connection is a
distinct type on purpose.

Source frontmatter: pillar Platform · areas [config, integrations]. **read**

## Status

`backlog` in the source. Quoted from `docs/stories/S-051-one-connection-config-shape.md:5`: `status: backlog`. **read**

## Provenance

Migrated from `docs/stories/S-051-one-connection-config-shape.md`, which is not deleted and now names this artifact.

- First written 2026-08-23 · last touched 2026-08-23 · 1 revision(s)
- Legacy id `S-051`, recorded as the reference `legacy:S-051`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill

## Scope

Derived 2026-09-07 by `story-scoper` at released base 4d0cd308 — cited.

- **Primary owner:** `crates/connectors-config/src/personal.rs` — cited; defines both shapes, their consumers and legacy parsing tests. Consolidate or document/test their intentional distinction without changing serialized fields.
- **Public exports:** `crates/connectors-config/src/lib.rs:21` — cited; exports both current type names.
- **Consumer regression:** `crates/integration-platform/src/tests.rs:31` — cited; constructs PlatformConnectionConfig directly.
- **Compatibility:** preserve existing personal configuration fields, the legacy b10x section alias and initiation spelling — cited.
- **Confidence:** high; duplicate definitions and direct consumers are located — cited.
- **Would collide with:** personal configuration types/validation, config exports or platform integration fixtures — cited.

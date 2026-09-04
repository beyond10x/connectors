---
format: aep.planning-md/1
id: story:one-connection-config-shape
kind: story
status: draft
title: One connection config shape
refs:
- provider: legacy
  reference: S-051
revision: 2
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

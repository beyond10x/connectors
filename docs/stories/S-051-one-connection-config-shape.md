---
id: S-051
title: "One connection config shape"
pillar: Platform
status: backlog
areas: [config, integrations]
---

# One connection config shape

## Goal

Since S-048 retired `approval_evidence_ref`, `ConnectionConfig` and `B10xConnectionConfig`
in connectors-config are structurally identical (S-048 implementation notes, 2026-08-23). Two
names for one shape invite drift: a field added to one silently diverges the other. Merge them
into one connection config shape — or document why the b10x-provider connection is a
distinct type on purpose.

## Acceptance

- connectors-config exposes a single connection config shape consumed by both call sites, or a
  doc comment on each type states the deliberate distinction and a test pins their divergence
  as intentional.
- Existing personal config files parse unchanged; no serialized field name moves.

## Progress

- 2026-08-23 — filed from the S-048 implementation notes (structural twin after the field
  retirement).

## Superseded by

`story:one-connection-config-shape` in the AEP planning store, at
`.engineering/planning/story/one-connection-config-shape.md`, now carries this story. Migrated 2026-09-04 by the
`aep-planning:story-migration` skill; this file is kept as the source it was migrated from and is
not the place the rung is recorded any more. The artifact quotes this file's `## Acceptance`
verbatim and names it in its own `## Provenance`.

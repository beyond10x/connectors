---
format: aep.planning-md/1
id: story:deployment-catalogs-are-external-packs
kind: story
status: draft
title: Deployment catalogs are external immutable packs
refs:
- provider: legacy
  reference: S-075
relations:
- derived_from: epic:deployment-packs
revision: 1
---
## Acceptance

Verbatim from `docs/stories/S-075-deployment-catalogs-are-external-packs.md:21`. **read**

- [ ] The hosted runtime accepts zero or more external catalog packs by immutable OCI digest. A
      mutable tag, local directory traversal, unknown schema version, duplicate provider authority
      or conflicting operation identity is a named startup refusal.
- [ ] A pack carries its schema version, content digest, provenance index and generated catalog
      bytes. The runtime verifies all of them before any item enters discovery or invocation.
- [ ] Merge order is deterministic and cannot replace or widen a core provider, operation risk,
      effect, authentication requirement or exposure decision. Extensions use an explicit namespace
      and may only narrow model visibility unless a reviewed deployment policy grants more.
- [ ] A private downstream build owns every deployment-specific provider declaration, specification,
      source-refresh script and generated pack. The generic repository and its OCI artifacts contain
      none of those identifiers or bytes.
- [ ] The current deployment-specific surface migrates without changing its stable provider,
      authority and operation identities. Golden discovery and invocation vectors agree before and
      after extraction.
- [ ] The Helm composition can mount or init-copy the pinned pack without baking it into the
      Connectors image, and reports the accepted pack digests through safe readiness/telemetry.
- [ ] CI reads the confidential-marker policy from a protected secret and fails both source and
      generated-artifact builds when a deployment identifier re-enters the generic repository.

## Context

Keep the generic Connectors source and release artifacts deployment-neutral while allowing a private
deployment to add proprietary providers, specifications and curated operation mappings as an
independently built, immutable catalog pack.

Source frontmatter: pillar Catalog · areas [catalog-reader, catalog-build, connectors-config, deployment] · design `../design/02-architecture.md`. **read**

Source `note:` field, quoted: “The core repository currently embeds a deployment-specific provider and generated bundle. Move ownership only after an external pack has the same deterministic and fail-closed guarantees.”

## Status

`backlog` in the source. Quoted from `docs/stories/S-075-deployment-catalogs-are-external-packs.md:5`: `status: backlog`. **read**

## Provenance

Migrated from `docs/stories/S-075-deployment-catalogs-are-external-packs.md`, which is not deleted and now names this artifact.

- First written 2026-09-01 · last touched 2026-09-02 · 2 revision(s)
- Legacy id `S-075`, recorded as the reference `legacy:S-075`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill

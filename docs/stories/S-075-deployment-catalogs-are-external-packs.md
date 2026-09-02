---
id: S-075
title: "Deployment catalogs are external immutable packs"
pillar: Catalog
status: backlog
priority:
design: ../design/02-architecture.md
epic: deployment-packs
areas: [catalog-reader, catalog-build, connectors-config, deployment]
note: "The core repository currently embeds a deployment-specific provider and generated bundle. Move ownership only after an external pack has the same deterministic and fail-closed guarantees."
---

# Deployment catalogs are external immutable packs

## Goal

Keep the generic Connectors source and release artifacts deployment-neutral while allowing a private
deployment to add proprietary providers, specifications and curated operation mappings as an
independently built, immutable catalog pack.

## Acceptance

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

## Progress

- 2026-09-02: the generic service-factory seam now admits an immutable, fully reviewable outbound
  MCP profile. It verifies the profile digest and exact live tool snapshot before a factory exists,
  then requires the ordinary complete deployment overlay before activation. This establishes the
  fail-closed dynamic-service join, but does **not** complete this story: hosted loading is not yet
  by immutable OCI digest, and the remaining acceptance items stay open.

## Notes

- This is a clean-HEAD migration, not a history rewrite.
- Removing the embedded bundle before the external loader and conformance vectors exist would break
  a current consumer; the extraction and the new loading seam ship as one coordinated cut.

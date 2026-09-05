---
format: aep.planning-md/1
id: story:deployment-date-order
kind: story
status: implemented
title: Pair deployment date filters with required ordering
relations:
- decomposes: epic:incremental-source-reads
- depends_on: story:brain-source-read-operations
scope:
- confidence: cited
  path: SOURCES.toml
- confidence: cited
  path: catalog
- confidence: cited
  path: connectors.lock
- confidence: cited
  path: crates/catalog-reader/catalog.pack
- confidence: cited
  path: crates/integration-catalog
- confidence: cited
  path: crates/integration-gitlab
- confidence: cited
  path: providers/gitlab.toml
- confidence: cited
  path: specs
revision: 6
---
## Requirements

A live read-only integration probe observed HTTP 400 for gitlab-deployment-list with an updated_after boundary; other incremental GitLab reads succeeded. GitLab's official API issue documents the server constraint: a deployment updated_after or updated_before filter requires order_by=updated_at, and that ordering requires a date filter. Source: https://gitlab.com/gitlab-org/gitlab/-/work_items/328500.

Derive this transport query parameter in both existing catalog and native placements without widening the closed caller input, credentials, grants or operation identity. Update authored specification/provenance and regenerate the shared catalog through the existing builder. This is a corrective continuation of the approved incremental source work discovered by its live validation, not a new provider capability.

## Acceptance

For after-only, before-only, both-boundaries and no-boundary requests, request-capture cases in both placements prove order_by=updated_at appears if and only if a date boundary exists; all existing gates pass and the same authorized live read succeeds after rebuilding the local daemon.

## Scope

Cited by the failing invocation and request dispatch: crates/integration-catalog, crates/integration-gitlab, providers/gitlab.toml, specs/gitlab/incremental-reads.openapi.yaml and its provenance. Coordinator owns generated catalog, catalog.pack and connectors.lock. No new entity is introduced.

## Integration evidence

The full repository gate exited zero:2068passing cases,0failed,27unchanged ignored, followed by catalog/docs/ESS projection checks. Eight new dispatch boundary cases passed across native and catalog placements. The coordinator reviewed both query derivations and regenerated the canonical catalog, pack and lock through catalog build. The exact previously failing authorized live deployment read succeeded with an empty items list and no next page after rebuilding and restarting the isolated local daemon; the old process exit was observed before replacement. No grants or credentials changed.

The operator subsequently selected glab as the temporary GitLab ingestion route while broader connector API support is delivered elsewhere. This completed narrow provider correction remains valid and introduces no additional feature scope. Broader GitLab connector work is not assigned here.

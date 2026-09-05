---
format: aep.planning-md/1
id: story:brain-source-read-operations
kind: story
status: active
title: Complete catalogued discovery and incremental read operations
relations:
- decomposes: epic:incremental-source-reads
scope:
- confidence: cited
  path: SOURCES.toml
- confidence: cited
  path: catalog
- confidence: cited
  path: catalog/confluence.catalog.json
- confidence: cited
  path: catalog/gitlab.catalog.json
- confidence: cited
  path: catalog/jira.catalog.json
- confidence: cited
  path: connectors.lock
- confidence: cited
  path: crates/catalog-build/tests/main/catalog_invariants.rs
- confidence: cited
  path: crates/catalog-reader/catalog.pack
- confidence: inferred
  path: crates/connectors-runtime
- confidence: inferred
  path: crates/integration-gitlab
- confidence: cited
  path: crates/integration-jira
- confidence: cited
  path: providers/confluence.toml
- confidence: cited
  path: providers/gitlab.toml
- confidence: cited
  path: providers/jira.toml
- confidence: cited
  path: specs
revision: 18
---
## Requirements

Expose the minimal missing read operations needed by issue, documentation and code-host ingestion: permitted project/space inventory and activity, changed Jira issues, Confluence changed pages with versions, and GitLab pipeline/deployment/default-branch activity listing where absent. Preserve existing operation identities, grants, secret handling and projections. Reuse existing reads and native datasource implementations rather than duplicating transport.

Every new API path, filter, pagination and auth detail must cite a fetched official vendor reference or existing authoritative spec. Add/update repository-authored specs and catalog through the repository's governed generation flow. No ad hoc brain HTTP adapter and no provider-auth fallback.

Contract tests cover pagination, overlapping update windows, stable IDs, grants, rate-limit errors, expired permission and scrubbed output. Offline package gates and full connector gate pass. Return exact operation refs and request/response shapes for the engine adapter implementor.

## Scope

Provider spec/catalog definitions and targeted runtime projections for Jira, Confluence and GitLab. Shared generated catalogs are single-writer. No writes to third-party systems and no adopter details in source or fixtures.

## Acceptance

Given existing read grants and a two-page fixture containing an overlapping update, a catalog-discovered incremental operation for each required provider returns complete revision-identifiable items under its documented continuation contract; the same call without its required read grant is refused before provider dispatch.

## Scope

Derived 2026-09-05 by `story-scoper`; entries distinguish evidence from expected changes.

- **Primary provider declarations:** `providers/jira.toml`, `providers/confluence.toml`, `providers/gitlab.toml` — cited; existing operation definitions are the catalog inputs (generation layout: `crates/catalog-build/src/workspace.rs:8`).
- **Specification/provenance inputs:** `specs`, `SOURCES.toml` — cited; story requires authored source specifications, and source registration already connects Jira inputs to the provider/runtime consumers (`SOURCES.toml:61`).
- **Generated provider outputs:** `catalog/jira.catalog.json`, `catalog/confluence.catalog.json`, `catalog/gitlab.catalog.json` — cited; deterministic canonical documents are owned by the catalog builder (`crates/catalog-build/src/workspace.rs:28`).
- **Shared generated outputs:** `connectors.lock`, `crates/catalog-reader/catalog.pack` — cited; only a whole-catalog build writes these (`crates/catalog-build/src/pipeline.rs:204`); coordinator owns final regeneration after integration.
- **Jira runtime:** `crates/integration-jira` — cited; `JIRA_OPERATIONS`, datasource request handling and scrubbed operation projections live in backend modules (`src/backend.rs:59`, `src/backend/datasource.rs:37`, `src/backend/operations.rs:342`, relative to this crate).
- **GitLab runtime:** `crates/integration-gitlab` — inferred; missing read exposure likely extends the existing operation allowlist, datasource request planning, projections and backend tests, whose owners are visible at `src/backend.rs:74`, `src/backend.rs:1925`, `src/backend.rs:2030` and `src/backend_tests.rs`, relative to this crate.
- **Catalog contract tests:** `crates/catalog-build/tests/main/catalog_invariants.rs` — cited; repository instructions require provider assertions in this existing consolidated suite; GitLab assertions already begin at line 401.
- **Possible integration wiring:** `crates/connectors-runtime` — inferred; only if an added read needs composition changes beyond existing native backends; Jira composition already exists in `src/composition.rs:866`, relative to this crate.
- **Documents:** no separate guide changes established; operation references and request/response shapes must be supplied in the implementor handoff — inferred.
- **Confidence:** medium — cited generation ownership and backend locations are solid; precise missing operations and whether Confluence needs runtime wiring remain to be determined against existing catalog contracts.
- **Would collide with:** edits to these provider declarations, their specifications/provenance, the Jira/GitLab runtime crates, consolidated catalog tests, runtime composition, or any whole-catalog regeneration — cited; shared outputs require one coordinator writer.

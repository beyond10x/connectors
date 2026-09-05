---
format: aep.planning-md/1
id: story:personal-gitlab-schedules-are-discoverable-and-governed
kind: story
status: active
title: Personal GitLab schedules are discoverable and governed
summary: Close the CLI discovery and validation gaps, then add write-gated GitLab pipeline-schedule operations.
tags:
- ready
- wave-cli
relations:
- derived_from: epic:local-product
- informed_by: story:explicit-target-never-implicit
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: SOURCES.toml
- confidence: cited
  path: catalog/gitlab.catalog.json
- confidence: cited
  path: connectors.lock
- confidence: cited
  path: crates/catalog-build/tests/main/catalog_invariants.rs
- confidence: inferred
  path: crates/catalog-cli/Cargo.toml
- confidence: inferred
  path: crates/catalog-cli/examples/vendor_gitlab.rs
- confidence: cited
  path: crates/connectors-cli/src/lib.rs
- confidence: inferred
  path: crates/connectors-cli/tests/search_bounds.rs
- confidence: inferred
  path: crates/connectors-runtime/src/registry.rs
- confidence: cited
  path: crates/integration-catalog/src/lib.rs
- confidence: inferred
  path: docs/guides/connect-gitlab.md
- confidence: inferred
  path: ess/system/domains/gitlab.yaml
- confidence: inferred
  path: ess/system/system.yaml
- confidence: cited
  path: providers/gitlab.toml
- confidence: inferred
  path: specs/gitlab.provenance.toml
- confidence: inferred
  path: specs/gitlab/coverage-19.4.toml
- confidence: inferred
  path: specs/gitlab/openapi-19.4.yaml
revision: 45
---


## Defect

An end-to-end check of the personal-local GitLab path on 2026-09-04 showed that a healthy,
callable connection is not an operable schedule-management surface:

- `connectors doctor -o json` reported the personal posture healthy and `gitlab-user-get` plus
  `gitlab-project-list` both succeeded through the local socket, while `connectors connection list
  --query gitlab` returned no connections even though `operation search --query gitlab` named the
  admitted catalog-backed GitLab connection; the two discovery surfaces therefore disagree about
  the same live connection.
- `connectors operation search --query 'pipeline schedule'` returned no operation because
  `providers/gitlab.toml` declares pipeline inspection but no pipeline-schedule list, create,
  update, or delete operations, so an operator cannot configure the internal scheduled trigger
  needed for periodic Atlas reconciliation through Connectors.
- `connectors operation search --limit 50` was accepted by clap and then surfaced
  `connector-unreachable: Connector request was invalid: InvalidInput: search bounds are invalid`;
  the actual maximum is the undocumented `MAX_SEARCH_RESULTS = 25` in
  `crates/protocol/src/operation.rs`, while `crates/connectors-cli/src/lib.rs` declares only a
  default and no range, making caller input look like transport failure.

The separate implicit hosted-target defect is already governed by
`story:explicit-target-never-implicit`; this story is informed by it and does not duplicate its
target-selection scope.

## Original scope requirements

- Make a catalog-backed personal-local connection appear consistently in `connection list` and
  `operation search`, with tests proving the same opaque connection reference is reported by both.
- Add source-grounded GitLab pipeline-schedule list, create, update, and delete operations to the
  existing provider declaration, generated catalog artifacts, and consolidated catalog invariant
  coverage; keep the read useful to operators and curate model exposure separately from
  callability.
- Classify schedule mutations with truthful effects, risk, idempotency, OAuth/PAT scope, and
  approval requirements, and preserve the personal aperture: mutations remain absent or refused
  unless that configured GitLab placement explicitly enables writes.
- Put protocol search bounds into clap validation and help for every affected CLI search command,
  and render an out-of-range value as local invalid input before any socket or network request.
- Document and test the shortest personal-local workflow for finding the connection, describing
  the schedule operations, and invoking them after explicit write enablement.

## Out of scope

- Creating or changing a production GitLab pipeline schedule as part of implementation.
- Enabling writes in an operator's personal configuration.
- Reworking hosted-versus-local target selection, which belongs to
  `story:explicit-target-never-implicit`.

## Acceptance

With a healthy personal-local GitLab connection, the default local CLI reports one consistent connection across connection and operation discovery, documents and rejects search limits above the protocol maximum before transport, exposes source-grounded pipeline-schedule list/create/update/delete operations, and proves schedule mutations remain refused until writes are explicitly enabled and approval requirements are satisfied.

## Readiness

Selected for implementation on 2026-09-06 at the operator's request. wave-cli: 8 of 10. The ready tag records selection; proposed is the pre-implementation lifecycle state, and existing active work stays active. Existing dependencies and implementation evidence requirements still apply.

## Scope

Derived 2026-09-06 by aep-drive story-scoper; coordinator records the returned surfaces.

- `providers/gitlab.toml` — cited.
- `specs/gitlab/openapi-19.4.yaml` — inferred.
- `specs/gitlab.provenance.toml` — inferred.
- `specs/gitlab/coverage-19.4.toml` — inferred.
- `crates/catalog-cli/examples/vendor_gitlab.rs` — inferred.
- `crates/catalog-cli/Cargo.toml` — inferred.
- `Cargo.lock` — inferred.
- `SOURCES.toml` — inferred.
- `catalog/gitlab.catalog.json` — cited.
- `connectors.lock` — cited.
- `crates/catalog-build/tests/main/catalog_invariants.rs` — cited.
- `crates/integration-catalog/src/lib.rs` — cited.
- `crates/connectors-cli/src/lib.rs` — cited.
- `crates/connectors-cli/tests/search_bounds.rs` — inferred.
- `crates/connectors-runtime/src/registry.rs` — inferred.
- `docs/guides/connect-gitlab.md` — inferred.
- `ess/system/domains/gitlab.yaml` — inferred.
- `ess/system/system.yaml` — inferred.

Medium confidence. Add catalog Connection capability using existing binding refs. Bound search at operations 1..25 and Connection/event 1..64 before transport. Use the fetched official OpenAPI and deterministic vendor-derived projection with provenance; do not author a replacement spec. Literal operation-id search needs an explicit supported query or separately tested token matching. Required approval metadata is not proof enforcement; retain the established local grant boundary. Type any newly introduced schedule resource from the fetched contract before implementation.

Would collide with any unit editing these files; directory entries require an additional containment review because AEP compares scope strings exactly.

## Execution queue

CLI execution queue 2026-09-06: 6 of 10. The urgent Slack delivery follow-up leads the queue. Original wave-cli readiness ordering remains historical context. Dependencies and measured scope govern dispatch order; priority is not a claim that prerequisites have landed.

## Source and model decision

The operator confirmed on 2026-09-06: if an official OpenAPI specification exists, use it. This standing rule is now explicit in AGENTS.md. Current fetched sources: https://docs.gitlab.com/api/openapi/ and the official GitLab OpenAPI 3.0.0 document at https://gitlab.com/gitlab-org/gitlab/-/raw/master/doc/api/openapi/openapi_v3.yaml (info.version 19.4; 1,367 paths; raw SHA-256 f9e830bd3d2b99c49d60a7713fe1a64f5164418aca24b559287daab075beb530). Resolve and record an immutable upstream revision before vendoring.

Preserve the complete scrubbed official OpenAPI as the vendor source. The first ingest selection covers GET/POST /api/v4/projects/{id}/pipeline_schedules and PUT/DELETE /api/v4/projects/{id}/pipeline_schedules/{pipeline_schedule_id}, with a deterministic inventory accounting for every source operation and the remaining coverage backlog. Preserve vendor schema truth, record the exact selections and declared example scrub, and derive catalog operations through OpenAPI ingest. No handwritten replacement endpoint/schema definitions. An importer limitation is implementation work to extend the importer, never permission for a handwritten replacement. Existing inline operations retain their provenance; no claim that the new source generated them.

The reverse OpenAPI draft was refined into value-only connectors.gitlab request/snapshot types. GitLab owns schedule lifecycle and owner identities; no Connector-owned schedule entity, credential relation or grant is invented. Losses remain explicit: omission versus null/defaults, vendor list response object versus documented array, and heterogeneous input collections outside this initial surface. Reviewed overlays must separately justify these discrepancies and policy decisions from the fetched official schedule/authentication references. The new schedule spec must not rewrite existing GitLab OAuth endpoints from incomplete vendor security metadata.

The source refresh tool is an offline Rust example under catalog-cli, using the downloaded pinned vendor file, deterministic selection/reference closure/scrub and exact hash checks. Normal catalog builds remain offline. Register the vendor-derived source and provenance. The multi-credential unit owns catalog Connection discovery first; this unit reuses that capability and proves consistent references for GitLab.

## Complete coverage and focused projections

The operator clarified the standing direction on 2026-09-06: official OpenAPI is mandatory when available; the usual connector goal is ALL operations. Focused surfaces are consumer projections or prebuilt Connectors projections. Administrative and regular-user surfaces may be separated, grounded in the provider permission model. A projection never grants authority.

The four pipeline-schedule operations are the first delivery slice selected by this wave, not the target GitLab API coverage. Keep the full official OpenAPI as source authority and account for every operation in a deterministic coverage inventory: admitted/catalogued, platform authentication flow, or an explicit remaining coverage/importer gap. Preserve a backlog for the remaining source operations. Do not let a four-operation source extraction become an invisible limit on connector scope. Narrow model exposure and complete underlying callability are independent decisions.

Before implementing the GitLab source path, revise its bounded vendoring proposal to retain the complete scrubbed vendor document and derive the schedule slice through reviewed ingest selections. Classify administrative versus regular-user operations only from source-grounded permissions, and carry unresolved classifications as explicit gaps rather than assuming access. Existing inline operation provenance remains truthful until each operation is migrated to the official source.

## Official source formats

The operator further clarified on 2026-09-06: connector definitions should normally never be hand-authored. When an official source format is unsupported, build the importer. Official OpenAPI is used whenever available. This supersedes any earlier suggestion that an unsupported importer permits authored endpoint/schema replacements. The only authored-source exception is a documented absence of a usable official machine-readable source, grounded in fetched official documentation. Full operation coverage and focused consumer/prebuilt projections remain the goal; administrative and regular-user projections do not grant authority.

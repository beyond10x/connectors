---
format: aep.planning-md/1
id: story:personal-gitlab-schedules-are-discoverable-and-governed
kind: story
status: implemented
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
  path: catalog
- confidence: cited
  path: catalog/gitlab.catalog.json
- confidence: cited
  path: connectors.lock
- confidence: cited
  path: crates/catalog-build/Cargo.toml
- confidence: cited
  path: crates/catalog-build/src/check.rs
- confidence: cited
  path: crates/catalog-build/src/contract.rs
- confidence: cited
  path: crates/catalog-build/src/document.rs
- confidence: cited
  path: crates/catalog-build/src/document_schema.rs
- confidence: cited
  path: crates/catalog-build/src/document_tests.rs
- confidence: cited
  path: crates/catalog-build/src/pack.rs
- confidence: cited
  path: crates/catalog-build/src/pipeline.rs
- confidence: cited
  path: crates/catalog-build/src/scaffold.rs
- confidence: cited
  path: crates/catalog-build/src/scaffold_tests.rs
- confidence: cited
  path: crates/catalog-build/src/workspace.rs
- confidence: cited
  path: crates/catalog-build/tests/main/catalog_invariants.rs
- confidence: cited
  path: crates/catalog-build/tests/main/no_network.rs
- confidence: inferred
  path: crates/catalog-cli/Cargo.toml
- confidence: inferred
  path: crates/catalog-cli/examples/vendor_gitlab.rs
- confidence: cited
  path: crates/catalog-cli/tests/offline_binary.rs
- confidence: cited
  path: crates/catalog-reader/catalog.pack
- confidence: cited
  path: crates/catalog-reader/src/lib.rs
- confidence: cited
  path: crates/catalog-reader/tests/main/pack.rs
- confidence: cited
  path: crates/catalog/src/lib.rs
- confidence: cited
  path: crates/catalog/src/table.rs
- confidence: cited
  path: crates/catalog/tests/main.rs
- confidence: cited
  path: crates/connector-resolve/Cargo.toml
- confidence: cited
  path: crates/connector-resolve/src/document.rs
- confidence: cited
  path: crates/connector-resolve/src/resolve.rs
- confidence: cited
  path: crates/connector-resolve/tests/adversary_gitlab_pass1.rs
- confidence: inferred
  path: crates/connector-spec/schema/provider-toml.schema.json
- confidence: cited
  path: crates/connector-spec/src/ir.rs
- confidence: cited
  path: crates/connector-spec/src/ir_parameters.rs
- confidence: cited
  path: crates/connector-spec/src/lib.rs
- confidence: cited
  path: crates/connector-spec/src/openapi.rs
- confidence: inferred
  path: crates/connector-spec/src/provider/declaration.rs
- confidence: cited
  path: crates/connector-spec/src/provider/loading.rs
- confidence: cited
  path: crates/connector-spec/src/provider/operation_validation.rs
- confidence: inferred
  path: crates/connector-spec/src/provider/patch_validation.rs
- confidence: inferred
  path: crates/connector-spec/src/provider/publishing.rs
- confidence: inferred
  path: crates/connector-spec/src/provider/schema_sync.rs
- confidence: cited
  path: crates/connector-spec/src/provider/validation.rs
- confidence: cited
  path: crates/connector-spec/src/schema_translation.rs
- confidence: cited
  path: crates/connector-spec/tests/main/ir_roundtrip.rs
- confidence: cited
  path: crates/connector-spec/tests/main/openapi_ingest.rs
- confidence: inferred
  path: crates/connector-spec/tests/main/operation_selection.rs
- confidence: cited
  path: crates/connector-spec/tests/main/provider_schema.rs
- confidence: cited
  path: crates/connector-spec/tests/main/response_schema_coverage.rs
- confidence: cited
  path: crates/connector-spec/tests/main/shipped_providers.rs
- confidence: inferred
  path: crates/connectors-cli/Cargo.lock
- confidence: cited
  path: crates/connectors-cli/src/lib.rs
- confidence: inferred
  path: crates/connectors-cli/tests/search_bounds.rs
- confidence: cited
  path: crates/connectors-runtime/Cargo.lock
- confidence: inferred
  path: crates/connectors-runtime/src/registry.rs
- confidence: inferred
  path: crates/connectors-runtime/tests/local_gitlab_schedules.rs
- confidence: cited
  path: crates/integration-catalog/src/lib.rs
- confidence: cited
  path: crates/service/src/planning.rs
- confidence: cited
  path: docs/design/01-domain-model.md
- confidence: cited
  path: docs/design/04-the-callers-contract.md
- confidence: inferred
  path: docs/guides/connect-gitlab.md
- confidence: cited
  path: ess/system/domains/catalog.yaml
- confidence: inferred
  path: ess/system/domains/gitlab.yaml
- confidence: inferred
  path: ess/system/system.yaml
- confidence: cited
  path: json-schemas.toml
- confidence: cited
  path: providers/gitlab.toml
- confidence: inferred
  path: specs/gitlab.provenance.toml
- confidence: inferred
  path: specs/gitlab/coverage-19.4.toml
- confidence: inferred
  path: specs/gitlab/openapi-19.4.yaml
revision: 99
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

## Importer scope extension

Measured stage 1: official GitLab commit eaeb4b8b88fdee3fe9b1d1a54397226cf191c201 reproduces raw SHA f9e830bd3d2b99c49d60a7713fe1a64f5164418aca24b559287daab075beb530. The complete document contains 1,847 operations, no duplicate/missing operation IDs, and no authentication-flow operation endpoints (OAuth appears in security definitions). This is source coverage, not a claim all operations are callable yet.

Additional inferred implementation scope, authorized before edits: crates/connector-spec/src/provider/declaration.rs, publishing.rs, patch_validation.rs, schema_sync.rs; crates/connector-spec/tests/main/operation_selection.rs. The extension supports a reviewed generated-schema array wrapper for vendor/documentation response discrepancies, retaining vendor-generated properties. Actual schema file path and any measured request-importer gap must be named before further scope extension. This follows the operator's rule to build importer capability rather than hand-author a replacement specification.

## Schema fidelity decision

Operator steering on 2026-09-06 requires selected request and response schemas to be one for one with the official specification. The bounded four-operation schedule slice remains selected. Full GitLab operation coverage remains inventoried separately; the practical Slack scope exception does not silently redefine GitLab's coverage inventory.

The pinned complete source is the official OpenAPI at commit eaeb4b8b88fdee3fe9b1d1a54397226cf191c201. Source defects must be named separately; do not repair object/array shape, narrow accepted parameter types, discard constraints, or invent missing schemas in handwritten overlays. Policy metadata such as risk, effects, scope and model exposure stays explicit and separate.

The earlier stage-1 response-array and parameter-schema correction proposal is superseded. Read-only inspection also measured existing generic contract loss: integer becomes number, enum/constraints disappear from the model-facing contract, object request-body semantics can be lost during flattening, and absent optional fields can be emitted as null. Fix the actual generic owners, with end-to-end fixtures for requiredness, omission/null, full object schemas and literal source equality. A green source/catalog stage alone cannot close runtime/CLI acceptance.

The new source retains all 1,847 operations, with four generated schedule operations, 20 legacy inline operations, 1,772 unreviewed coverage gaps and 51 importer-gap rows. Actual multipart import refusals affect 52 operations total: those 51 gap rows and one legacy operation. Keep existing inline provenance truthful. Recompute this measured inventory if source or admission changes.

## Source-fidelity execution

Atlas proposed ADR 0040 (connector-catalog-source-fidelity) records catalog schema 3 and the producer/reader/resolver and relying-party movement. It is a reviewable migration decision under the operator's existing source-fidelity requirement; it does not claim release or delivery. The coordinator drafted RequestSemantics and RequestParameterSemantics in ess/system/domains/catalog.yaml before implementation. ESS validation over ess/system reports: connectors v1 — 10 file(s), valid. An initial invocation on system.yaml alone refused undeclared domains because it loaded only the header; the complete-directory validation resolved that invocation error without changing domain ownership.

The closed semantics are legacy_v1 and openapi_3_0_json_v1. ParamSet carries the profile plus optional body_required, meaningful only with body_schema. The required canonical operation profile prevents old or unknown request interpretation. Keep literal source schemas, translate OAS 3.0 semantics deterministically into caller JSON Schema, preserve request-body presence and literal strings, and refuse unsupported constructs. Remove the unused response_arrays feature introduced by this uncommitted stage rather than retaining a new hand-correction facility. Existing legacy behavior remains explicit and is not relabeled faithful.

Additional machine scope now records the generic importer/translation, contract/document/build/reader/table/resolver, profile tests, catalog directory and JSON inventory. The catalog directory scope owns only deterministic generated documents, pack/schema and their source registration; no other active worker owns generated catalog output. It is deliberately recorded as a directory and additionally reviewed for containment. Preserve the old schema 2 identity/bytes and emit a distinct schema 3 identity/file. Registry, CLI and integration-catalog/lib.rs remain deferred until their current owners hand off. The GitLab agent may return a small integration-catalog schema-selection patch for coordinator integration; it must not race the credentials owner.

The exact algorithm/test proposal is the implementor's source-fidelity-design.md retained in the wave's private scratch. Its required cases independently compare all four literal source schema closures, nullable/enum/oneOf behavior, body absence/null/defaults, string bodies, safe namespaced path encoding, unknown profile/old-reader refusals and every retained legacy behavior. No production provider call or operator configuration change is authorized. Source defects remain visible and must not be silently repaired.

## Current migration authority

Fresh clean Atlas authority is3f0dbbf7701ce636df510316282d1c96a917df01 in managed tree wt-09ac7303c4aa. Its latest accepted ADR is0039, the ESS conformance-count reader-before-writer migration. The Connectors wave still aligns with accepted ADR0038's generic platform/independent extension boundary; ADR0039 does not change Connector provider selection, schema fidelity, grant or custody rules.

Our unpublished migration drafts were replayed through AEP on a branch from that exact remote commit, reusing clean managed tree wt-19c8158c5cae as plan/connector-cli-migrations. Rate-limit protocol proposal is now ADR0040; source-fidelity catalog proposal is ADR0041. The prior local5e3a327 proposal branch remains a recovery reference; its0039/0040 numbering is superseded and not published. No planning journal was textually merged or copied.

Atlas's full fences.sh exited1. Catalog, live Pages, projections and brand passed. Shared primary state fails documentation collection (AgentIDE b10x-docs/v4 unsupported by the pinned collector), Website Docs System pin, and map grounding (widgets lacks Serves). A bot-observer test also failed its error-message assertion; the exact focused rerun passed, and a full Rust rerun follows. Coordinator fixed one newly introduced relative ADR link; the managed Markdown checker now reports158files and0findings. The full gate remains red; the draft is not pushed and none of these observations claims delivery.

Pipe worktree wt-55b6348b9982 GC completed through exact reviewed-id application. The manager recorded remote recovery through origin:refs/heads/wave/cli-ten-slack-first and origin:refs/pull/14/head before non-forced removal. No other tree was selected.

One-shot source970e4af56f7a4ca1b9689c885fabd3a519bbe0ca is bot-authored and bot-committed after618 passing affected tests (2existing ignored), strict clippy/fmt/module fence. First tests-only attack is dispatched to adversary_one_shot_1 in the same owned tree; no AEP/Git writes delegated. Full base for its review remains3df1cd2df32472a4afc8feeaac8c46c9de8d1b55. The implementation report and original red fixtures remain under the recorded scratch triple.

## Complete schema 3 implementation gate — 2026-09-06

The complete publication candidate 900fac0a435b6e0d2922825f366e0c9607150400 retains reviewed GitLab source/tests and the independently reviewed structured-rate unit, plus published main 4b32397df2cc2f5bd5ee1c5737891163fb750957. All twelve declared workspace gates pass, including both runtime feature configurations: 2,244 passing executions, zero failures and 27 existing ignored tests. All thirteen strict all-target Clippy configurations, twelve formatting and locked/offline metadata checks, deterministic 65-provider/69-artifact catalog checks, 85 rate-v2 conformance vectors, final links/stories/ESS/generated projection and the deployment-identity refusal guard pass. The complete report is [verification-report:cli-schema3-publication-gate-20260906](../verification-report/cli-schema3-publication-gate-20260906.md).

The first whole-unit GitLab adversary remains green and immutable at review-result:cli-gitlab-adversary-1-20260906. It independently covered the four complete selected source schema closures, resolver behavior, runtime authority and CLI discovery. The official complete 1,847-operation source inventory remains explicit; this implementation makes the selected schedule slice faithful and callable under existing admission, not every GitLab operation. The later rate integration preserves the source and frozen schema 2 while completing canonical schema 3 as one producer/reader/executable unit.

The measured initial SQL formatting failure was byte-identical to published main; its separately recorded three-file formatter correction is the only publication-gate source change after final rate review. Both disk interruptions, the original all-local-ref scan findings and exact scoped-history/public-ref verification remain retained in the complete report. Current nonplanning source matches all 998 frozen files. This records implementation and full local verification; source publication/CI, documentation delivery and the matching final installed client/daemon update remain delivery steps. No live GitLab operation, external consumer upgrade, release or tag is claimed.

## Published migration proposals — 2026-09-06

Atlas draft PR23 now publishes exact commit bfb731377a448ab89443c44852d622601f6c1363. ADR0041 is the Operation v2 rate migration; ADR0042 is canonical schema3 source fidelity; ADR0043 is later schema4 personal OAuth; ADR0044 is later Operationv3/Connectionv2 authentication remediation. Atlas main's accepted ESS decision owns0040. Earlier number references above describe their historical proposal snapshots; this dated mapping is current and those earlier records remain unchanged.

All four proposals and the full published-consumer pin audit were read back as exact remote Git blobs. The proposals remain proposed on a draft PR; publishing them records neither architecture acceptance nor external adoption. ADR0041's dated authentication section and separate ADR0044 name the subsequent migration and limit the earlier Connection/auth exclusions to the rate unit. SDK-before-embedded-host ordering, separate Devcenter locks, Agent Platform digest coupling, Org Brain's unknown deployed executable and Zwirn's legacy namespace/audience are explicitly recorded. This wave changes no external consumer repository.

The complete Atlas fence at that same commit ran149passing Rust tests, with catalog, live Pages, projection, Markdown and brand green. It exited1 for three existing primary-workspace failures: AgentIDE's unsupported v4 manifest, Website's stale Docs System package pin and Widgets' missing Serves section. The normal bot-wrapper update of the existing PR23 branch succeeded through the corrected installed guard; no hook bypass or attestation commit was used. Publication does not claim those unrelated organization fences are green.

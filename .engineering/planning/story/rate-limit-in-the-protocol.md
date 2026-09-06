---
format: aep.planning-md/1
id: story:rate-limit-in-the-protocol
kind: story
status: active
title: A provider's rate limit is a protocol fact, not a sentence
tags:
- ready
- wave-cli
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: inferred
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: catalog/connector-document-v3.schema.json
- confidence: inferred
  path: catalog/slack.catalog.json
- confidence: inferred
  path: connectors.lock
- confidence: inferred
  path: contracts/connector-operation/v0alpha2
- confidence: inferred
  path: crates/catalog-build/src/contract.rs
- confidence: cited
  path: crates/catalog-build/src/document.rs
- confidence: cited
  path: crates/catalog-build/src/document_schema.rs
- confidence: cited
  path: crates/catalog-build/tests/main/catalog_invariants.rs
- confidence: inferred
  path: crates/catalog-build/tests/main/ess_claim_fence.rs
- confidence: inferred
  path: crates/catalog-reader/catalog.pack
- confidence: cited
  path: crates/catalog/src/lib.rs
- confidence: cited
  path: crates/catalog/src/table.rs
- confidence: inferred
  path: crates/catalog/tests/main/consumer_api.rs
- confidence: inferred
  path: crates/catalog/tests/main/pack_table.rs
- confidence: inferred
  path: crates/connector-spec/Cargo.toml
- confidence: inferred
  path: crates/connector-spec/schema/provider-toml.schema.json
- confidence: cited
  path: crates/connector-spec/src/ir.rs
- confidence: cited
  path: crates/connector-spec/src/lib.rs
- confidence: cited
  path: crates/connector-spec/src/provider.rs
- confidence: cited
  path: crates/connector-spec/src/provider/declaration.rs
- confidence: cited
  path: crates/connector-spec/src/provider/publishing.rs
- confidence: cited
  path: crates/connector-spec/src/provider/schema_sync.rs
- confidence: inferred
  path: crates/connector-spec/src/rate_limit.rs
- confidence: inferred
  path: crates/connector-spec/tests/golden/authored-input-schema.error
- confidence: inferred
  path: crates/connector-spec/tests/golden/operation-auth-typo.error
- confidence: cited
  path: crates/connector-spec/tests/main/determinism.rs
- confidence: inferred
  path: crates/connector-spec/tests/main/ir_roundtrip.rs
- confidence: cited
  path: crates/connector-spec/tests/main/service_partition.rs
- confidence: inferred
  path: crates/connectors-cli/Cargo.lock
- confidence: inferred
  path: crates/connectors-cli/src/error.rs
- confidence: cited
  path: crates/connectors-cli/src/lib.rs
- confidence: inferred
  path: crates/connectors-cli/tests/adversary_fence_probe.rs
- confidence: inferred
  path: crates/connectors-cli/tests/adversary_fence_probe_pass2.rs
- confidence: inferred
  path: crates/connectors-cli/tests/cli_surface.rs
- confidence: inferred
  path: crates/connectors-cli/tests/cli_surface_drift.rs
- confidence: inferred
  path: crates/connectors-cli/tests/cli_surface_pass_two.rs
- confidence: cited
  path: crates/connectors-cli/tests/one_shot_operations.rs
- confidence: cited
  path: crates/connectors-client/src/lib.rs
- confidence: cited
  path: crates/connectors-client/src/response.rs
- confidence: inferred
  path: crates/connectors-console/Cargo.lock
- confidence: cited
  path: crates/connectors-console/src/envelope.rs
- confidence: cited
  path: crates/connectors-console/src/output.rs
- confidence: inferred
  path: crates/connectors-runtime/Cargo.lock
- confidence: cited
  path: crates/connectors-runtime/src/registry.rs
- confidence: cited
  path: crates/connectors-runtime/src/service_bundle.rs
- confidence: inferred
  path: crates/connectors-runtime/tests/rate_adversary_registry.rs
- confidence: inferred
  path: crates/driver-audio/Cargo.lock
- confidence: inferred
  path: crates/driver-cdp/Cargo.lock
- confidence: inferred
  path: crates/driver-sip/Cargo.lock
- confidence: inferred
  path: crates/driver-speech/Cargo.lock
- confidence: inferred
  path: crates/driver-sql/Cargo.lock
- confidence: cited
  path: crates/integration-catalog/src/lib.rs
- confidence: inferred
  path: crates/integration-catalog/src/tests.rs
- confidence: cited
  path: crates/integration-gitlab/src/backend.rs
- confidence: cited
  path: crates/integration-jira/src/backend/operations.rs
- confidence: cited
  path: crates/integration-kubernetes/src/hosted.rs
- confidence: cited
  path: crates/integration-kubernetes/src/local.rs
- confidence: cited
  path: crates/integration-kubernetes/src/local_inventory.rs
- confidence: cited
  path: crates/integration-kubernetes/src/workloads.rs
- confidence: cited
  path: crates/integration-mcp/src/lib.rs
- confidence: cited
  path: crates/integration-monitoring/src/backend.rs
- confidence: cited
  path: crates/integration-platform/src/datasource.rs
- confidence: cited
  path: crates/integration-platform/src/surface.rs
- confidence: cited
  path: crates/integration-sip/src/backend/mod.rs
- confidence: inferred
  path: crates/integration-slack/src/backend.rs
- confidence: cited
  path: crates/integration-slack/src/backend/api_runtime.rs
- confidence: cited
  path: crates/integration-slack/src/backend_tests.rs
- confidence: cited
  path: crates/protocol/Cargo.toml
- confidence: cited
  path: crates/protocol/README.md
- confidence: inferred
  path: crates/protocol/examples/operation_v2_bundle.rs
- confidence: cited
  path: crates/protocol/src/operation.rs
- confidence: inferred
  path: crates/protocol/src/operation/legacy.rs
- confidence: inferred
  path: crates/protocol/src/operation/schema.rs
- confidence: inferred
  path: crates/protocol/src/operation/wire.rs
- confidence: cited
  path: crates/protocol/tests/bundles.rs
- confidence: inferred
  path: crates/protocol/tests/rate_adversary.rs
- confidence: inferred
  path: crates/rtvbp-voice-endpoint/Cargo.lock
- confidence: cited
  path: crates/server/src/egress.rs
- confidence: inferred
  path: crates/server/src/egress_rate_adversary_tests.rs
- confidence: cited
  path: crates/server/src/hosted.rs
- confidence: cited
  path: crates/server/src/hosted/mcp/toolset.rs
- confidence: inferred
  path: crates/server/src/hosted/operation_transport.rs
- confidence: inferred
  path: crates/server/src/hosted/tests/contract_validation.rs
- confidence: inferred
  path: crates/server/src/hosted/tests/docs.rs
- confidence: cited
  path: crates/server/src/hosted/tests/enforcement.rs
- confidence: cited
  path: crates/server/src/hosted/tests/mcp.rs
- confidence: cited
  path: crates/server/src/hosted/tests/mcp_monitoring.rs
- confidence: cited
  path: crates/server/src/hosted/tests/signal.rs
- confidence: cited
  path: crates/server/src/local.rs
- confidence: inferred
  path: crates/server/tests/rate_adversary_local.rs
- confidence: inferred
  path: crates/service/Cargo.toml
- confidence: cited
  path: crates/service/src/egress.rs
- confidence: cited
  path: crates/service/src/lib.rs
- confidence: inferred
  path: crates/service/src/rate_limit.rs
- confidence: inferred
  path: crates/voice-local-audio/Cargo.lock
- confidence: inferred
  path: crates/voice-runtime/Cargo.lock
- confidence: cited
  path: docs/architecture/interfaces.md
- confidence: cited
  path: docs/design/04-the-callers-contract.md
- confidence: cited
  path: ess/system/components.yaml
- confidence: cited
  path: ess/system/domains/catalog.yaml
- confidence: inferred
  path: ess/system/domains/connection.yaml
- confidence: cited
  path: ess/system/domains/runtime.yaml
- confidence: cited
  path: ess/system/system.yaml
- confidence: cited
  path: json-schemas.toml
- confidence: inferred
  path: providers/slack.toml
revision: 141
---
# Story: a provider's rate limit is a protocol fact, not a sentence

## Defect
`OperationErrorCode` has no rate-limit variant and the response envelope has no `retry_after`;
a 429 from Slack surfaces as `unavailable` with the text "the provider refused the request with
HTTP 429 — the provider is rate-limiting this credential". A caller that must pace itself (the
org-brain ledger walked 530 pages in one run and hit 51 such refusals) has to match the wording.

## Shape
- `OperationErrorCode::RateLimited` with `retry_after_seconds: Option<u64>` taken from the
  provider's `Retry-After` header when present.
- Catalogue operations may declare their provider's rate tier so `describe` exposes a suggested
  interval; the personal runtime may pace requests per connection to that interval.

## Acceptance

- A definite provider429 with one Retry-After:30 returns rate_limited and retry_after_seconds:30 through v2, CLI JSON/YAML and MCP structuredContent; missing/malformed/ambiguous headers preserve no invented delay. Existing v1 clients receive their explicit legacy projection.
- slack-conversations-history describe exposes source-grounded conditional rate advice, applicability and advisory intervals. It distinguishes minimum tier allowance from published ceiling, identifies unestablished rates, and never silently selects an application category.
- Frozen v1 bytes remain unchanged; complete v2 request/response vectors, dual-version transport tests and no-retry tests cover the actual client/server boundaries. Ambiguous writes remain OutcomeUnknown. Vendor request/response schema bytes and semantics remain unchanged by rate-policy overlays.

## Readiness

Selected for implementation on 2026-09-06 at the operator's request. wave-cli: 9 of 10. The ready tag records selection; proposed is the pre-implementation lifecycle state, and existing active work stays active. Existing dependencies and implementation evidence requirements still apply.

## Scope

Derived2026-09-06 by story-scoper and corrected by coordinator against the active source-fidelity stage. Every entry below is an exact typed scheduling path; generated schema3 is distinct from frozen schema2.

- `crates/protocol/src/operation.rs` — cited.
- `crates/protocol/src/operation/legacy.rs` — inferred.
- `crates/protocol/src/operation/wire.rs` — inferred.
- `crates/protocol/tests/bundles.rs` — cited.
- `contracts/connector-operation/v0alpha2` — inferred.
- `json-schemas.toml` — cited.
- `crates/service/src/egress.rs` — cited.
- `crates/server/src/egress.rs` — cited.
- `crates/server/src/local.rs` — cited.
- `crates/server/src/hosted.rs` — cited.
- `crates/server/src/hosted/mcp/toolset.rs` — cited.
- `crates/connectors-client/src/lib.rs` — cited.
- `crates/connectors-client/src/response.rs` — cited.
- `crates/connectors-console/src/envelope.rs` — cited.
- `crates/connectors-console/src/output.rs` — cited.
- `crates/connectors-cli/src/lib.rs` — cited.
- `crates/connectors-cli/tests/cli_surface.rs` — inferred.
- `crates/integration-catalog/src/lib.rs` — cited.
- `crates/integration-slack/src/backend/api_runtime.rs` — cited.
- `crates/integration-platform/src/datasource.rs` — cited.
- `crates/connector-spec/src/ir.rs` — cited.
- `crates/connector-spec/src/lib.rs` — cited.
- `crates/connector-spec/src/provider.rs` — cited.
- `crates/connector-spec/src/provider/declaration.rs` — cited.
- `crates/connector-spec/src/provider/publishing.rs` — cited.
- `crates/connector-spec/src/provider/schema_sync.rs` — cited.
- `crates/connector-spec/schema/provider-toml.schema.json` — inferred.
- `crates/connector-spec/tests/main/determinism.rs` — cited.
- `crates/connector-spec/tests/main/ir_roundtrip.rs` — cited.
- `crates/connector-spec/tests/main/service_partition.rs` — cited.
- `crates/catalog-build/src/document.rs` — cited.
- `crates/catalog-build/src/document_schema.rs` — cited.
- `crates/catalog-build/tests/main/catalog_invariants.rs` — cited.
- `crates/catalog/src/lib.rs` — cited.
- `crates/catalog/src/table.rs` — cited.
- `crates/catalog/tests/main/consumer_api.rs` — inferred.
- `providers/slack.toml` — inferred.
- `catalog/slack.catalog.json` — inferred.
- `catalog/connector-document-v3.schema.json` — cited.
- `crates/catalog-reader/catalog.pack` — inferred.
- `connectors.lock` — inferred.
- `ess/system/domains/catalog.yaml` — cited.
- `ess/system/domains/runtime.yaml` — cited.
- `crates/integration-gitlab/src/backend.rs` — cited.
- `crates/integration-jira/src/backend/operations.rs` — cited.
- `crates/integration-monitoring/src/backend.rs` — cited.
- `crates/integration-mcp/src/lib.rs` — cited.
- `crates/integration-platform/src/surface.rs` — cited.
- `crates/integration-sip/src/backend/mod.rs` — cited.
- `crates/integration-kubernetes/src/hosted.rs` — cited.
- `crates/integration-kubernetes/src/local.rs` — cited.
- `crates/integration-kubernetes/src/local_inventory.rs` — cited.
- `crates/integration-kubernetes/src/workloads.rs` — cited.
- `crates/connectors-runtime/src/registry.rs` — cited.
- `crates/connectors-runtime/src/service_bundle.rs` — cited.
- `crates/server/src/hosted/tests/enforcement.rs` — cited.
- `crates/server/src/hosted/tests/mcp.rs` — cited.
- `crates/server/src/hosted/tests/mcp_monitoring.rs` — cited.
- `crates/server/src/hosted/tests/signal.rs` — cited.
- `crates/server/src/hosted/tests/contract_validation.rs` — inferred.

High confidence for located owners and constructor closure. Catalog/spec work waits for GitLab fidelity; local transport/CLI waits for one-shot integration. Directory contract paths require containment review. Resolver request semantics are excluded; rates come from the typed catalog. Coordinator owns ESS, AEP and shared-source integration.

# Scope

Derived2026-09-06 by story-scoper and corrected by coordinator against the active source-fidelity stage. Every entry below is an exact typed scheduling path; generated schema3 is distinct from frozen schema2.

- `crates/protocol/src/operation.rs` — cited.
- `crates/protocol/src/operation/legacy.rs` — inferred.
- `crates/protocol/src/operation/wire.rs` — inferred.
- `crates/protocol/tests/bundles.rs` — cited.
- `contracts/connector-operation/v0alpha2` — inferred.
- `json-schemas.toml` — cited.
- `crates/service/src/egress.rs` — cited.
- `crates/server/src/egress.rs` — cited.
- `crates/server/src/local.rs` — cited.
- `crates/server/src/hosted.rs` — cited.
- `crates/server/src/hosted/mcp/toolset.rs` — cited.
- `crates/connectors-client/src/lib.rs` — cited.
- `crates/connectors-client/src/response.rs` — cited.
- `crates/connectors-console/src/envelope.rs` — cited.
- `crates/connectors-console/src/output.rs` — cited.
- `crates/connectors-cli/src/lib.rs` — cited.
- `crates/connectors-cli/tests/cli_surface.rs` — inferred.
- `crates/integration-catalog/src/lib.rs` — cited.
- `crates/integration-slack/src/backend/api_runtime.rs` — cited.
- `crates/integration-platform/src/datasource.rs` — cited.
- `crates/connector-spec/src/ir.rs` — cited.
- `crates/connector-spec/src/lib.rs` — cited.
- `crates/connector-spec/src/provider.rs` — cited.
- `crates/connector-spec/src/provider/declaration.rs` — cited.
- `crates/connector-spec/src/provider/publishing.rs` — cited.
- `crates/connector-spec/src/provider/schema_sync.rs` — cited.
- `crates/connector-spec/schema/provider-toml.schema.json` — inferred.
- `crates/connector-spec/tests/main/determinism.rs` — cited.
- `crates/connector-spec/tests/main/ir_roundtrip.rs` — cited.
- `crates/connector-spec/tests/main/service_partition.rs` — cited.
- `crates/catalog-build/src/document.rs` — cited.
- `crates/catalog-build/src/document_schema.rs` — cited.
- `crates/catalog-build/tests/main/catalog_invariants.rs` — cited.
- `crates/catalog/src/lib.rs` — cited.
- `crates/catalog/src/table.rs` — cited.
- `crates/catalog/tests/main/consumer_api.rs` — inferred.
- `providers/slack.toml` — inferred.
- `catalog/slack.catalog.json` — inferred.
- `catalog/connector-document-v3.schema.json` — cited.
- `catalog/catalog.pack` — inferred.
- `connectors.lock` — inferred.
- `ess/system/domains/catalog.yaml` — cited.
- `ess/system/domains/runtime.yaml` — cited.
- `crates/integration-gitlab/src/backend.rs` — cited.
- `crates/integration-jira/src/backend/operations.rs` — cited.
- `crates/integration-monitoring/src/backend.rs` — cited.
- `crates/integration-mcp/src/lib.rs` — cited.
- `crates/integration-platform/src/surface.rs` — cited.
- `crates/integration-sip/src/backend/mod.rs` — cited.
- `crates/integration-kubernetes/src/hosted.rs` — cited.
- `crates/integration-kubernetes/src/local.rs` — cited.
- `crates/integration-kubernetes/src/local_inventory.rs` — cited.
- `crates/integration-kubernetes/src/workloads.rs` — cited.
- `crates/connectors-runtime/src/registry.rs` — cited.
- `crates/connectors-runtime/src/service_bundle.rs` — cited.
- `crates/server/src/hosted/tests/enforcement.rs` — cited.
- `crates/server/src/hosted/tests/mcp.rs` — cited.
- `crates/server/src/hosted/tests/mcp_monitoring.rs` — cited.
- `crates/server/src/hosted/tests/signal.rs` — cited.
- `crates/server/src/hosted/tests/contract_validation.rs` — inferred.

High confidence for located owners and constructor closure. Catalog/spec work waits for GitLab fidelity; local transport/CLI waits for one-shot integration. Directory contract paths require containment review. Resolver request semantics are excluded; rates come from the typed catalog. Coordinator owns ESS, AEP and shared-source integration.

## Execution queue

CLI execution queue 2026-09-06: 8 of 10. The urgent Slack delivery follow-up leads the queue. Original wave-cli readiness ordering remains historical context. Dependencies and measured scope govern dispatch order; priority is not a claim that prerequisites have landed.

## Version migration decision

The source/consumer inventory establishes that structured rate_limit fields require ConnectorOperation v0alpha2. Preserve the full frozen v0alpha1 bundle byte for byte and serve both versions through explicit transport adapters, replying in the requested version. Existing v1 receives the legacy Unavailable shape without v2-only fields. No automatic invoke resend on version failure. Internal 429 parsing and existing RateLimit IR alone do not change contract identity.

A concrete proposed Atlas ADR 0039 now names Service SDK, Devcenter, Workspace, Agent Platform, Org Brain, legacy Zwirn/Platform and Website, with provider-first order and SDK-before-embedded-host ABI alignment. It is in managed tree wt-19c8158c5cae on plan/connector-operation-v2. It is proposed and does not claim architecture acceptance, consumer upgrades, release or deployment. Existing missing Atlas dependency/pin observations and v1 schema/runtime purpose/session_signal discrepancies are recorded without rewriting frozen bytes. Auth remediation is a separate trusted-surface decision and adds no speculative v2 fields.

## Complete rate-limit decision

Preserve connector-spec::RateLimit{requests:u32,per_seconds:u32,bucket:Option<String>} and its existing serialization. Add separate ConditionalRateLimit alternatives with bounded applies_when, optional PublishedRate{requests,per_seconds,basis:MinimumAllowance|Ceiling}, and source_url. Absence of a numeric rate means this source establishes none, not unlimited or zero. Runtime exposes every alternative and never interprets applicability text as policy or infers app category from credentials.

ConnectorOperation v0alpha2 adds RateLimited, optional retry_after_seconds:u64 only on that code, and optional OperationRateAdvice{fixed,alternatives}. Each alternative carries its declaration and optional suggested_interval_ms, deterministically ceil(per_seconds*1000/requests). Rates are positive, lists/text bounded, and a missing rate has no interval. This is advisory spacing, not a promised burst ceiling or automatic pacing. ESS catalog/runtime value types validated over all ten files before dispatch; exact numeric/conditional constraints remain explicitly UNMAPPED in the ESS subset and enforced by Rust/schema validators.

Current official Slack history and rate-limit pages were fetched2026-09-06: https://docs.slack.dev/reference/methods/conversations.history/ and https://docs.slack.dev/apis/web-api/rate-limits/. Publish three source-grounded history alternatives: Marketplace/internal applications under the documented cursor-pagination and method/workspace/app conditions, 50 per60seconds MinimumAllowance (1200ms advice); new commercial applications/installations outside Marketplace from2025-05-29, 1 per60seconds Ceiling (60000ms); existing external installations exempt from that new posted limit, no numeric rate established by that statement. The15-object restriction is source context, not permission to rewrite selected vendor request/response schemas.

Only retry-after is newly admitted from provider responses. A single unsigned decimal delta, including0, after trimming ASCII space/tab yields a u64 delay. Missing/empty/signed/fractional/overflowing/comma-joined/multiple values yield None. HTTP-date is explicitly unsupported in this bounded implementation. The actual server egress HeaderMap extraction must detect duplicates before flattening. Never echo raw headers or provider bodies. A definite429 is a refused operation; ambiguous mutating transport outcomes remain OutcomeUnknown. Curated Slack's blanket mutating-error conversion and audit must distinguish that definite refusal.

Atlas proposal0040 requires a new operation contract while freezing all v0alpha1 bundle bytes. Snapshot deployed legacy DTOs, strictly decode each supported version at local/hosted boundaries, convert through the same admission/backend owner, and reply in the requested version. V1 receives Unavailable with no v2 delay/advice. Preserve and separately test already-deployed purpose/session_signal behavior; it remains distinct from conformity to the frozen v1 schema. No automatic invoke resend or silent version fallback. New v2 includes complete request/response schema and positive/adversarial vectors; client/CLI/MCP preserve optional delay, retriable and nonzero refusal status. Datasource keeps its current representation.

Stage protocol/egress work first. Catalog/spec/typed-reader/schema files remain owned by the GitLab fidelity stage until coordinator handoff; its schema3 may incorporate the proposed conditional metadata before that migration freezes, otherwise version it separately. Local transport and CLI source receive one-shot integration before their stage. Do not borrow the failed multi-credential unit, reserve auth fields, change consumer pins or deploy/release under this unit.

## Stage1 generator and lock scope

Stage1 inspection found no existing ConnectorOperation bundle generator. Add a deterministic Rust example `crates/protocol/examples/operation_v2_bundle.rs` with clap-derived explicit check/write modes, schema projection `crates/protocol/src/operation/schema.rs`, and protocol manifest dev-dependencies on the already-pinned workspace clap/jsonschema versions. These paths and all twelve gate workspace lockfiles are recorded before edits. Refresh lock metadata offline after the manifest change and report exact changed lock rows; do not update dependency versions. Both schema and Rust readers must independently exercise complete v2 request/response positive and adversarial vectors. Frozen v1 files remain byte-identical. This extends stage1 tooling ownership, not any held runtime/catalog/CLI owner.

## First complete adversary result and bounded repair assignment

Immutable review-result:cli-rate-adversary-1-20260906 records the complete first pass against90735eac. The twelve retained deciding cases are committed incc143696f02e082ee64158907e562cad629cffea. All prior tests/source were preserved and the full frozen evidence manifest verified. Complete affected suites measured root1205 pass/2 new failures/4 existing ignored, runtime346 pass/2 existing ignored, CLI130 pass, console87 pass; all strict Clippy/format checks passed.

The two introduced contract warnings share a URL-validation mismatch: uppercase HTTPS and literal path space pass the Rust wire/authoring readers but fail the schemas; port65536 passes the schemas but fails Rust. The current shipped Slack documents remain valid. Actual client/authoring caller reachability and its limits are in the immutable report. This is not evidence of a deployed malformed provider or credential effect.

The original implementor owns a bounded repair fromcc143696. Establish one explicit URL spelling/range policy consistently across the existing wire reader/schema, authoring reader/provider-schema and canonical schema3 generator; preserve supported positive controls including ordinary lowercase HTTPS and valid explicit ports. Address the grammar classes rather than special-casing the three strings. Preserve all first-pass cases and all frozen v1/schema2/vendor bytes. Regenerate affected new v2/schema3 artifacts only through their owners and retain deterministic fixed-point, conformance and schema-fidelity evidence. No retry, grant, protocol-version retirement or runtime behavior expansion belongs to this repair.

Current exact source owners already cover protocol operation wire/schema and bundle example/tests, connector-spec rate_limit/provider schema generation, catalog-build document_schema and schema invariants, new contract bundle and schema3 artifact. If a shared helper or dependency requires another path, request it before writing. ESS/AEP/Git/integration/operator actions stay coordinator-owned. Run the two retained failed cases first on the assigned base, then implement and rerun all affected suites, strict checks and regeneration gates. The second adversary pass is the final ordinary pass under the approved wave. No successful test_result or fixed review outcome is recorded before measured repair evidence exists.

## URL parser and manifest closure for the measured repair

Both retained URL cases were reproduced oncc143696 before edits, one failure each. The implementor identifies the normalization difference between WHATWG URL parsing and the RFC3986 URI parser used by the published schema validator. Use the already locked fluent-uri0.4.1 as a direct reader dependency so both sides interpret URI syntax consistently. Add the root Cargo.toml workspace dependency and the already scoped protocol/connector-spec manifest edges; connector-spec's now-unused url edge may be removed after verifying no other owner uses it. Preserve every existing dependency version/checksum and close all twelve metadata graphs through Cargo.

The explicit citation spelling policy is literal lowercase https, ASCII RFC3986 URI, nonempty host, absent userinfo/fragment, and optional decimal port within u16. Preserve valid explicit443, leading zero spellings and an empty port where RFC3986 permits it; percent-encoded path/query text is retained exactly. Rust must not normalize a spelling that the schema rejects. No new helper file is assigned: existing rate_limit/wire/schema owners can define the property and validation helpers. The canonical schema3 property uses its owning spec helper; the existing hand-authored provider schema retains its ownership model with an explicit synchronization assertion. Additional tests should compare the grammar classes and all three schema projections, preserving the two adversary assertions and prior positive cases.

## URI contract repair handed to final review — 2026-09-06

Frozen repair2f89a6fccf9878ed8275565d68667cbd1fc08d43 fixes the two retained first-pass URL mismatches using the pinned RFC URI parser and a shared explicit lowercase-HTTPS host/port/encoding profile. The coordinator verified24scoped source/artifact hashes and122frozen evidence entries, complete raw/portable report equality apart from the home-directory prefix, and exact bot author/committer. All original adversary test bytes remain preserved, with only two appended grammar groups.

Full affected suites measured1770→1772 executed: root1209, runtime346, CLI130 and console87 nowpass, with6existingignored. Strict Clippy/formatting, all12locked/offline dependency graphs,85conformancevectors, canonical/provider/wire grammar equality, repeated deterministic generation and whole-stage fidelity pass. These are actual four-workspace results, not a full twelve-workspace repository gate or final independent verdict. No release, external consumer upgrade or source publication is claimed.

Complete immutable handoff is in `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol/stage2-repair-*`. Raw report SHA256eeb4522f474113e47b75749a9893c34685afc80217ed2b6e5444aac36c872e54; portable report SHA256b87e68a1b80f2580839238603877363ec99a08b70357c85dfd313d04bae95fc6; evidence manifest SHA25682ac97a1f2147542558ad39bf13e790d9f7cbadacb30d43b78be02b94f9edad9; source patch SHA2561589d34a9fc3d390cc101e754512a6c3d2d31cb232ab9821982e8454e4c6d622. Every existing version/source/checksum is preserved across the10changed locks; the driver-sql closure adds only two already-pinned root packages.

The second/final ordinary adversary pass is assigned against the complete frozen unit in wt-af054beacfba. At most seven additive parameterized attacks use the existing12test owners; preserve all source/earlier cases and report actual caller reachability/origin. Full four-workspace ordinary-parallel suites and strict checks follow the deciding cases. Completed exact owned targets may be cleaned only after outside-evidence/source verification to preserve the20GiBfloor. No third ordinary attack or production repair by the adversary is authorized if the final pass is red. Complete schema3 publication and full integration gates remain pending; shared runtime owners are not released yet.

## Publication guidance assignment — 2026-09-06

The read-only schema 3 preflight identifies missing public explanation of the completed rate source. Assign CHANGELOG.md, crates/protocol/README.md, docs/architecture/interfaces.md and docs/design/04-the-callers-contract.md solely for concise publication guidance grounded in frozen source 2f89a6fccf9878ed8275565d68667cbd1fc08d43 and Atlas proposed migration records 0040/0041. The implementor may edit these four paths in the coordinator's managed integration checkout; no source, generated output, planning, manifest, lock, release identity, provider data or remote write is assigned. The adversary's separate rate checkout remains untouched.

Explain Operation v2 and strict v1 compatibility, advisory rate metadata, definite 429 and trusted delay, no automatic resend, and schema 3's required reader/producer migration with preserved source semantics. Keep the unreleased entry under Unreleased. Mark proposed external migration and source-versus-release/install status accurately; do not claim OAuth schema 4, Operation v3, consumer adoption or a completed final review. Preserve the design's existing text and append its dated conditional-metadata note. Validate links/text without a new Cargo build, freeze a four-path patch and short report, then return source to the coordinator for the accepted schema 3 assembly.

## Final rate adversary green — 2026-09-06

Immutable review-result cli-rate-adversary-2-20260906 retains the complete final independent report. Frozen repair source 2f89a6fccf9878ed8275565d68667cbd1fc08d43 plus seven additive cases is committed as 0c026610e67f3ec6da0a8014813f3d88c4e72799. The coordinator verified all 108 evidence entries, exact seven-path source hashes, insertion-only preservation, exact source patch and complete portable report equality except for the home prefix. Report SHA256 193e58ed85dec84579a383c7f239f6fba8b952f7454333946495233d59aee4d6; patch SHA256 074a14c8b34081cb700cd47155d6cf416f9b14da6d7b1b3847cda0ddc0271eef; evidence manifest SHA256 92f92a514d37d211bab07312162facb30b9c6e3e3a80e2422f51a65577d8c395.

The final ordinary parallel affected suites passed root 1214, runtime 347, CLI 131 and console 87: 1779 total, zero failures and six unchanged ignored cases. Strict Clippy/formatting pass in every lane. The seven independent new cases and two retained URL countercases each executed exactly once in their deciding selections and passed. Findings are empty; no third pass is needed. The initial zero-test selector miss and shared-disk floor incident remain explicitly recorded in the immutable report. Whole canonical/vendor fidelity, schema 2 and v1 byte preservation, all old case names and existing assertions remain verified.

The source is now released for the coordinator's local integration and downstream implementation. This acceptance does not publish schema 3 or complete the twelve-workspace gate. Assemble the exact published Git framing source, selected planning evidence, current deterministic ESS projection and bounded public guidance without the uncalled private OAuth kernel; run the complete repository and publication gates before source publication. Schema 4 writer publication remains later. Full-wave integration, installation and external consumer adoption remain separate outcomes.

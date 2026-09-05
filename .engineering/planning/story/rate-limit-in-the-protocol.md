---
format: aep.planning-md/1
id: story:rate-limit-in-the-protocol
kind: story
status: proposed
title: A provider's rate limit is a protocol fact, not a sentence
tags:
- ready
- wave-cli
scope:
- confidence: inferred
  path: catalog/slack.catalog.json
- confidence: inferred
  path: connectors.lock
- confidence: inferred
  path: contracts/connector-operation/v0alpha2
- confidence: cited
  path: crates/catalog-build/tests/main/catalog_invariants.rs
- confidence: inferred
  path: crates/catalog/src/lib.rs
- confidence: inferred
  path: crates/catalog/src/table.rs
- confidence: inferred
  path: crates/client/src/lib.rs
- confidence: inferred
  path: crates/client/src/response.rs
- confidence: inferred
  path: crates/connector-resolve/src/document.rs
- confidence: cited
  path: crates/connector-spec/src/ir.rs
- confidence: cited
  path: crates/connectors-cli/src/lib.rs
- confidence: cited
  path: crates/connectors-console/src/envelope.rs
- confidence: cited
  path: crates/connectors-console/src/output.rs
- confidence: cited
  path: crates/connectors-runtime/src/registry.rs
- confidence: cited
  path: crates/connectors-runtime/src/service_bundle.rs
- confidence: cited
  path: crates/integration-catalog/src/lib.rs
- confidence: cited
  path: crates/integration-gitlab/src/backend.rs
- confidence: cited
  path: crates/integration-jira/src/backend/operations.rs
- confidence: cited
  path: crates/integration-kubernetes/src
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
- confidence: cited
  path: crates/integration-slack/src/backend/api_runtime.rs
- confidence: cited
  path: crates/protocol/src/operation.rs
- confidence: inferred
  path: crates/protocol/src/operation/legacy.rs
- confidence: cited
  path: crates/protocol/tests/bundles.rs
- confidence: cited
  path: crates/server/src/hosted.rs
- confidence: inferred
  path: crates/server/src/hosted/mcp/toolset.rs
- confidence: cited
  path: crates/server/src/hosted/tests
- confidence: inferred
  path: crates/server/src/local.rs
- confidence: inferred
  path: ess/system/domains/catalog.yaml
- confidence: inferred
  path: json-schemas.toml
- confidence: inferred
  path: providers/slack.toml
revision: 56
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
- A 429 with `Retry-After: 30` is returned as `rate_limited` with `retry_after_seconds: 30`.
- `slack-conversations-history` `describe` names the interval Slack's tier implies.

## Readiness

Selected for implementation on 2026-09-06 at the operator's request. wave-cli: 9 of 10. The ready tag records selection; proposed is the pre-implementation lifecycle state, and existing active work stays active. Existing dependencies and implementation evidence requirements still apply.

## Scope

Derived 2026-09-06 by aep-drive story-scoper; coordinator records the returned surfaces.

- `crates/protocol/src/operation.rs` — cited.
- `contracts/connector-operation/v0alpha1` — cited.
- `crates/protocol/tests/bundles.rs` — cited.
- `crates/integration-catalog/src/lib.rs` — cited.
- `crates/integration-slack/src/backend/api_runtime.rs` — cited.
- `crates/connector-spec/src/ir.rs` — cited.
- `providers/slack.toml` — inferred.
- `catalog/slack.catalog.json` — inferred.
- `connectors.lock` — inferred.
- `crates/catalog/src/lib.rs` — inferred.
- `crates/catalog/src/table.rs` — inferred.
- `crates/connector-resolve/src/document.rs` — inferred.
- `crates/catalog-build/tests/main/catalog_invariants.rs` — cited.
- `crates/connectors-runtime/src/registry.rs` — cited.
- `crates/connectors-console/src/envelope.rs` — cited.
- `crates/connectors-console/src/output.rs` — cited.
- `crates/connectors-cli/src/lib.rs` — cited.
- `crates/server/src/hosted/mcp/toolset.rs` — inferred.
- `crates/integration-platform/src/datasource.rs` — cited.
- `crates/server/src/hosted.rs` — cited.
- `crates/server/src/hosted/tests` — cited.
- `crates/connectors-runtime/src/service_bundle.rs` — cited.
- `crates/integration-monitoring/src/backend.rs` — cited.
- `crates/integration-platform/src/surface.rs` — cited.
- `crates/integration-jira/src/backend/operations.rs` — cited.
- `crates/integration-sip/src/backend/mod.rs` — cited.
- `crates/integration-kubernetes/src` — cited.
- `crates/integration-mcp/src/lib.rs` — cited.
- `crates/integration-gitlab/src/backend.rs` — cited.
- `ess/system/domains/catalog.yaml` — inferred.

Medium confidence. Existing RateLimit IR/build metadata should be projected, not recreated. Admit only retry-after response headers. Preserve optional retry delay through protocol, CLI and MCP; account for all description constructors and the exhaustive datasource error mapping. Do not claim one universal Slack history quota: research app-category rules. Keep ambiguous write outcomes truthful. Frozen v0alpha1 bytes require a new contract version and coordinated migration before changes.

Would collide with any unit editing these files; directory entries require an additional containment review because AEP compares scope strings exactly.

## Execution queue

CLI execution queue 2026-09-06: 8 of 10. The urgent Slack delivery follow-up leads the queue. Original wave-cli readiness ordering remains historical context. Dependencies and measured scope govern dispatch order; priority is not a claim that prerequisites have landed.

## Version migration decision

The source/consumer inventory establishes that structured rate_limit fields require ConnectorOperation v0alpha2. Preserve the full frozen v0alpha1 bundle byte for byte and serve both versions through explicit transport adapters, replying in the requested version. Existing v1 receives the legacy Unavailable shape without v2-only fields. No automatic invoke resend on version failure. Internal 429 parsing and existing RateLimit IR alone do not change contract identity.

A concrete proposed Atlas ADR 0039 now names Service SDK, Devcenter, Workspace, Agent Platform, Org Brain, legacy Zwirn/Platform and Website, with provider-first order and SDK-before-embedded-host ABI alignment. It is in managed tree wt-19c8158c5cae on plan/connector-operation-v2. It is proposed and does not claim architecture acceptance, consumer upgrades, release or deployment. Existing missing Atlas dependency/pin observations and v1 schema/runtime purpose/session_signal discrepancies are recorded without rewriting frozen bytes. Auth remediation is a separate trusted-surface decision and adds no speculative v2 fields.

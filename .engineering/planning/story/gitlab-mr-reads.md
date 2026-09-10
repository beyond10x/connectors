---
format: aep.planning-md/1
id: story:gitlab-mr-reads
kind: story
status: active
title: Read GitLab merge requests and bounded update windows
relations:
- decomposes: initiative:complete-local-connectors
- informed_by: story:gitlab-ci-runtime
- informed_by: story:persistent-gitlab-journey
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: README.md
- confidence: inferred
  path: adapters/gitlab
- confidence: inferred
  path: crates/connectors-host/tests/local_foundation.rs
- confidence: inferred
  path: crates/connectors-spec
- confidence: inferred
  path: docs
- confidence: inferred
  path: spec-kinds/adapter/v2
- confidence: inferred
  path: website
revision: 10
---
## Outcome

From the existing eight-read GitLab CLI, add generated merge_request.get and merge_requests.list operations so a saved, admitted connection can inspect an MR and traverse a fixed inclusive update window. Each successful result is a bounded native observation with truthful traversal completeness; it grants no merge or write authority.

## Model and source

adapters/gitlab/spec/ess/domains/merge_requests.yaml declares the native Observation value, validated by the repository-pinned ESS 0.20.0 before this story. adapters/gitlab/contracts/merge-requests/v1alpha1/semantics.md owns selected identities, nullable fields, bounded text/SHA/timestamps, update windows, cursor context and asynchronous status interpretation. The unchanged upstream OpenAPI reverse-imported successfully through aep plan reverse openapi; its full-provider ownership/lifecycle UNMAPPED decisions are excluded by selecting a scalar read value, not inventing an entity graph. The upstream list-shape discrepancy remains a native finish obligation.

## Implementation and dependencies

Use authored adapter v2 GET mappings and generated requests/descriptor/runtime obligations. Native code validates calendar bounds, response identity, item shape, filter/window/order and continuations. Local executable composition adds two explicitly read-scoped requirements; the business adapter receives only the existing authenticated HTTP capability. No new credential or database access. Existing persistent journey and CI stories own their respective lifecycle and CI implementations and outstanding sandbox evidence; this story reuses those mechanisms and preserves their eight operations.

Single implementation writer works directly on local main. Shared adapter.json, generated output, GitLab library/composition/tests, source docs and website publication overlap the CI story; implementation changes are serialized after its committed local CI checkpoint c22ddfe, not dispatched concurrently. CI/persistence stories remain active for their distinct missing sandbox evidence. Builds, packaging and CLI journeys that share selected executable paths are serialized. Critics are read-only; this agent alone writes AEP.

## Acceptance

A caller using an admitted saved GitLab connection can move a fixed update window from uncollected to observed traversal exhaustion by following the bounded MR pages until next_cursor is null and complete is true, and retrieve each selected MR by project-local iid as a bounded native observation.

## Verification prerequisites

Prove that transition in both the disposable production CLI journey and a dedicated GitLab sandbox, including nullable deleted-source/head fields and unknown merge status. All selected fields obey the native contract and preserve current permission, credential reuse and exact process ownership. Failure cases cover malformed identities/calendar/response shapes, oversized data, state/window/order/duplicate violations, nonadvancing/absent continuations, cursor selection/connection partition mismatch, schema/project/policy refusal before provider work and safe unavailable/permission/not-found failures. A failure never publishes a complete page or advances a consumer checkpoint. Mutable offset collection is explicitly not a snapshot or lossless change feed.

Run native ESS validation/compilation, adapter generation and drift, conformance and boundaries, required repository gate including Rust 1.88, production CLI tests, affected docs/website checks and local packaging. Retain commands/results/source/executable/artifact identities; timestamped receipts are separate from deterministic payloads. Dedicated sandbox evidence is mandatory and cannot be replaced by fixtures. credential-blocker:gitlab-runtime-sandbox withholds that evidence.

## Scope boundaries and next work

This claims only the MR read portion of C14 and GitLab MR collection portion of C21. It does not close either acceptance scenario or the GitLab phase. Native validation, exact-head MR create/update/merge and the shared approval/audit/attempt/dispatch/idempotency binding remain ordered work under initiative:complete-local-connectors; model and review their missing semantics before decomposition. Other GitLab changed-record collection remains with that initiative. Full GitLab precedes Kubernetes, then PostgreSQL, then MCP and remaining providers. No write advertisement, implicit retries, native OAuth onboarding, external Connectors publication, Atlas registration or Harness upgrade.

## Discovered generation seam

The first MR generation run refused the source state enum until the authored constraint used the complete native enum in source order, including locked; the conservative importer compares exact constraint values, not enum sets. The next refusal identifies source format:date-time as unsupported. Extend only the existing scalar RequestMapping binding: preserve the known string date-time annotation with an explicit identical caller annotation, reject other formats/non-string formats and constrained prepare bindings, and validate formatted constants explicitly. Generic generated inputs remain strings; the native prepare obligation performs calendar/window validation before transport. No new entity, credential capability or provider vocabulary enters shared code. Native read fixtures exercise invalid dates before any HTTP call. Source bytes remain unchanged.

Affected scope additionally includes crates/connectors-spec and spec-kinds/adapter/v2. Add importer tests for preserved format constraints and refusal of unknown, missing/mismatched or binding formats; update the existing synthetic source fixture to type the MR iid correctly. This is the bounded additive generator prerequisite discovered during the reviewed read implementation, not a write or timestamp transport capability. Four critics approved the decomposition in final round 2; their review records cover MR story revision 5. The initial dispatch incorrectly called that revision 7 because evidence records were assumed to increment body revisions; reviewers checked actual revision 5 bytes. Both acceptance findings were fixed with separate outcomes. The native state filter now also includes locked, as the pinned source requires.

## Gate-discovered contention verification

The first full MR gate passed native tests but failed the existing 16-round four-caller setup test: one success, two MetadataUnavailable responses and one ConfigurationExists. The owning metadata implementation already uses a two-second exclusive lifecycle-lock deadline (crates/connectors-host/src/local/metadata.rs:lifecycle_lock), covering fsync/migrations and last-close sidecar retirement; a contending setup may legitimately exhaust that bound. The existing test required every loser to return ConfigurationExists regardless of storage/scheduling delay, and recorded no elapsed times.

Keep the bounded runtime unchanged. Strengthen the test's observation: retain per-caller elapsed time, accept MetadataUnavailable only after the existing two-second bound, and require every loser to converge to ConfigurationExists on a subsequent explicit call once the successful setup has completed; final metadata inspection remains mandatory. Add a deterministic held-metadata-lock case proving bounded refusal and recovery after release. An early MetadataUnavailable still fails. This verifies the actual bounded availability contract without raising the timeout, retrying business effects or treating an unexplained early refusal as contention. The original failure is retained; its exact internal failure stage was not instrumented, so attribution to lock expiry remains an inference from the observed outcomes and existing code.

## Local MR runtime checkpoint — 2026-09-10

The two generated MR reads now run through the saved-connection CLI and narrow authenticated HTTP capability. Six native MR tests pass; five production CLI journeys, including MR update-window exhaustion and restart reuse, pass in 225.97 seconds. The final required gate passes with Rust 1.88, ESS, generation/conformance, workspace tests, Clippy and library boundaries. Website build/typecheck/reference checks and local Linux image packaging pass. docs/evidence/gitlab-mr-20260910/README.md retains commands, failures, final results, 463 source hashes and distinct tested/package executable identities.

An initial gate failure exposed a test assumption about concurrent setup's existing two-second metadata-lock bound. Runtime code is unchanged. The revised test still refuses early unavailability and requires every loser to converge after the winner completes; a deterministic held-lock refusal/recovery case and the full gate pass. The original failure and unconfirmed internal-stage attribution remain explicit in evidence.

The local image build is not container business-runtime or dedicated GitLab sandbox evidence. The credential blocker remains open, this story remains active, and C14/C21 and the full GitLab phase are not complete. Next comes separately modeled native validation/guarded writes and the shared mutation foundation, before Kubernetes and PostgreSQL. No external publication or deployment occurred.

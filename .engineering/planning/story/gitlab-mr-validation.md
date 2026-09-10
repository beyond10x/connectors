---
format: aep.planning-md/1
id: story:gitlab-mr-validation
kind: story
status: active
title: Validate a pinned GitLab MR head and selected pipeline through the CLI
relations:
- decomposes: initiative:complete-local-connectors
- informed_by: story:gitlab-mr-reads
- informed_by: story:gitlab-ci-runtime
- informed_by: story:persistent-gitlab-journey
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: adapters/gitlab
- confidence: inferred
  path: crates/connectors-conformance
- confidence: inferred
  path: crates/connectors-spec
- confidence: inferred
  path: docs
- confidence: inferred
  path: website
revision: 6
---
## Outcome

Expose native pinned-head MR validation through the existing generated operation-invocation CLI, as an independent read prerequisite for C14. Do not enable a provider write or treat a passing read as an approval.

## Acceptance

After a restarted CLI using a saved connection reports checks_passed=true for an opened, non-draft, mergeable MR with the requested exact SHA and successful selected head pipeline, changing that MR head makes the same pinned validation request report checks_passed=false with head_changed and merge_performed=false.

## Model and ownership

adapters/gitlab/contracts/merge-requests/v1alpha1/validation.md selects the input, bounded output, ordered blockers and conservative read boundary. adapters/gitlab/spec/ess/domains/merge_requests.yaml declares ValidationInput, PipelineCheck and Validation alongside the existing Observation, with no persistent entity or new relation. The pinned ESS validator reports `connectors_gitlab v1 — 4 file(s), valid` before this decomposition. The unchanged vendor OpenAPI declares head_pipeline at line 88458. Generated mappings and descriptor own the typed public operation; native codecs own identity, status and SHA checks. Shared contracts, host credential/mutation/policy code and public CLI grammar need no new provider semantics.

This is a partial C14 validation observation, not an exhaustive independent audit of project rules or a stable future-merge receipt. Exact selected successful source-SHA pipelines are required; merged-result/train SHA equivalence is not inferred. Unknown asynchronous merge statuses refuse a passing result. Later mutation admission must repeat current checks and use the native atomic SHA guard. Dedicated GitLab sandbox evidence is required before this story is implemented.

## Verification

Cover each fixed blocker and combinations, unknown status, missing/null and malformed pipeline, wrong identity/project and bounds, invalid input and allowlist/permission refusal before HTTP, safe provider failures, exact generated GET with no extra query, and output-schema agreement. Prove passing and negative observations through private HTTPS/qualified keyring fixtures using the production CLI after restart and credential-file removal. Reuse the existing eleven-key tests only if their inputs are unchanged; run the affected ESS, generation/conformance and full Rust 1.88 gate, all existing GitLab CLI journeys, website/reference checks and local GitLab packaging. Retain commands, failures, source/artifact identities and limitations under docs/evidence/gitlab-mr-validation-20260910. Fixtures cannot satisfy the dedicated sandbox blocker or complete full C14.

## Scope and sequencing

One implementation and AEP writer works directly on primary main, serializing shared files, generated outputs and two-job Cargo builds. Native adapter source/spec/tests, generated GitLab bundle, support documentation, website/reference selection, conformance inventories and generation tooling tests are the affected surfaces. Earlier MR reads and persistent lifecycle supply existing runtime foundations, but their dedicated sandbox blockers do not prevent independent local validation implementation. Four read-only planning critics review this addition against the parent and sibling ownership; no parallel implementation is scheduled.

The initiative retains actual approval preparation/issuance, authenticated subject policy, production clock qualification, generated mutation ingress and full dispatch composition, native writes and dedicated provider acceptance. Existing clock research has not qualified a source and no SystemTime shortcut is admitted. C14 create/update head-guard choice is still unanswered. Full GitLab precedes Kubernetes, PostgreSQL, MCP and remaining providers, with original packaging, compatibility, reproducibility and publication boundaries intact.

## Local validation checkpoint — 2026-09-10

The generated merge_request.validate operation is implemented as an explicitly classified native read through the production CLI. Twelve native MR tests and the complete Rust 1.88 gate pass. Six optimized generic-CLI/private HTTPS/keyring journeys pass in 92.79 seconds, including the final GET-only assertion, saved connection restart, passing-to-changed-head validation, missing checks and current permission/schema/project refusal. Native ESS has 17 declarations; shared ESS remains at 415. No shared runtime, credential, mutation or dependency input changed.

Local image packaging, website typecheck/build/public audit, reference drift and all fifteen example tests pass. docs/evidence/gitlab-mr-validation-20260910/README.md records exact source/tool/binary/image identities, failure corrections and limits. The final site indexes 117 pages and audits 477 public files. Four final independent planning critics approve after the two acceptance findings were fixed with separate outcomes. AEP output remains valid with the known empty-findings notices.

This story remains active: dedicated GitLab sandbox evidence has not been supplied. A checks_passed result is a bounded mutable observation and always reports merge_performed=false; it is not a complete C14 result, issuance proof, write grant or stable future-check receipt. The parent still owns local approval issuance, authenticated subject policy, qualified time, full mutation dispatch, native writes and the unanswered create/update head-guard choice, before the remaining provider phases. Source is integrated locally; publication/deployment/Atlas registration remain outside this increment.

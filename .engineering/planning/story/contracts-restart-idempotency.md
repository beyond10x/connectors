---
format: aep.planning-md/1
id: story:contracts-restart-idempotency
kind: story
status: implemented
title: State exact idempotency guarantees for lifecycle operations
tags:
- P2
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
- depends_on: story:contracts-idempotency-scope
scope:
- confidence: cited
  path: contracts/operations/v1alpha1/semantics.md
- confidence: cited
  path: docs/adapters/docker.md
- confidence: cited
  path: docs/adapters/kubernetes.md
- confidence: inferred
  path: docs/evidence/restart-visibility-20260908
- confidence: inferred
  path: ess/domains/mutations.yaml
revision: 8
---
## Context

Priority: **P2**. Sources: `F11`, `E08` in `specification:contract-review-intake-20260908`.

Docker restart is both natural and none; a generated Kubernetes annotation timestamp alone does not establish natural idempotency.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Define none/natural/keyed by exact repeat guarantees. Preserve Docker id/name selection with a required stable identity binding and bounded preflight; select per-operation result/no-op semantics and fixed parameters. Preserve Kubernetes namespace/name/UID/resourceVersion, fix one generated marker per prepared candidate, select host-keyed replay, distinguish new intent and earlier lost replies, and pin provider evidence without claiming universal 409/422 no-effect proof.

## Acceptance

Every selected Docker lifecycle and Kubernetes restart operation has one complete target/intent/idempotency/result contract whose repeated, lost-response and changed-precondition cases agree across the shared and adapter documents.

## Verification scenarios

Docker desired-state no-op and restart repeat; ID/name replacement and configured name/label admission; fixed signal/timeout; lost reply versus fresh invocation. Kubernetes original UID/RV, fixed generated marker, accepted patch versus rollout completion, exact key replay/conflict, stale original version after loss, new preconditions/replaced object, no hidden refetch or automatic resend. Generic none/natural/keyed claims and approval/ledger ordering. Pinned official evidence, minimal ESS values, shape negatives and accepted semantic counterexamples, textual scenarios, full existing gate and two independent reviews.

## Scope

- cited: `contracts/operations/v1alpha1/semantics.md`
- cited: `docs/adapters/kubernetes.md`
- cited: `docs/adapters/docker.md`

Source locations: `contracts/operations/v1alpha1/semantics.md:54`; `docs/adapters/kubernetes.md:55`; `docs/adapters/docker.md:25`; `docs/adapters/docker.md:47`; `../connectors/crates/integration-kubernetes/src/local_workloads.rs:296`.

- inferred: `ess/domains/mutations.yaml`
- inferred: `docs/evidence/restart-visibility-20260908`

## Dependencies and edit coordination

Semantic prerequisites: `story:contracts-idempotency-scope`.

Shared edit surfaces with `story:contracts-mutation-outcomes`, `story:contracts-idempotency-scope`, `story:contracts-federated-approval`, `story:contracts-credential-evidence`, `story:contracts-wire-compatibility`, `story:contracts-permission-budgets`, `story:contracts-mutation-classification`, `story:contracts-discovery-coverage`, `story:contracts-discovery-profiles`, `story:contracts-acquisition-profiles`, `story:contracts-evidence-precision`, `story:contracts-persistence-ownership`, `story:contracts-mutation-visibility`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

The operator authorized local semantic hardening together with ESS models and local checkpoints. Root is the sole tracked-file editor in the primary checkout under the repository single-agent rule; two independent reviewers use isolated ignored evidence. This story edits normative contracts, minimal private values and review evidence, with concrete unsupported predicates/relations kept UNMAPPED. No runtime, public codec, adapter-kind schema or additional adapter implementation is added. Work and recovery copies stay local. The original text-only/no-ESS/no-commit boundary is superseded by the authorized specification goal. Shared files are edited serially by root; restart and visibility keep separate finding ownership.

## Reviewed completion — 2026-09-08

Operations §5.2 and Docker/Kubernetes §4.1 now select complete repeat/target/intent/acknowledgement rules. Docker natural start/stop and none restart bind the exact daemon/full ID, one admitted bounded inspect and fixed signal/wait without synchronized state or historical replay claims. Kubernetes selects host-keyed observation plus original UID and canonical positive conditional resourceVersion, fixes one epoch-ms marker/body per candidate, never refetches/rebases, and returns accepted PATCH without rollout or lost-response reconciliation claims. Nine individual findings (seven initial plus RV-A-02/RV-B-01) are fixed; F11/E08 close in the original ledger.

Both final independent rechecks approve this story with zero residuals: review-result:restart-visibility-a-recheck2-20260908 and review-result:restart-visibility-b-recheck2-20260908. Exact reports, frozen sources and individual outcomes are preserved in docs/evidence/restart-visibility-20260908/dispositions.md and reviews/.

Verification: ESS 0.20.0 validates 13 files / 215 declarations; two 222-artifact projections match and seven private values have unchanged generated copies. Forty-six shape expectations include nine rejected shapes and nineteen accepted semantic counterexamples; forty-eight textual cases and eight exact provider archives distinguish selected behavior from execution. Existing full gate/MSRV1.88 passes 50 Rust tests and compiles 222 scenarios/34 authored; separate sessions compile 13/201. The narrow final correction changes an ESS comment only and compiles byte-identical IR, so the existing runtime gate was retained. No new runtime, public codec or adapter-kind schema implementation; provider/policy/storage predicates remain explicit later binding obligations. Root edited tracked files serially; work and checkpoints stay local.

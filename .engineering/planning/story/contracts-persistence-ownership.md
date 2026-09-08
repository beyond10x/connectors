---
format: aep.planning-md/1
id: story:contracts-persistence-ownership
kind: story
status: implemented
title: Inventory host persistence ports and atomicity owners
tags:
- P2
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
- depends_on: story:contracts-mutation-outcomes
- depends_on: story:contracts-idempotency-scope
- depends_on: story:contracts-refresh-coordination
- depends_on: story:contracts-management-boundary
- depends_on: story:contracts-discovery-coverage
- depends_on: story:contracts-federated-approval
scope:
- confidence: cited
  path: contracts/auth/acquisition/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/connection/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/custody/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/evidence/v1alpha1/semantics.md
- confidence: inferred
  path: contracts/auth/management.md
- confidence: cited
  path: contracts/discovery/mediated_route/v1alpha1/semantics.md
- confidence: cited
  path: contracts/discovery/resources/v1alpha1/semantics.md
- confidence: cited
  path: contracts/operations/v1alpha1/semantics.md
- confidence: inferred
  path: contracts/service/compatibility.md
- confidence: inferred
  path: contracts/service/delegation.md
- confidence: inferred
  path: contracts/sessions/v1alpha1/semantics.md
- confidence: cited
  path: docs/design.md
- confidence: inferred
  path: docs/evidence/profile-persistence-20260908
- confidence: inferred
  path: ess/domains/credentials.yaml
- confidence: inferred
  path: ess/domains/discovery.yaml
- confidence: inferred
  path: ess/domains/refresh.yaml
revision: 17
---
## Context

Priority: **P2**. Sources: `E28` in `specification:contract-review-intake-20260908`.

Several host stores are named in separate obligations without one inspectable ownership and atomicity inventory.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Add a narrow persistence ownership section to the existing design document and cross-link contract obligations; cover every cited port, state owner, required atomic operations and failure handoff without creating a universal store abstraction. Refresh and mutation stories own their semantics; this story consolidates them.

## Acceptance

After revision, every cited persistent obligation maps to exactly one named port owner and its required atomicity in the inventory.

## Verification scenarios

- Inventory includes connection metadata, acquisition/refresh, mutation/approval/idempotency, discovery/routes, evidence and custody.
- Immutable secret writes, active-reference CAS, exclusion and one-shot spending remain distinct requirements.
- No cross-service transaction is promised without an explicit supported binding.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/auth/acquisition/v1alpha1/semantics.md`
- cited: `contracts/auth/connection/v1alpha1/semantics.md`
- cited: `contracts/auth/custody/v1alpha1/semantics.md`
- cited: `contracts/auth/evidence/v1alpha1/semantics.md`
- inferred: `contracts/auth/management.md`
- cited: `contracts/discovery/mediated_route/v1alpha1/semantics.md`
- cited: `contracts/discovery/resources/v1alpha1/semantics.md`
- cited: `contracts/operations/v1alpha1/semantics.md`
- inferred: `contracts/service/compatibility.md`
- inferred: `contracts/service/delegation.md`
- inferred: `contracts/sessions/v1alpha1/semantics.md`
- cited: `docs/design.md`
- inferred: `docs/evidence/profile-persistence-20260908`
- inferred: `ess/domains/credentials.yaml`
- inferred: `ess/domains/discovery.yaml`
- inferred: `ess/domains/refresh.yaml`

## Dependencies and edit coordination

Semantic prerequisites: `story:contracts-mutation-outcomes`, `story:contracts-idempotency-scope`, `story:contracts-refresh-coordination`, `story:contracts-management-boundary`, `story:contracts-discovery-coverage`, `story:contracts-federated-approval`.

Shared edit surfaces with `story:contracts-mutation-outcomes`, `story:contracts-idempotency-scope`, `story:contracts-federated-approval`, `story:contracts-refresh-coordination`, `story:contracts-credential-evidence`, `story:contracts-wire-compatibility`, `story:contracts-connection-readiness`, `story:contracts-permission-budgets`, `story:contracts-anonymous-auth`, `story:contracts-mutation-classification`, `story:contracts-restart-idempotency`, `story:contracts-discovery-coverage`, `story:contracts-read-refresh-retry`, `story:contracts-host-composition`, `story:contracts-management-boundary`, `story:contracts-discovery-profiles`, `story:contracts-acquisition-profiles`, `story:contracts-documentation-index`, `story:contracts-evidence-precision`, `story:contracts-media-controls`, `story:contracts-supported-vocabulary`, `story:contracts-mutation-visibility`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This approved local specification-hardening story revises normative contracts/design and settled ESS values using pinned ESS 0.20.0, retaining explicit UNMAPPED implementation and persistent relation requirements. It introduces no runtime code or current adapter-kind schema change. The existing source findings and stories authorize this work; their earlier text-only/no-commit boundary is superseded by the operator’s hardening instructions. Root is the sole tracked editor in primary main; two independent reviewers write ignored snapshots only. No concurrent implementation wave or external publication. Existing persistent ESS entities are reused; this inventory does not invent lifecycles, delete semantics or a universal storage abstraction. Local checkpoint commits and local recovery backup remain authorized.

## Completion evidence

E28 is fixed for the approved semantic specification scope. Normative revisions, 43 declared textual traces, 44 schema expectations, actual ESS entity inventory and validation limits are recorded in [verification](../../../docs/evidence/profile-persistence-20260908/verification.md) and [dispositions](../../../docs/evidence/profile-persistence-20260908/dispositions.md). Both immutable final independent reviews profile-persistence-a-recheck1-20260908 and profile-persistence-b-recheck1-20260908 approve with zero residual findings. ESS 0.20.0 validates 13 files/204 declarations; two 211-artifact projections match; the existing full gate/MSRV1.88 passes 50 Rust tests. E32 explicitly selects configured authority identity and defers physical-cluster attestation. E28 selects eighteen logical port owners/atomic groups/handoffs without claiming missing persistent entity models or a backend. No runtime/adapter-kind schema change or external publication; no recognizer, storage, authority or sequential conformance execution is claimed.

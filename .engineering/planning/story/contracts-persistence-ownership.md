---
format: aep.planning-md/1
id: story:contracts-persistence-ownership
kind: story
status: draft
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
- confidence: cited
  path: contracts/discovery/mediated_route/v1alpha1/semantics.md
- confidence: cited
  path: contracts/discovery/resources/v1alpha1/semantics.md
- confidence: cited
  path: contracts/operations/v1alpha1/semantics.md
- confidence: cited
  path: docs/design.md
revision: 3
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

- cited: `contracts/auth/custody/v1alpha1/semantics.md`
- cited: `contracts/auth/connection/v1alpha1/semantics.md`
- cited: `contracts/auth/acquisition/v1alpha1/semantics.md`
- cited: `contracts/operations/v1alpha1/semantics.md`
- cited: `contracts/discovery/resources/v1alpha1/semantics.md`
- cited: `contracts/discovery/mediated_route/v1alpha1/semantics.md`
- cited: `contracts/auth/evidence/v1alpha1/semantics.md`
- cited: `docs/design.md`

Source locations: `contracts/auth/connection/v1alpha1/semantics.md:118`; `contracts/operations/v1alpha1/semantics.md:153`; `contracts/auth/acquisition/v1alpha1/semantics.md:116`; `contracts/discovery/resources/v1alpha1/semantics.md:110`; `contracts/discovery/mediated_route/v1alpha1/semantics.md:98`; `contracts/auth/evidence/v1alpha1/semantics.md:101`; `contracts/auth/custody/v1alpha1/semantics.md:94`; `docs/design.md:961`.

## Dependencies and edit coordination

Semantic prerequisites: `story:contracts-mutation-outcomes`, `story:contracts-idempotency-scope`, `story:contracts-refresh-coordination`, `story:contracts-management-boundary`, `story:contracts-discovery-coverage`, `story:contracts-federated-approval`.

Shared edit surfaces with `story:contracts-mutation-outcomes`, `story:contracts-idempotency-scope`, `story:contracts-federated-approval`, `story:contracts-refresh-coordination`, `story:contracts-credential-evidence`, `story:contracts-wire-compatibility`, `story:contracts-connection-readiness`, `story:contracts-permission-budgets`, `story:contracts-anonymous-auth`, `story:contracts-mutation-classification`, `story:contracts-restart-idempotency`, `story:contracts-discovery-coverage`, `story:contracts-read-refresh-retry`, `story:contracts-host-composition`, `story:contracts-management-boundary`, `story:contracts-discovery-profiles`, `story:contracts-acquisition-profiles`, `story:contracts-documentation-index`, `story:contracts-evidence-precision`, `story:contracts-media-controls`, `story:contracts-supported-vocabulary`, `story:contracts-mutation-visibility`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This interactive, local-only story revises textual contracts, adapter design and their conformance scenarios; it does not implement runtime behavior, edit generated schemas, change ESS, commit/publish, or expand the implemented adapter set. Existing typed declarations are in `ess/system.yaml` and `ess/domains/declarations.yaml`; proposed runtime entities are not claimed to be modeled. Any later entity-bearing implementation decomposition must first use the ESS workflow for the settled semantics and retain unresolved relations as UNMAPPED. These documentation stories do not introduce a new typed entity or assert an unresolved cardinality.

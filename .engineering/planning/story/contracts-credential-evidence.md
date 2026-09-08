---
format: aep.planning-md/1
id: story:contracts-credential-evidence
kind: story
status: active
title: Bind readiness evidence to the credential actually dispatched
tags:
- P1
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
scope:
- confidence: cited
  path: contracts/auth/capability/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/connection/v1alpha1/semantics.md
- confidence: inferred
  path: contracts/auth/evidence/v1alpha1/scenarios/configured-replacement-identity.yaml
- confidence: inferred
  path: contracts/auth/evidence/v1alpha1/scenarios/rotation-before-dispatch.yaml
- confidence: inferred
  path: contracts/auth/evidence/v1alpha1/scenarios/same-identity-refresh.yaml
- confidence: cited
  path: contracts/auth/evidence/v1alpha1/semantics.md
- confidence: inferred
  path: contracts/auth/evidence/v1alpha1/verification.md
- confidence: cited
  path: docs/adapters/kubernetes.md
- confidence: inferred
  path: ess/domains/credential_evidence.yaml
revision: 11
---
## Context

Priority: **P1**. Sources: `F05` in `specification:contract-review-intake-20260908`.

Configured credential replacement can preserve age-valid evidence for another account while dispatch uses new material.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Specify credential-generation evidence validity and admission/dispatch consistency; distinguish same-identity rotation from reassignment without exposing secret-derived identifiers in public metadata.

## Acceptance

After revision, every credential-replacement scenario uses evidence valid for the dispatched credential identity.

## Verification scenarios

- Account A token replaced by account B token → explicit reassignment or refusal.
- Rotation occurs between admission and dispatch → revalidation or stable validated lease.
- Same-identity refresh invalidates only the checks the contract explicitly permits retaining.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

Derived 2026-09-08 by `aep-drive:story-scoper`; coordinator records the approved allocation.

- **cited:** `contracts/auth/capability/v1alpha1/semantics.md` — existing story surface.
- **cited:** `contracts/auth/connection/v1alpha1/semantics.md` — existing story surface.
- **cited:** `contracts/auth/evidence/v1alpha1/semantics.md` — existing story surface.
- **cited:** `docs/adapters/kubernetes.md` — existing story surface.
- **inferred:** `ess/domains/credential_evidence.yaml` — private ESS/model verification authoring.
- **inferred:** `contracts/auth/evidence/v1alpha1/verification.md` — private ESS/model verification authoring.
- **inferred:** `contracts/auth/evidence/v1alpha1/scenarios/configured-replacement-identity.yaml` — private ESS/model verification authoring.
- **inferred:** `contracts/auth/evidence/v1alpha1/scenarios/rotation-before-dispatch.yaml` — private ESS/model verification authoring.
- **inferred:** `contracts/auth/evidence/v1alpha1/scenarios/same-identity-refresh.yaml` — private ESS/model verification authoring.

- **Confidence: medium (inferred):** existing prose is cited; the modeling decomposition follows the approved semantic wave.
- **Would collide (inferred):** other edits to these exact paths. Shared credential identity, ESS registration and gate changes belong to coordinator preparation, completed before the private branches fork.
- **Coordinator allocation (inferred):** `ess/domains/credentials.yaml`, `ess/system.yaml`, and `crates/connectors-build/src/gate.rs` are prerequisites supplied by `specification:auth-hardening-wave-20260908`, not concurrent unit edit surfaces. Scopers originally named this integration overlap; it is explicitly serialized, not claimed disjoint.

## Dependencies and edit coordination

No semantic prerequisite is declared within this draft set.

Shared edit surfaces with `story:contracts-wire-compatibility`, `story:contracts-connection-readiness`, `story:contracts-permission-budgets`, `story:contracts-anonymous-auth`, `story:contracts-restart-idempotency`, `story:contracts-discovery-coverage`, `story:contracts-read-refresh-retry`, `story:contracts-management-boundary`, `story:contracts-discovery-profiles`, `story:contracts-acquisition-profiles`, `story:contracts-evidence-precision`, `story:contracts-media-controls`, `story:contracts-supported-vocabulary`, `story:contracts-persistence-ownership`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

The operator approved F04 and F05 as a two-story semantic hardening wave, with local commits and merges and no model budget limit. This supersedes the previous ESS/commit exclusion. Scope includes textual contracts, adapter design, ESS declarations and authored conformance scenarios; it excludes runtime auth implementation, generated-schema changes, publication and adapter-set expansion.

The shared identity seam is `ess/domains/credentials.yaml`; `ess/system.yaml` registers each private domain. Unsettled relations and runtime predicates remain explicit `UNMAPPED` obligations. Authored scenarios are compiled, not executed runtime conformance. Each unit requires a textual failure/scenario matrix, pinned ESS validation/compilation and authored scenario compilation with zero refusals, adversarial review and the full integration gate. See `specification:auth-hardening-wave-20260908` for scope ownership and evidence.

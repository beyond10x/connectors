---
format: aep.planning-md/1
id: story:contracts-credential-evidence
kind: story
status: draft
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
- confidence: cited
  path: contracts/auth/evidence/v1alpha1/semantics.md
- confidence: cited
  path: docs/adapters/kubernetes.md
revision: 2
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

- cited: `contracts/auth/connection/v1alpha1/semantics.md`
- cited: `contracts/auth/evidence/v1alpha1/semantics.md`
- cited: `contracts/auth/capability/v1alpha1/semantics.md`
- cited: `docs/adapters/kubernetes.md`

Source locations: `contracts/auth/connection/v1alpha1/semantics.md:100`; `contracts/auth/evidence/v1alpha1/semantics.md:65`; `contracts/auth/evidence/v1alpha1/semantics.md:73`; `contracts/auth/capability/v1alpha1/semantics.md:67`.

## Dependencies and edit coordination

No semantic prerequisite is declared within this draft set.

Shared edit surfaces with `story:contracts-wire-compatibility`, `story:contracts-connection-readiness`, `story:contracts-permission-budgets`, `story:contracts-anonymous-auth`, `story:contracts-restart-idempotency`, `story:contracts-discovery-coverage`, `story:contracts-read-refresh-retry`, `story:contracts-management-boundary`, `story:contracts-discovery-profiles`, `story:contracts-acquisition-profiles`, `story:contracts-evidence-precision`, `story:contracts-media-controls`, `story:contracts-supported-vocabulary`, `story:contracts-persistence-ownership`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This interactive, local-only story revises textual contracts, adapter design and their conformance scenarios; it does not implement runtime behavior, edit generated schemas, change ESS, commit/publish, or expand the implemented adapter set. Existing typed declarations are in `ess/system.yaml` and `ess/domains/declarations.yaml`; proposed runtime entities are not claimed to be modeled. Any later entity-bearing implementation decomposition must first use the ESS workflow for the settled semantics and retain unresolved relations as UNMAPPED. These documentation stories do not introduce a new typed entity or assert an unresolved cardinality.

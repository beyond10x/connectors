---
format: aep.planning-md/1
id: story:contracts-federated-approval
kind: story
status: draft
title: Bind approvals consistently across federation
tags:
- P1
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
- depends_on: story:contracts-idempotency-scope
- depends_on: story:contracts-wire-compatibility
scope:
- confidence: cited
  path: contracts/operations/v1alpha1/semantics.md
- confidence: cited
  path: contracts/service/v1alpha1/semantics.md
revision: 2
---
## Context

Priority: **P1**. Sources: `F03` in `specification:contract-review-intake-20260908`.

Gateway caller, operation and descriptor revision differ from leaf coordinates while approval evidence must remain unchanged.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Choose canonical approval subjects, client visibility, trusted authority mapping and verifier/redemption owner; distinguish routing freshness from signed identity and prohibit authority broadening.

## Acceptance

After revision, a gateway-to-leaf approval trace has one verifiable subject and redemption owner at every hop.

## Verification scenarios

- Admitted caller approves a gateway-visible operation and the selected leaf verifies the documented canonical subjects.
- Another origin or revision cannot reuse approval; a broad gateway credential cannot substitute for narrowed caller authority.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/operations/v1alpha1/semantics.md`
- cited: `contracts/service/v1alpha1/semantics.md`

Source locations: `contracts/service/v1alpha1/semantics.md:109`; `contracts/operations/v1alpha1/semantics.md:99`; `contracts/operations/v1alpha1/semantics.md:154`.

## Dependencies and edit coordination

Semantic prerequisites: `story:contracts-idempotency-scope`, `story:contracts-wire-compatibility`.

Shared edit surfaces with `story:contracts-mutation-outcomes`, `story:contracts-idempotency-scope`, `story:contracts-wire-compatibility`, `story:contracts-mutation-classification`, `story:contracts-restart-idempotency`, `story:contracts-read-refresh-retry`, `story:contracts-evidence-precision`, `story:contracts-persistence-ownership`, `story:contracts-mutation-visibility`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This interactive, local-only story revises textual contracts, adapter design and their conformance scenarios; it does not implement runtime behavior, edit generated schemas, change ESS, commit/publish, or expand the implemented adapter set. Existing typed declarations are in `ess/system.yaml` and `ess/domains/declarations.yaml`; proposed runtime entities are not claimed to be modeled. Any later entity-bearing implementation decomposition must first use the ESS workflow for the settled semantics and retain unresolved relations as UNMAPPED. These documentation stories do not introduce a new typed entity or assert an unresolved cardinality.

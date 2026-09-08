---
format: aep.planning-md/1
id: story:contracts-idempotency-scope
kind: story
status: draft
title: Define idempotency ownership and replay admission
tags:
- P1
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
- depends_on: story:contracts-mutation-outcomes
scope:
- confidence: cited
  path: contracts/operations/v1alpha1/semantics.md
- confidence: cited
  path: docs/adapters/atlassian.md
revision: 2
---
## Context

Priority: **P1**. Sources: `F02` in `specification:contract-review-intake-20260908`.

Same key and canonical input do not identify a request across callers, connections, operations or origins.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Define namespace, fingerprint, retention and revision-change treatment as textual policy; require current authority for replay and do not equate canonical input with complete request identity.

## Acceptance

After revision, the idempotency matrix distinguishes replay from an unrelated effect for every listed authority and revision boundary.

## Verification scenarios

- Same key/body across caller, operation, connection and federation origin → explicitly separate or refused, never cross-authority result disclosure.
- Same scoped key with changed input → conflict; current access revoked before replay → refusal.
- Retention expiry and in-flight/unknown outcome records have explicit replay behavior.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/operations/v1alpha1/semantics.md`
- cited: `docs/adapters/atlassian.md`

Source locations: `contracts/operations/v1alpha1/semantics.md:100`; `contracts/operations/v1alpha1/semantics.md:107`; `contracts/operations/v1alpha1/semantics.md:120`; `docs/adapters/atlassian.md:72`.

## Dependencies and edit coordination

Semantic prerequisites: `story:contracts-mutation-outcomes`.

Shared edit surfaces with `story:contracts-mutation-outcomes`, `story:contracts-federated-approval`, `story:contracts-refresh-coordination`, `story:contracts-wire-compatibility`, `story:contracts-mutation-classification`, `story:contracts-restart-idempotency`, `story:contracts-document-admission`, `story:contracts-evidence-precision`, `story:contracts-persistence-ownership`, `story:contracts-mutation-visibility`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This interactive, local-only story revises textual contracts, adapter design and their conformance scenarios; it does not implement runtime behavior, edit generated schemas, change ESS, commit/publish, or expand the implemented adapter set. Existing typed declarations are in `ess/system.yaml` and `ess/domains/declarations.yaml`; proposed runtime entities are not claimed to be modeled. Any later entity-bearing implementation decomposition must first use the ESS workflow for the settled semantics and retain unresolved relations as UNMAPPED. These documentation stories do not introduce a new typed entity or assert an unresolved cardinality.

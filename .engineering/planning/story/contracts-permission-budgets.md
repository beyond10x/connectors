---
format: aep.planning-md/1
id: story:contracts-permission-budgets
kind: story
status: draft
title: Define authorization-check budgets for namespace fan-out
tags:
- P2
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
- depends_on: story:contracts-credential-evidence
scope:
- confidence: cited
  path: contracts/auth/evidence/v1alpha1/semantics.md
- confidence: cited
  path: docs/adapters/kubernetes.md
revision: 2
---
## Context

Priority: **P2**. Sources: `F08` in `specification:contract-review-intake-20260908`.

The one-check-per-invocation ceiling cannot satisfy exact checks for two uncached namespaces.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Define the check unit, maximum fan-out and exhaustion behavior while retaining per-target evidence and bounded provider work.

## Acceptance

After revision, the multi-namespace scenario fits the declared authorization-check budget without using unchecked namespaces.

## Verification scenarios

- Two uncached namespaces, one allowed and one denied → documented partial result.
- More authorization targets than the budget → explicit bounded outcome.
- Cached evidence is reused only within its exact valid target and credential scope.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/auth/evidence/v1alpha1/semantics.md`
- cited: `docs/adapters/kubernetes.md`

Source locations: `contracts/auth/evidence/v1alpha1/semantics.md:74`; `contracts/auth/evidence/v1alpha1/semantics.md:81`; `contracts/auth/evidence/v1alpha1/semantics.md:89`; `docs/adapters/kubernetes.md:43`.

## Dependencies and edit coordination

Semantic prerequisites: `story:contracts-credential-evidence`.

Shared edit surfaces with `story:contracts-credential-evidence`, `story:contracts-wire-compatibility`, `story:contracts-connection-readiness`, `story:contracts-restart-idempotency`, `story:contracts-discovery-coverage`, `story:contracts-discovery-profiles`, `story:contracts-acquisition-profiles`, `story:contracts-evidence-precision`, `story:contracts-media-controls`, `story:contracts-persistence-ownership`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This interactive, local-only story revises textual contracts, adapter design and their conformance scenarios; it does not implement runtime behavior, edit generated schemas, change ESS, commit/publish, or expand the implemented adapter set. Existing typed declarations are in `ess/system.yaml` and `ess/domains/declarations.yaml`; proposed runtime entities are not claimed to be modeled. Any later entity-bearing implementation decomposition must first use the ESS workflow for the settled semantics and retain unresolved relations as UNMAPPED. These documentation stories do not introduce a new typed entity or assert an unresolved cardinality.

---
format: aep.planning-md/1
id: story:contracts-discovery-coverage
kind: story
status: draft
title: Separate incomplete discovery from confirmed withdrawal
tags:
- P2
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
- depends_on: story:contracts-permission-budgets
scope:
- confidence: cited
  path: contracts/discovery/mediated_route/v1alpha1/semantics.md
- confidence: cited
  path: contracts/discovery/resources/v1alpha1/semantics.md
- confidence: cited
  path: docs/adapters/grafana.md
- confidence: cited
  path: docs/adapters/kubernetes.md
revision: 2
---
## Context

Priority: **P2**. Sources: `F13` in `specification:contract-review-intake-20260908`.

Atomic replacement treats omissions from capped, denied or failed refreshes as confirmed disappearance.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Define coverage scope, complete versus partial generations and withdrawal authority; retain unknown/stale evidence without granting access and keep current permission denial authoritative.

## Acceptance

After revision, incomplete refreshes cannot produce a confirmed withdrawal without complete evidence for the affected scope.

## Verification scenarios

- Previously complete set followed by cap exhaustion, denied namespace or provider failure.
- A complete successful refresh proves an existing observation absent.
- A retained stale observation cannot bypass current denial on a child route.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/discovery/resources/v1alpha1/semantics.md`
- cited: `contracts/discovery/mediated_route/v1alpha1/semantics.md`
- cited: `docs/adapters/kubernetes.md`
- cited: `docs/adapters/grafana.md`

Source locations: `contracts/discovery/resources/v1alpha1/semantics.md:68`; `contracts/discovery/resources/v1alpha1/semantics.md:78`; `contracts/discovery/resources/v1alpha1/semantics.md:80`; `contracts/discovery/resources/v1alpha1/semantics.md:87`.

## Dependencies and edit coordination

Semantic prerequisites: `story:contracts-permission-budgets`.

Shared edit surfaces with `story:contracts-credential-evidence`, `story:contracts-wire-compatibility`, `story:contracts-permission-budgets`, `story:contracts-anonymous-auth`, `story:contracts-restart-idempotency`, `story:contracts-log-continuation`, `story:contracts-host-composition`, `story:contracts-discovery-profiles`, `story:contracts-acquisition-profiles`, `story:contracts-evidence-precision`, `story:contracts-supported-vocabulary`, `story:contracts-persistence-ownership`, `story:contracts-tenant-header`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This interactive, local-only story revises textual contracts, adapter design and their conformance scenarios; it does not implement runtime behavior, edit generated schemas, change ESS, commit/publish, or expand the implemented adapter set. Existing typed declarations are in `ess/system.yaml` and `ess/domains/declarations.yaml`; proposed runtime entities are not claimed to be modeled. Any later entity-bearing implementation decomposition must first use the ESS workflow for the settled semantics and retain unresolved relations as UNMAPPED. These documentation stories do not introduce a new typed entity or assert an unresolved cardinality.

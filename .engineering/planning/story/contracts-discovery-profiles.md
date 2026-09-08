---
format: aep.planning-md/1
id: story:contracts-discovery-profiles
kind: story
status: draft
title: Make discovery profile binding and Kubernetes recognition explicit
tags:
- P2
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
- depends_on: story:contracts-discovery-coverage
scope:
- confidence: cited
  path: contracts/discovery/resources/v1alpha1/semantics.md
- confidence: cited
  path: docs/adapters/grafana.md
- confidence: cited
  path: docs/adapters/kubernetes.md
revision: 2
---
## Context

Priority: **P2**. Sources: `E07`, `E14`, `E32` in `specification:contract-review-intake-20260908`.

Generic resources.observe with a caller profile differs from adapter-chosen operation IDs and descriptor-fixed profiles; Kubernetes matching and cluster identity are underspecified.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Define operation naming and receiver-owned profile selection, exact Argo CD identity recognition preserving the cited name-or-label rule, and the configured instance/API-origin identity boundary or an explicit deferred disposition. Argo recognition remains observational, not a new callable adapter.

## Acceptance

After revision, each discovery declaration resolves to one fixed profile and source scope with unambiguous recognition rules.

## Verification scenarios

- Wrong request profile cannot retarget an operation advertised for another profile.
- Argo API Service identified by exact name or exact stable app.kubernetes.io/name label; metrics/repo-server/redis are not mistaken for it.
- Distinct configured clusters have distinct source identity; cluster-identity requirements not supported yet are named as deferred.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/discovery/resources/v1alpha1/semantics.md`
- cited: `docs/adapters/kubernetes.md`
- cited: `docs/adapters/grafana.md`

Source locations: `contracts/discovery/resources/v1alpha1/semantics.md:31`; `contracts/discovery/resources/v1alpha1/semantics.md:34`; `contracts/discovery/resources/v1alpha1/semantics.md:125`; `docs/adapters/kubernetes.md:56`; `docs/adapters/kubernetes.md:78`; `docs/adapters/grafana.md:61`; `docs/design.md:939`; `../connectors/docs/design/10-local-kubernetes-context-and-resource-discovery.md:175`.

## Dependencies and edit coordination

Semantic prerequisites: `story:contracts-discovery-coverage`.

Shared edit surfaces with `story:contracts-credential-evidence`, `story:contracts-wire-compatibility`, `story:contracts-permission-budgets`, `story:contracts-anonymous-auth`, `story:contracts-restart-idempotency`, `story:contracts-log-continuation`, `story:contracts-discovery-coverage`, `story:contracts-host-composition`, `story:contracts-acquisition-profiles`, `story:contracts-evidence-precision`, `story:contracts-supported-vocabulary`, `story:contracts-persistence-ownership`, `story:contracts-tenant-header`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This interactive, local-only story revises textual contracts, adapter design and their conformance scenarios; it does not implement runtime behavior, edit generated schemas, change ESS, commit/publish, or expand the implemented adapter set. Existing typed declarations are in `ess/system.yaml` and `ess/domains/declarations.yaml`; proposed runtime entities are not claimed to be modeled. Any later entity-bearing implementation decomposition must first use the ESS workflow for the settled semantics and retain unresolved relations as UNMAPPED. These documentation stories do not introduce a new typed entity or assert an unresolved cardinality.

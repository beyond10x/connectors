---
format: aep.planning-md/1
id: story:contracts-anonymous-auth
kind: story
status: draft
title: Represent deliberate anonymous and parent-authenticated access
tags:
- P2
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
- depends_on: story:contracts-wire-compatibility
scope:
- confidence: cited
  path: contracts/auth/capability/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/profile/v1alpha1/semantics.md
- confidence: cited
  path: contracts/discovery/mediated_route/v1alpha1/semantics.md
- confidence: cited
  path: docs/adapters/grafana.md
revision: 2
---
## Context

Priority: **P2**. Sources: `F09`, `E10` in `specification:contract-review-intake-20260908`.

Monitoring declares none profiles and null credentials that the closed auth/capability vocabularies cannot represent.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Choose an explicit representation for anonymous direct access and separate parent-authenticated mediation; state purpose/scheme/capability semantics and refuse credential-missing fallback.

## Acceptance

After revision, every monitoring auth row maps to an admitted profile with an explicit credential-placement rule.

## Verification scenarios

- Configured anonymous direct request proceeds under its own admitted profile.
- Missing bearer/basic material refuses without anonymous fallback.
- Mediated child uses parent authentication without acquiring or exposing the parent's secret.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/auth/profile/v1alpha1/semantics.md`
- cited: `contracts/auth/capability/v1alpha1/semantics.md`
- cited: `contracts/discovery/mediated_route/v1alpha1/semantics.md`
- cited: `docs/adapters/grafana.md`

Source locations: `contracts/auth/profile/v1alpha1/semantics.md:62`; `contracts/auth/profile/v1alpha1/semantics.md:64`; `contracts/auth/capability/v1alpha1/semantics.md:67`; `docs/adapters/grafana.md:74`; `contracts/discovery/mediated_route/v1alpha1/semantics.md:52`.

## Dependencies and edit coordination

Semantic prerequisites: `story:contracts-wire-compatibility`.

Shared edit surfaces with `story:contracts-credential-evidence`, `story:contracts-wire-compatibility`, `story:contracts-log-continuation`, `story:contracts-discovery-coverage`, `story:contracts-read-refresh-retry`, `story:contracts-host-composition`, `story:contracts-discovery-profiles`, `story:contracts-acquisition-profiles`, `story:contracts-evidence-precision`, `story:contracts-media-controls`, `story:contracts-supported-vocabulary`, `story:contracts-persistence-ownership`, `story:contracts-tenant-header`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This interactive, local-only story revises textual contracts, adapter design and their conformance scenarios; it does not implement runtime behavior, edit generated schemas, change ESS, commit/publish, or expand the implemented adapter set. Existing typed declarations are in `ess/system.yaml` and `ess/domains/declarations.yaml`; proposed runtime entities are not claimed to be modeled. Any later entity-bearing implementation decomposition must first use the ESS workflow for the settled semantics and retain unresolved relations as UNMAPPED. These documentation stories do not introduce a new typed entity or assert an unresolved cardinality.

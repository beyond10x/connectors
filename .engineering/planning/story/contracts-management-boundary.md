---
format: aep.planning-md/1
id: story:contracts-management-boundary
kind: story
status: draft
title: Clarify ownership and admission of connection management
tags:
- P2
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
- depends_on: story:contracts-wire-compatibility
scope:
- confidence: cited
  path: contracts/auth/acquisition/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/connection/v1alpha1/semantics.md
- confidence: cited
  path: docs/design.md
revision: 2
---
## Context

Priority: **P2**. Sources: `E04` in `specification:contract-review-intake-20260908`.

Calling management ordinary adapter operations obscures the split between host persistence, provider protocol work and admitted remote management.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Specify orchestration owner, management admission/discovery and federation routing; preserve secret/private callback non-disclosure. Design §16.1 forbids exposing private capabilities as discovery data, not all use of an operation envelope, so document the chosen boundary without treating dedicated HTTP routes as already required.

## Acceptance

After revision, a remote connection-management trace assigns each action to its owner without advertising private callback or registration authority.

## Verification scenarios

- Begin/complete/list/revoke identify host versus provider responsibilities and admitted caller scope.
- Federated completion reaches its owning coordinator.
- Ordinary discovery cannot reveal private callback capability or registration secrets.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/auth/connection/v1alpha1/semantics.md`
- cited: `contracts/auth/acquisition/v1alpha1/semantics.md`
- cited: `docs/design.md`

Source locations: `contracts/auth/connection/v1alpha1/semantics.md:64`; `contracts/auth/connection/v1alpha1/semantics.md:118`; `contracts/auth/acquisition/v1alpha1/semantics.md:30`; `docs/design.md:35`; `docs/design.md:785`.

## Dependencies and edit coordination

Semantic prerequisites: `story:contracts-wire-compatibility`.

Shared edit surfaces with `story:contracts-refresh-coordination`, `story:contracts-credential-evidence`, `story:contracts-wire-compatibility`, `story:contracts-connection-readiness`, `story:contracts-read-refresh-retry`, `story:contracts-host-composition`, `story:contracts-acquisition-profiles`, `story:contracts-documentation-index`, `story:contracts-persistence-ownership`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This interactive, local-only story revises textual contracts, adapter design and their conformance scenarios; it does not implement runtime behavior, edit generated schemas, change ESS, commit/publish, or expand the implemented adapter set. Existing typed declarations are in `ess/system.yaml` and `ess/domains/declarations.yaml`; proposed runtime entities are not claimed to be modeled. Any later entity-bearing implementation decomposition must first use the ESS workflow for the settled semantics and retain unresolved relations as UNMAPPED. These documentation stories do not introduce a new typed entity or assert an unresolved cardinality.

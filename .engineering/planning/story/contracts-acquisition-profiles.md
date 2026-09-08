---
format: aep.planning-md/1
id: story:contracts-acquisition-profiles
kind: story
status: draft
title: Align configured and OAuth acquisition profile requirements
tags:
- P2
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
scope:
- confidence: cited
  path: contracts/README.md
- confidence: cited
  path: contracts/auth/acquisition/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/profile/v1alpha1/semantics.md
- confidence: cited
  path: docs/adapters/docker.md
- confidence: cited
  path: docs/adapters/grafana.md
- confidence: cited
  path: docs/adapters/kubernetes.md
revision: 2
---
## Context

Priority: **P2**. Sources: `E09`, `E29` in `specification:contract-review-intake-20260908`.

static_config appears in auth.profile and adapter rows but has no acquisition profile semantics; the OAuth rule requires authorize_url even for client credentials.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Define deployment-supplied static configuration separately from interactive secret entry and state the supported/reserved flow matrix with flow-specific endpoint requirements.

## Acceptance

After revision, every advertised acquisition flow has a supported path whose endpoint requirements match that flow.

## Verification scenarios

- static_config binds deployment-supplied references without inventing an interactive acquisition flow.
- Authorization code requires its authorization/token endpoints and source evidence.
- Client credentials requires its token endpoint without an authorization endpoint.
- A reserved flow cannot be advertised as implemented.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/auth/acquisition/v1alpha1/semantics.md`
- cited: `contracts/auth/profile/v1alpha1/semantics.md`
- cited: `docs/adapters/kubernetes.md`
- cited: `docs/adapters/docker.md`
- cited: `docs/adapters/grafana.md`
- cited: `contracts/README.md`

Source locations: `contracts/auth/acquisition/v1alpha1/semantics.md:12`; `contracts/auth/profile/v1alpha1/semantics.md:65`; `contracts/auth/profile/v1alpha1/semantics.md:77`; `docs/adapters/kubernetes.md:63`; `docs/adapters/docker.md:55`; `docs/adapters/grafana.md:74`; `contracts/README.md:47`.

## Dependencies and edit coordination

No semantic prerequisite is declared within this draft set.

Shared edit surfaces with `story:contracts-refresh-coordination`, `story:contracts-credential-evidence`, `story:contracts-wire-compatibility`, `story:contracts-connection-readiness`, `story:contracts-permission-budgets`, `story:contracts-anonymous-auth`, `story:contracts-restart-idempotency`, `story:contracts-log-continuation`, `story:contracts-discovery-coverage`, `story:contracts-read-refresh-retry`, `story:contracts-host-composition`, `story:contracts-management-boundary`, `story:contracts-discovery-profiles`, `story:contracts-documentation-index`, `story:contracts-evidence-precision`, `story:contracts-media-controls`, `story:contracts-supported-vocabulary`, `story:contracts-persistence-ownership`, `story:contracts-tenant-header`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This interactive, local-only story revises textual contracts, adapter design and their conformance scenarios; it does not implement runtime behavior, edit generated schemas, change ESS, commit/publish, or expand the implemented adapter set. Existing typed declarations are in `ess/system.yaml` and `ess/domains/declarations.yaml`; proposed runtime entities are not claimed to be modeled. Any later entity-bearing implementation decomposition must first use the ESS workflow for the settled semantics and retain unresolved relations as UNMAPPED. These documentation stories do not introduce a new typed entity or assert an unresolved cardinality.

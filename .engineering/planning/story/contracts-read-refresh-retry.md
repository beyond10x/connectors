---
format: aep.planning-md/1
id: story:contracts-read-refresh-retry
kind: story
status: draft
title: Version and bound read redispatch after refresh
tags:
- P2
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
- depends_on: story:contracts-wire-compatibility
- depends_on: story:contracts-refresh-coordination
- depends_on: story:contracts-credential-evidence
scope:
- confidence: cited
  path: contracts/auth/acquisition/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/capability/v1alpha1/semantics.md
- confidence: cited
  path: contracts/service/v1alpha1/semantics.md
revision: 2
---
## Context

Priority: **P2**. Sources: `F15`, `E05` in `specification:contract-review-intake-20260908`.

The inherited service prohibits automatic retries while the capability mandates one redispatch after a read 401.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Record the disposition change and selected compatible profile; define shared deadline, maximum exchange count, refresh errors and admission/evidence checks on redispatch.

## Acceptance

After revision, each negotiated read profile determines one bounded 401-response sequence without silently changing existing service semantics.

## Verification scenarios

- Old read profile produces the documented single-call error.
- Explicit retry profile allows only its declared refresh/redispatch sequence within one budget.
- Mutation 401 never redispatches the business effect.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/service/v1alpha1/semantics.md`
- cited: `contracts/auth/capability/v1alpha1/semantics.md`
- cited: `contracts/auth/acquisition/v1alpha1/semantics.md`

Source locations: `contracts/service/v1alpha1/semantics.md:43`; `contracts/auth/capability/v1alpha1/semantics.md:70`; `contracts/auth/capability/v1alpha1/semantics.md:91`; `contracts/auth/acquisition/v1alpha1/semantics.md:77`.

## Dependencies and edit coordination

Semantic prerequisites: `story:contracts-wire-compatibility`, `story:contracts-refresh-coordination`, `story:contracts-credential-evidence`.

Shared edit surfaces with `story:contracts-federated-approval`, `story:contracts-refresh-coordination`, `story:contracts-credential-evidence`, `story:contracts-wire-compatibility`, `story:contracts-connection-readiness`, `story:contracts-anonymous-auth`, `story:contracts-management-boundary`, `story:contracts-acquisition-profiles`, `story:contracts-media-controls`, `story:contracts-supported-vocabulary`, `story:contracts-persistence-ownership`, `story:contracts-mutation-visibility`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This interactive, local-only story revises textual contracts, adapter design and their conformance scenarios; it does not implement runtime behavior, edit generated schemas, change ESS, commit/publish, or expand the implemented adapter set. Existing typed declarations are in `ess/system.yaml` and `ess/domains/declarations.yaml`; proposed runtime entities are not claimed to be modeled. Any later entity-bearing implementation decomposition must first use the ESS workflow for the settled semantics and retain unresolved relations as UNMAPPED. These documentation stories do not introduce a new typed entity or assert an unresolved cardinality.

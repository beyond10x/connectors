---
format: aep.planning-md/1
id: story:contracts-restart-idempotency
kind: story
status: draft
title: State exact idempotency guarantees for lifecycle operations
tags:
- P2
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
- depends_on: story:contracts-idempotency-scope
scope:
- confidence: cited
  path: contracts/operations/v1alpha1/semantics.md
- confidence: cited
  path: docs/adapters/docker.md
- confidence: cited
  path: docs/adapters/kubernetes.md
revision: 2
---
## Context

Priority: **P2**. Sources: `F11`, `E08` in `specification:contract-review-intake-20260908`.

Docker restart is both natural and none; a generated Kubernetes annotation timestamp alone does not establish natural idempotency.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

State stable request intent, provider preconditions and repeated-call result semantics before choosing natural, keyed or none. Preserve the old UID/resourceVersion evidence: the claim that every repeated dispatch necessarily rolls out again is not established.

## Acceptance

After revision, each listed lifecycle operation has one idempotency classification justified by its repeated-request scenarios.

## Verification scenarios

- Docker summary and operation map agree for start, stop and restart.
- Kubernetes lost response then same request with original UID/resourceVersion has a documented effect and result.
- A new invocation or changed precondition is distinguished from replay.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/operations/v1alpha1/semantics.md`
- cited: `docs/adapters/kubernetes.md`
- cited: `docs/adapters/docker.md`

Source locations: `contracts/operations/v1alpha1/semantics.md:54`; `docs/adapters/kubernetes.md:55`; `docs/adapters/docker.md:25`; `docs/adapters/docker.md:47`; `../connectors/crates/integration-kubernetes/src/local_workloads.rs:296`.

## Dependencies and edit coordination

Semantic prerequisites: `story:contracts-idempotency-scope`.

Shared edit surfaces with `story:contracts-mutation-outcomes`, `story:contracts-idempotency-scope`, `story:contracts-federated-approval`, `story:contracts-credential-evidence`, `story:contracts-wire-compatibility`, `story:contracts-permission-budgets`, `story:contracts-mutation-classification`, `story:contracts-discovery-coverage`, `story:contracts-discovery-profiles`, `story:contracts-acquisition-profiles`, `story:contracts-evidence-precision`, `story:contracts-persistence-ownership`, `story:contracts-mutation-visibility`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This interactive, local-only story revises textual contracts, adapter design and their conformance scenarios; it does not implement runtime behavior, edit generated schemas, change ESS, commit/publish, or expand the implemented adapter set. Existing typed declarations are in `ess/system.yaml` and `ess/domains/declarations.yaml`; proposed runtime entities are not claimed to be modeled. Any later entity-bearing implementation decomposition must first use the ESS workflow for the settled semantics and retain unresolved relations as UNMAPPED. These documentation stories do not introduce a new typed entity or assert an unresolved cardinality.

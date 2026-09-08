---
format: aep.planning-md/1
id: story:contracts-mutation-visibility
kind: story
status: draft
title: Distinguish implemented, enabled and discoverable mutations
tags:
- P2
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
- depends_on: story:contracts-wire-compatibility
- depends_on: story:contracts-mutation-classification
scope:
- confidence: cited
  path: contracts/operations/v1alpha1/semantics.md
- confidence: cited
  path: contracts/service/v1alpha1/semantics.md
- confidence: cited
  path: docs/adapters/atlassian.md
revision: 2
---
## Context

Priority: **P2**. Sources: `E12` in `specification:contract-review-intake-20260908`.

The reviewer infers disabled operations are unadvertised from an implemented-only rule, which does not establish that claim; the textual visibility/refusal policy still needs explicit cases.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Document a visibility and refusal matrix for unimplemented, implemented-disabled and enabled-but-unauthorized operations; preserve the design's separate implemented/enabled/ready/authorized facts. Do not add a disabled field or remove Forbidden solely on the unsupported inference.

## Acceptance

After revision, each mutation availability state has one documented discovery and invocation result.

## Verification scenarios

- Unimplemented ID → advertised/lookup behavior explicitly distinguished from configured-disabled operation.
- Disabled implementation is consistently visible or hidden under a stated policy.
- Enabled but unauthorized caller cannot gain access through discovery.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/operations/v1alpha1/semantics.md`
- cited: `contracts/service/v1alpha1/semantics.md`
- cited: `docs/adapters/atlassian.md`

Source locations: `contracts/operations/v1alpha1/semantics.md:98`; `contracts/service/v1alpha1/semantics.md:19`; `docs/adapters/atlassian.md:111`; `crates/connectors-core/src/lib.rs:86`; `docs/design.md:350`.

## Dependencies and edit coordination

Semantic prerequisites: `story:contracts-wire-compatibility`, `story:contracts-mutation-classification`.

Shared edit surfaces with `story:contracts-mutation-outcomes`, `story:contracts-idempotency-scope`, `story:contracts-federated-approval`, `story:contracts-refresh-coordination`, `story:contracts-wire-compatibility`, `story:contracts-mutation-classification`, `story:contracts-restart-idempotency`, `story:contracts-document-admission`, `story:contracts-read-refresh-retry`, `story:contracts-evidence-precision`, `story:contracts-persistence-ownership`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This interactive, local-only story revises textual contracts, adapter design and their conformance scenarios; it does not implement runtime behavior, edit generated schemas, change ESS, commit/publish, or expand the implemented adapter set. Existing typed declarations are in `ess/system.yaml` and `ess/domains/declarations.yaml`; proposed runtime entities are not claimed to be modeled. Any later entity-bearing implementation decomposition must first use the ESS workflow for the settled semantics and retain unresolved relations as UNMAPPED. These documentation stories do not introduce a new typed entity or assert an unresolved cardinality.

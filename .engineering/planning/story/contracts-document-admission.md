---
format: aep.planning-md/1
id: story:contracts-document-admission
kind: story
status: draft
title: Define trusted document membership before content access
tags:
- P2
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
scope:
- confidence: cited
  path: contracts/datasources/records/v1alpha1/semantics.md
- confidence: cited
  path: docs/adapters/atlassian.md
revision: 2
---
## Context

Priority: **P2**. Sources: `F14` in `specification:contract-review-intake-20260908`.

An opaque unseen page ID does not locally prove space membership before the required no-provider-call admission boundary.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Choose trusted receiver-owned membership, a scope-constrained lookup or bounded authorization lookup; define unknown/stale membership and movement between spaces without trusting caller assertions.

## Acceptance

After revision, the document admission trace determines whether an unseen or moved page may be read without relying on caller-asserted membership.

## Verification scenarios

- Unseen page ID in an allowed versus forbidden space.
- Page moves after cached membership; content retrieval rechecks the declared freshness/binding rule.
- Authorization lookup, if selected, has explicit admitted destination, data and work bounds.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/datasources/records/v1alpha1/semantics.md`
- cited: `docs/adapters/atlassian.md`

Source locations: `contracts/datasources/records/v1alpha1/semantics.md:34`; `contracts/datasources/records/v1alpha1/semantics.md:63`; `contracts/datasources/records/v1alpha1/semantics.md:83`; `docs/adapters/atlassian.md:106`.

## Dependencies and edit coordination

No semantic prerequisite is declared within this draft set.

Shared edit surfaces with `story:contracts-idempotency-scope`, `story:contracts-refresh-coordination`, `story:contracts-wire-compatibility`, `story:contracts-evidence-precision`, `story:contracts-mutation-visibility`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This interactive, local-only story revises textual contracts, adapter design and their conformance scenarios; it does not implement runtime behavior, edit generated schemas, change ESS, commit/publish, or expand the implemented adapter set. Existing typed declarations are in `ess/system.yaml` and `ess/domains/declarations.yaml`; proposed runtime entities are not claimed to be modeled. Any later entity-bearing implementation decomposition must first use the ESS workflow for the settled semantics and retain unresolved relations as UNMAPPED. These documentation stories do not introduce a new typed entity or assert an unresolved cardinality.

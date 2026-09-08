---
format: aep.planning-md/1
id: story:contracts-documentation-index
kind: story
status: draft
title: Reconcile contract and adapter coverage indexes
tags:
- P3
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
- depends_on: story:contracts-discovery-profiles
- depends_on: story:contracts-acquisition-profiles
- depends_on: story:contracts-media-controls
- depends_on: story:contracts-anonymous-auth
scope:
- confidence: cited
  path: contracts/README.md
- confidence: cited
  path: docs/design.md
revision: 3
---
## Context

Priority: **P3**. Sources: `E13`, `E22` in `specification:contract-review-intake-20260908`.

The deferred-family list omits configuration, and the adapter matrix omits contracts its adapter documents require.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Reconcile the index against the revised adapter plans and design, preserving implemented/proposed/deferred distinctions. Counts and tables derive from documented support, not executable-generation work.

## Acceptance

After revision, a row-by-row index audit finds no omitted declared dependency or deferred family.

## Verification scenarios

- Configuration appears with the correct deferred status.
- Kubernetes/Docker/Grafana auth dependencies and media records are reflected.
- Family counts and links match the revised files.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/README.md`
- cited: `docs/design.md`

Source locations: `contracts/README.md:29`; `contracts/README.md:36`; `docs/design.md:180`; `docs/design.md:1284`.

## Dependencies and edit coordination

Semantic prerequisites: `story:contracts-discovery-profiles`, `story:contracts-acquisition-profiles`, `story:contracts-media-controls`, `story:contracts-anonymous-auth`.

Shared edit surfaces with `story:contracts-host-composition`, `story:contracts-management-boundary`, `story:contracts-acquisition-profiles`, `story:contracts-media-controls`, `story:contracts-persistence-ownership`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This interactive, local-only story revises textual contracts, adapter design and their conformance scenarios; it does not implement runtime behavior, edit generated schemas, change ESS, commit/publish, or expand the implemented adapter set. Existing typed declarations are in `ess/system.yaml` and `ess/domains/declarations.yaml`; proposed runtime entities are not claimed to be modeled. Any later entity-bearing implementation decomposition must first use the ESS workflow for the settled semantics and retain unresolved relations as UNMAPPED. These documentation stories do not introduce a new typed entity or assert an unresolved cardinality.

## Partial completion during independent review remediation — 2026-09-08

`story:independent-review-remediation` addresses overlapping A-M3/B-F05: the index now includes the proposed governed service document, distinguishes 22 table rows from 17 semantic documents, and lists all four deferred families including configuration. This also closes this story's original E13. The current index and design §30 agree (five implemented rows in one document; seventeen proposed rows in sixteen documents). E22's adapter dependency matrix and prerequisite-dependent reconciliations remain owned by this draft story; its lifecycle has not been advanced or declared fully complete. Evidence and final review dispositions are recorded by the remediation story.

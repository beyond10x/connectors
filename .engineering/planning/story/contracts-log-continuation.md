---
format: aep.planning-md/1
id: story:contracts-log-continuation
kind: story
status: draft
title: Make bounded log continuation truthful at timestamp ties
tags:
- P2
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
scope:
- confidence: cited
  path: contracts/datasources/logs/v1alpha1/semantics.md
- confidence: cited
  path: docs/adapters/grafana.md
revision: 2
---
## Context

Priority: **P2**. Sources: `F12` in `specification:contract-review-intake-20260908`.

Timestamp-only continuation can omit entries or repeat forever when a tie exceeds the page limit.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Define interval inclusivity, duplicates, ordering and progress; require reliable continuation or explicit non-resumable partial output within bounds.

## Acceptance

After revision, the log continuation scenarios promise neither omission-free pagination nor progress where the selected provider continuation cannot deliver it.

## Verification scenarios

- 1,001 entries at one timestamp with page size 1,000, including identical lines across streams.
- Inclusive and exclusive edge behavior are explicit.
- Provider without reliable tie continuation returns a truthful partial/non-resumable result.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/datasources/logs/v1alpha1/semantics.md`
- cited: `docs/adapters/grafana.md`

Source locations: `contracts/datasources/logs/v1alpha1/semantics.md:67`; `contracts/datasources/logs/v1alpha1/semantics.md:75`; `contracts/datasources/logs/v1alpha1/semantics.md:94`.

## Dependencies and edit coordination

No semantic prerequisite is declared within this draft set.

Shared edit surfaces with `story:contracts-wire-compatibility`, `story:contracts-anonymous-auth`, `story:contracts-discovery-coverage`, `story:contracts-host-composition`, `story:contracts-discovery-profiles`, `story:contracts-acquisition-profiles`, `story:contracts-evidence-precision`, `story:contracts-tenant-header`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This interactive, local-only story revises textual contracts, adapter design and their conformance scenarios; it does not implement runtime behavior, edit generated schemas, change ESS, commit/publish, or expand the implemented adapter set. Existing typed declarations are in `ess/system.yaml` and `ess/domains/declarations.yaml`; proposed runtime entities are not claimed to be modeled. Any later entity-bearing implementation decomposition must first use the ESS workflow for the settled semantics and retain unresolved relations as UNMAPPED. These documentation stories do not introduce a new typed entity or assert an unresolved cardinality.

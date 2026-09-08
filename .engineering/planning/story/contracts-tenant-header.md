---
format: aep.planning-md/1
id: story:contracts-tenant-header
kind: story
status: draft
title: Use one configuration name for the monitoring tenant header
tags:
- P3
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

Priority: **P3**. Sources: `E30` in `specification:contract-review-intake-20260908`.

Preserved tenant-header configuration and the adapter example name different shapes without a mapping.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Choose the supported configuration representation or explicitly document the mapping from the old spelling; preserve receiver-owned tenant binding and explain which surface is normative.

## Acceptance

After revision, the log contract and monitoring example identify the same receiver-owned tenant-header configuration.

## Verification scenarios

- Old tenant_header wording and extra_headers example have an explicit single meaning or migration mapping.
- The request cannot select a different tenant header.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/datasources/logs/v1alpha1/semantics.md`
- cited: `docs/adapters/grafana.md`

Source locations: `contracts/datasources/logs/v1alpha1/semantics.md:24`; `docs/adapters/grafana.md:94`.

## Dependencies and edit coordination

No semantic prerequisite is declared within this draft set.

Shared edit surfaces with `story:contracts-wire-compatibility`, `story:contracts-anonymous-auth`, `story:contracts-log-continuation`, `story:contracts-discovery-coverage`, `story:contracts-host-composition`, `story:contracts-discovery-profiles`, `story:contracts-acquisition-profiles`, `story:contracts-evidence-precision`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This interactive, local-only story revises textual contracts, adapter design and their conformance scenarios; it does not implement runtime behavior, edit generated schemas, change ESS, commit/publish, or expand the implemented adapter set. Existing typed declarations are in `ess/system.yaml` and `ess/domains/declarations.yaml`; proposed runtime entities are not claimed to be modeled. Any later entity-bearing implementation decomposition must first use the ESS workflow for the settled semantics and retain unresolved relations as UNMAPPED. These documentation stories do not introduce a new typed entity or assert an unresolved cardinality.

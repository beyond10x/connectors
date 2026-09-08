---
format: aep.planning-md/1
id: story:contracts-host-composition
kind: story
status: draft
title: Assign mediated adapter wiring to an explicit composition owner
tags:
- P2
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
scope:
- confidence: cited
  path: contracts/discovery/mediated_route/v1alpha1/semantics.md
- confidence: cited
  path: docs/adapters/grafana.md
- confidence: cited
  path: docs/design.md
revision: 2
---
## Context

Priority: **P2**. Sources: `E03` in `specification:contract-review-intake-20260908`.

Assigning composition loading to generic server.rs leaves the owner of concrete adapter wiring unclear.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Document the composition boundary and dependency direction while retaining the no-host-to-concrete-adapter invariant. Colocation alone is not a contradiction: a composition executable may inject ports; do not force a new wire proxy or composition implementation in this story.

## Acceptance

After revision, the mediated composition diagram identifies a wiring owner without requiring the generic host to import a concrete adapter.

## Verification scenarios

- One executable links parent and child implementations and injects the generic host ports.
- Each adapter remains independently usable.
- Unsupported placement fails explicitly rather than introducing an accidental generic proxy.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/discovery/mediated_route/v1alpha1/semantics.md`
- cited: `docs/adapters/grafana.md`
- cited: `docs/design.md`

Source locations: `contracts/discovery/mediated_route/v1alpha1/semantics.md:67`; `contracts/discovery/mediated_route/v1alpha1/semantics.md:99`; `docs/adapters/grafana.md:11`; `docs/design.md:158`; `docs/design.md:253`; `docs/design.md:798`.

## Dependencies and edit coordination

No semantic prerequisite is declared within this draft set.

Shared edit surfaces with `story:contracts-wire-compatibility`, `story:contracts-anonymous-auth`, `story:contracts-log-continuation`, `story:contracts-discovery-coverage`, `story:contracts-management-boundary`, `story:contracts-discovery-profiles`, `story:contracts-acquisition-profiles`, `story:contracts-documentation-index`, `story:contracts-evidence-precision`, `story:contracts-persistence-ownership`, `story:contracts-tenant-header`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This interactive, local-only story revises textual contracts, adapter design and their conformance scenarios; it does not implement runtime behavior, edit generated schemas, change ESS, commit/publish, or expand the implemented adapter set. Existing typed declarations are in `ess/system.yaml` and `ess/domains/declarations.yaml`; proposed runtime entities are not claimed to be modeled. Any later entity-bearing implementation decomposition must first use the ESS workflow for the settled semantics and retain unresolved relations as UNMAPPED. These documentation stories do not introduce a new typed entity or assert an unresolved cardinality.

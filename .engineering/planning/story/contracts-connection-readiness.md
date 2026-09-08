---
format: aep.planning-md/1
id: story:contracts-connection-readiness
kind: story
status: draft
title: Separate connection viability from per-operation eligibility
tags:
- P2
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
- depends_on: story:contracts-credential-evidence
scope:
- confidence: cited
  path: contracts/auth/acquisition/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/connection/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/evidence/v1alpha1/semantics.md
revision: 2
---
## Context

Priority: **P2**. Sources: `F07`, `E19`, `E21` in `specification:contract-review-intake-20260908`.

Missing write scope globally blocks a connection despite a required successful read; pending/disabled and scope failure names disagree.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Define one readiness reduction and canonical state/error vocabulary across public tables, conformance and future-model notes; preserve per-invocation authority checks. This repairs prose, not an ESS lifecycle implementation.

## Acceptance

After revision, each readiness scenario yields the same eligibility and state name across connection, acquisition and evidence documents.

## Verification scenarios

- Read with required scope present and write scope absent → permitted read and refused write.
- Pending, disabled, custody failure and revoked each have one globally or operation-locally defined consequence.
- Use one canonical insufficient_scope name wherever that same failure is meant.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/auth/connection/v1alpha1/semantics.md`
- cited: `contracts/auth/evidence/v1alpha1/semantics.md`
- cited: `contracts/auth/acquisition/v1alpha1/semantics.md`

Source locations: `contracts/auth/connection/v1alpha1/semantics.md:58`; `contracts/auth/connection/v1alpha1/semantics.md:74`; `contracts/auth/connection/v1alpha1/semantics.md:127`; `contracts/auth/evidence/v1alpha1/semantics.md:88`; `contracts/auth/acquisition/v1alpha1/semantics.md:47`; `contracts/auth/acquisition/v1alpha1/semantics.md:61`.

## Dependencies and edit coordination

Semantic prerequisites: `story:contracts-credential-evidence`.

Shared edit surfaces with `story:contracts-refresh-coordination`, `story:contracts-credential-evidence`, `story:contracts-wire-compatibility`, `story:contracts-permission-budgets`, `story:contracts-read-refresh-retry`, `story:contracts-management-boundary`, `story:contracts-acquisition-profiles`, `story:contracts-media-controls`, `story:contracts-persistence-ownership`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This interactive, local-only story revises textual contracts, adapter design and their conformance scenarios; it does not implement runtime behavior, edit generated schemas, change ESS, commit/publish, or expand the implemented adapter set. Existing typed declarations are in `ess/system.yaml` and `ess/domains/declarations.yaml`; proposed runtime entities are not claimed to be modeled. Any later entity-bearing implementation decomposition must first use the ESS workflow for the settled semantics and retain unresolved relations as UNMAPPED. These documentation stories do not introduce a new typed entity or assert an unresolved cardinality.

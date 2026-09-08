---
format: aep.planning-md/1
id: story:contracts-refresh-coordination
kind: story
status: draft
title: Specify refresh exclusion and recovery after owner loss
tags:
- P1
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
scope:
- confidence: cited
  path: contracts/auth/acquisition/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/custody/v1alpha1/semantics.md
- confidence: cited
  path: docs/adapters/atlassian.md
revision: 2
---
## Context

Priority: **P1**. Sources: `F04` in `specification:contract-review-intake-20260908`.

Acquisition requires a custody lease although custody promises only immutable versions and CAS; lease takeover can repeat an uncertain rotating refresh.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Assign the coordinator and its required atomic operations without choosing a backend; define exclusion before exchange, durable attempt ownership, stale-owner fencing and uncertain-exchange recovery.

## Acceptance

After revision, the refresh failure matrix permits no second rotating exchange while the previous exchange may have consumed the token.

## Verification scenarios

- Two replicas contend before exchange; only one is authorized to send.
- Owner loss after send but before publication → documented uncertain/recovery path, never takeover-based replay.
- Stale owner attempts publication after successor takeover; publication and revocation race.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/auth/acquisition/v1alpha1/semantics.md`
- cited: `contracts/auth/custody/v1alpha1/semantics.md`
- cited: `docs/adapters/atlassian.md`

Source locations: `contracts/auth/acquisition/v1alpha1/semantics.md:78`; `contracts/auth/custody/v1alpha1/semantics.md:57`; `docs/adapters/atlassian.md:56`.

## Dependencies and edit coordination

No semantic prerequisite is declared within this draft set.

Shared edit surfaces with `story:contracts-idempotency-scope`, `story:contracts-wire-compatibility`, `story:contracts-connection-readiness`, `story:contracts-document-admission`, `story:contracts-read-refresh-retry`, `story:contracts-management-boundary`, `story:contracts-acquisition-profiles`, `story:contracts-evidence-precision`, `story:contracts-persistence-ownership`, `story:contracts-mutation-visibility`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This interactive, local-only story revises textual contracts, adapter design and their conformance scenarios; it does not implement runtime behavior, edit generated schemas, change ESS, commit/publish, or expand the implemented adapter set. Existing typed declarations are in `ess/system.yaml` and `ess/domains/declarations.yaml`; proposed runtime entities are not claimed to be modeled. Any later entity-bearing implementation decomposition must first use the ESS workflow for the settled semantics and retain unresolved relations as UNMAPPED. These documentation stories do not introduce a new typed entity or assert an unresolved cardinality.

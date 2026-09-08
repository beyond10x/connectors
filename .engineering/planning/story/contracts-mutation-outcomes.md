---
format: aep.planning-md/1
id: story:contracts-mutation-outcomes
kind: story
status: draft
title: Make mutation outcomes truthful across dispatch and recovery
tags:
- P1
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
scope:
- confidence: cited
  path: contracts/operations/v1alpha1/semantics.md
revision: 2
---
## Context

Priority: **P1**. Sources: `F01`, `E01`, `E11` in `specification:contract-review-intake-20260908`.

Approval spending cannot prove dispatch; deadline handling and the single-code error vocabulary do not consistently express attempt state.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Define the normative outcome table independently of approval; distinguish an error cause from attempt status without prematurely choosing a wire field. Assign service and transport deadline obligations in §8 and settle an in-flight duplicate wait expiring after another attempt may dispatch. Current read-only code is evidence of future binding work, not a claim that a mutation implementation is already broken; wire encoding belongs to contracts-wire-compatibility.

## Acceptance

After revision, the mutation outcome table gives one truthful classification for every listed dispatch, deadline and recovery scenario.

## Verification scenarios

- Crash before dispatch with durable proof → aborted; uncertain dispatch → indeterminate, including approval:not_required.
- Crash after spend before send and after send before outcome cannot be distinguished without further evidence.
- Deadline before send versus response loss after send; duplicate waiter deadline must not imply that the original attempt never ran.
- Store unavailable before dispatch has an unambiguous cause and attempt classification.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/operations/v1alpha1/semantics.md`

Source locations: `contracts/operations/v1alpha1/semantics.md:90`; `contracts/operations/v1alpha1/semantics.md:102`; `contracts/operations/v1alpha1/semantics.md:111`; `contracts/operations/v1alpha1/semantics.md:120`; `contracts/operations/v1alpha1/semantics.md:131`; `crates/connectors-host/src/server.rs:142`; `crates/connectors-host/src/http.rs:145`.

## Dependencies and edit coordination

No semantic prerequisite is declared within this draft set.

Shared edit surfaces with `story:contracts-idempotency-scope`, `story:contracts-federated-approval`, `story:contracts-wire-compatibility`, `story:contracts-mutation-classification`, `story:contracts-restart-idempotency`, `story:contracts-evidence-precision`, `story:contracts-persistence-ownership`, `story:contracts-mutation-visibility`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This interactive, local-only story revises textual contracts, adapter design and their conformance scenarios; it does not implement runtime behavior, edit generated schemas, change ESS, commit/publish, or expand the implemented adapter set. Existing typed declarations are in `ess/system.yaml` and `ess/domains/declarations.yaml`; proposed runtime entities are not claimed to be modeled. Any later entity-bearing implementation decomposition must first use the ESS workflow for the settled semantics and retain unresolved relations as UNMAPPED. These documentation stories do not introduce a new typed entity or assert an unresolved cardinality.

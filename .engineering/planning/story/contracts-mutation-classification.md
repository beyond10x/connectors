---
format: aep.planning-md/1
id: story:contracts-mutation-classification
kind: story
status: draft
title: Align SIP mutation effects with the shared discriminator
tags:
- P2
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
- depends_on: story:contracts-mutation-outcomes
scope:
- confidence: cited
  path: contracts/operations/v1alpha1/semantics.md
- confidence: cited
  path: contracts/sessions/v1alpha1/semantics.md
- confidence: cited
  path: docs/adapters/media-session.md
revision: 2
---
## Context

Priority: **P2**. Sources: `F10`, `E06` in `specification:contract-review-intake-20260908`.

SIP dial omits required external_write and places human_visible in the executable effects list.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Choose whether external_write includes session establishment or other explicit effects qualify independently; align SIP declarations and conformance, keeping human visibility in semantic_effects.

## Acceptance

After revision, the planned SIP dial satisfies the same mutation classification rule that its validator obligation requires.

## Verification scenarios

- SIP dial accepted as a correctly declared mutation.
- A declaration with only descriptive human visibility cannot acquire mutation authority.
- Unsupported or incomplete effect declarations fail before execution.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/operations/v1alpha1/semantics.md`
- cited: `docs/adapters/media-session.md`
- cited: `contracts/sessions/v1alpha1/semantics.md`

Source locations: `contracts/operations/v1alpha1/semantics.md:51`; `contracts/operations/v1alpha1/semantics.md:155`; `docs/adapters/media-session.md:37`; `docs/adapters/media-session.md:63`; `contracts/sessions/v1alpha1/semantics.md:65`.

## Dependencies and edit coordination

Semantic prerequisites: `story:contracts-mutation-outcomes`.

Shared edit surfaces with `story:contracts-mutation-outcomes`, `story:contracts-idempotency-scope`, `story:contracts-federated-approval`, `story:contracts-session-revocation`, `story:contracts-wire-compatibility`, `story:contracts-restart-idempotency`, `story:contracts-evidence-precision`, `story:contracts-media-controls`, `story:contracts-persistence-ownership`, `story:contracts-mutation-visibility`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This interactive, local-only story revises textual contracts, adapter design and their conformance scenarios; it does not implement runtime behavior, edit generated schemas, change ESS, commit/publish, or expand the implemented adapter set. Existing typed declarations are in `ess/system.yaml` and `ess/domains/declarations.yaml`; proposed runtime entities are not claimed to be modeled. Any later entity-bearing implementation decomposition must first use the ESS workflow for the settled semantics and retain unresolved relations as UNMAPPED. These documentation stories do not introduce a new typed entity or assert an unresolved cardinality.

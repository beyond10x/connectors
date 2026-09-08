---
format: aep.planning-md/1
id: story:contracts-session-revocation
kind: story
status: draft
title: Define bounded traffic cessation and terminal media reasons
tags:
- P1
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
scope:
- confidence: cited
  path: contracts/media/v1alpha1/semantics.md
- confidence: cited
  path: contracts/sessions/v1alpha1/semantics.md
- confidence: cited
  path: docs/adapters/media-session.md
revision: 2
---
## Context

Priority: **P1**. Sources: `F06`, `E20` in `specification:contract-review-intake-20260908`.

Entering closing does not bound continuing media access; media_incompatible is absent from the sessions reason vocabulary while media_overload is already present.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Define a single revocation/closing traffic policy with bounded queue drain and teardown, covering direct paths and unresponsive peers; align shared terminal reasons. Numeric bounds must be selected or explicitly classified, not inferred from maximum call duration.

## Acceptance

After revision, the session termination table defines a bounded data cutoff and a valid terminal reason for every listed revocation or media-failure scenario.

## Verification scenarios

- Peer ignores close → data cessation within the declared bound independent of peer cooperation.
- Queued input/output at revocation has explicit treatment; direct paths enforce the same authority cutoff.
- Incompatible negotiation and overload use reasons admitted by the shared sessions vocabulary.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/sessions/v1alpha1/semantics.md`
- cited: `contracts/media/v1alpha1/semantics.md`
- cited: `docs/adapters/media-session.md`

Source locations: `contracts/sessions/v1alpha1/semantics.md:84`; `contracts/sessions/v1alpha1/semantics.md:101`; `contracts/media/v1alpha1/semantics.md:62`; `contracts/media/v1alpha1/semantics.md:67`; `docs/adapters/media-session.md:89`.

## Dependencies and edit coordination

No semantic prerequisite is declared within this draft set.

Shared edit surfaces with `story:contracts-wire-compatibility`, `story:contracts-mutation-classification`, `story:contracts-evidence-precision`, `story:contracts-media-controls`, `story:contracts-supported-vocabulary`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This interactive, local-only story revises textual contracts, adapter design and their conformance scenarios; it does not implement runtime behavior, edit generated schemas, change ESS, commit/publish, or expand the implemented adapter set. Existing typed declarations are in `ess/system.yaml` and `ess/domains/declarations.yaml`; proposed runtime entities are not claimed to be modeled. Any later entity-bearing implementation decomposition must first use the ESS workflow for the settled semantics and retain unresolved relations as UNMAPPED. These documentation stories do not introduce a new typed entity or assert an unresolved cardinality.

---
format: aep.planning-md/1
id: story:contracts-media-controls
kind: story
status: draft
title: Separate media operations, transport controls and redemption evidence
tags:
- P2
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
scope:
- confidence: cited
  path: contracts/README.md
- confidence: cited
  path: contracts/auth/capability/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/evidence/v1alpha1/semantics.md
- confidence: cited
  path: contracts/media/v1alpha1/semantics.md
- confidence: cited
  path: contracts/sessions/v1alpha1/semantics.md
- confidence: cited
  path: docs/adapters/media-session.md
revision: 2
---
## Context

Priority: **P2**. Sources: `E23`, `E24` in `specification:contract-review-intake-20260908`.

Media controls are listed as operation IDs, and redemption ledger is listed as if it were a readiness evidence check.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Separate ordinary operations from duplex transport messages and place one-shot redemption under the capability that owns it; use only actual evidence-check names in evidence columns.

## Acceptance

After revision, every media adapter table entry maps to its declared operation, control-message or evidence/capability boundary.

## Verification scenarios

- close/signal/interrupt map to transport controls unless an explicit separately admitted operation is deliberately specified.
- Redemption is enforced by inbound/session authority capability rather than invented auth.evidence vocabulary.
- Discovery and auth matrices expose the correctly classified capabilities.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `docs/adapters/media-session.md`
- cited: `contracts/README.md`
- cited: `contracts/auth/capability/v1alpha1/semantics.md`
- cited: `contracts/auth/evidence/v1alpha1/semantics.md`
- cited: `contracts/sessions/v1alpha1/semantics.md`
- cited: `contracts/media/v1alpha1/semantics.md`

Source locations: `docs/adapters/media-session.md:65`; `docs/adapters/media-session.md:75`; `contracts/README.md:52`; `contracts/auth/capability/v1alpha1/semantics.md:51`; `contracts/auth/evidence/v1alpha1/semantics.md:12`; `contracts/sessions/v1alpha1/semantics.md:61`; `contracts/media/v1alpha1/semantics.md:58`.

## Dependencies and edit coordination

No semantic prerequisite is declared within this draft set.

Shared edit surfaces with `story:contracts-credential-evidence`, `story:contracts-session-revocation`, `story:contracts-wire-compatibility`, `story:contracts-connection-readiness`, `story:contracts-permission-budgets`, `story:contracts-anonymous-auth`, `story:contracts-mutation-classification`, `story:contracts-read-refresh-retry`, `story:contracts-acquisition-profiles`, `story:contracts-documentation-index`, `story:contracts-evidence-precision`, `story:contracts-supported-vocabulary`, `story:contracts-persistence-ownership`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This interactive, local-only story revises textual contracts, adapter design and their conformance scenarios; it does not implement runtime behavior, edit generated schemas, change ESS, commit/publish, or expand the implemented adapter set. Existing typed declarations are in `ess/system.yaml` and `ess/domains/declarations.yaml`; proposed runtime entities are not claimed to be modeled. Any later entity-bearing implementation decomposition must first use the ESS workflow for the settled semantics and retain unresolved relations as UNMAPPED. These documentation stories do not introduce a new typed entity or assert an unresolved cardinality.

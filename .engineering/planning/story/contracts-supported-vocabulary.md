---
format: aep.planning-md/1
id: story:contracts-supported-vocabulary
kind: story
status: draft
title: Record support and disposition for reserved vocabulary
tags:
- P3
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
scope:
- confidence: cited
  path: contracts/auth/capability/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/profile/v1alpha1/semantics.md
- confidence: cited
  path: contracts/datasources/series/v1alpha1/semantics.md
- confidence: cited
  path: contracts/discovery/resources/v1alpha1/semantics.md
- confidence: cited
  path: contracts/media/v1alpha1/semantics.md
revision: 2
---
## Context

Priority: **P3**. Sources: `E27` in `specification:contract-review-intake-20260908`.

Reserved names may imply support or create unnecessary abstractions, but reservation alone does not prove an architectural violation.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Inventory the cited reserved names and record supported, explicitly reserved/refused, or removed disposition with a requirement/source rationale. Do not delete a name needed by a selected adapter solely because the reviewer proposed blanket removal.

## Acceptance

After revision, each cited reserved name has an explicit support disposition that cannot be mistaken for advertised implementation.

## Verification scenarios

- http_signing, oauth2_password, hold/transfer, instant/labels and address locator each have an explicit outcome.
- Selected provider requirements justify retention; reserved-only names remain unadvertised/refused.
- Unsupported semantics are not modeled as implemented.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/auth/profile/v1alpha1/semantics.md`
- cited: `contracts/auth/capability/v1alpha1/semantics.md`
- cited: `contracts/media/v1alpha1/semantics.md`
- cited: `contracts/datasources/series/v1alpha1/semantics.md`
- cited: `contracts/discovery/resources/v1alpha1/semantics.md`

Source locations: `contracts/auth/profile/v1alpha1/semantics.md:64`; `contracts/auth/profile/v1alpha1/semantics.md:65`; `contracts/auth/profile/v1alpha1/semantics.md:103`; `contracts/auth/capability/v1alpha1/semantics.md:12`; `contracts/media/v1alpha1/semantics.md:13`; `contracts/datasources/series/v1alpha1/semantics.md:12`; `contracts/discovery/resources/v1alpha1/semantics.md:62`; `docs/design.md:49`.

## Dependencies and edit coordination

No semantic prerequisite is declared within this draft set.

Shared edit surfaces with `story:contracts-credential-evidence`, `story:contracts-session-revocation`, `story:contracts-wire-compatibility`, `story:contracts-anonymous-auth`, `story:contracts-discovery-coverage`, `story:contracts-read-refresh-retry`, `story:contracts-discovery-profiles`, `story:contracts-acquisition-profiles`, `story:contracts-evidence-precision`, `story:contracts-media-controls`, `story:contracts-persistence-ownership`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This interactive, local-only story revises textual contracts, adapter design and their conformance scenarios; it does not implement runtime behavior, edit generated schemas, change ESS, commit/publish, or expand the implemented adapter set. Existing typed declarations are in `ess/system.yaml` and `ess/domains/declarations.yaml`; proposed runtime entities are not claimed to be modeled. Any later entity-bearing implementation decomposition must first use the ESS workflow for the settled semantics and retain unresolved relations as UNMAPPED. These documentation stories do not introduce a new typed entity or assert an unresolved cardinality.

---
format: aep.planning-md/1
id: story:contracts-wire-compatibility
kind: story
status: draft
title: Define versioned compatibility for proposed contract extensions
tags:
- P1
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
- depends_on: story:contracts-mutation-outcomes
scope:
- confidence: cited
  path: contracts/auth/acquisition/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/capability/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/connection/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/evidence/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/profile/v1alpha1/semantics.md
- confidence: cited
  path: contracts/datasources/logs/v1alpha1/semantics.md
- confidence: cited
  path: contracts/datasources/records/v1alpha1/semantics.md
- confidence: cited
  path: contracts/datasources/series/v1alpha1/semantics.md
- confidence: cited
  path: contracts/discovery/mediated_route/v1alpha1/semantics.md
- confidence: cited
  path: contracts/discovery/resources/v1alpha1/semantics.md
- confidence: cited
  path: contracts/media/v1alpha1/semantics.md
- confidence: cited
  path: contracts/operations/v1alpha1/semantics.md
- confidence: cited
  path: contracts/service/v1alpha1/semantics.md
- confidence: cited
  path: contracts/sessions/v1alpha1/semantics.md
revision: 2
---
## Context

Priority: **P1**. Sources: `E02` in `specification:contract-review-intake-20260908`.

New descriptor fields and closed error codes are described beside no-wire-change claims; strict v1alpha1 readers reject them.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Produce one compatibility matrix covering the proposed families and the already proposed v1alpha2 envelope; distinguish contract/profile versions from wire versions, define old-reader refusal and negotiation, and assign binding obligations. Preserve valid no-change claims only for explicitly unchanged configured/read profiles. Other stories own behavior; this story owns its transport/version disposition.

## Acceptance

After revision, every proposed public field, error and profile extension has an explicit wire/version disposition in the compatibility matrix.

## Verification scenarios

- Old reader/new descriptor and new error → declared refusal or explicitly supported representation.
- New reader/old profile retains old semantics; unsupported profile is not silently downgraded.
- Outcome classifications from contracts-mutation-outcomes have an unambiguous representable encoding.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/service/v1alpha1/semantics.md`
- cited: `contracts/operations/v1alpha1/semantics.md`
- cited: `contracts/auth/connection/v1alpha1/semantics.md`
- cited: `contracts/auth/profile/v1alpha1/semantics.md`
- cited: `contracts/auth/acquisition/v1alpha1/semantics.md`
- cited: `contracts/auth/capability/v1alpha1/semantics.md`
- cited: `contracts/auth/evidence/v1alpha1/semantics.md`
- cited: `contracts/sessions/v1alpha1/semantics.md`
- cited: `contracts/media/v1alpha1/semantics.md`
- cited: `contracts/discovery/resources/v1alpha1/semantics.md`
- cited: `contracts/discovery/mediated_route/v1alpha1/semantics.md`
- cited: `contracts/datasources/records/v1alpha1/semantics.md`
- cited: `contracts/datasources/logs/v1alpha1/semantics.md`
- cited: `contracts/datasources/series/v1alpha1/semantics.md`

Source locations: `contracts/auth/capability/v1alpha1/semantics.md:99`; `contracts/auth/connection/v1alpha1/semantics.md:111`; `contracts/auth/profile/v1alpha1/semantics.md:110`; `contracts/sessions/v1alpha1/semantics.md:71`; `contracts/discovery/mediated_route/v1alpha1/semantics.md:55`; `contracts/operations/v1alpha1/semantics.md:144`; `crates/connectors-core/src/lib.rs:13`; `crates/connectors-core/src/lib.rs:31`; `crates/connectors-core/src/lib.rs:71`.

## Dependencies and edit coordination

Semantic prerequisites: `story:contracts-mutation-outcomes`.

Shared edit surfaces with `story:contracts-mutation-outcomes`, `story:contracts-idempotency-scope`, `story:contracts-federated-approval`, `story:contracts-refresh-coordination`, `story:contracts-credential-evidence`, `story:contracts-session-revocation`, `story:contracts-connection-readiness`, `story:contracts-permission-budgets`, `story:contracts-anonymous-auth`, `story:contracts-mutation-classification`, `story:contracts-restart-idempotency`, `story:contracts-log-continuation`, `story:contracts-discovery-coverage`, `story:contracts-document-admission`, `story:contracts-read-refresh-retry`, `story:contracts-host-composition`, `story:contracts-management-boundary`, `story:contracts-discovery-profiles`, `story:contracts-acquisition-profiles`, `story:contracts-evidence-precision`, `story:contracts-media-controls`, `story:contracts-supported-vocabulary`, `story:contracts-persistence-ownership`, `story:contracts-tenant-header`, `story:contracts-mutation-visibility`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This interactive, local-only story revises textual contracts, adapter design and their conformance scenarios; it does not implement runtime behavior, edit generated schemas, change ESS, commit/publish, or expand the implemented adapter set. Existing typed declarations are in `ess/system.yaml` and `ess/domains/declarations.yaml`; proposed runtime entities are not claimed to be modeled. Any later entity-bearing implementation decomposition must first use the ESS workflow for the settled semantics and retain unresolved relations as UNMAPPED. These documentation stories do not introduce a new typed entity or assert an unresolved cardinality.

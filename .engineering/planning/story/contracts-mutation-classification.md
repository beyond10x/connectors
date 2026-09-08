---
format: aep.planning-md/1
id: story:contracts-mutation-classification
kind: story
status: implemented
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
- confidence: inferred
  path: docs/adapters/atlassian.md
- confidence: cited
  path: docs/adapters/media-session.md
- confidence: inferred
  path: docs/evidence/mutation-profiles-20260908
- confidence: inferred
  path: ess/domains/mutations.yaml
revision: 12
---
## Context

Priority: **P2**. Sources: `F10`, `E06` in `specification:contract-review-intake-20260908`.

SIP dial omits required external_write and places human_visible in the executable effects list.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Keep external_write as the mandatory discriminator for externally observable business changes, including an outbound call. Separate executable from descriptive effects and update SIP/Atlassian examples. Define the SIP business-effect commitment separately from full stream/application readiness, preserving applied evidence after delivery/readiness failure, conservative partial/unknown outcomes and the no-redial rule. Invalid declarations must be refused before advertisement or dispatch; future validation is distinct from current ESS value-shape checks.

## Acceptance

Every SIP and related mutation declaration uses the shared effect vocabulary and discriminator, and every dial/readiness/terminal scenario has one consistent business-effect classification and ready-handle rule across operations, sessions and the adapter document.

## Verification scenarios

- Correct SIP declaration; descriptive-only hints cannot authorize a mutation; network read remains a read; missing, unknown, unsupported or incompatible effects refuse before advertisement/dispatch.
- Pre-gate refusal; definitive no-effect refusal; possible dial/lost answer; known ringing without definitive establishment; confirmed SIP establishment followed by application failure; full ready receipt; receipt delivery loss; cancellation and later termination preserve effect knowledge.
- ESS validates minimal values and deterministic schemas; invalid shapes refuse and schema-valid semantic contradictions explicitly demonstrate predicate limits. Existing runtime regression gates do not execute these new semantics.
- Two independent reviews and exact-source evidence are required before completion.

## Scope

- cited: `contracts/operations/v1alpha1/semantics.md`
- cited: `contracts/sessions/v1alpha1/semantics.md`
- cited: `docs/adapters/media-session.md`
- inferred: `docs/adapters/atlassian.md`
- inferred: `ess/domains/mutations.yaml`
- inferred: `docs/evidence/mutation-profiles-20260908`

Serialized with restart/visibility owners on shared operation and adapter documents; the two initial reports also contain findings that remain assigned to those later stories.

## Dependencies and edit coordination

Semantic prerequisites: `story:contracts-mutation-outcomes`.

Shared edit surfaces with `story:contracts-mutation-outcomes`, `story:contracts-idempotency-scope`, `story:contracts-federated-approval`, `story:contracts-session-revocation`, `story:contracts-wire-compatibility`, `story:contracts-restart-idempotency`, `story:contracts-evidence-precision`, `story:contracts-media-controls`, `story:contracts-persistence-ownership`, `story:contracts-mutation-visibility`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

The operator authorized local semantic hardening together with ESS models and local checkpoints. This story updates the shared effect discriminator, SIP effect knowledge versus ready-session publication, and matching adapter declarations. Minimal private values belong in the existing mutations domain; OperationDeclaration extensions and predicate/runtime enforcement remain explicitly UNMAPPED. No runtime, adapter-kind schema, public codec or adapter implementation is added. Root is the sole tracked-file writer in the primary checkout under the repository single-agent rule; independent reviewers write only isolated ignored evidence. All work and backup remain local. Restart and visibility findings stay with their own stories.

## Completion evidence — 2026-09-08

F10/E06 and MP-A-01/02, MP-B-01/02, MP-B-C01 are fixed in the normative contracts and minimal ESS values. Both final independent classification-only reviews approve; exact reports/snapshots and five journal outcomes are preserved. See docs/evidence/mutation-profiles-20260908/classification-verification.md and classification-dispositions.md. Runtime and source-specific protocol proof are future binding obligations. Initial restart/visibility findings remain assigned to their own stories and are not included in this completion.

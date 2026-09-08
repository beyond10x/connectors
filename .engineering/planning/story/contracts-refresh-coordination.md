---
format: aep.planning-md/1
id: story:contracts-refresh-coordination
kind: story
status: active
title: Specify refresh exclusion and recovery after owner loss
tags:
- P1
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
scope:
- confidence: inferred
  path: contracts/auth/acquisition/v1alpha1/scenarios
- confidence: cited
  path: contracts/auth/acquisition/v1alpha1/semantics.md
- confidence: inferred
  path: contracts/auth/acquisition/v1alpha1/verification.md
- confidence: cited
  path: contracts/auth/custody/v1alpha1/semantics.md
- confidence: cited
  path: docs/adapters/atlassian.md
- confidence: inferred
  path: ess/domains/refresh.yaml
revision: 9
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

Derived 2026-09-08 by `aep-drive:story-scoper`; coordinator records the approved allocation.

- **cited:** `contracts/auth/acquisition/v1alpha1/semantics.md` — existing story surface.
- **cited:** `contracts/auth/custody/v1alpha1/semantics.md` — existing story surface.
- **cited:** `docs/adapters/atlassian.md` — existing story surface.
- **inferred:** `ess/domains/refresh.yaml` — private ESS/model verification authoring.
- **inferred:** `contracts/auth/acquisition/v1alpha1/scenarios` — private ESS/model verification authoring.
- **inferred:** `contracts/auth/acquisition/v1alpha1/verification.md` — private ESS/model verification authoring.

- **Confidence: medium (inferred):** existing prose is cited; the modeling decomposition follows the approved semantic wave.
- **Would collide (inferred):** other edits to these exact paths. Shared credential identity, ESS registration and gate changes belong to coordinator preparation, completed before the private branches fork.
- **Coordinator allocation (inferred):** `ess/domains/credentials.yaml`, `ess/system.yaml`, and `crates/connectors-build/src/gate.rs` are prerequisites supplied by `specification:auth-hardening-wave-20260908`, not concurrent unit edit surfaces. Scopers originally named this integration overlap; it is explicitly serialized, not claimed disjoint.

## Dependencies and edit coordination

No semantic prerequisite is declared within this draft set.

Shared edit surfaces with `story:contracts-idempotency-scope`, `story:contracts-wire-compatibility`, `story:contracts-connection-readiness`, `story:contracts-document-admission`, `story:contracts-read-refresh-retry`, `story:contracts-management-boundary`, `story:contracts-acquisition-profiles`, `story:contracts-evidence-precision`, `story:contracts-persistence-ownership`, `story:contracts-mutation-visibility`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

The operator approved F04 and F05 as a two-story semantic hardening wave, with local commits and merges and no model budget limit. This supersedes the previous ESS/commit exclusion. Scope includes textual contracts, adapter design, ESS declarations and authored conformance scenarios; it excludes runtime auth implementation, generated-schema changes, publication and adapter-set expansion.

The shared identity seam is `ess/domains/credentials.yaml`; `ess/system.yaml` registers each private domain. Unsettled relations and runtime predicates remain explicit `UNMAPPED` obligations. Authored scenarios are compiled, not executed runtime conformance. Each unit requires a textual failure/scenario matrix, pinned ESS validation/compilation and authored scenario compilation with zero refusals, adversarial review and the full integration gate. See `specification:auth-hardening-wave-20260908` for scope ownership and evidence.

---
format: aep.planning-md/1
id: story:contracts-mutation-visibility
kind: story
status: implemented
title: Distinguish implemented, enabled and discoverable mutations
tags:
- P2
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
- depends_on: story:contracts-wire-compatibility
- depends_on: story:contracts-mutation-classification
scope:
- confidence: cited
  path: contracts/operations/v1alpha1/semantics.md
- confidence: inferred
  path: contracts/service/compatibility.md
- confidence: cited
  path: contracts/service/v1alpha1/semantics.md
- confidence: inferred
  path: contracts/service/v1alpha2/semantics.md
- confidence: cited
  path: docs/adapters/atlassian.md
- confidence: inferred
  path: docs/adapters/docker.md
- confidence: inferred
  path: docs/evidence/restart-visibility-20260908
- confidence: inferred
  path: ess/domains/declarations.yaml
revision: 8
---
## Context

Priority: **P2**. Sources: `E12` in `specification:contract-review-intake-20260908`.

The reviewer infers disabled operations are unadvertised from an implemented-only rule, which does not establish that claim; the textual visibility/refusal policy still needs explicit cases.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Select one extended describe/lookup/admission matrix for static-bearer, identity-audience and delegated bindings. Keep implementation, binding support, enablement, metadata/result authority and dependency readiness separate. Define safe refusal precedence and exact-key replay before new-attempt readiness. Preserve actual legacy descriptor-only lookup and private adapter guard behavior without new public disabled fields.

## Acceptance

Every availability row has a consistent current-revision discovery and invocation result, with explicit auth/stale/refusal precedence, safe result disclosure, federation intersection and unchanged legacy projection behavior.

## Verification scenarios

Unimplemented/unbound; implemented-disabled; host-policy denial/unavailability; connection/resource denial; not-ready dependency; missing execution approval; exact admitted replay despite unavailable new-attempt dependencies; current/old descriptor revisions; guessed hidden ID; static shared principal and delegated leaf intersection. Read actual core/server/Kubernetes filter/guard; minimal private ESS facts/decision shapes do not implement policy; two independent reviewers and bounded verification.

## Scope

- cited: `contracts/operations/v1alpha1/semantics.md`
- cited: `contracts/service/v1alpha1/semantics.md`
- cited: `docs/adapters/atlassian.md`

Source locations: `contracts/operations/v1alpha1/semantics.md:98`; `contracts/service/v1alpha1/semantics.md:19`; `docs/adapters/atlassian.md:111`; `crates/connectors-core/src/lib.rs:86`; `docs/design.md:350`.

- inferred: `contracts/service/v1alpha2/semantics.md`
- inferred: `contracts/service/compatibility.md`
- inferred: `docs/adapters/docker.md`
- inferred: `ess/domains/declarations.yaml`
- inferred: `docs/evidence/restart-visibility-20260908`

## Dependencies and edit coordination

Semantic prerequisites: `story:contracts-wire-compatibility`, `story:contracts-mutation-classification`.

Shared edit surfaces with `story:contracts-mutation-outcomes`, `story:contracts-idempotency-scope`, `story:contracts-federated-approval`, `story:contracts-refresh-coordination`, `story:contracts-wire-compatibility`, `story:contracts-mutation-classification`, `story:contracts-restart-idempotency`, `story:contracts-document-admission`, `story:contracts-read-refresh-retry`, `story:contracts-evidence-precision`, `story:contracts-persistence-ownership`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

The operator authorized local semantic hardening together with ESS models and local checkpoints. Root is the sole tracked-file editor in the primary checkout under the repository single-agent rule; two independent reviewers use isolated ignored evidence. This story edits normative contracts, minimal private values and review evidence, with concrete unsupported predicates/relations kept UNMAPPED. No runtime, public codec, adapter-kind schema or additional adapter implementation is added. Work and recovery copies stay local. The original text-only/no-ESS/no-commit boundary is superseded by the authorized specification goal. Shared files are edited serially by root; restart and visibility keep separate finding ownership.

## Reviewed completion — 2026-09-08

The extended matrix covers static-bearer, identity-audience and delegated profiles with separate implementation/binding/enablement/metadata/lookup/result/readiness facts. Current authority precedes stale/private disclosure; permitted fresh bound/enabled lookup precedes operation schema and key observation; only new candidates require fresh approval/dependency preflight. Descriptions are metadata-only, federation intersects current admission, and legacy descriptor-vector lookup remains distinct. Three individual findings (MP-A-07, MP-B-06 and RV-A-01) are fixed; E12 closes in the original ledger.

Both final independent rechecks approve this story with zero residuals: review-result:restart-visibility-a-recheck2-20260908 and review-result:restart-visibility-b-recheck2-20260908. Exact reports, frozen sources and individual outcomes are preserved in docs/evidence/restart-visibility-20260908/dispositions.md and reviews/.

Verification: ESS 0.20.0 validates 13 files / 215 declarations; two 222-artifact projections match and seven private values have unchanged generated copies. Forty-six shape expectations include nine rejected shapes and nineteen accepted semantic counterexamples; forty-eight textual cases and eight exact provider archives distinguish selected behavior from execution. Existing full gate/MSRV1.88 passes 50 Rust tests and compiles 222 scenarios/34 authored; separate sessions compile 13/201. The narrow final correction changes an ESS comment only and compiles byte-identical IR, so the existing runtime gate was retained. No new runtime, public codec or adapter-kind schema implementation; provider/policy/storage predicates remain explicit later binding obligations. Root edited tracked files serially; work and checkpoints stay local.

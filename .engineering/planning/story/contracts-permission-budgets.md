---
format: aep.planning-md/1
id: story:contracts-permission-budgets
kind: story
status: implemented
title: Define authorization-check budgets for namespace fan-out
tags:
- P2
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
- depends_on: story:contracts-credential-evidence
scope:
- confidence: cited
  path: contracts/auth/evidence/v1alpha1/semantics.md
- confidence: inferred
  path: contracts/discovery/mediated_route/v1alpha1/semantics.md
- confidence: inferred
  path: contracts/discovery/resources/v1alpha1/semantics.md
- confidence: inferred
  path: contracts/service/compatibility.md
- confidence: cited
  path: docs/adapters/kubernetes.md
- confidence: inferred
  path: docs/evidence/auth-profile-budget-20260908
- confidence: inferred
  path: ess/domains/auth_access.yaml
- confidence: inferred
  path: ess/system.yaml
revision: 10
---
## Context

Priority: **P2**. Sources: `F08` in `specification:contract-review-intake-20260908`.

The one-check-per-invocation ceiling cannot satisfy exact checks for two uncached namespaces.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Define the check unit, maximum fan-out and exhaustion behavior while retaining per-target evidence and bounded provider work.

## Acceptance

After revision, the multi-namespace scenario fits the declared authorization-check budget without using unchecked namespaces.

## Verification scenarios

- Two uncached namespaces, one allowed and one denied → documented partial result.
- More authorization targets than the budget → explicit bounded outcome.
- Cached evidence is reused only within its exact valid target and credential scope.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/auth/evidence/v1alpha1/semantics.md`
- inferred: `contracts/discovery/mediated_route/v1alpha1/semantics.md`
- inferred: `contracts/discovery/resources/v1alpha1/semantics.md`
- inferred: `contracts/service/compatibility.md`
- cited: `docs/adapters/kubernetes.md`
- inferred: `docs/evidence/auth-profile-budget-20260908`
- inferred: `ess/domains/auth_access.yaml`
- inferred: `ess/system.yaml`

Root serializes tracked edits for the approved auth-access/acquisition/budget cluster; two independent reviewers write only ignored snapshots. Shared paths are coordinated here, not a concurrent implementation wave.

## Dependencies and edit coordination

Semantic prerequisites: `story:contracts-credential-evidence`.

Shared edit surfaces with `story:contracts-credential-evidence`, `story:contracts-wire-compatibility`, `story:contracts-connection-readiness`, `story:contracts-restart-idempotency`, `story:contracts-discovery-coverage`, `story:contracts-discovery-profiles`, `story:contracts-acquisition-profiles`, `story:contracts-evidence-precision`, `story:contracts-media-controls`, `story:contracts-persistence-ownership`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This approved local specification-hardening story revises contracts and adapter design, models settled values through ESS 0.20.0, and records explicit UNMAPPED semantic/ownership constraints and independent review evidence. Runtime code and adapter-kind schema changes remain out of scope. Generic schema checks do not execute admission or sequential scenarios. Local commits and the local recovery backup are authorized; no external publication. Root is the sole tracked-file editor in primary main under the repository override; two independent read-only reviewers retain ignored snapshots. These three existing stories are serialized through one editor, not a parallel implementation wave.

## Completion evidence

F08 are fixed for this approved semantic specification scope. Normative corrections, 44 declared textual traces, 50 schema expectations and exact ESS projection/gate limits are recorded in [verification](../../../docs/evidence/auth-profile-budget-20260908/verification.md) and [dispositions](../../../docs/evidence/auth-profile-budget-20260908/dispositions.md). Both immutable final independent review records auth-access-a-final-20260908 and auth-access-b-final-20260908 approve with zero remaining findings. ESS 0.20.0 validates 12 files/183 declarations; two 191-artifact projections match; 50 existing Rust tests/MSRV1.88 gate pass. This is not runtime execution of the new semantics. No implementation/adapter-kind schema change or external publication.

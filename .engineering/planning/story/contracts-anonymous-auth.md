---
format: aep.planning-md/1
id: story:contracts-anonymous-auth
kind: story
status: implemented
title: Represent deliberate anonymous and parent-authenticated access
tags:
- P2
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
- depends_on: story:contracts-wire-compatibility
scope:
- confidence: cited
  path: contracts/auth/capability/v1alpha1/semantics.md
- confidence: inferred
  path: contracts/auth/connection/v1alpha1/semantics.md
- confidence: inferred
  path: contracts/auth/evidence/v1alpha1/semantics.md
- confidence: inferred
  path: contracts/auth/management.md
- confidence: cited
  path: contracts/auth/profile/v1alpha1/semantics.md
- confidence: cited
  path: contracts/discovery/mediated_route/v1alpha1/semantics.md
- confidence: inferred
  path: contracts/discovery/resources/v1alpha1/semantics.md
- confidence: inferred
  path: contracts/service/compatibility.md
- confidence: cited
  path: docs/adapters/grafana.md
- confidence: inferred
  path: docs/evidence/auth-profile-budget-20260908
- confidence: inferred
  path: ess/domains/auth_access.yaml
- confidence: inferred
  path: ess/system.yaml
revision: 10
---
## Context

Priority: **P2**. Sources: `F09`, `E10` in `specification:contract-review-intake-20260908`.

Monitoring declares none profiles and null credentials that the closed auth/capability vocabularies cannot represent.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Choose an explicit representation for anonymous direct access and separate parent-authenticated mediation; state purpose/scheme/capability semantics and refuse credential-missing fallback.

## Acceptance

After revision, every monitoring auth row maps to an admitted profile with an explicit credential-placement rule.

## Verification scenarios

- Configured anonymous direct request proceeds under its own admitted profile.
- Missing bearer/basic material refuses without anonymous fallback.
- Mediated child uses parent authentication without acquiring or exposing the parent's secret.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/auth/capability/v1alpha1/semantics.md`
- inferred: `contracts/auth/connection/v1alpha1/semantics.md`
- inferred: `contracts/auth/evidence/v1alpha1/semantics.md`
- inferred: `contracts/auth/management.md`
- cited: `contracts/auth/profile/v1alpha1/semantics.md`
- cited: `contracts/discovery/mediated_route/v1alpha1/semantics.md`
- inferred: `contracts/discovery/resources/v1alpha1/semantics.md`
- inferred: `contracts/service/compatibility.md`
- cited: `docs/adapters/grafana.md`
- inferred: `docs/evidence/auth-profile-budget-20260908`
- inferred: `ess/domains/auth_access.yaml`
- inferred: `ess/system.yaml`

Root serializes tracked edits for the approved auth-access/acquisition/budget cluster; two independent reviewers write only ignored snapshots. Shared paths are coordinated here, not a concurrent implementation wave.

## Dependencies and edit coordination

Semantic prerequisites: `story:contracts-wire-compatibility`.

Shared edit surfaces with `story:contracts-credential-evidence`, `story:contracts-wire-compatibility`, `story:contracts-log-continuation`, `story:contracts-discovery-coverage`, `story:contracts-read-refresh-retry`, `story:contracts-host-composition`, `story:contracts-discovery-profiles`, `story:contracts-acquisition-profiles`, `story:contracts-evidence-precision`, `story:contracts-media-controls`, `story:contracts-supported-vocabulary`, `story:contracts-persistence-ownership`, `story:contracts-tenant-header`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This approved local specification-hardening story revises contracts and adapter design, models settled values through ESS 0.20.0, and records explicit UNMAPPED semantic/ownership constraints and independent review evidence. Runtime code and adapter-kind schema changes remain out of scope. Generic schema checks do not execute admission or sequential scenarios. Local commits and the local recovery backup are authorized; no external publication. Root is the sole tracked-file editor in primary main under the repository override; two independent read-only reviewers retain ignored snapshots. These three existing stories are serialized through one editor, not a parallel implementation wave.

## Completion evidence

F09/E10 are fixed for this approved semantic specification scope. Normative corrections, 44 declared textual traces, 50 schema expectations and exact ESS projection/gate limits are recorded in [verification](../../../docs/evidence/auth-profile-budget-20260908/verification.md) and [dispositions](../../../docs/evidence/auth-profile-budget-20260908/dispositions.md). Both immutable final independent review records auth-access-a-final-20260908 and auth-access-b-final-20260908 approve with zero remaining findings. ESS 0.20.0 validates 12 files/183 declarations; two 191-artifact projections match; 50 existing Rust tests/MSRV1.88 gate pass. This is not runtime execution of the new semantics. No implementation/adapter-kind schema change or external publication.

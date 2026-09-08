---
format: aep.planning-md/1
id: story:contracts-acquisition-profiles
kind: story
status: implemented
title: Align configured and OAuth acquisition profile requirements
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
  path: contracts/auth/acquisition/v1alpha1/semantics.md
- confidence: inferred
  path: contracts/auth/management.md
- confidence: cited
  path: contracts/auth/profile/v1alpha1/semantics.md
- confidence: inferred
  path: contracts/service/compatibility.md
- confidence: cited
  path: docs/adapters/docker.md
- confidence: cited
  path: docs/adapters/grafana.md
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

Priority: **P2**. Sources: `E09`, `E29` in `specification:contract-review-intake-20260908`.

static_config appears in auth.profile and adapter rows but has no acquisition profile semantics; the OAuth rule requires authorize_url even for client credentials.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Define deployment-supplied static configuration separately from interactive secret entry and state the supported/reserved flow matrix with flow-specific endpoint requirements.

## Acceptance

After revision, every advertised acquisition flow has a supported path whose endpoint requirements match that flow.

## Verification scenarios

- static_config binds deployment-supplied references without inventing an interactive acquisition flow.
- Authorization code requires its authorization/token endpoints and source evidence.
- Client credentials requires its token endpoint without an authorization endpoint.
- A reserved flow cannot be advertised as implemented.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/README.md`
- cited: `contracts/auth/acquisition/v1alpha1/semantics.md`
- inferred: `contracts/auth/management.md`
- cited: `contracts/auth/profile/v1alpha1/semantics.md`
- inferred: `contracts/service/compatibility.md`
- cited: `docs/adapters/docker.md`
- cited: `docs/adapters/grafana.md`
- cited: `docs/adapters/kubernetes.md`
- inferred: `docs/evidence/auth-profile-budget-20260908`
- inferred: `ess/domains/auth_access.yaml`
- inferred: `ess/system.yaml`

Root serializes tracked edits for the approved auth-access/acquisition/budget cluster; two independent reviewers write only ignored snapshots. Shared paths are coordinated here, not a concurrent implementation wave.

## Dependencies and edit coordination

No semantic prerequisite is declared within this draft set.

Shared edit surfaces with `story:contracts-refresh-coordination`, `story:contracts-credential-evidence`, `story:contracts-wire-compatibility`, `story:contracts-connection-readiness`, `story:contracts-permission-budgets`, `story:contracts-anonymous-auth`, `story:contracts-restart-idempotency`, `story:contracts-log-continuation`, `story:contracts-discovery-coverage`, `story:contracts-read-refresh-retry`, `story:contracts-host-composition`, `story:contracts-management-boundary`, `story:contracts-discovery-profiles`, `story:contracts-documentation-index`, `story:contracts-evidence-precision`, `story:contracts-media-controls`, `story:contracts-supported-vocabulary`, `story:contracts-persistence-ownership`, `story:contracts-tenant-header`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This approved local specification-hardening story revises contracts and adapter design, models settled values through ESS 0.20.0, and records explicit UNMAPPED semantic/ownership constraints and independent review evidence. Runtime code and adapter-kind schema changes remain out of scope. Generic schema checks do not execute admission or sequential scenarios. Local commits and the local recovery backup are authorized; no external publication. Root is the sole tracked-file editor in primary main under the repository override; two independent read-only reviewers retain ignored snapshots. These three existing stories are serialized through one editor, not a parallel implementation wave.

## Completion evidence

E09/E29 are fixed for this approved semantic specification scope. Normative corrections, 44 declared textual traces, 50 schema expectations and exact ESS projection/gate limits are recorded in [verification](../../../docs/evidence/auth-profile-budget-20260908/verification.md) and [dispositions](../../../docs/evidence/auth-profile-budget-20260908/dispositions.md). Both immutable final independent review records auth-access-a-final-20260908 and auth-access-b-final-20260908 approve with zero remaining findings. ESS 0.20.0 validates 12 files/183 declarations; two 191-artifact projections match; 50 existing Rust tests/MSRV1.88 gate pass. This is not runtime execution of the new semantics. No implementation/adapter-kind schema change or external publication.

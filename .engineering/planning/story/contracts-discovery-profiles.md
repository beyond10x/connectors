---
format: aep.planning-md/1
id: story:contracts-discovery-profiles
kind: story
status: implemented
title: Make discovery profile binding and Kubernetes recognition explicit
tags:
- P2
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
- depends_on: story:contracts-discovery-coverage
scope:
- confidence: inferred
  path: contracts/discovery/mediated_route/v1alpha1/semantics.md
- confidence: cited
  path: contracts/discovery/resources/v1alpha1/semantics.md
- confidence: inferred
  path: contracts/service/compatibility.md
- confidence: cited
  path: docs/adapters/grafana.md
- confidence: cited
  path: docs/adapters/kubernetes.md
- confidence: inferred
  path: docs/evidence/profile-persistence-20260908
- confidence: inferred
  path: ess/domains/discovery.yaml
revision: 12
---
## Context

Priority: **P2**. Sources: `E07`, `E14`, `E32` in `specification:contract-review-intake-20260908`.

Generic resources.observe with a caller profile differs from adapter-chosen operation IDs and descriptor-fixed profiles; Kubernetes matching and cluster identity are underspecified.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Define operation naming and receiver-owned profile selection, exact Argo CD identity recognition preserving the cited name-or-label rule, and the configured instance/API-origin identity boundary or an explicit deferred disposition. Argo recognition remains observational, not a new callable adapter.

## Acceptance

After revision, each discovery declaration resolves to one fixed profile and source scope with unambiguous recognition rules.

## Verification scenarios

- Wrong request profile cannot retarget an operation advertised for another profile.
- Argo API Service identified by exact name or exact stable app.kubernetes.io/name label; metrics/repo-server/redis are not mistaken for it.
- Distinct configured clusters have distinct source identity; cluster-identity requirements not supported yet are named as deferred.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- inferred: `contracts/discovery/mediated_route/v1alpha1/semantics.md`
- cited: `contracts/discovery/resources/v1alpha1/semantics.md`
- inferred: `contracts/service/compatibility.md`
- cited: `docs/adapters/grafana.md`
- cited: `docs/adapters/kubernetes.md`
- inferred: `docs/evidence/profile-persistence-20260908`
- inferred: `ess/domains/discovery.yaml`

## Dependencies and edit coordination

Semantic prerequisites: `story:contracts-discovery-coverage`.

Shared edit surfaces with `story:contracts-credential-evidence`, `story:contracts-wire-compatibility`, `story:contracts-permission-budgets`, `story:contracts-anonymous-auth`, `story:contracts-restart-idempotency`, `story:contracts-log-continuation`, `story:contracts-discovery-coverage`, `story:contracts-host-composition`, `story:contracts-acquisition-profiles`, `story:contracts-evidence-precision`, `story:contracts-supported-vocabulary`, `story:contracts-persistence-ownership`, `story:contracts-tenant-header`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This approved local specification-hardening story revises normative contracts/design and settled ESS values using pinned ESS 0.20.0, retaining explicit UNMAPPED implementation and persistent relation requirements. It introduces no runtime code or current adapter-kind schema change. The existing source findings and stories authorize this work; their earlier text-only/no-commit boundary is superseded by the operator’s hardening instructions. Root is the sole tracked editor in primary main; two independent reviewers write ignored snapshots only. No concurrent implementation wave or external publication. Existing persistent ESS entities are reused; this inventory does not invent lifecycles, delete semantics or a universal storage abstraction. Local checkpoint commits and local recovery backup remain authorized.

## Completion evidence

E07/E14/E32 is fixed for the approved semantic specification scope. Normative revisions, 43 declared textual traces, 44 schema expectations, actual ESS entity inventory and validation limits are recorded in [verification](../../../docs/evidence/profile-persistence-20260908/verification.md) and [dispositions](../../../docs/evidence/profile-persistence-20260908/dispositions.md). Both immutable final independent reviews profile-persistence-a-recheck1-20260908 and profile-persistence-b-recheck1-20260908 approve with zero residual findings. ESS 0.20.0 validates 13 files/204 declarations; two 211-artifact projections match; the existing full gate/MSRV1.88 passes 50 Rust tests. E32 explicitly selects configured authority identity and defers physical-cluster attestation. E28 selects eighteen logical port owners/atomic groups/handoffs without claiming missing persistent entity models or a backend. No runtime/adapter-kind schema change or external publication; no recognizer, storage, authority or sequential conformance execution is claimed.

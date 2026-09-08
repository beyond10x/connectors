---
format: aep.planning-md/1
id: story:contracts-host-composition
kind: story
status: implemented
title: Assign mediated adapter wiring to an explicit composition owner
tags:
- P2
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
scope:
- confidence: inferred
  path: contracts/discovery/composition.md
- confidence: cited
  path: contracts/discovery/mediated_route/v1alpha1/semantics.md
- confidence: inferred
  path: contracts/service/compatibility.md
- confidence: cited
  path: docs/adapters/grafana.md
- confidence: inferred
  path: docs/adapters/kubernetes.md
- confidence: cited
  path: docs/design.md
- confidence: inferred
  path: docs/evidence/discovery-coverage-20260908
- confidence: inferred
  path: ess/domains/discovery.yaml
- confidence: inferred
  path: ess/system.yaml
revision: 9
---
## Context

Priority: **P2**. Sources: `E03` in `specification:contract-review-intake-20260908`.

Assigning composition loading to generic server.rs leaves the owner of concrete adapter wiring unclear.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Document the composition boundary and dependency direction while retaining the no-host-to-concrete-adapter invariant. Colocation alone is not a contradiction: a composition executable may inject ports; do not force a new wire proxy or composition implementation in this story.

## Acceptance

After revision, the mediated composition diagram identifies a wiring owner without requiring the generic host to import a concrete adapter.

## Verification scenarios

- One executable links parent and child implementations and injects the generic host ports.
- Each adapter remains independently usable.
- Unsupported placement fails explicitly rather than introducing an accidental generic proxy.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- inferred: `contracts/discovery/composition.md`
- cited: `contracts/discovery/mediated_route/v1alpha1/semantics.md`
- inferred: `contracts/service/compatibility.md`
- cited: `docs/adapters/grafana.md`
- inferred: `docs/adapters/kubernetes.md`
- cited: `docs/design.md`
- inferred: `docs/evidence/discovery-coverage-20260908`
- inferred: `ess/domains/discovery.yaml`
- inferred: `ess/system.yaml`

## Dependencies and edit coordination

No semantic prerequisite is declared within this draft set.

Shared edit surfaces with `story:contracts-wire-compatibility`, `story:contracts-anonymous-auth`, `story:contracts-log-continuation`, `story:contracts-discovery-coverage`, `story:contracts-management-boundary`, `story:contracts-discovery-profiles`, `story:contracts-acquisition-profiles`, `story:contracts-documentation-index`, `story:contracts-evidence-precision`, `story:contracts-persistence-ownership`, `story:contracts-tenant-header`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

This approved local specification-hardening story revises contracts/adapter design, models settled values with pinned ESS0.20.0, and records explicit UNMAPPED semantics/ownership plus independent review evidence. Runtime code and current adapter-kind schemas remain unchanged. Generic schema/compiled-scenario checking does not execute admission, observation, transport or persistence. Local commits and local recovery backup are authorized; no external publication. Root is the sole tracked-file editor in primary main under the repository override; two independent reviewers write ignored snapshots only. Shared surfaces are serialized through root, not a concurrent implementation wave. Discovery-profiles remains separately owned and draft pending discovery-coverage completion.

## Completion evidence

E03 is fixed for this approved semantic specification scope. Exact normative corrections, 46 declared textual traces, 63 schema expectations and their limits are recorded in [verification](../../../docs/evidence/discovery-coverage-20260908/verification.md) and [dispositions](../../../docs/evidence/discovery-coverage-20260908/dispositions.md). Both immutable final independent reviews discovery-a-final-20260908 and discovery-b-final-20260908 approve with zero residual findings. ESS 0.20.0 validates 13 files/197 declarations; two 204-artifact projections match; the existing full gate/MSRV1.88 passes 50 Rust tests. This is not execution of provider, publisher, route, authority or composition semantics. No runtime/adapter-kind schema change or external publication. Discovery-profiles E07/E14/E32 and persistent ownership bindings remain separate work.

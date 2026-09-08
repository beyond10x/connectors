---
format: aep.planning-md/1
id: story:contracts-connection-readiness
kind: story
status: active
title: Separate connection viability from per-operation eligibility
tags:
- P2
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
- depends_on: story:contracts-credential-evidence
scope:
- confidence: cited
  path: contracts/auth/acquisition/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/connection/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/evidence/v1alpha1/semantics.md
- confidence: inferred
  path: contracts/auth/profile/v1alpha1/semantics.md
- confidence: inferred
  path: contracts/service/compatibility.md
- confidence: inferred
  path: docs/evidence/connection-semantics-20260908/
- confidence: inferred
  path: ess/domains/connection_admission.yaml
- confidence: inferred
  path: ess/system.yaml
revision: 6
---
## Context

Priority: **P2**. Sources: `F07`, `E19`, `E21` in `specification:contract-review-intake-20260908`.

Missing write scope globally blocks a connection despite a required successful read; pending/disabled and scope failure names disagree.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Define one readiness reduction and canonical state/error vocabulary across public tables, conformance and future-model notes; preserve per-invocation authority checks. This repairs prose, not an ESS lifecycle implementation.

## Acceptance

After revision, each readiness scenario yields the same eligibility and state name across connection, acquisition and evidence documents.

## Verification scenarios

- Read with required scope present and write scope absent → permitted read and refused write.
- Pending, disabled, custody failure and revoked each have one globally or operation-locally defined consequence.
- Use one canonical insufficient_scope name wherever that same failure is meant.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- cited: `contracts/auth/connection/v1alpha1/semantics.md`
- cited: `contracts/auth/evidence/v1alpha1/semantics.md`
- cited: `contracts/auth/acquisition/v1alpha1/semantics.md`

Source locations: `contracts/auth/connection/v1alpha1/semantics.md:58`; `contracts/auth/connection/v1alpha1/semantics.md:74`; `contracts/auth/connection/v1alpha1/semantics.md:127`; `contracts/auth/evidence/v1alpha1/semantics.md:88`; `contracts/auth/acquisition/v1alpha1/semantics.md:47`; `contracts/auth/acquisition/v1alpha1/semantics.md:61`.

## Dependencies and edit coordination

Semantic prerequisites: `story:contracts-credential-evidence`.

Shared edit surfaces with `story:contracts-refresh-coordination`, `story:contracts-credential-evidence`, `story:contracts-wire-compatibility`, `story:contracts-permission-budgets`, `story:contracts-read-refresh-retry`, `story:contracts-management-boundary`, `story:contracts-acquisition-profiles`, `story:contracts-media-controls`, `story:contracts-persistence-ownership`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

The active specification-hardening goal authorizes this local contract and ESS revision and local checkpoint. Root is the sole source editor in primary main; two independent reviewers are read-only, so no parallel implementation worktree or wave is implied. No runtime code, adapter schema edits, extra adapters, external publication or Atlas changes are included.

The readiness reduction and host-management admission/ownership decisions receive typed value homes in `ess/domains/connection_admission.yaml`. These are observed facts and trusted decisions, not caller-provided authority or a fictitious verifier. Existing credential generation/evidence/refresh and attempt models remain authoritative. Persistent Connection/Acquisition/AuthProfile ownership models, complete protected completion codecs and concrete storage integration are not silently invented to validate this slice; known values reference existing qualified bindings and unresolved entity relations remain named UNMAPPED. This is revision of the approved existing stories, not new implementation decomposition.

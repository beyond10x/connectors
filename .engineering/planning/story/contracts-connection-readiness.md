---
format: aep.planning-md/2
id: story:contracts-connection-readiness
kind: story
status: implemented
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
revision: 10
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

- cited: `contracts/auth/acquisition/v1alpha1/semantics.md`
- cited: `contracts/auth/connection/v1alpha1/semantics.md`
- cited: `contracts/auth/evidence/v1alpha1/semantics.md`
- inferred: `contracts/auth/profile/v1alpha1/semantics.md`
- inferred: `contracts/service/compatibility.md`
- inferred: `docs/evidence/connection-semantics-20260908/`
- inferred: `ess/domains/connection_admission.yaml`
- inferred: `ess/system.yaml`

Both stories share the listed auth/service/model/evidence files; root is their sole source writer. No concurrent implementation wave or additional adapter is implied.

## Dependencies and edit coordination

Semantic prerequisites: `story:contracts-credential-evidence`.

Shared edit surfaces with `story:contracts-refresh-coordination`, `story:contracts-credential-evidence`, `story:contracts-wire-compatibility`, `story:contracts-permission-budgets`, `story:contracts-read-refresh-retry`, `story:contracts-management-boundary`, `story:contracts-acquisition-profiles`, `story:contracts-media-controls`, `story:contracts-persistence-ownership`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

The active specification-hardening goal authorizes this local contract and ESS revision and local checkpoint. Root is the sole source editor in primary main; two independent reviewers are read-only, so no parallel implementation worktree or wave is implied. No runtime code, adapter schema edits, extra adapters, external publication or Atlas changes are included.

The readiness reduction and host-management admission/ownership decisions receive typed value homes in `ess/domains/connection_admission.yaml`. These are observed facts and trusted decisions, not caller-provided authority or a fictitious verifier. Existing credential generation/evidence/refresh and attempt models remain authoritative. Persistent Connection/Acquisition/AuthProfile ownership models, complete protected completion codecs and concrete storage integration are not silently invented to validate this slice; known values reference existing qualified bindings and unresolved entity relations remain named UNMAPPED. This is revision of the approved existing stories, not new implementation decomposition.

## Resolution and verification

F07/E19/E21 are resolved in connection §4.1, profile §4.1 and the synchronized acquisition/evidence contracts: global seven-state viability uses baseline grants and current administrative/dependency facts, then exact-operation eligibility applies matching auth/configuration, required scopes/permissions/verification and current authority. Optional write scope and target-specific failures no longer poison unrelated reads. Local revoked is terminal, disabled is orthogonal, stale baseline is pending, and insufficient_scope is one canonical acquisition/operation refusal rather than a global status. Inactive failed repair does not poison valid old material. ESS connection_admission values record these questions without a fictitious Connection lifecycle.

Verification is recorded in docs/evidence/connection-semantics-20260908/verification.md: 50 existing Rust tests/full gate/MSRV1.88 pass; ESS0.20.0 validates 11 files/167 declarations; two generations of 176 schema artifacts are byte-identical. The 80 type expectations, 16 viability/18 eligibility/7 target examples and 16 textual traces are precisely scoped; no reducer, current policy, callback, provider or database implementation is executed or introduced. Independent raw reviews and individual fixed outcomes precede final lifecycle closure.

## Final independent acceptance

Both final independent reviewers approve this semantic scope with zero remaining findings: review-result:connection-semantics-r3-20260908 and review-result:connection-ownership-r3-20260908. Their immutable reports, earlier findings and fixed per-finding outcomes are preserved. Final normative/evidence hashes match; reviewer A's two earlier workflow snapshots are distinguished from later AEP-only mutations. Verification and final provenance are docs/evidence/connection-semantics-20260908/verification.md and checkpoint.md. The specification story is complete; no runtime implementation or advertisement is claimed.

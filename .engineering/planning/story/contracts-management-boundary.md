---
format: aep.planning-md/2
id: story:contracts-management-boundary
kind: story
status: implemented
title: Clarify ownership and admission of connection management
tags:
- P2
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
- depends_on: story:contracts-wire-compatibility
scope:
- confidence: inferred
  path: contracts/README.md
- confidence: cited
  path: contracts/auth/acquisition/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/connection/v1alpha1/semantics.md
- confidence: inferred
  path: contracts/auth/management.md
- confidence: inferred
  path: contracts/service/compatibility.md
- confidence: cited
  path: contracts/service/v1alpha2/semantics.md
- confidence: cited
  path: docs/design.md
- confidence: inferred
  path: docs/evidence/connection-semantics-20260908/
- confidence: inferred
  path: ess/domains/connection_admission.yaml
- confidence: inferred
  path: ess/system.yaml
revision: 12
---
## Context

Priority: **P2**. Sources: `E04` in `specification:contract-review-intake-20260908`.

Calling management ordinary adapter operations obscures the split between host persistence, provider protocol work and admitted remote management.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Specify orchestration owner, management admission/discovery and federation routing; preserve secret/private callback non-disclosure. Design §16.1 forbids exposing private capabilities as discovery data, not all use of an operation envelope, so document the chosen boundary without treating dedicated HTTP routes as already required.

## Acceptance

After revision, a remote connection-management trace assigns each action to its owner without advertising private callback or registration authority.

## Verification scenarios

- Begin/complete/list/revoke identify host versus provider responsibilities and admitted caller scope.
- Federated completion reaches its owning coordinator.
- Ordinary discovery cannot reveal private callback capability or registration secrets.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

- inferred: `contracts/README.md`
- cited: `contracts/auth/acquisition/v1alpha1/semantics.md`
- cited: `contracts/auth/connection/v1alpha1/semantics.md`
- inferred: `contracts/auth/management.md`
- inferred: `contracts/service/compatibility.md`
- cited: `contracts/service/v1alpha2/semantics.md`
- cited: `docs/design.md`
- inferred: `docs/evidence/connection-semantics-20260908/`
- inferred: `ess/domains/connection_admission.yaml`
- inferred: `ess/system.yaml`

Both stories share the listed auth/service/model/evidence files; root is their sole source writer. No concurrent implementation wave or additional adapter is implied.

## Dependencies and edit coordination

Semantic prerequisites: `story:contracts-wire-compatibility`.

Shared edit surfaces with `story:contracts-refresh-coordination`, `story:contracts-credential-evidence`, `story:contracts-wire-compatibility`, `story:contracts-connection-readiness`, `story:contracts-read-refresh-retry`, `story:contracts-host-composition`, `story:contracts-acquisition-profiles`, `story:contracts-documentation-index`, `story:contracts-persistence-ownership`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

The active specification-hardening goal authorizes this local contract and ESS revision and local checkpoint. Root is the sole source editor in primary main; two independent reviewers are read-only, so no parallel implementation worktree or wave is implied. No runtime code, adapter schema edits, extra adapters, external publication or Atlas changes are included.

The readiness reduction and host-management admission/ownership decisions receive typed value homes in `ess/domains/connection_admission.yaml`. These are observed facts and trusted decisions, not caller-provided authority or a fictitious verifier. Existing credential generation/evidence/refresh and attempt models remain authoritative. Persistent Connection/Acquisition/AuthProfile ownership models, complete protected completion codecs and concrete storage integration are not silently invented to validate this slice; known values reference existing qualified bindings and unresolved entity relations remain named UNMAPPED. This is revision of the approved existing stories, not new implementation decomposition.

## Resolution and verification

E04 is resolved in contracts/auth/management.md and the synchronized connection/acquisition/service/design references: host dispatch/coordinator owns management admission, metadata, publication and local revocation; provider hooks own provider protocol work, custody owns sensitive versions. Instance/connection/acquisition targets are explicit and obey omission/string wire rules. Safe metadata and admitted repair/revoke have independent prerequisites from business readiness. One logical coordinator retains acquisition ownership through federation, rechecks current authority and repair revision, and receives protected completion. Ordinary results contain safe refs/action kinds; only an admitted confidential UI channel receives interactive completion authority. Noninteractive client credentials require no UI. Local revocation and provider cleanup outcomes remain separate. Missing concrete management effect/approval/codec bindings remain advertisement gates, not implicit broad grants.

Verification is recorded in docs/evidence/connection-semantics-20260908/verification.md: 50 existing Rust tests/full gate/MSRV1.88 pass; ESS0.20.0 validates 11 files/167 declarations; two generations of 176 schema artifacts are byte-identical. The 80 type expectations, 16 viability/18 eligibility/7 target examples and 16 textual traces are precisely scoped; no reducer, current policy, callback, provider or database implementation is executed or introduced. Independent raw reviews and individual fixed outcomes precede final lifecycle closure.

## Final independent acceptance

Both final independent reviewers approve this semantic scope with zero remaining findings: review-result:connection-semantics-r3-20260908 and review-result:connection-ownership-r3-20260908. Their immutable reports, earlier findings and fixed per-finding outcomes are preserved. Final normative/evidence hashes match; reviewer A's two earlier workflow snapshots are distinguished from later AEP-only mutations. Verification and final provenance are docs/evidence/connection-semantics-20260908/verification.md and checkpoint.md. The specification story is complete; no runtime implementation or advertisement is claimed.

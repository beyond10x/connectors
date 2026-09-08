---
format: aep.planning-md/1
id: story:contracts-idempotency-scope
kind: story
status: implemented
title: Define idempotency ownership and replay admission
tags:
- P1
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
- depends_on: story:contracts-mutation-outcomes
scope:
- confidence: cited
  path: contracts/operations/v1alpha1/idempotency-verification.md
- confidence: cited
  path: contracts/operations/v1alpha1/scenarios/idempotency-aborted-result.yaml
- confidence: cited
  path: contracts/operations/v1alpha1/scenarios/idempotency-known-result-expiry.yaml
- confidence: cited
  path: contracts/operations/v1alpha1/scenarios/idempotency-namespace-values.yaml
- confidence: cited
  path: contracts/operations/v1alpha1/scenarios/idempotency-pending-no-expiry.yaml
- confidence: cited
  path: contracts/operations/v1alpha1/scenarios/idempotency-refused-result.yaml
- confidence: cited
  path: contracts/operations/v1alpha1/scenarios/idempotency-unknown-no-expiry.yaml
- confidence: cited
  path: contracts/operations/v1alpha1/semantics.md
- confidence: cited
  path: docs/adapters/atlassian.md
- confidence: cited
  path: ess/domains/idempotency.yaml
- confidence: cited
  path: ess/domains/mutations.yaml
- confidence: cited
  path: ess/system.yaml
revision: 11
---
## Context

Priority: **P1**. Sources: `F02` in `specification:contract-review-intake-20260908`.

Same key and canonical input do not identify a request across callers, connections, operations or origins.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Define namespace, fingerprint, retention and revision-change treatment as normative policy with typed ESS values and a reservation lifecycle; require current authority for replay and do not equate canonical input with complete request identity.

## Acceptance

After revision, the idempotency matrix distinguishes replay from an unrelated effect for every listed authority and revision boundary. Namespace/fingerprint values and replay reservation lifecycle have a validated ESS home, with compiled scenarios and explicit limits on what the model checks.

## Verification scenarios

- Same key/body across caller, operation, connection and federation origin → explicitly separate or refused, never cross-authority result disclosure.
- Same scoped key with changed input → conflict; current access revoked before replay → refusal.
- Retention expiry and in-flight/unknown outcome records have explicit replay behavior.

Verify the textual decision matrix, validate and compile with pinned ESS 0.20.0, compile authored traces/generated obligations in the full repository gate, and obtain two independent semantic reviews. Record exact source and scenario coverage without claiming runtime execution; host admission, uniqueness, clock and storage fault tests remain future binding work.

## Scope

- cited: `contracts/operations/v1alpha1/semantics.md`
- cited: `docs/adapters/atlassian.md`
- cited: `ess/system.yaml`
- cited: `ess/domains/idempotency.yaml`
- cited: `ess/domains/mutations.yaml`
- cited: `contracts/operations/v1alpha1/idempotency-verification.md`
- cited: `contracts/operations/v1alpha1/scenarios/idempotency-aborted-result.yaml`
- cited: `contracts/operations/v1alpha1/scenarios/idempotency-known-result-expiry.yaml`
- cited: `contracts/operations/v1alpha1/scenarios/idempotency-namespace-values.yaml`
- cited: `contracts/operations/v1alpha1/scenarios/idempotency-pending-no-expiry.yaml`
- cited: `contracts/operations/v1alpha1/scenarios/idempotency-refused-result.yaml`
- cited: `contracts/operations/v1alpha1/scenarios/idempotency-unknown-no-expiry.yaml`

Source locations: `contracts/operations/v1alpha1/semantics.md:100`; `contracts/operations/v1alpha1/semantics.md:107`; `contracts/operations/v1alpha1/semantics.md:120`; `docs/adapters/atlassian.md:72`.

## Dependencies and edit coordination

Semantic prerequisites: `story:contracts-mutation-outcomes`.

Shared edit surfaces with `story:contracts-mutation-outcomes`, `story:contracts-federated-approval`, `story:contracts-refresh-coordination`, `story:contracts-wire-compatibility`, `story:contracts-mutation-classification`, `story:contracts-restart-idempotency`, `story:contracts-document-admission`, `story:contracts-evidence-precision`, `story:contracts-persistence-ownership`, `story:contracts-mutation-visibility`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

The operator authorized continuing semantic hardening alongside ESS after committing the previous story as `34f298a` on 2026-09-08. This supersedes the original text-only boundary for this activated story. `ess/domains/idempotency.yaml` validates and compiles with ESS 0.20.0: AuthorityScope, Origin, KeyNamespace and RequestFingerprint are typed values; KeyReservation has a host-generated identity, one explicit reference to AttemptRecord, and Pending/Replayable/Quarantined/Expired lifecycle states with command causation.

Namespace authority is receiver-derived. Caller/origin changes isolate namespaces; operation, connection, input and semantic/configuration revision changes conflict within one namespace. Replay requires current authority but does not spend another approval. Durable known results have a fixed replay window; pending/unknown reservations cannot expire into another dispatch. These rules and their host algorithms remain reviewable proposals, not runtime mutation support.

UNMAPPED: authority/Connection entity relations, canonical tuple derivation, current policy evaluation, atomic unique reservation with attempt creation, lifecycle coupling, retention clock/eviction and result storage. ESS does not prove those runtime facts. No runtime mutation/wire implementation, new adapter, generated schema, publication or Atlas change is in scope. Remaining source findings retain their existing story ownership. This is one local writer with independent read-only reviewers, not an implementation wave.

## Completion evidence — 2026-09-08

F02 is fixed at the semantic-contract/ESS-model level. `contracts/operations/v1alpha1/semantics.md` §5.1 defines the receiver-derived namespace, full fingerprint, current replay admission, atomic ownership and conservative retention; the Atlassian adapter plan applies it. `contracts/operations/v1alpha1/idempotency-verification.md` maps each boundary to the current model/scenario check and still-required host test.

Pinned ESS 0.20.0 validates four files and compiles 43 declarations. The full Rust repository gate exited 0 with repository-local TMPDIR: 39 workspace tests passed; 67 ESS scenarios compiled (15 authored, including six F02 traces), zero refusals. These are not runtime conformance results. A temporary malformed namespace-field control was refused with ESS-AUTHOR-013/014, exit 1. The first gate bootstrap failed on /tmp quota; the successful retry changed only the temporary-directory environment. Exact commands and transcripts are linked in the verification record.

Independent final reviewers `review-result:idempotency-semantics-r2-20260908` and `review-result:idempotency-model-r2-20260908` approve. Semantic round one found IS1 (a cache-miss/claim-spend race); the authoritative admitted winner recheck now handles it, and fixed review_outcome evidence is recorded. Only contract/audit prose changed after the passing gate; ESS and scenario sources are unchanged.

The user is resolving concurrent versioning documents separately. This story makes no wire-version decision or runtime mutation implementation. The F02 changes were committed locally as `8903166`; prerequisite changes were committed as `34f298a` at the user's request before this story started.

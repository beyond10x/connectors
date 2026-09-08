---
format: aep.planning-md/1
id: story:contracts-federated-approval
kind: story
status: implemented
title: Bind approvals consistently across federation
tags:
- P1
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
- depends_on: story:contracts-idempotency-scope
- depends_on: story:contracts-wire-compatibility
scope:
- confidence: cited
  path: contracts/README.md
- confidence: cited
  path: contracts/operations/v1alpha1/scenarios/
- confidence: cited
  path: contracts/operations/v1alpha1/semantics.md
- confidence: cited
  path: contracts/service/compatibility.md
- confidence: inferred
  path: contracts/service/delegation.md
- confidence: cited
  path: contracts/service/v1alpha1/semantics.md
- confidence: cited
  path: contracts/service/v1alpha2/semantics.md
- confidence: cited
  path: docs/cli-migration-v1-to-v2.md
- confidence: inferred
  path: docs/evidence/federated-approval-20260908/
- confidence: cited
  path: docs/stack-integration-proposal.md
- confidence: inferred
  path: ess/domains/delegation.yaml
- confidence: cited
  path: ess/domains/idempotency.yaml
- confidence: cited
  path: ess/domains/mutations.yaml
- confidence: cited
  path: ess/system.yaml
revision: 12
---
## Context

Priority: **P1**. Sources: `F03` in `specification:contract-review-intake-20260908`.

Gateway caller, operation and descriptor revision differ from leaf coordinates while approval evidence must remain unchanged.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Choose canonical approval subjects, client visibility, trusted authority mapping and verifier/redemption owner; distinguish routing freshness from signed identity and prohibit authority broadening.

## Acceptance

After revision, a gateway-to-leaf approval trace has one verifiable subject and redemption owner at every hop.

## Verification scenarios

- Admitted caller approves a gateway-visible operation and the selected leaf verifies the documented canonical subjects.
- Another origin or revision cannot reuse approval; a broad gateway credential cannot substitute for narrowed caller authority.

Verification is a textual scenario audit: record the chosen rule and expected observation/refusal for every scenario, cross-check all cited documents, and link the resulting review evidence before claiming the story complete. Runtime conformance implementation is subsequent work.

## Scope

The machine-readable scope includes the cited operations/service semantics, compatibility and index, CLI/stack guidance, existing idempotency/mutation domains, ESS system, and idempotency scenario directory. The new delegation contract/domain and verification directory are inferred implementation surfaces of this specification-only story. The original v1alpha1 service is read for compatibility evidence; its runtime is unchanged. Source edits remain single-writer in primary main; both independent reviewers are read-only.

## Dependencies and edit coordination

Semantic prerequisites: `story:contracts-idempotency-scope`, `story:contracts-wire-compatibility`.

Shared edit surfaces with `story:contracts-mutation-outcomes`, `story:contracts-idempotency-scope`, `story:contracts-wire-compatibility`, `story:contracts-mutation-classification`, `story:contracts-restart-idempotency`, `story:contracts-read-refresh-retry`, `story:contracts-evidence-precision`, `story:contracts-persistence-ownership`, `story:contracts-mutation-visibility`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

The active specification-hardening goal authorizes this local semantic/ESS revision and local checkpoint. No runtime code, generated adapter schema, extra adapter, external publication or Atlas mutation is included. Root edits primary main under the single-agent repository override; the two independent reviewers are read-only.

The normative owner will be `contracts/service/delegation.md`, with changes to service/v1alpha2, compatibility, operations and the proposed stack/CLI guidance. The preparation result is a value, not a reservation or persisted entity. Canonical subjects, exact delegated/approval assertion values and known replay/redemption ownership are modeled in `ess/domains/delegation.yaml`; existing idempotency values retain their semantics with explicit optional executor and route fingerprint. Known links to the existing executing ServiceConfiguration and AttemptRecord are modeled explicitly; unresolved issuer, policy and storage relationships remain UNMAPPED. The new host approval preparation read covers both implicit configured and explicit managed connections; it does not depend on inventing a complete management CRUD API.

Existing signed module dispatch is pinned characterization evidence, not the new binding. The old closed module-request token cannot gain realm, executor or approval fields silently. Current Atlas ADRs govern authority; predecessor Platform ADR numbering is kept separate. F03 selects complete one-hop subject/framing/verification and nonce-versus-approval ownership rules. Runtime conformance and concrete issuer/persistence integration remain explicit future obligations. This is an existing story revision, not an implementation decomposition or parallel wave.

## Resolution and evidence

The one-hop protocol is selected in `contracts/service/delegation.md`: complete canonical leaf subjects and trusted origin/route context; admitted safe preparation for implicit configured and explicit managed connections; distinct Ed25519 compact-JWS delivery and issuer approval types; exact bounded proof windows and final post-nonce-ack current-fact checks; one executing-leaf spend authority and separate business dispatch fence. Gateway presentation freshness remains separate from signed leaf/route identity. Existing exact-key replay precedes old approval validation. Required-null wire values and explicit fingerprint mutation-request/v2 prevent silent reinterpretation of the proposed old private shape.

`ess/domains/delegation.yaml` types the values and immutable DeliveryReceipt/ApprovalRedemption, with explicit references to existing ServiceConfiguration and AttemptRecord. No fake behavior commands or guessed issuer/policy/configuration cardinalities were introduced. Concrete wire/signature/clock/uniqueness/current-authority/storage enforcement is named UNMAPPED and remains an advertisement gate.

`docs/evidence/federated-approval-20260908/verification.md` distinguishes 20 crypto primitive observations, 12 generated-schema expectations, 23 arithmetic/truth-table expectations, 15 canonical coordinate perturbations and 26 textual traces from future runtime conformance. Generic ESS Optional schemas reject mandatory-null wire values; the evidence reports that limit. Pinned ESS 0.20.0 validates 10 files/153 declarations; 163 generated schema artifacts are deterministic. Existing 15 operations and 13 session authored scenarios compile without refusals; combined gate synthesis has 222 obligations (34 authored), and the separate session synthesis 201 (13 authored). The full gate and MSRV1.88 pass with 50 existing Rust tests; no runtime implementation was added.

Immutable independent reviews are recorded as federation-subjects and federation-binding results. Their original reports are preserved; outcomes identify each corrected finding. Final review evidence is added before lifecycle closure. This closes only F03, not the remaining source ledger or broad specification goal.

## Final independent acceptance

Both final independent reviewers approve with zero residual findings. `review-result:federation-subjects-final-20260908` closes the final federated-route fixture correction; `review-result:federation-binding-r3-20260908` closes the error-name correction. All 84/54 final reviewed source hashes match. The 21 individual reviewer findings have fixed review_outcome evidence; original reports remain immutable. Final provenance is `docs/evidence/federated-approval-20260908/checkpoint.md`. This implements the specification story only; no delegated runtime or issuer/store integration is implemented or advertised.

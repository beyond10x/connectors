---
format: aep.planning-md/1
id: story:contracts-mutation-outcomes
kind: story
status: implemented
title: Make mutation outcomes truthful across dispatch and recovery
tags:
- P1
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
scope:
- confidence: cited
  path: contracts/operations/v1alpha1/scenarios/deadline-after-gate.yaml
- confidence: cited
  path: contracts/operations/v1alpha1/scenarios/deadline-before-gate.yaml
- confidence: cited
  path: contracts/operations/v1alpha1/scenarios/definitive-refusal.yaml
- confidence: cited
  path: contracts/operations/v1alpha1/scenarios/definitive-success.yaml
- confidence: cited
  path: contracts/operations/v1alpha1/scenarios/event-claim-uncertainty.yaml
- confidence: cited
  path: contracts/operations/v1alpha1/scenarios/indeterminate-is-terminal.yaml
- confidence: cited
  path: contracts/operations/v1alpha1/scenarios/recovery-after-gate.yaml
- confidence: cited
  path: contracts/operations/v1alpha1/scenarios/recovery-before-gate.yaml
- confidence: cited
  path: contracts/operations/v1alpha1/scenarios/response-loss-no-approval.yaml
- confidence: cited
  path: contracts/operations/v1alpha1/semantics.md
- confidence: cited
  path: contracts/operations/v1alpha1/verification.md
- confidence: cited
  path: crates/connectors-build/src/gate.rs
- confidence: cited
  path: ess/domains/mutations.yaml
- confidence: cited
  path: ess/system.yaml
revision: 9
---
## Context

Priority: **P1**. Sources: `F01`, `E01`, `E11` in `specification:contract-review-intake-20260908`.

Approval spending cannot prove dispatch; deadline handling and the single-code error vocabulary do not consistently express attempt state.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Define the normative outcome table independently of approval; distinguish an error cause from attempt status without prematurely choosing a wire field. Assign service and transport deadline obligations in §8 and settle an in-flight duplicate wait expiring after another attempt may dispatch. Current read-only code is evidence of future binding work, not a claim that a mutation implementation is already broken; wire encoding belongs to contracts-wire-compatibility.

## Acceptance

After revision, the mutation outcome table gives one truthful classification for every listed dispatch, deadline and recovery scenario, and the corresponding proposed lifecycle has a validated ESS home with compiled positive and adversarial scenarios. The evidence distinguishes model checks from unimplemented runtime guarantees.

## Verification scenarios

- Crash before dispatch with durable proof → aborted; uncertain dispatch → indeterminate, including approval:not_required.
- Crash after spend before send and after send before outcome cannot be distinguished without further evidence.
- Deadline before send versus response loss after send; duplicate waiter deadline must not imply that the original attempt never ran.
- Store unavailable before dispatch has an unambiguous cause and attempt classification.

Verification combines the normative scenario audit with ESS 0.20.0 validation, IR compilation, authored scenario compilation and generated conformance obligations. The full repository gate must retain these checks. Review the corrected rules with two independent readers and link evidence before closure. Compiled scenarios are not executed runtime conformance; actual dispatch, crash, deadline and storage fault injection remain future binding obligations.

## Scope

- cited: `contracts/operations/v1alpha1/semantics.md`
- cited: `ess/system.yaml`
- cited: `ess/domains/mutations.yaml`
- cited: `contracts/operations/v1alpha1/scenarios/deadline-after-gate.yaml`
- cited: `contracts/operations/v1alpha1/scenarios/deadline-before-gate.yaml`
- cited: `contracts/operations/v1alpha1/scenarios/definitive-refusal.yaml`
- cited: `contracts/operations/v1alpha1/scenarios/definitive-success.yaml`
- cited: `contracts/operations/v1alpha1/scenarios/event-claim-uncertainty.yaml`
- cited: `contracts/operations/v1alpha1/scenarios/indeterminate-is-terminal.yaml`
- cited: `contracts/operations/v1alpha1/scenarios/recovery-after-gate.yaml`
- cited: `contracts/operations/v1alpha1/scenarios/recovery-before-gate.yaml`
- cited: `contracts/operations/v1alpha1/scenarios/response-loss-no-approval.yaml`
- cited: `contracts/operations/v1alpha1/verification.md`
- cited: `crates/connectors-build/src/gate.rs`

Source locations: `contracts/operations/v1alpha1/semantics.md:90`; `contracts/operations/v1alpha1/semantics.md:102`; `contracts/operations/v1alpha1/semantics.md:111`; `contracts/operations/v1alpha1/semantics.md:120`; `contracts/operations/v1alpha1/semantics.md:131`; `crates/connectors-host/src/server.rs:142`; `crates/connectors-host/src/http.rs:145`.

## Dependencies and edit coordination

No semantic prerequisite is declared within this draft set.

Shared edit surfaces with `story:contracts-idempotency-scope`, `story:contracts-federated-approval`, `story:contracts-wire-compatibility`, `story:contracts-mutation-classification`, `story:contracts-restart-idempotency`, `story:contracts-evidence-precision`, `story:contracts-persistence-ownership`, `story:contracts-mutation-visibility`. These collisions require serialized edits or a coordinated integration owner; the stories are not a pre-approved concurrent wave. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

The operator authorized semantic hardening together with ESS modeling on 2026-09-08, after checkpoint commit `57be07c`. This supersedes the original text-only boundary for this story. The validated typed home is `ess/domains/mutations.yaml`: AttemptRecord has a host-generated AttemptId, explicit instance reference, and Prepared/Dispatching/terminal lifecycle transitions caused by trusted ledger commands. EffectKnowledge and Observation distinguish classification from cause without choosing a public wire field.

UNMAPPED: runtime operation qualification, Connection, ApprovalRedemption, tenant/principal, idempotency ownership, durable atomicity, evidence provenance, and classification projection. No unspecified relation is invented to pass validation. These gaps and future host tests remain explicit in the contract and verification record.

Local contract/model/scenario and Rust verification-tooling changes are authorized. Runtime mutation behavior, wire codecs, generated adapter schemas, added adapters, publication and Atlas changes remain outside this story. Other stories keep their finding ownership; refine their ESS scope before activation. No parallel implementation wave is started.

## Completion evidence — 2026-09-08

F01, E01 and E11 are fixed at the semantic-contract/model level. The normative outcome table and service/transport/duplicate-waiter obligations are in `contracts/operations/v1alpha1/semantics.md` §§3–6/8; the finding-by-scenario audit is `contracts/operations/v1alpha1/verification.md`. No mutation runtime is claimed implemented by this story's completion.

Pinned ESS 0.20.0 validates and compiles the three-file model. Nine authored traces and 36 generated obligations compile with zero refusals. A temporary undeclared-state negative control fails with ESS-AUTHOR-017 and exit 1. The full repository gate command in the verification record exited 0: 39 workspace tests passed, none failed or ignored; formatting, Clippy, drift, library boundaries and planning validation passed. No runtime conformance target was executed; UNMAPPED host and relation obligations remain explicit.

Independent reviews `review-result:mutation-outcomes-semantics-20260908` and `review-result:mutation-outcomes-model-20260908` both approve with no in-scope findings. These reviews cover the corrected first story, not sibling outcomes or the later catalog additions. The current hardening changes remain local and uncommitted; checkpoint `57be07c` preserves the pre-hardening state.

---
format: aep.planning-md/1
id: story:contracts-refresh-coordination
kind: story
status: implemented
title: Specify refresh exclusion and recovery after owner loss
tags:
- P1
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
scope:
- confidence: inferred
  path: contracts/auth/acquisition/v1alpha1/scenarios
- confidence: cited
  path: contracts/auth/acquisition/v1alpha1/scenarios/authorization-wins-fence-race.yaml
- confidence: cited
  path: contracts/auth/acquisition/v1alpha1/scenarios/authorized-owner-loss-no-second-exchange.yaml
- confidence: cited
  path: contracts/auth/acquisition/v1alpha1/scenarios/binding-replacement-before-publication.yaml
- confidence: cited
  path: contracts/auth/acquisition/v1alpha1/scenarios/durable-response-wins-owner-loss-race.yaml
- confidence: cited
  path: contracts/auth/acquisition/v1alpha1/scenarios/expired-candidate-before-publication.yaml
- confidence: cited
  path: contracts/auth/acquisition/v1alpha1/scenarios/late-response-after-quarantine.yaml
- confidence: cited
  path: contracts/auth/acquisition/v1alpha1/scenarios/published-reply-loss-no-reexchange.yaml
- confidence: cited
  path: contracts/auth/acquisition/v1alpha1/scenarios/recovery-owner-loss-never-reopens-source.yaml
- confidence: cited
  path: contracts/auth/acquisition/v1alpha1/scenarios/reserved-owner-fenced-before-successor.yaml
- confidence: cited
  path: contracts/auth/acquisition/v1alpha1/scenarios/revocation-before-authorization.yaml
- confidence: cited
  path: contracts/auth/acquisition/v1alpha1/scenarios/revocation-before-publication.yaml
- confidence: cited
  path: contracts/auth/acquisition/v1alpha1/scenarios/stored-response-recovery-rejects-stale-publisher.yaml
- confidence: cited
  path: contracts/auth/acquisition/v1alpha1/scenarios/two-replicas-one-authorization.yaml
- confidence: cited
  path: contracts/auth/acquisition/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/acquisition/v1alpha1/verification.md
- confidence: cited
  path: contracts/auth/custody/v1alpha1/semantics.md
- confidence: cited
  path: docs/adapters/atlassian.md
- confidence: cited
  path: ess/domains/refresh.yaml
revision: 29
---
## Context

Priority: **P1**. Sources: `F04` in `specification:contract-review-intake-20260908`.

Acquisition requires a custody lease although custody promises only immutable versions and CAS; lease takeover can repeat an uncertain rotating refresh.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Assign the coordinator and its required atomic operations without choosing a backend; define exclusion before exchange, durable attempt ownership, stale-owner fencing and uncertain-exchange recovery.

## Acceptance

After revision, the refresh failure matrix permits no second rotating exchange while the previous exchange may have consumed the token.

## Verification scenarios

- Two replicas contend before exchange; only one is authorized to send.
- Owner loss after send but before publication → documented uncertain/recovery path, never takeover-based replay.
- Stale owner attempts publication after successor takeover; publication and revocation race.

Verification combines the textual scenario audit with pinned ESS model validation, compilation and authored scenario compilation. Record the chosen rule and expected observation/refusal, cross-check cited documents, preserve adversarial reviews and close on the combined repository gate. Scenario compilation is not runtime conformance; the runtime binding remains subsequent work.

## Scope

Confirmed at delivery by the implementor scope audit and committed unit diff. All original cited surfaces were correct; the inferred ESS domain, verification file and scenario locations were absent/reserved as expected and are now authored. No inferred location proved wrong.

- **cited (delivered):** `contracts/auth/acquisition/v1alpha1/scenarios/authorization-wins-fence-race.yaml`
- **cited (delivered):** `contracts/auth/acquisition/v1alpha1/scenarios/authorized-owner-loss-no-second-exchange.yaml`
- **cited (delivered):** `contracts/auth/acquisition/v1alpha1/scenarios/binding-replacement-before-publication.yaml`
- **cited (delivered):** `contracts/auth/acquisition/v1alpha1/scenarios/durable-response-wins-owner-loss-race.yaml`
- **cited (delivered):** `contracts/auth/acquisition/v1alpha1/scenarios/expired-candidate-before-publication.yaml`
- **cited (delivered):** `contracts/auth/acquisition/v1alpha1/scenarios/late-response-after-quarantine.yaml`
- **cited (delivered):** `contracts/auth/acquisition/v1alpha1/scenarios/published-reply-loss-no-reexchange.yaml`
- **cited (delivered):** `contracts/auth/acquisition/v1alpha1/scenarios/recovery-owner-loss-never-reopens-source.yaml`
- **cited (delivered):** `contracts/auth/acquisition/v1alpha1/scenarios/reserved-owner-fenced-before-successor.yaml`
- **cited (delivered):** `contracts/auth/acquisition/v1alpha1/scenarios/revocation-before-authorization.yaml`
- **cited (delivered):** `contracts/auth/acquisition/v1alpha1/scenarios/revocation-before-publication.yaml`
- **cited (delivered):** `contracts/auth/acquisition/v1alpha1/scenarios/stored-response-recovery-rejects-stale-publisher.yaml`
- **cited (delivered):** `contracts/auth/acquisition/v1alpha1/scenarios/two-replicas-one-authorization.yaml`
- **cited (delivered):** `contracts/auth/acquisition/v1alpha1/semantics.md`
- **cited (delivered):** `contracts/auth/acquisition/v1alpha1/verification.md`
- **cited (delivered):** `contracts/auth/custody/v1alpha1/semantics.md`
- **cited (delivered):** `docs/adapters/atlassian.md`
- **cited (delivered):** `ess/domains/refresh.yaml`

The extra author/reviewer cases extend only the allocated private scenario directory. Original inferred allocation remains in the journal and wave page. Shared credentials.yaml, system.yaml and Rust gate changes were coordinator preparation before fork, with no concurrent implementor edits. Runtime auth implementation, generated files, migration proposals and publication remain outside scope. Confidence: high for final delivered paths (coordinator inference from the committed diff and reports).

## Dependencies and edit coordination

No semantic prerequisite is declared within this draft set.

Shared edit surfaces with `story:contracts-idempotency-scope`, `story:contracts-wire-compatibility`, `story:contracts-connection-readiness`, `story:contracts-document-admission`, `story:contracts-read-refresh-retry`, `story:contracts-management-boundary`, `story:contracts-acquisition-profiles`, `story:contracts-evidence-precision`, `story:contracts-persistence-ownership`, `story:contracts-mutation-visibility`. These collisions require serialized edits or a coordinated integration owner; this general collision inventory does not authorize concurrency. The operator subsequently approved F04/F05 as the specific wave recorded in `specification:auth-hardening-wave-20260908`, with shared preparation serialized before disjoint unit edits. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

The operator approved F04 and F05 as a two-story semantic hardening wave, with local commits and merges and no model budget limit. This supersedes the previous ESS/commit exclusion. Scope includes textual contracts, adapter design, ESS declarations and authored conformance scenarios; it excludes runtime auth implementation, generated-schema changes, publication and adapter-set expansion.

The shared identity seam is `ess/domains/credentials.yaml`; `ess/system.yaml` registers each private domain. Unsettled relations and runtime predicates remain explicit `UNMAPPED` obligations. Authored scenarios are compiled, not executed runtime conformance. Each unit requires a textual failure/scenario matrix, pinned ESS validation/compilation and authored scenario compilation with zero refusals, adversarial review and the full integration gate. See `specification:auth-hardening-wave-20260908` for scope ownership and evidence.

## Completion evidence

Semantic/model acceptance verified against integration merge `2a4404958449b1b91c5d494c5e4473ab6725bd8f`. Normative rules and failure matrix are in `contracts/auth/acquisition/v1alpha1/semantics.md` and `verification.md`; typed domain is `ess/domains/refresh.yaml`. Private authored count13; reviewer pass1 returned0findings and added one boundary scenario. Full gate with --msrv exited0:39Rust tests,92ESS declarations,169compiled scenarios including34authored,0refusals. Auth runtime execution remains0; UNMAPPED predicates and atomicity are implementation obligations, not claims discharged by the compiler. See docs/waves/auth-hardening-20260908/integration/gate-summary.json and full-gate.log.

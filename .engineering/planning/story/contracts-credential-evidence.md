---
format: aep.planning-md/1
id: story:contracts-credential-evidence
kind: story
status: implemented
title: Bind readiness evidence to the credential actually dispatched
tags:
- P1
- contract-review
relations:
- decomposes: epic:contract-semantics-remediation
- informed_by: specification:contract-review-intake-20260908
scope:
- confidence: cited
  path: contracts/auth/capability/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/connection/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/evidence/v1alpha1/scenarios/configured-replacement-identity.yaml
- confidence: cited
  path: contracts/auth/evidence/v1alpha1/scenarios/retained-lineage-expires-before-dispatch.yaml
- confidence: cited
  path: contracts/auth/evidence/v1alpha1/scenarios/revocation-before-dispatch.yaml
- confidence: cited
  path: contracts/auth/evidence/v1alpha1/scenarios/rotation-before-dispatch.yaml
- confidence: cited
  path: contracts/auth/evidence/v1alpha1/scenarios/same-identity-refresh.yaml
- confidence: cited
  path: contracts/auth/evidence/v1alpha1/scenarios/unvalidated-configured-replacement.yaml
- confidence: cited
  path: contracts/auth/evidence/v1alpha1/semantics.md
- confidence: cited
  path: contracts/auth/evidence/v1alpha1/verification.md
- confidence: cited
  path: docs/adapters/kubernetes.md
- confidence: cited
  path: ess/domains/credential_evidence.yaml
revision: 26
---
## Context

Priority: **P1**. Sources: `F05` in `specification:contract-review-intake-20260908`.

Configured credential replacement can preserve age-valid evidence for another account while dispatch uses new material.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

## Required revision

Specify credential-generation evidence validity and admission/dispatch consistency; distinguish same-identity rotation from reassignment without exposing secret-derived identifiers in public metadata.

## Acceptance

After revision, every credential-replacement scenario uses evidence valid for the dispatched credential identity.

## Verification scenarios

- Account A token replaced by account B token → explicit reassignment or refusal.
- Rotation occurs between admission and dispatch → revalidation or stable validated lease.
- Same-identity refresh invalidates only the checks the contract explicitly permits retaining.

Verification combines the textual scenario audit with pinned ESS model validation, compilation and authored scenario compilation. Record the chosen rule and expected observation/refusal, cross-check cited documents, preserve adversarial reviews and close on the combined repository gate. Scenario compilation is not runtime conformance; the runtime binding remains subsequent work.

## Scope

Confirmed at delivery by the implementor scope audit and committed unit diff. All original cited surfaces were correct; the inferred ESS domain, verification file and scenario locations were absent/reserved as expected and are now authored. No inferred location proved wrong.

- **cited (delivered):** `contracts/auth/capability/v1alpha1/semantics.md`
- **cited (delivered):** `contracts/auth/connection/v1alpha1/semantics.md`
- **cited (delivered):** `contracts/auth/evidence/v1alpha1/scenarios/configured-replacement-identity.yaml`
- **cited (delivered):** `contracts/auth/evidence/v1alpha1/scenarios/retained-lineage-expires-before-dispatch.yaml`
- **cited (delivered):** `contracts/auth/evidence/v1alpha1/scenarios/revocation-before-dispatch.yaml`
- **cited (delivered):** `contracts/auth/evidence/v1alpha1/scenarios/rotation-before-dispatch.yaml`
- **cited (delivered):** `contracts/auth/evidence/v1alpha1/scenarios/same-identity-refresh.yaml`
- **cited (delivered):** `contracts/auth/evidence/v1alpha1/scenarios/unvalidated-configured-replacement.yaml`
- **cited (delivered):** `contracts/auth/evidence/v1alpha1/semantics.md`
- **cited (delivered):** `contracts/auth/evidence/v1alpha1/verification.md`
- **cited (delivered):** `docs/adapters/kubernetes.md`
- **cited (delivered):** `ess/domains/credential_evidence.yaml`

The extra author/reviewer cases extend only the allocated private scenario directory. Original inferred allocation remains in the journal and wave page. Shared credentials.yaml, system.yaml and Rust gate changes were coordinator preparation before fork, with no concurrent implementor edits. Runtime auth implementation, generated files, migration proposals and publication remain outside scope. Confidence: high for final delivered paths (coordinator inference from the committed diff and reports).

## Dependencies and edit coordination

No semantic prerequisite is declared within this draft set.

Shared edit surfaces with `story:contracts-wire-compatibility`, `story:contracts-connection-readiness`, `story:contracts-permission-budgets`, `story:contracts-anonymous-auth`, `story:contracts-restart-idempotency`, `story:contracts-discovery-coverage`, `story:contracts-read-refresh-retry`, `story:contracts-management-boundary`, `story:contracts-discovery-profiles`, `story:contracts-acquisition-profiles`, `story:contracts-evidence-precision`, `story:contracts-media-controls`, `story:contracts-supported-vocabulary`, `story:contracts-persistence-ownership`. These collisions require serialized edits or a coordinated integration owner; this general collision inventory does not authorize concurrency. The operator subsequently approved F04/F05 as the specific wave recorded in `specification:auth-hardening-wave-20260908`, with shared preparation serialized before disjoint unit edits. Semantic dependencies do not stand in for edit-collision records. Each story owns only its listed findings, not sibling outcomes on the same file.

## Boundary and modeling

The operator approved F04 and F05 as a two-story semantic hardening wave, with local commits and merges and no model budget limit. This supersedes the previous ESS/commit exclusion. Scope includes textual contracts, adapter design, ESS declarations and authored conformance scenarios; it excludes runtime auth implementation, generated-schema changes, publication and adapter-set expansion.

The shared identity seam is `ess/domains/credentials.yaml`; `ess/system.yaml` registers each private domain. Unsettled relations and runtime predicates remain explicit `UNMAPPED` obligations. Authored scenarios are compiled, not executed runtime conformance. Each unit requires a textual failure/scenario matrix, pinned ESS validation/compilation and authored scenario compilation with zero refusals, adversarial review and the full integration gate. See `specification:auth-hardening-wave-20260908` for scope ownership and evidence.

## Scope refinement during authoring

Implementor confirmed the private model and three planned authored scenarios; added inferred verification surfaces `contracts/auth/evidence/v1alpha1/scenarios/unvalidated-configured-replacement.yaml` and `contracts/auth/evidence/v1alpha1/scenarios/revocation-before-dispatch.yaml` to cover missing validation and known revocation at dispatch. This extends only the assigned private scenario directory.

## Completion evidence

Semantic/model acceptance verified against integration merge `2a4404958449b1b91c5d494c5e4473ab6725bd8f`. Normative rules and failure matrix are in `contracts/auth/evidence/v1alpha1/semantics.md` and `verification.md`; typed domain is `ess/domains/credential_evidence.yaml`. Private authored count6; reviewer pass1 returned0findings and added one boundary scenario. Full gate with --msrv exited0:39Rust tests,92ESS declarations,169compiled scenarios including34authored,0refusals. Auth runtime execution remains0; UNMAPPED predicates and atomicity are implementation obligations, not claims discharged by the compiler. See docs/waves/auth-hardening-20260908/integration/gate-summary.json and full-gate.log.

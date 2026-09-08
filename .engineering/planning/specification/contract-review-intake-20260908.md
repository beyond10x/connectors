---
format: aep.planning-md/1
id: specification:contract-review-intake-20260908
kind: specification
status: draft
title: Consolidated contract review intake and finding ownership
relations:
- derived_from: specification:contract-driven-connectors-design
- informed_by: review-result:contract-semantics-20260908
- informed_by: review-result:contract-docs-external-20260908
revision: 6
---
## Intake

The operator asked to ingest the external review before creating stories for all priorities. This record reconciles 15 consolidated findings with all 33 external findings: **48 source items, each assigned exactly once to one of 27 draft stories**. Grouping preserves each source ID; it does not claim 48 independent defects.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

Original review files were `.local/review/2026-09-08-contract-semantics-review.md` and `.local/review/2026-09-08-contract-docs-independent-review.md`. Their complete contents are preserved in the two immutable review-result artifacts so the plan does not depend on ignored workspace files. The original source severity is retained below; triaged priority is a separate judgment.

## Priority policy

P0 means an established critical current failure requiring immediate containment; none is established by these proposed-contract reviews. P1 means foundational outcome/authority/lifecycle or compatibility rules that must settle before implementation builds on them. P2 means a substantive profile or ownership inconsistency to fix before the affected profile. P3 means evidence, index or support-vocabulary precision. An owner's priority is the highest-priority item it carries. There are **0 P0, 7 P1, 16 P2, 4 P3 stories**. No source finding is discarded because of priority.

## Disposition and ownership

| Source | Original severity | Triaged | Sole story owner | Disposition |
|---|---|---|---|---|
| F01 | P1 | P1 | `story:contracts-mutation-outcomes` | Fixed in semantic contracts/ESS by `story:contracts-mutation-outcomes`; see `contracts/operations/v1alpha1/verification.md` and both immutable mutation-outcomes review records. Runtime binding remains future work. |
| F02 | P1 | P1 | `story:contracts-idempotency-scope` | Fixed in semantic contracts/ESS by `story:contracts-idempotency-scope`; see `contracts/operations/v1alpha1/idempotency-verification.md` and both final independent review records. Runtime algorithms remain future binding work. |
| F03 | P1 | P1 | `story:contracts-federated-approval` | Accepted for remediation; assigned, not fixed. |
| F04 | P1 | P1 | `story:contracts-refresh-coordination` | Fixed at semantic-contract/ESS level in the auth hardening wave; see `contracts/auth/acquisition/v1alpha1/verification.md`, immutable adversarial review and the full gate at `2a440495`. Runtime binding remains future work. |
| F05 | P1 | P1 | `story:contracts-credential-evidence` | Fixed at semantic-contract/ESS level in the auth hardening wave; see `contracts/auth/evidence/v1alpha1/verification.md`, immutable adversarial review and the full gate at `2a440495`. Runtime binding remains future work. |
| F06 | P1 | P1 | `story:contracts-session-revocation` | Accepted for remediation; assigned, not fixed. |
| F07 | P2 | P2 | `story:contracts-connection-readiness` | Accepted for remediation; assigned, not fixed. |
| F08 | P2 | P2 | `story:contracts-permission-budgets` | Accepted for remediation; assigned, not fixed. |
| F09 | P2 | P2 | `story:contracts-anonymous-auth` | Accepted for remediation; assigned, not fixed. |
| F10 | P2 | P2 | `story:contracts-mutation-classification` | Accepted for remediation; assigned, not fixed. |
| F11 | P2 | P2 | `story:contracts-restart-idempotency` | Accepted for remediation; assigned, not fixed. |
| F12 | P2 | P2 | `story:contracts-log-continuation` | Accepted for remediation; assigned, not fixed. |
| F13 | P2 | P2 | `story:contracts-discovery-coverage` | Accepted for remediation; assigned, not fixed. |
| F14 | P2 | P2 | `story:contracts-document-admission` | Accepted for remediation; assigned, not fixed. |
| F15 | P2 | P2 | `story:contracts-read-refresh-retry` | Accepted for remediation; assigned, not fixed. |
| E01 | blocking | P1 | `story:contracts-mutation-outcomes` | Fixed in semantic contracts/ESS by `story:contracts-mutation-outcomes`; see `contracts/operations/v1alpha1/verification.md` and both immutable mutation-outcomes review records. Runtime binding remains future work. |
| E02 | blocking | P1 | `story:contracts-wire-compatibility` | Qualified: unchanged configured behavior can remain compatible; new managed descriptors/errors need explicit versioning. |
| E03 | blocking | P2 | `story:contracts-host-composition` | Qualified: colocation is allowed; concrete wiring ownership must preserve the generic-host dependency invariant. |
| E04 | should-fix | P2 | `story:contracts-management-boundary` | Qualified: the design prohibits exposing private callback authority, not every use of an operations envelope. |
| E05 | should-fix | P2 | `story:contracts-read-refresh-retry` | Accepted for remediation; assigned, not fixed. |
| E06 | should-fix | P2 | `story:contracts-mutation-classification` | Accepted for remediation; assigned, not fixed. |
| E07 | should-fix | P2 | `story:contracts-discovery-profiles` | Accepted for remediation; assigned, not fixed. |
| E08 | should-fix | P2 | `story:contracts-restart-idempotency` | Qualified duplicate: old UID/resourceVersion preconditions prevent asserting that every repeated dispatch causes another rollout. |
| E09 | should-fix | P2 | `story:contracts-acquisition-profiles` | Accepted for remediation; assigned, not fixed. |
| E10 | should-fix | P2 | `story:contracts-anonymous-auth` | Accepted for remediation; assigned, not fixed. |
| E11 | should-fix | P2 | `story:contracts-mutation-outcomes` | Fixed in semantic contracts/ESS by `story:contracts-mutation-outcomes`; see `contracts/operations/v1alpha1/verification.md` and both immutable mutation-outcomes review records. Runtime binding remains future work. |
| E12 | should-fix | P2 | `story:contracts-mutation-visibility` | Not established as stated: implemented-only advertisement does not imply disabled operations are hidden; story resolves the actual visibility ambiguity. |
| E13 | should-fix | P3 | `story:contracts-documentation-index` | Accepted for remediation; assigned, not fixed. |
| E14 | should-fix | P2 | `story:contracts-discovery-profiles` | Accepted with correction: the old rule uses exact name OR exact stable name label, not a mandatory name-and-label conjunction. |
| E15 | nit | P3 | `story:contracts-evidence-precision` | Accepted for remediation; assigned, not fixed. |
| E16 | nit | P3 | `story:contracts-evidence-precision` | Accepted for remediation; assigned, not fixed. |
| E17 | nit | P3 | `story:contracts-evidence-precision` | Accepted for remediation; assigned, not fixed. |
| E18 | nit | P3 | `story:contracts-evidence-precision` | Accepted for remediation; assigned, not fixed. |
| E19 | nit | P2 | `story:contracts-connection-readiness` | Accepted for remediation; assigned, not fixed. |
| E20 | nit | P2 | `story:contracts-session-revocation` | Accepted for remediation; assigned, not fixed. |
| E21 | nit | P2 | `story:contracts-connection-readiness` | Accepted for remediation; assigned, not fixed. |
| E22 | nit | P3 | `story:contracts-documentation-index` | Accepted for remediation; assigned, not fixed. |
| E23 | nit | P2 | `story:contracts-media-controls` | Accepted for remediation; assigned, not fixed. |
| E24 | nit | P2 | `story:contracts-media-controls` | Accepted for remediation; assigned, not fixed. |
| E25 | nit | P3 | `story:contracts-evidence-precision` | Accepted evidence qualification; no vendor API correctness is claimed by this intake. |
| E26 | nit | P3 | `story:contracts-evidence-precision` | Accepted example/normative distinction; preserve intentionally selected contract defaults rather than blindly relabel every number. |
| E27 | nit | P3 | `story:contracts-supported-vocabulary` | Qualified: inventory reserved support and rationale; blanket removal is not established by the cited design rule. |
| E28 | nit | P2 | `story:contracts-persistence-ownership` | Accepted for remediation; assigned, not fixed. |
| E29 | nit | P2 | `story:contracts-acquisition-profiles` | Accepted for remediation; assigned, not fixed. |
| E30 | nit | P3 | `story:contracts-tenant-header` | Accepted for remediation; assigned, not fixed. |
| E31 | nit | P3 | `story:contracts-evidence-precision` | Accepted evidence limitation: an untyped item schema does not guarantee the claimed status projection. |
| E32 | nit | P2 | `story:contracts-discovery-profiles` | Accepted for remediation; assigned, not fixed. |
| E33 | nit | P3 | `story:contracts-evidence-precision` | Accepted for remediation; assigned, not fixed. |

## Adjudication limits

The stronger external claims are qualified rather than silently adopted: timeout implementation gaps are future obligations; same-process composition does not imply forbidden imports; management envelopes are not categorically forbidden by design §16.1; disabled is not synonymous with unimplemented; Kubernetes old preconditions matter; exact Argo name OR label is the cited rule; and reservation alone does not prove a vocabulary should be deleted. Their owning stories specify the residual clarification or correction, so a disputed premise does not disappear from the plan.

The first review's additional open decisions remain visible, but were not among F01–F15: exact-generation route renewal, provider-specific query-scope enforcement, document-body/truncation representation, connection uniqueness and subject:none binding context, and later backend/framing/limit choices. Their scope must be settled before an affected implementation is decomposed; this intake does not pretend the current entity model resolves them. Docker vendor verification remains a declared authoring obligation. No external service or vendor API was queried for this intake.

## Modeling boundary

The operator subsequently authorized hardening semantic contracts together with their ESS models, after local checkpoint `57be07c` on 2026-09-08. The original text-only planning boundary is superseded for activated hardening stories. Model settled identities, fields and lifecycle rules with the pinned ESS workflow; compile corresponding scenarios and keep unsupported semantics and unresolved relations explicitly UNMAPPED. Runtime mutation implementation, wire expansion, extra adapters, publication and Atlas changes remain outside this epic. Refine each owning story's machine-readable scope before activation; ESS files add shared edit surfaces and do not authorize a concurrent implementation wave.

The first completed hardening story is `story:contracts-mutation-outcomes`, owning F01/E01/E11. Its typed home is `ess/domains/mutations.yaml`; its gate checks are specification validation and scenario compilation, not a claim of a runtime conformance binding.

## Completion accounting

F01/F02/F04/F05/E01/E11 are **fixed at the semantic-contract/model level**; the remaining **42 source findings are assigned, not fixed**. Drafting or reviewing the plan is not evidence that a contract defect is fixed. Future closure records the revised normative passages and scenario results against each owner's source IDs; a qualified item can close through an explicit supported disposition without implementing the external reviewer's suggested architecture.

## Acceptance

After ingestion, every finding from the previously separate 15-item and 33-item reviews has exactly one recorded story owner and an explicit disposition in this ledger.

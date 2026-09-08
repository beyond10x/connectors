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
revision: 12
---
## Intake

The operator asked to ingest the external review before creating stories for all priorities. This record reconciles 15 consolidated findings with all 33 external findings: **48 source items, each assigned exactly once to one of 27 originally drafted stories**. Grouping preserves each source ID; it does not claim 48 independent defects.

Baseline for all file:line citations is local commit `db1c329` unless an old-repository path is named. Source findings F01–F15 are preserved in `review-result:contract-semantics-20260908`; E01–E33 are preserved in `review-result:contract-docs-external-20260908`.

Original review files were `.local/review/2026-09-08-contract-semantics-review.md` and `.local/review/2026-09-08-contract-docs-independent-review.md`. Their complete contents are preserved in the two immutable review-result artifacts so the plan does not depend on ignored workspace files. The original source severity is retained below; triaged priority is a separate judgment.

## Priority policy

P0 means an established critical current failure requiring immediate containment; none is established by these proposed-contract reviews. P1 means foundational outcome/authority/lifecycle or compatibility rules that must settle before implementation builds on them. P2 means a substantive profile or ownership inconsistency to fix before the affected profile. P3 means evidence, index or support-vocabulary precision. An owner's priority is the highest-priority item it carries. There are **0 P0, 7 P1, 16 P2, 4 P3 stories**. No source finding is discarded because of priority.

## Disposition and ownership

| Source | Original severity | Triaged | Sole story owner | Disposition |
|---|---|---|---|---|
| F01 | P1 | P1 | `story:contracts-mutation-outcomes` | Fixed in semantic contracts/ESS by `story:contracts-mutation-outcomes`; see `contracts/operations/v1alpha1/verification.md` and both immutable mutation-outcomes review records. Runtime binding remains future work. |
| F02 | P1 | P1 | `story:contracts-idempotency-scope` | Fixed in semantic contracts/ESS by `story:contracts-idempotency-scope`; see `contracts/operations/v1alpha1/idempotency-verification.md` and both final independent review records. Runtime algorithms remain future binding work. |
| F03 | P1 | P1 | `story:contracts-federated-approval` | Fixed at semantic-contract/ESS level: canonical leaf subject and preparation, scoped exact delivery/approval proofs, receiver lifetime and nonce-entry rules, sole leaf redemption and conservative forwarding failures. Two final independent approvals and bounded verification evidence; runtime remains unimplemented. |
| F04 | P1 | P1 | `story:contracts-refresh-coordination` | Fixed at semantic-contract/ESS level in the auth hardening wave; see `contracts/auth/acquisition/v1alpha1/verification.md`, immutable adversarial review and the full gate at `2a440495`. Runtime binding remains future work. |
| F05 | P1 | P1 | `story:contracts-credential-evidence` | Fixed at semantic-contract/ESS level in the auth hardening wave; see `contracts/auth/evidence/v1alpha1/verification.md`, immutable adversarial review and the full gate at `2a440495`. Runtime binding remains future work. |
| F06 | P1 | P1 | `story:contracts-session-revocation` | Fixed at semantic-contract/ESS level by `story:contracts-session-revocation`; sessions §4.1 and verification.md record bounded cutoff/teardown, independent approvals and explicit runtime obligations. |
| F07 | P2 | P2 | `story:contracts-connection-readiness` | Fixed at semantic-contract/ESS-value level: one global viability reduction plus exact-operation eligibility; optional write scope and target-specific failures no longer poison eligible reads. Two final independent approvals and bounded verification evidence. |
| F08 | P2 | P2 | `story:contracts-permission-budgets` | Fixed at semantic-contract/ESS-value level: exact target/cache binding, bounded 64-target/64-call/4-concurrent authorization, preflight refusal and explicit denied-target coverage. Both final independent reviewers approve; see docs/evidence/auth-profile-budget-20260908/verification.md. Runtime remains unimplemented. |
| F09 | P2 | P2 | `story:contracts-anonymous-auth` | Fixed at semantic-contract/ESS-value level: explicit anonymous and parent-authenticated profile/placement rules without fabricated child credentials or fallback. Both final independent reviewers approve; see docs/evidence/auth-profile-budget-20260908/verification.md. Runtime remains unimplemented. |
| F10 | P2 | P2 | `story:contracts-mutation-classification` | Accepted for remediation; assigned, not fixed. |
| F11 | P2 | P2 | `story:contracts-restart-idempotency` | Accepted for remediation; assigned, not fixed. |
| F12 | P2 | P2 | `story:contracts-log-continuation` | Accepted for remediation; assigned, not fixed. |
| F13 | P2 | P2 | `story:contracts-discovery-coverage` | Accepted for remediation; assigned, not fixed. |
| F14 | P2 | P2 | `story:contracts-document-admission` | Accepted for remediation; assigned, not fixed. |
| F15 | P2 | P2 | `story:contracts-read-refresh-retry` | Accepted for remediation; assigned, not fixed. |
| E01 | blocking | P1 | `story:contracts-mutation-outcomes` | Fixed in semantic contracts/ESS by `story:contracts-mutation-outcomes`; see `contracts/operations/v1alpha1/verification.md` and both immutable mutation-outcomes review records. Runtime binding remains future work. |
| E02 | blocking | P1 | `story:contracts-wire-compatibility` | Fixed by `story:contracts-wire-compatibility`: one version/field/profile matrix, explicit v1alpha2 routes, unchanged-read legacy projection with invoke enforcement, exact mutation/audit encoding and ESS values; 68 current decoder vectors, 27 textual response vectors, full gate and two final independent approvals. |
| E03 | blocking | P2 | `story:contracts-host-composition` | Qualified: colocation is allowed; concrete wiring ownership must preserve the generic-host dependency invariant. |
| E04 | should-fix | P2 | `story:contracts-management-boundary` | Fixed by host-owned separately admitted management, explicit instance/connection/acquisition targets, one coordinator through federation, protected UI/completion boundaries and separate local/provider revocation observations. Operation-envelope reuse does not assign coordination to adapters; concrete bindings remain advertisement gates. |
| E05 | should-fix | P2 | `story:contracts-read-refresh-retry` | Accepted for remediation; assigned, not fixed. |
| E06 | should-fix | P2 | `story:contracts-mutation-classification` | Accepted for remediation; assigned, not fixed. |
| E07 | should-fix | P2 | `story:contracts-discovery-profiles` | Accepted for remediation; assigned, not fixed. |
| E08 | should-fix | P2 | `story:contracts-restart-idempotency` | Qualified duplicate: old UID/resourceVersion preconditions prevent asserting that every repeated dispatch causes another rollout. |
| E09 | should-fix | P2 | `story:contracts-acquisition-profiles` | Fixed at semantic-contract/ESS-value level: separate static_config activation and managed acquisition with flow-specific endpoint/field requirements. Both final independent reviewers approve; see docs/evidence/auth-profile-budget-20260908/verification.md. Runtime remains unimplemented. |
| E10 | should-fix | P2 | `story:contracts-anonymous-auth` | Fixed at semantic-contract/ESS-value level: distinct anonymous/via_parent monitoring rows and admitted no-child-credential capability/configuration semantics. Both final independent reviewers approve; see docs/evidence/auth-profile-budget-20260908/verification.md. Runtime remains unimplemented. |
| E11 | should-fix | P2 | `story:contracts-mutation-outcomes` | Fixed in semantic contracts/ESS by `story:contracts-mutation-outcomes`; see `contracts/operations/v1alpha1/verification.md` and both immutable mutation-outcomes review records. Runtime binding remains future work. |
| E12 | should-fix | P2 | `story:contracts-mutation-visibility` | Not established as stated: implemented-only advertisement does not imply disabled operations are hidden; story resolves the actual visibility ambiguity. |
| E13 | should-fix | P3 | `story:contracts-documentation-index` | Fixed during `story:independent-review-remediation`: configuration is in the deferred-family list and index/design counts agree. The owner remains draft for E22 and prerequisite-dependent coverage. |
| E14 | should-fix | P2 | `story:contracts-discovery-profiles` | Accepted with correction: the old rule uses exact name OR exact stable name label, not a mandatory name-and-label conjunction. |
| E15 | nit | P3 | `story:contracts-evidence-precision` | Accepted for remediation; assigned, not fixed. |
| E16 | nit | P3 | `story:contracts-evidence-precision` | Accepted for remediation; assigned, not fixed. |
| E17 | nit | P3 | `story:contracts-evidence-precision` | Accepted for remediation; assigned, not fixed. |
| E18 | nit | P3 | `story:contracts-evidence-precision` | Accepted for remediation; assigned, not fixed. |
| E19 | nit | P2 | `story:contracts-connection-readiness` | Fixed: acquisition payloads/scenarios consistently use insufficient_scope, distinct from generic evidence result insufficient and the global readiness state. |
| E20 | nit | P2 | `story:contracts-session-revocation` | Fixed by `story:contracts-session-revocation`: media_incompatible and media_overload share the sessions terminal vocabulary; both authored cases compile and both independent rechecks approve. |
| E21 | nit | P2 | `story:contracts-connection-readiness` | Fixed: one seven-state viability vocabulary includes pending/disabled/revoked with deterministic precedence; status is a reduction, not a fictitious ESS Connection lifecycle. |
| E22 | nit | P3 | `story:contracts-documentation-index` | Accepted for remediation; assigned, not fixed. |
| E23 | nit | P2 | `story:contracts-media-controls` | Accepted for remediation; assigned, not fixed. |
| E24 | nit | P2 | `story:contracts-media-controls` | Accepted for remediation; assigned, not fixed. |
| E25 | nit | P3 | `story:contracts-evidence-precision` | Accepted evidence qualification; no vendor API correctness is claimed by this intake. |
| E26 | nit | P3 | `story:contracts-evidence-precision` | Accepted example/normative distinction; preserve intentionally selected contract defaults rather than blindly relabel every number. |
| E27 | nit | P3 | `story:contracts-supported-vocabulary` | Qualified: inventory reserved support and rationale; blanket removal is not established by the cited design rule. |
| E28 | nit | P2 | `story:contracts-persistence-ownership` | Accepted for remediation; assigned, not fixed. |
| E29 | nit | P2 | `story:contracts-acquisition-profiles` | Fixed at semantic-contract/ESS-value level: client-credentials token-only endpoint requirement and explicit specified/reserved/implemented support matrix. Both final independent reviewers approve; see docs/evidence/auth-profile-budget-20260908/verification.md. Runtime remains unimplemented. |
| E30 | nit | P3 | `story:contracts-tenant-header` | Fixed by `story:contracts-tenant-header`: logs §4.1 and the Grafana child example use receiver-owned http.extra_headers; the earlier tenant_header label is not an alias or implemented key. Textual audit and independent review pass. |
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

**21 of 48 source findings are fixed; 27 remain assigned, not fixed.** Thirteen of the 27 owner stories are implemented for their specification scope; fourteen remain draft. The documentation-index story remains draft for E22 and its dependencies even though E13 is fixed. Each source ID retains exactly one owner in the table above.

The latest checkpoint closes F08/F09/E09/E10/E29 through explicit anonymous/parent-authenticated access, bounded exact-target permission checks and flow-specific acquisition paths. Both independent final reviewers approve; 21 individual review findings have fixed outcomes. Evidence: [verification](../../../docs/evidence/auth-profile-budget-20260908/verification.md), [dispositions](../../../docs/evidence/auth-profile-budget-20260908/dispositions.md), [review provenance](../../../docs/evidence/auth-profile-budget-20260908/checkpoint.md). ESS values and schema checks do not execute runtime authority, transport, acquisition, cache or budget behavior.

Earlier reviewed checkpoints remain recorded in the owning stories and epic: mutation outcomes/idempotency, refresh/credential evidence, session revocation/tenant headers, complete wire compatibility, federated approval, and connection viability/host management. Persistence consolidation still depends on discovery coverage. Remaining original findings and the broader catalog/stack/open-decision scope keep the overall goal active. A plan/review alone is not a fixed defect; future closure requires the corresponding normative revision and honest verification evidence.

## Acceptance

After ingestion, every finding from the previously separate 15-item and 33-item reviews has exactly one recorded story owner and an explicit disposition in this ledger.

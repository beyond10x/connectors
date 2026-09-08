---
format: aep.planning-md/1
id: epic:contract-semantics-remediation
kind: epic
status: active
title: Resolve the contract and adapter design review findings
tags:
- contract-review
relations:
- derived_from: specification:contract-driven-connectors-design
- informed_by: specification:contract-review-intake-20260908
revision: 12
---
## Context

The operator paused further implementation to settle textual contracts, then requested stories for every finding after incorporating an external review. The owning design remains `specification:contract-driven-connectors-design`; the full finding ledger and severity rationale are `specification:contract-review-intake-20260908`.

## Outcome

Resolve or explicitly dispose of all **48 source findings** (F01–F15 and E01–E33) through the **27 stories** below, preserving one owner per finding and recording the revised contract rules and textual conformance evidence.

## Acceptance

After the child stories complete, the intake ledger maps every source finding to a reviewed contract correction or an explicit evidence-backed disposition.

## Stories and priorities

| Priority | Story | Findings |
|---|---|---|
| P1 | `story:contracts-mutation-outcomes` — Make mutation outcomes truthful across dispatch and recovery | F01, E01, E11 |
| P1 | `story:contracts-idempotency-scope` — Define idempotency ownership and replay admission | F02 |
| P1 | `story:contracts-federated-approval` — Bind approvals consistently across federation | F03 |
| P1 | `story:contracts-refresh-coordination` — Specify refresh exclusion and recovery after owner loss | F04 |
| P1 | `story:contracts-credential-evidence` — Bind readiness evidence to the credential actually dispatched | F05 |
| P1 | `story:contracts-session-revocation` — Define bounded traffic cessation and terminal media reasons | F06, E20 |
| P1 | `story:contracts-wire-compatibility` — Define versioned compatibility for proposed contract extensions | E02 |
| P2 | `story:contracts-connection-readiness` — Separate connection viability from per-operation eligibility | F07, E19, E21 |
| P2 | `story:contracts-permission-budgets` — Define authorization-check budgets for namespace fan-out | F08 |
| P2 | `story:contracts-anonymous-auth` — Represent deliberate anonymous and parent-authenticated access | F09, E10 |
| P2 | `story:contracts-mutation-classification` — Align SIP mutation effects with the shared discriminator | F10, E06 |
| P2 | `story:contracts-restart-idempotency` — State exact idempotency guarantees for lifecycle operations | F11, E08 |
| P2 | `story:contracts-log-continuation` — Make bounded log continuation truthful at timestamp ties | F12 |
| P2 | `story:contracts-discovery-coverage` — Separate incomplete discovery from confirmed withdrawal | F13 |
| P2 | `story:contracts-document-admission` — Define trusted document membership before content access | F14 |
| P2 | `story:contracts-read-refresh-retry` — Version and bound read redispatch after refresh | F15, E05 |
| P2 | `story:contracts-host-composition` — Assign mediated adapter wiring to an explicit composition owner | E03 |
| P2 | `story:contracts-management-boundary` — Clarify ownership and admission of connection management | E04 |
| P2 | `story:contracts-discovery-profiles` — Make discovery profile binding and Kubernetes recognition explicit | E07, E14, E32 |
| P2 | `story:contracts-acquisition-profiles` — Align configured and OAuth acquisition profile requirements | E09, E29 |
| P3 | `story:contracts-documentation-index` — Reconcile contract and adapter coverage indexes | E13, E22 |
| P3 | `story:contracts-evidence-precision` — Correct citations and distinguish examples from verified provider facts | E15, E16, E17, E18, E25, E26, E31, E33 |
| P2 | `story:contracts-media-controls` — Separate media operations, transport controls and redemption evidence | E23, E24 |
| P3 | `story:contracts-supported-vocabulary` — Record support and disposition for reserved vocabulary | E27 |
| P2 | `story:contracts-persistence-ownership` — Inventory host persistence ports and atomicity owners | E28 |
| P3 | `story:contracts-tenant-header` — Use one configuration name for the monitoring tenant header | E30 |
| P2 | `story:contracts-mutation-visibility` — Distinguish implemented, enabled and discoverable mutations | E12 |

No P0 is supported by the reviewed evidence; the seven P1 stories establish the foundational rules. Priorities are not approval or evidence of implementation. Mutation outcomes and idempotency scope are implemented as semantic hardening with ESS models; the other 25 children remain draft. The planning verification below records the earlier draft baseline.

## Sequencing and shared files

Follow the explicit depends_on edges for semantic inputs, not a single global serial chain. Machine-readable file scope is recorded for every story, and each body names the siblings sharing those files. Shared-document changes must be serialized or coordinated by one integration owner; this set is not an authorized concurrent implementation wave. Index reconciliation follows its declared table-producing stories. Evidence corrections and support-vocabulary review own only their source findings even when touching the same documents.

## Scope and exclusions

The operator subsequently authorized hardening semantic contracts together with their ESS models, after local checkpoint `57be07c` on 2026-09-08. The original text-only planning boundary is superseded for activated hardening stories. Model settled identities, fields and lifecycle rules with the pinned ESS workflow; compile corresponding scenarios and keep unsupported semantics and unresolved relations explicitly UNMAPPED. Runtime mutation implementation, wire expansion, extra adapters, publication and Atlas changes remain outside this epic. Refine each owning story's machine-readable scope before activation; ESS files add shared edit surfaces and do not authorize a concurrent implementation wave.

The first completed hardening story is `story:contracts-mutation-outcomes`, owning F01/E01/E11. Its typed home is `ess/domains/mutations.yaml`; its gate checks are specification validation and scenario compilation, not a claim of a runtime conformance binding.

The work covers contracts, their adapter plans, the contract index and design cross-references. The current implemented adapters remain Kubernetes, GitLab and SQL; documenting additional adapter plans is not implementation authorization. No remote, Atlas, rollout or old-repository changes are included. First-review open decisions outside F01–F15 remain in the intake record; this epic promises the 48 enumerated findings rather than silently deciding those product questions.

## Review of this decomposition

Run the four planning perspectives: `aep-plan:plan-critic-acceptance`, `aep-plan:plan-critic-design`, `aep-plan:plan-critic-scope`, and `aep-plan:plan-critic-parallel-safety`. Record their exact verdicts as immutable review-result artifacts and any revision outcomes. The harness has three worker slots, so the four independent readers use the same frozen draft in two batches with no cross-review feedback. Its callable agent interface does not expose the plugin agent types or the requested Sonnet model; use general subagents reading the exact role files with available `gpt-5.6-sol` at high effort, and record this deviation. This is an interactive run with no approval-bypass records.

## Planning verification — 2026-09-08

The final critic round approved the decomposition in all four lanes: [acceptance](../review-result/contract-plan-acceptance-r2-20260908.md), [design](../review-result/contract-plan-design-r2-20260908.md), [scope](../review-result/contract-plan-scope-r2-20260908.md), [parallel-safety](../review-result/contract-plan-parallel-safety-r2-20260908.md). Round one found a missing intake acceptance condition and a missing dependency from persistence ownership to federated approval; both were corrected through AEP and have fixed review_outcome evidence. There were no second-round findings, no third round, and no approval bypasses.

Coverage checks establish 48 source findings with one owner each, 27 draft stories, 104 cited file-scope entries and 159 acknowledged intersecting story pairs; dependencies are acyclic. Both original review bodies are retained verbatim. The planning agent changed no contract, runtime, ESS or old-repository source. The source findings remain assigned, not fixed.

Tool limitations: installed AEP reports protocol 0.54.0; validation succeeds but warns for the two original prose reviews and for approval records whose findings blocks contain an empty list. The CLI also printed revision 4 for the added persistence dependency while show/file metadata report revision 3; the actual body, edge and history were verified, without manual metadata repair. These diagnostics do not substitute for contract conformance and do not change the draft status.

Concurrent authoring was observed during the final check: `contracts/catalog/v1alpha1/semantics.md`, `docs/adapters/catalog.md`, and the catalog additions to `contracts/README.md` appeared outside this planning work. The 48-item intake remains anchored to `db1c329`; these later catalog additions have not been reviewed by this panel and are preserved unchanged by the planning agent. Reconcile the index against the then-current reviewed documents when its story is taken up.

## Hardening progress — 2026-09-08

`story:contracts-mutation-outcomes` is complete: F01/E01/E11 corrected in the contract and proposed ESS model, nine authored and 36 generated scenarios compiled, full gate passed, two independent readers approved. Runtime mutation behavior remains unimplemented. The intake now records 4 semantic fixes and 44 outstanding source findings; the other 25 stories remain draft. The first story now has exact scenario-file scopes; future stories must refine shared ESS scope before activation. This does not authorize a parallel implementation wave.

`story:contracts-idempotency-scope` is also complete: F02 corrected in the namespace/fingerprint/replay/retention rules and proposed ESS reservation model; full gate passed with 39 tests and 67 compiled ESS scenarios, and both final independent reviewers approved. A first-round cache-miss/claim-spend race was corrected through an authoritative winner recheck. Runtime authorization, atomicity and clock tests remain explicit obligations. The user is handling concurrent versioning documents separately; this story made no wire-version decision.

## Current specification checkpoint — session revocation and tenant-header naming

The auth wave completed `story:contracts-refresh-coordination` and `story:contracts-credential-evidence` (F04/F05), with verification and adversarial evidence under `docs/waves/auth-hardening-20260908/`. The latest single-agent checkpoint completes `story:contracts-session-revocation` (F06/E20) and `story:contracts-tenant-header` (E30). Session hardening passed two independent final rechecks, 13 authored ESS scenarios plus a separate 77-act model/trace/deadline audit, and the full gate with 50 existing Rust tests. No runtime implementation changed in this specification checkpoint; session timing, queues/devices, real authority and teardown remain executable obligations for future bindings.

Current ledger: 10/48 source findings fixed, 38 assigned and open; six contract stories implemented, 21 draft. E13 was fixed separately in the documentation inventory during independent-review remediation, but its documentation-index owner remains draft for E22 and prerequisite-dependent coverage. This supersedes the historical progress counts above. The active broad specification goal is not complete. Remaining work includes wire compatibility, federation approval, other auth/discovery/datasource semantics and the remaining adapter/index evidence. No wire-version decision or implementation wave was inferred from this checkpoint.

## Current specification checkpoint — wire compatibility

`story:contracts-wire-compatibility` closes E02 with one matrix covering all 15 proposed v1alpha1 families and the governed service binding. Explicit v1alpha2 paths, complete describe/invoke/audit/mutation shapes, exact error mapping, an invoke-enforced unchanged-read projection, safe catalog errors and consistent generic limits resolve the incompatible defaults. Public ESS values preserve existing effect/attempt semantics without introducing runtime code or guessed persistence. Both independent final reviewers approve with zero residual E02 findings. Evidence includes 68 existing-decoder vectors, 27 proposed textual vectors and the full gate with 50 Rust tests/MSRV 1.88; ESS 0.20.0 validates nine files. See [verification](../../../docs/evidence/wire-compatibility-20260908/verification.md).

Current ledger: **11/48 original findings fixed, 37 open; seven owner stories implemented for specification scope, 20 draft**. This supersedes the earlier checkpoint counts. F03 federated approval is now the next foundational semantic dependency; delegation remains explicitly unbound until it is settled. The broader specification goal remains active. No runtime or adapter-kind/generated schema change, external publication or implementation wave is included.

## Current specification checkpoint — federated approval

F03 is fixed by `story:contracts-federated-approval`: one canonical leaf subject exposed through safe admitted preparation, distinct exact delegated/issuer proof framing, receiver-enforced proof windows and post-nonce acknowledgement admission, sole-leaf redemption and unchanged F01/F02 effect/replay ownership. ESS models the values and immutable receipts with known links; concrete runtime/policy/issuer/persistence bindings remain advertisement prerequisites. Both final independent reviewers approve with zero residual findings; 21 individual review findings have fixed outcomes. Evidence is `docs/evidence/federated-approval-20260908/verification.md` and `checkpoint.md`. Full gate/MSRV1.88 passes with 50 existing Rust tests; ESS0.20.0 validates 10 files/153 declarations and deterministically produces 163 schema artifacts. No runtime or adapter implementation changed.

Current ledger: **12/48 source findings fixed, 36 assigned and open; eight owner stories implemented, 19 draft.** This supersedes the historical counts above, not their evidence. The broad specification stabilization goal remains active. Connection management/readiness, remaining auth/discovery/datasource/media semantics, persistence consolidation, catalog and later stack review obligations still need their own closure. All work remains local.

## Current specification checkpoint — connection viability and management

The latest checkpoint implements story:contracts-connection-readiness (F07/E19/E21) and story:contracts-management-boundary (E04) for their semantic scope. One ordered connection-wide viability reduction is separate from exact-operation scope/permission/verification and current authority. Host-owned management remains independently admitted when business credentials fail; exact targets, one acquisition coordinator, protected continuation/completion, and local-versus-provider revocation are explicit. ESS adds minimal typed values and identifies absent persistent owner models honestly. Both final independent reviewers approve with zero remaining findings; 25 individual reviewer findings have fixed outcomes.

The full gate/MSRV1.88 passes with 50 existing Rust tests; ESS0.20.0 validates 11 files/167 declarations and two generations of 176 schema artifacts match byte-for-byte. Eighty schema expectations and the declared decision/textual cases have precisely bounded evidence in docs/evidence/connection-semantics-20260908/verification.md. No runtime or adapter implementation changed.

Current ledger: **16/48 source findings fixed, 32 assigned and open; ten owner stories implemented, 17 draft.** This supersedes prior counts without rewriting their historical evidence. The broad specification goal remains active, including remaining auth/discovery/datasource/media semantics, persistence consolidation, catalog and later stack review. All work remains local.

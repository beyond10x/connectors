---
format: aep.planning-md/1
id: epic:contract-semantics-remediation
kind: epic
status: draft
title: Resolve the contract and adapter design review findings
tags:
- contract-review
relations:
- derived_from: specification:contract-driven-connectors-design
- informed_by: specification:contract-review-intake-20260908
revision: 3
---
## Context

The operator paused further implementation to settle textual contracts, then requested stories for every finding after incorporating an external review. The owning design remains `specification:contract-driven-connectors-design`; the full finding ledger and severity rationale are `specification:contract-review-intake-20260908`.

## Outcome

Resolve or explicitly dispose of all **48 source findings** (F01–F15 and E01–E33) through the **27 draft stories** below, preserving one owner per finding and recording the revised contract rules and textual conformance evidence.

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

No P0 is supported by the reviewed evidence; the seven P1 stories establish the foundational rules. Priorities are not approval or evidence of implementation. All children remain draft.

## Sequencing and shared files

Follow the explicit depends_on edges for semantic inputs, not a single global serial chain. Machine-readable file scope is recorded for every story, and each body names the siblings sharing those files. Shared-document changes must be serialized or coordinated by one integration owner; this set is not an authorized concurrent implementation wave. Index reconciliation follows its declared table-producing stories. Evidence corrections and support-vocabulary review own only their source findings even when touching the same documents.

## Scope and exclusions

This interactive, local-only story revises textual contracts, adapter design and their conformance scenarios; it does not implement runtime behavior, edit generated schemas, change ESS, commit/publish, or expand the implemented adapter set. Existing typed declarations are in `ess/system.yaml` and `ess/domains/declarations.yaml`; proposed runtime entities are not claimed to be modeled. Any later entity-bearing implementation decomposition must first use the ESS workflow for the settled semantics and retain unresolved relations as UNMAPPED. These documentation stories do not introduce a new typed entity or assert an unresolved cardinality.

The work covers contracts, their adapter plans, the contract index and design cross-references. The current implemented adapters remain Kubernetes, GitLab and SQL; documenting additional adapter plans is not implementation authorization. No remote, Atlas, rollout or old-repository changes are included. First-review open decisions outside F01–F15 remain in the intake record; this epic promises the 48 enumerated findings rather than silently deciding those product questions.

## Review of this decomposition

Run the four planning perspectives: `aep-plan:plan-critic-acceptance`, `aep-plan:plan-critic-design`, `aep-plan:plan-critic-scope`, and `aep-plan:plan-critic-parallel-safety`. Record their exact verdicts as immutable review-result artifacts and any revision outcomes. The harness has three worker slots, so the four independent readers use the same frozen draft in two batches with no cross-review feedback. Its callable agent interface does not expose the plugin agent types or the requested Sonnet model; use general subagents reading the exact role files with available `gpt-5.6-sol` at high effort, and record this deviation. This is an interactive run with no approval-bypass records.

## Planning verification — 2026-09-08

The final critic round approved the decomposition in all four lanes: [acceptance](../review-result/contract-plan-acceptance-r2-20260908.md), [design](../review-result/contract-plan-design-r2-20260908.md), [scope](../review-result/contract-plan-scope-r2-20260908.md), [parallel-safety](../review-result/contract-plan-parallel-safety-r2-20260908.md). Round one found a missing intake acceptance condition and a missing dependency from persistence ownership to federated approval; both were corrected through AEP and have fixed review_outcome evidence. There were no second-round findings, no third round, and no approval bypasses.

Coverage checks establish 48 source findings with one owner each, 27 draft stories, 104 cited file-scope entries and 159 acknowledged intersecting story pairs; dependencies are acyclic. Both original review bodies are retained verbatim. The planning agent changed no contract, runtime, ESS or old-repository source. The source findings remain assigned, not fixed.

Tool limitations: installed AEP reports protocol 0.54.0; validation succeeds but warns for the two original prose reviews and for approval records whose findings blocks contain an empty list. The CLI also printed revision 4 for the added persistence dependency while show/file metadata report revision 3; the actual body, edge and history were verified, without manual metadata repair. These diagnostics do not substitute for contract conformance and do not change the draft status.

Concurrent authoring was observed during the final check: `contracts/catalog/v1alpha1/semantics.md`, `docs/adapters/catalog.md`, and the catalog additions to `contracts/README.md` appeared outside this planning work. The 48-item intake remains anchored to `db1c329`; these later catalog additions have not been reviewed by this panel and are preserved unchanged by the planning agent. Reconcile the index against the then-current reviewed documents when its story is taken up.

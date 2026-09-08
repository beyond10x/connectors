---
format: aep.planning-md/1
id: review-result:profile-persistence-a-recheck1-20260908
kind: review-result
status: active
title: Discovery profiles and persistence reviewer A final approval
relations:
- reviews: story:contracts-discovery-profiles
- reviews: story:contracts-persistence-ownership
revision: 1
---
# Independent recheck A — discovery profiles and persistence ownership

Verdict: **approve** for the E07/E14/E32/E28 specification packet. **0 residual findings** (0 P0/P1/P2/P3). All eight initial findings PP-A-01–08 are addressed in the reviewed bytes. No runtime implementation or broader specification-goal completion is implied.

## Frozen review scope

Before analysis, 57 current normative/evidence inputs were frozen under `sources/`, with exact SHA-256 values in `source-hashes.json`. This includes the resource/route/composition owners, relevant auth/operations/delegation/session/service contracts, adapter docs, design, all ESS files, the explicit verification packet and separate session/IR compilation artifacts. Planning, dispositions, closure/provenance records and other reviewer output were excluded. The initial review's pinned old sources remain the Argo preservation/disposition reference at old commit `81459ac42ddd518d3942f4b079841e9e0ed6efc8`.

The two 211-artifact projection directories were independently frozen before checking them; `projection-source-hashes.json` records all 422 files under `projection-sources/`. `independent-checks.json` records the read-only evidence audit. No source/planning files changed and no runtime tests, provider calls, recognizer or storage implementation were run.

## Initial finding dispositions

| Finding | Resolution in reviewed sources |
|---|---|
| PP-A-01 fixed declaration/profile/source | Resource discovery §4.5 lines 143–150 selects the two concrete adapter declarations, one receiver-owned source rule and exactly one configured source per invocation; rejects conflicting selections and request profile/source/namespace overrides. The family name is conceptual, not a wildcard dispatcher. Supporting reader/realization remains an advertisement gate. |
| PP-A-02 Argo and candidate distinction | Resource discovery lines 152–158 specifies ASCII lowercase, exact whole name OR stable-label Argo recognition, exact-arm priority, explicit old alias exclusion, preserved monitoring priority, unknown observations and required-null candidates. Argo is always observation-only. Kubernetes/Grafana summaries and compatibility agree with the new recognition field. |
| PP-A-03 configured versus physical identity | Resource discovery lines 160–162 binds host instance/connection/canonical HTTPS API origin/trust policy; preserves distinct configured identities; changes authority through a new connection. Physical cluster attestation, automatic deduplication and unknown replacement detection are explicitly unclaimed. Known replacement invalidates continuity. |
| PP-A-04 inspectable ownership inventory | Design §31 lines 1294–1315 lists 18 narrow logical port responsibilities with owned state, atomic decisions, acknowledgement/failure handoffs and family links. Additional deferred obligations are named at line 1335. Port names do not claim existing traits/backends. |
| PP-A-05 common metadata ordering | Design §31.1 lines 1319–1325 requires the shared binding metadata authority for credential/connection/refresh and coupled discovery/route decisions; an equivalent alternative is not currently selected. Custody and external policy/provider boundaries remain separately acknowledged, with conservative ambiguity. |
| PP-A-06 distinct irreversible receipts/audit | Design lines 1304–1308 and 1321–1325 preserve atomic attempt/key grouping, separate exact leaf approval spending, nonce consumption, audit anchors and first-outcome knowledge. Uncertain acknowledgements grant no send; history cannot be reset by backup/replica divergence. Existing owner-specific replay/retention rules remain authoritative. |
| PP-A-07 sessions and deferred persistent work | SessionRedemptionPort and SessionSupervisorPort distinguish single-use establishment from live leases/terminal accounting; restart cannot recreate continuity. EventClaimSpendPort is explicitly unselected and distinct from named deferred event/checkpoint and assignment obligations. No future event/session protocol is silently implemented. |
| PP-A-08 ESS and retention truthfulness | Design §31.2 inventories the eleven actual entities, values/transient admissions, undeclared owner models and remaining UNMAPPED relations. Custody's conceptual entity table is corrected. Per-owner retention and capacity preserve safety facts; private references stay out of ordinary diagnostics. Seven new discovery types are values, not invented persistent owners. |

The added private auth publication fence is consistent with the settled F02/F03 distinction: same-identity refresh or same-target route evidence renewal invalidates current evidence/admissions without becoming an effect-relevant fingerprint change. Auth connection, custody, acquisition, operations and refresh ESS comments now use that interpretation. Changed target/identity/operation meaning still changes the semantic binding; generation/fence success supplies no replay authorization.

## Independently checked evidence

- All **44 schema expectations** matched the frozen selected schemas: **7 rejected shapes** and the **3 explicitly accepted semantic counterexamples**. These counterexamples correctly demonstrate that wrong operation/profile pairing, non-HTTPS origin and declared Argo confidence in Kubernetes require normative context checks outside the shapes.
- All **7 selected schema copies** match their manifest and projection bytes. Both complete **211-artifact** projections match each other and the recorded hash manifest.
- The **11-entity inventory** exactly matches the reviewed YAML identities, relationships and lifecycles; its durable-implementation disclaimer is accurate.
- The **43 textual traces** are uniquely identified and agree with the reviewed decisions. They are explicitly not reported as executed recognizer, canonicalizer, storage or fault scenarios.
- The frozen gate transcript records **50 passing existing Rust tests**, successful format/lint/dependency/MSRV 1.88 checks, **13 ESS files / 204 declarations**, and **222 compiled scenarios including 34 authored**, with zero refusals. Separate session artifacts contain **13 authored / 201 synthesized expectations**; the verification correctly explains their exclusion from the gate's authored roots.

## Limits

Approval covers semantic consolidation and accurately bounded evidence. URL canonicalization, provider interpretation, physical source continuity, secret snapshot provenance, real CAS/uniqueness/durability, clock and policy enforcement, crash/failover, actual send cardinality and live session cutoff remain implementation/binding conformance obligations. Undeclared persistent entity relationships must be settled through ESS before affected entity-bearing implementation decomposition. Nothing in the packet claims those guarantees have been executed by this value audit or the existing runtime gate.

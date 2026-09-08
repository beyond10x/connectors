---
format: aep.planning-md/1
id: review-result:profile-persistence-b-recheck1-20260908
kind: review-result
status: active
title: Discovery profiles and persistence reviewer B final approval
relations:
- reviews: story:contracts-discovery-profiles
- reviews: story:contracts-persistence-ownership
revision: 1
---
# Independent reviewer B — discovery profiles and persistence ownership recheck 1

Verdict: **APPROVE for specification stabilization. Zero residual findings (P0/P1/P2/P3: 0/0/0/0).** The revised E07/E14/E32 and E28 packet resolves PP-B-01 through PP-B-10. There are no PP-B-R1 findings.

This review evaluates the complete revised packet against baseline `175053fd2c951dc9564ff587a636775ab8027d2e`. Exact reviewed normative/evidence bytes are frozen under `snapshot/` and identified by `source-hashes.json` (60 files). Additional historical and existing compilation inputs have separate hash manifests. Other reviewer output, dispositions, mutable AEP and closure bookkeeping were excluded. No source/planning edits, runtime suites, provider calls or backend experiments were performed.

## Baseline finding closure

| Finding | Sole story owner | Independent disposition and current evidence |
|---|---|---|
| PP-B-01 | contracts-discovery-profiles | Closed. Resource discovery §4.5 (`semantics.md:141–153`) binds each concrete receiver declaration to exactly one profile and configured source rule, rejects missing/ambiguous/mismatched selection and closes input to limit/cursor. Gateway aliases retain the leaf meaning. Compatibility and both adapter summaries agree; unsupported readers remain unadvertised. |
| PP-B-02 | contracts-discovery-profiles | Closed. Resource discovery §4.5 defines whole-token Service name **OR** stable app.kubernetes.io/name recognition, ASCII lowercase, no trimming, deterministic precedence and observation-only Argo. Ordinary metrics/repo/redis component identities do not match. The three broader old label aliases are explicitly excluded for Argo while monitoring retains all four labels and its old priority. A forged matching label remains only an inference, never route or credential authority. |
| PP-B-03 | contracts-discovery-profiles | Closed. Resource discovery §4.5 distinguishes configured instance/connection/canonical API authority/trust revision from physical cluster identity. Display/context aliases and coincident origins cannot merge configured sources. Changed target/trust boundaries require new admission; unchanged-authority physical replacement is explicitly not attested, while known replacement invalidates continuity. No extra provider probe is invented. |
| PP-B-04 | contracts-persistence-ownership | Closed. Design §31 (`design.md:1292–1315`) gives 18 singular logical owners, including separately unselected EventClaimSpendPort, with owned facts, decisions, failure/retention handoffs and family links. Deferred assignment/checkpoint/job/artifact/telemetry owners remain visible. Logical ownership does not imply a universal database or current Rust port implementation. |
| PP-B-05 | contracts-persistence-ownership | Closed. Design §31.1 names the shared linearizable binding metadata group, coupled discovery/route comparisons and atomic mutation reserve/prepare plus terminal/replay settlement. A disconnected-store implementation cannot claim those atomic predicates; no unselected equivalent protocol is assumed. |
| PP-B-06 | contracts-persistence-ownership | Closed. Design §31.1 preserves independent custody, committed refresh response, audit admission/finalization, nonce consume/post-ack admission, approval spend and live dispatch-gate acknowledgements. Readback/recovery never supplies a fresh send permit; abort around spend remains safe without inventing a spend/provider transaction. Leaf and gateway ownership remain separate. |
| PP-B-07 | contracts-persistence-ownership | Closed. Design §§31.1–31.2 retain owner-specific safety history and windows: unresolved keys and approval tombstones do not expire automatically; nonce retirement needs trusted expiry; credential cleanup cannot erase refresh consumption; discovery eviction is not withdrawal. Capacity and stale-backup/failover ambiguity refuse affected authority instead of resetting uniqueness. |
| PP-B-08 | contracts-persistence-ownership | Closed. Design §31.2 and custody §9 accurately distinguish the eleven declared ESS entities from unmodeled Connection/Acquisition/AuthProfile/custody/Audit/discovery ownership. Custody no longer claims an implemented CredentialSet lifecycle. Connection §8 updates authoritative facts and derives readiness instead of permitting arbitrary status writes. |
| PP-B-09 | contracts-persistence-ownership | Closed. Design §31.2, connection §4, acquisition §4.1, custody's publication sketch, operations §5.1 and ESS refresh comments consistently distinguish private auth publication fences from semantic F02/F03 revisions. Same-identity material refresh invalidates current admissions without alone changing a request fingerprint; effect-relevant meaning changes still require a new semantic binding/revision. |
| PP-B-10 | contracts-persistence-ownership | Closed. Custody §4 (`semantics.md:66`) and design §31.2 forbid custody scope/version/generation/private-route coordinates in ordinary diagnostics. Separately admitted safe correlation does not turn access-controlled internal state into a public logging surface. |

## Independent evidence check

`evidence-audit.json` records the actual checks:

- All 60 frozen normative/evidence source hashes matched the working tree at audit completion.
- Independently evaluated all 44 values against the copied Draft 2020-12 schemas: 44 expected decisions match `type-results.json`, including seven rejected shapes. Wrong operation/profile pairing, non-HTTPS origin and declared Argo confidence remain three explicitly accepted semantic counterexamples, not validator successes for those predicates.
- Both existing projection directories contain the same 211 byte-identical artifacts matching the manifest; all seven selected copies match. The compiled IR adds exactly seven discovery value types; old types and entity/command sections remain unchanged (only domains/types change).
- Parsed frozen ESS YAML independently and matched all eleven entity identities, relations and lifecycles to the supplied inventory. Entity declaration does not prove durable storage or execute atomic groups.
- Read all 43 distinct textual traces against their normative owners. They are conformance expectations, explicitly not executed recognition, canonicalization, persistence, clock, authority or crash tests.
- Inspected the supplied successful gate log: 50 existing Rust tests, MSRV 1.88 check, 13 ESS files/204 declarations and 222 compiled scenarios including 34 authored. This reviewer did not rerun that gate.
- Independently checked the separate session artifacts: 13 authored and 201 synthesized scenarios, identical provenance, with every authored scenario unchanged in the synthesized suite. This is compilation evidence only.
- Froze old `local.rs` and `local_tests.rs` from `81459ac42ddd518d3942f4b079841e9e0ed6efc8` in `historical/`. Their cited recognizer/test lines support lowercase normalization, the historical broad label set, whole-token Argo handling and monitoring priority. The new narrower Argo alias selection is honestly labeled a profile change; no current-vendor or byte-for-byte historical equivalence is claimed.

## Limits

Approval covers the selected textual semantics, value-shape evidence and accurate ownership inventory. The public observation codec, canonicalizer, recognizer, configured-source admission, persistent entity graph, concrete backend, atomicity implementation and restart/fault conformance remain explicit implementation/advertisement prerequisites. This approval supplies none of those proofs and authorizes no rollout. No remaining defect was found within the two stories' bounded specification scope.

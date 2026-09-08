---
format: aep.planning-md/1
id: review-result:connection-semantics-r3-20260908
kind: review-result
status: active
title: Independent final connection semantics approval
relations:
- reviews: story:contracts-connection-readiness
- reviews: story:contracts-management-boundary
revision: 1
---
# Connection semantics — independent reviewer A, final recheck

**Final verdict: approve the specification checkpoint for both existing stories. Remaining findings: P0 0 / P1 0 / P2 0 / P3 0.** Initial requirements CS-A01–A08 and draft findings CS-A-R2-01–03 are closed. This is approval of the selected semantics and accurately bounded evidence, not runtime advertisement.

Baseline: `12c11f4b43cd6cf5ff67d3243017887b28788344`. `source-hashes-final.json` preserves 49 exact source/evidence hashes, with matching copies in `sources-final/`, including supplemental session author/synthesis outputs. All 49 snapshot hashes were verified. Earlier reports and snapshots remain immutable. This reviewer read no other reviewer output, edited no source/planning file and ran no runtime test or semantic reducer.

## Final source consistency

The seven-state ordered viability reduction and separate operation eligibility remain coherent across connection, profile, acquisition and evidence. Baseline minimum grants differ from requested optional and per-operation grants. Positive failure of the current credential differs from stale/unknown evidence and an inactive failed repair candidate. Optional operation verification does not poison global viability; universal required verification gates baseline publication. Completed acquisition means acknowledged baseline publication, while current eligibility still depends on fresh authority, generation and applicable evidence.

The surviving stale-evidence and repair-mismatch contradictions were removed. Interactive continuation requirements are scoped to browser/protected-entry flows; client credentials may complete without an invented UI action. Grant-policy not_granted versus provider/resource forbidden follows E02. Absent requires_auth keeps the explicit configured binding branch without anonymous fallback.

The final management wording cleanly separates inspect/local revoke from repair of terminal records: revoked records remain inspectable/revocable but cannot be repaired or re-enabled. The wire target rule is exact: describe/revoke/repair require one nonempty string Invocation.connection; create/list/status omit the member and reject any supplied selector, including JSON null. F03 nullable signed claim values do not change the Invocation codec. Protected completion stays with the owning coordinator and its stored target, outside generic invoke.

Management prerequisites remain independent of broken business credentials. Host metadata/coordinator authority owns publication and local cutoff; provider protocol work and custody retain their separate responsibilities. Provider revocation uncertainty cannot restore local authority or justify an automatic repeat. The separately admitted trusted UI receives any actionable continuation; ordinary results and discovery receive safe refs/kinds only. Incomplete effect/approval/idempotency/protected-codec bindings remain explicit advertisement gates.

## Evidence independently checked

| Evidence | Result and limit |
|---|---|
| Declared decision cases | Read all 16 viability, 18 eligibility and 7 management-target examples against the final owners. Priority overlaps, baseline versus operation failures, current host denial, explicit configured binding and management target ownership have consistent expected decisions. No reducer was executed. |
| Type expectations | Independently checked all 80 stored schema expectations against the retained ESS-generated schemas; all match. This includes four expected structural/vocabulary rejections. |
| Deliberate semantic contradiction | Independently confirmed the generic schema accepts `{eligible: true, error: not_granted}`. The evidence explicitly identifies its semantic invalidity and correctly demonstrates that schema shape does not execute cross-value admission rules. |
| Generated artifacts | Verified all 13 retained schema hashes against the manifest and the two existing output directories. Each directory contains 176 artifacts and every corresponding hash is identical. No generated schema was edited or regeneration needed for review. |
| Textual traces | Read all 16 expected traces covering acquisition/publication, optional verification, repair isolation, no-browser flow, exact selector omission, management grants, owner-preserving federation, current completion permission, revocation and action disclosure. They are labeled expected consequences, not runtime observations. |
| Recorded repository gate | The retained log records 50 Rust tests passed, zero failed/ignored; successful formatting/lint/boundary and Rust 1.88 workspace/all-target checks; 11 valid ESS files and 167 declarations; and 222 synthesized scenarios including 34 authored with zero refusals. The reviewer inspected this prior run and did not rerun runtime tests. |
| Separate sessions | Preserved and inspected 13 authored and 201 synthesized session scenarios with matching specification/contract provenance. These are compiled expectations, not session execution. |

The one-off fixture transcript checks schema shape and writes declared expected facts/decisions; it does not disguise an implementation of the chosen reduction as conformance evidence. The totals in verification.md agree with the retained outputs. Generic Optional omission and ESS Timestamp values remain explicitly different from required-null/public Unix-millisecond wire encoding.

## Limits

No current-state reducer, provider, policy engine, clock, metadata/custody store, callback, management coordinator or dispatch gate was executed by this review. Ordered reduction, freshness/applicability, scope/alternative matching, exact generation/target correlation, current authority, durable publication/revocation and protected transport behavior remain required host/binding obligations. Concrete Connection/Acquisition/AuthProfile persistence models, management mutation/approval/idempotency payloads and UI/callback codecs remain unimplemented and must not be advertised on the strength of these value types.

Within the authorized specification-hardening scope, no residual semantic or evidence defect was found.

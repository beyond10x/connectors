---
format: aep.planning-md/1
id: review-result:federation-subjects-final-20260908
kind: review-result
status: active
title: Independent F03 final subjects approval after fixture correction
relations:
- reviews: story:contracts-federated-approval
revision: 1
---
# F03 final amendment to independent recheck A

**Final verdict: approve the proposed F03 specification. Remaining findings: P0 0 / P1 0 / P2 0 / P3 0.** This supersedes only the outstanding-finding status of `recheck-2.md`; that immutable report and its original source snapshot remain preserved.

Final source: `source-hashes-final.json`, containing 84 exact source/evidence hashes with matching copies in `sources-final/`. The additional supplemental copy preserves the separately checked sessions synthesis output. Baseline remains `b5ace3f29d6deb1dbc9e5ddd50a8a8b02152cb2f`. No other reviewer output was inspected, and this reviewer made no source/planning/runtime change and ran no runtime test.

## F03-A-R3-01 is closed

The federated candidate in `contracts/operations/v1alpha1/scenarios/idempotency-namespace-values.yaml` now has its own full fingerprint with route `{gateway_instance: trusted-gateway, route_id: fixture-route, route_revision: route-r1}`. Its gateway matches the admitted federated origin. The three direct candidates retain `route: null`.

Independently parsed the corrected YAML and the retained regenerated `operations-authored.json`: all four origin/route pairs match exactly between source and compiled obligation. The refreshed recorded gate collects all 15 authored operations and synthesizes 222 scenarios including 34 authored, with zero refusals. Its recorded Rust totals remain 50 passed, zero failed/ignored; ESS remains 10 valid files and 153 compiled declarations. The fixture fix therefore has matching regenerated evidence.

## Final protocol/evidence consistency

The author-corrected failure table now uses the existing E02 `timeout` and `invalid_input` codes; textual S10 agrees. No extra error code is introduced. The exact type-dependent lifetime formulas, bounded integer encodings, conservative clock checks and post-nonce acknowledgement serialization point remain unchanged from the accepted semantic correction. The verification text records the final disposition and does not claim those runtime mechanisms were executed.

The amended snapshot changes only the identified scenario, its regenerated operations output and gate log, the two error-code spellings, corresponding textual evidence and verification disposition text relative to the first final-pass snapshot. There is no broader source change requiring another semantic review. All 84 final snapshot hashes were verified after capture.

## Limits of approval

All initial A01–A08 decisions, the prior lifetime finding F03-A-R2-01, and the final fixture finding F03-A-R3-01 are closed for specification stabilization. The approved preparation/subject/trust/route/nonce/redemption/observation rules still require their concrete host bindings before advertisement.

The independent primitive and data checks in `recheck-2.md` remain applicable: 20 public-fixture signature/body results, 12 generated type expectations, five canonical fixture digests, 21 retained projection hashes and two identical 163-artifact output sets. Generic ESS schemas reject the required-null wire representation; that documented projection limit is not a wire codec. Arithmetic/truth-table checks and textual traces establish declared relationships, not execution of clocks, nonce storage waits, revocation, durable spend/uniqueness, admission/policy, audit, HTTP transport or provider calls. The full repository gate protects existing implementation and declaration integrity. None of this evidence authorizes advertising an unimplemented delegated runtime.

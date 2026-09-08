---
format: aep.planning-md/1
id: review-result:federation-subjects-r3-20260908
kind: review-result
status: active
title: Independent F03 final subjects review
relations:
- reviews: story:contracts-federated-approval
revision: 1
---
# F03 independent final semantic recheck A — round 3

Verdict for this frozen checkpoint: **ship the proposed specification with one fixture correction**. Counts: **P0 0 / P1 0 / P2 1 / P3 0**. The previous verifier-rule finding is closed. There is one remaining inconsistency in a positive authored scenario; no further normative protocol defect was found.

This record preserves the initial final-pass observation. The author has subsequently reported its correction; that correction and refreshed evidence are assessed in a separate immutable amendment, rather than rewriting this report.

## Reviewed source

Baseline: `b5ace3f29d6deb1dbc9e5ddd50a8a8b02152cb2f`. `source-hashes-r3.json` records 83 source/evidence files frozen with matching copies in `sources-r3/`. The scope includes delegation and reconciled service/operation contracts, all ESS domain inputs, operation/session authored scenarios, and the F03 evidence bundle. References below identify that snapshot. No other reviewer's output was inspected. No source, planning or runtime implementation was changed; no runtime test or receiver was executed by this reviewer.

The author separately corrected the draft's post-entry deadline spelling to E02 `timeout` while this review was in progress. The first frozen snapshot predates that author correction; the final amendment captures it. It is not a remaining protocol choice.

## Finding

### F03-A-R3-01 — P2 — Federated positive reservation inherits a direct null route

**Source:** `contracts/operations/v1alpha1/scenarios/idempotency-namespace-values.yaml:195–206`, inheriting the anchor at lines 58–68. Normative requirements: `contracts/service/delegation.md:51,64` and the operations fingerprint definition.

The positive federated reservation names origin `{kind: federated, authority_ref: trusted-gateway}` but reuses `fingerprint: *id001`. That anchor contains `route: null`. F03 requires a non-null route with gateway identity matching the admitted federated origin. The fixture says its callers/origins were separately admitted and expects successful reservation, so this is an inconsistent positive example, not an intentionally rejected adversarial value.

ESS authoring still accepts it because cross-value origin/route admission is explicitly outside the generic type checker. Therefore a passing authored/synthesized scenario count does not correct the semantic inconsistency.

**Required correction:** retain the shared null route for the three direct-origin examples; give the federated candidate a full fingerprint with `{gateway_instance: trusted-gateway, route_id, route_revision}`. Regenerate the retained operations-authored output and refresh the relevant synthesis evidence. Do not introduce a separate business key or origin namespace to make the example pass.

## Previous findings and final protocol review

**F03-A-R2-01 is closed.** Section 5 now assigns `iat` to the floor of the trusted bounded clock interval midpoint and requires receivers to enforce the exact `nbf = iat - 5`, `exp = iat + 25/295` relationships by proof type. All time members have explicit bounded integer encoding and checked arithmetic. Ordinary current-time admission still requires `nbf <= lower` and `upper < exp`. Overlong, shorter, wrong-profile, malformed and out-of-time proofs cannot pass merely because their signatures are valid.

The post-nonce acknowledgement entry decision uses current trusted time, key validity/revocation, issuer projection/realm admission and remaining signed deadline. Failure consumes the acknowledged nonce while granting no authenticated entry. This prevents an earlier pre-await check from authorizing entry after a suspended store acknowledgement. The existing leaf-only approval-spend decision rechecks current proof/key/admission and expiry, and remains separate from the durable dispatch gate.

Initial A01–A08 remain resolved: canonical leaf identity and all effect-relevant revisions are explicit; managed and configured preparation use admitted metadata; gateway presentation freshness is separate; optional identity/executor is preserved; the executing logical leaf owns unique redemption and attempts; delivery nonce, approval spend and business-key observation are separate; gateway failures preserve known effects and original correlation; and ESS models values/receipts and known references without pretending to implement trust, uniqueness or persistence.

Preparation continues to be a host-owned selected read with no provider credential material, provider I/O, reservation or spend. Its dedicated advertised limits and gateway control traffic are explicit. Eventual invocation including approval evidence independently satisfies its own request bound. No model or documentation claims that preparation guarantees future admission or capacity.

## Evidence inspected

| Evidence | Independent observation |
|---|---|
| Recorded full gate | The retained log totals 50 Rust tests passed, zero failed/ignored, and records successful formatting/lint/boundary/MSRV checks, ESS validation of 10 files, 153 declarations, and synthesis of 222 scenarios including 34 authored. This is inspected prior execution evidence, not a gate rerun by this reviewer. |
| ESS projections | All 21 retained schema hashes match their manifest and the original generated outputs. The two existing generated directories contain 163 artifacts each with identical hashes. No schema generation or source mutation was needed for this check. |
| Authored scenario outputs | 15 operations and 13 sessions are retained. The existing sessions suite contains 201 scenarios with matching specification/contract provenance. The operations positive-value inconsistency above is visible despite successful authoring. |
| Crypto primitive fixtures | Independently verified all 20 stored signature/raw-body expectations using only the published deterministic fixture public keys; decoded protected headers/claims also match the sidecar JSON. Malformed protocol claims deliberately retaining valid signatures are correctly described as requiring future receiver refusal. No issuer authority or receiver was executed. |
| Type projections | Independently matched all 12 retained schema expectations against the retained generated schemas. Nine semantic values pass; three structural negatives refuse. All nine corresponding explicit-null wire values fail the generic optional projection, as the evidence openly records. |
| Canonical subject fixtures | Independently matched the five stored canonical JSON fixture encodings and digests. Reviewed all 15 coordinate perturbations and their limited equality claim. They are not all independently admissible alternate contexts. |
| Time and trace evidence | Inspected the exact arithmetic transcript and 23 expected arithmetic/truth-table results, plus 26 textual traces. These accurately distinguish claimed trusted facts from real clocks, waits, revocation, durable consumption, spend, policy, audit or provider execution. |

The one-off Python transcript is described as bounded fixture inspection rather than installed executable project tooling or a general canonicalizer. Generic ESS optional omission is explicitly not the normative required-null wire encoding. Its limitation is also named UNMAPPED in the model, so the passing type-value checks do not claim that a wire codec exists.

## Evidence and acceptance limits

This review accepts the selected specification, subject to the one fixture correction. It does not authorize runtime advertisement. Receiver/issuer integration, trusted Identity/policy derivation, current clock observations, durable uniqueness/replication and anti-rollback, admission/spend serialization, dispatch fencing, HTTP/proxy behavior, audit and provider conformance remain implementation obligations. The existing repository gate protects current code and the ESS compiler checks declarations/obligations; neither demonstrates those future runtime properties.

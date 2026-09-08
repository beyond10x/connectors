---
format: aep.planning-md/1
id: review-result:federation-binding-r3-20260908
kind: review-result
status: active
title: Independent F03 final binding approval
relations:
- reviews: story:contracts-federated-approval
revision: 1
---
# Independent F03 protocol recheck B — round 2

Verdict: **approve the semantic checkpoint**. Remaining findings: **0 blockers, 0 majors, 0 minors, 0 nits**. Both prior P1 findings are closed. One additional P2 error-name defect found during this final pass was corrected and rechecked before the final snapshot.

This is approval of the proposed specification and its accurately bounded evidence, not runtime approval. Delegation remains unimplemented and unadvertised; receiver, issuer, current policy, trusted clocks, durable uniqueness/fencing, real wire codecs and fault conformance remain explicit advertisement prerequisites.

## Reviewed source and independence

The final immutable snapshot is `recheck-2-snapshot/`; `source-hashes-r3.json` records all 54 reviewed source/evidence files against baseline b5ace3f29d6deb1dbc9e5ddd50a8a8b02152cb2f. All 54 hashes still matched the working source when this report was written. Initial foundation sources, including clean current Atlas and predecessor signed-module-request evidence, remain frozen in `source-hashes.json`; prior draft bytes remain in `source-hashes-r2.json` and `recheck-1-snapshot/`. No tracked source/planning files were changed, no implementation or runtime tests were run, no other reviewer's report was read, and no extra agent was delegated.

The review includes the normative delegation contract, service v1alpha2 and compatibility, operations/idempotency semantics, ESS subject/proof/receipt and existing attempt models, namespace scenarios, relevant CLI/stack statements, and the final F03 evidence. Shared verification disposition labels were reviewed as provenance only; they did not substitute for independent findings.

## Finding closure

| Finding | Final disposition |
|---|---|
| FBR-01, P1 — receiver proof lifetime | Closed. Delegation §§4.1/5 require JSON integers in [0, 9007199254740991], excluding booleans/fractions/strings, with checked arithmetic and conversions. Receivers enforce nbf=iat−5 and exp=iat+25 for delivery or iat+295 for approval, at initial verification, final entry and spending. Trusted interval width is bounded, nbf<=lower and upper<exp; equality at expiry refuses. Underflow, overflow, past nbf, overlong/wrong-profile and future windows have explicit negative arithmetic/cryptographic examples. |
| FBR-02, P1 — entry after nonce-store suspension | Closed. §4.1 defines final authenticated entry after definite nonce acknowledgement as the admission serialization point. Current trusted time, key validity/revocation, issuer projection/realm and remaining signed deadline are rechecked after the wait. Failure grants no entry and preserves the consumed nonce; no rollback or transport retry. §5 limits the later-expiry continuation rule to an already admitted interaction. S12–S15 and the post-ack truth table reflect this ordering without claiming a real suspension test. |
| FBR-03, P2 — nonexistent E02 error name discovered during final pass | Fixed during review. Delegation §7 and scenario S10 initially named `invalid_request` for malformed application decoding. E02's closed enum and HTTP 400 mapping require `invalid_input`. Both reviewed final files now use `invalid_input`. The post-ack exhausted deadline also uses the valid E02 `timeout` code. No compatibility enum expansion was needed. |

FB-01 through FB-08 remain closed: the canonical subject and admitted preparation expose exact executing-leaf authority/target/input; presentation freshness differs from signed target/route identity; current receiver policy narrows configured gateway trust; optional realm/executor and qualified identities are preserved; delivery and issuer approval have separate types/audiences and exact byte/request binding; bodyless describe has private nonce correlation and public request_id:null; transport nonce, approval spend and dispatch fence remain distinct; only the leaf owns the business attempt/spend; and uncertain forwarding, result disclosure and audit preserve E02 outcome truthfulness.

The extended fingerprint is explicitly tagged mutation-request/v2, including nullable route binding. It cannot silently reinterpret a proposed v1 reservation. The federated namespace example now carries its matching non-null trusted-gateway route, while direct examples retain route:null. A changed route under the same stable origin therefore conflicts with a live key instead of creating fresh authority.

## Independent evidence audit

The detailed independent observations are in `recheck-2-evidence-audit.json`.

- All 20 compact-JWS fixtures decode to their recorded canonical header/claim bytes and meet their framing-size bounds. Verification using the published deterministic public keys reproduces every expected signature result. Raw body digest observations match, including the signed proof whose body was subsequently changed. A valid signature on a deliberately wrong type, algorithm, receiver or time shape remains insufficient for protocol admission.
- The three positive delivery examples independently match describe/prepare/invoke method, path, purpose and body correlation. Describe has empty body and all five required-null selection/correlation coordinates. The invocation carries the exact recorded approval evidence/reference, and its admitted authority matches the approved subject.
- All 12 generated-schema expectations match: nine semantic values accepted and three structural negatives refused. All nine required-null wire values are rejected by generic ESS optional projection, exactly as disclosed. Generated schemas must not be used as the wire codec. The model separately leaves constants, mandatory-null encoding, bounds, crypto/current trust and cross-field predicates UNMAPPED.
- Five canonical subject examples have correct canonical byte strings and digests. Independent perturbation of each of the 15 listed coordinates changes canonical equality. These are coordinate checks, not proof that every alternate value constitutes an admissible context.
- All 23 time expectations agree with independently recomputed bounded integer/profile/clock predicates or the explicitly declared post-ack fact conjunction. Six post-ack rows are truth tables only; they execute no clock, revocation, persistence wait or entry decision.
- The two generated output directories contain 163 byte-identical artifacts, and all 21 retained schema copies match both the generation outputs and their recorded hashes. The retained gate log records 50 passing existing Rust tests, successful Rust 1.88 checking, 10 valid ESS files/153 declarations and a completed passing gate. No gate was rerun by this reviewer.
- The 26 textual traces preserve safe preparation, scoped authority, correlation, uniqueness retention, same-key observation before fresh approval validation, winner recheck after a miss, sole-leaf spending, gate ambiguity and no resend. The records accurately distinguish declaration/scenario compilation, primitive/type/arithmetic checks and future runtime behavior.

No remaining concrete semantic contradiction was found in the reviewed final scope. Existing implementation gaps are named ownership and advertisement obligations, not hidden evidence claims or a reason to add runtime implementation to this review.

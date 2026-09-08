---
format: aep.planning-md/1
id: review-result:federation-binding-r2-20260908
kind: review-result
status: active
title: Independent F03 binding review round 2
relations:
- reviews: story:contracts-federated-approval
revision: 1
---
# Independent F03 protocol recheck B — round 1

Verdict: **ship with two time-admission fixes**. Both are semantic receiver requirements, not a request to implement runtime now. Snapshot hashes are in `source-hashes-r2.json`; immutable source bytes are in `recheck-1-snapshot/`. Root's not-yet-written evidence vectors were not reviewed. No source/planning files changed, no other reviewer output read and no subagents delegated.

## FBR-01 — P1: receivers do not explicitly enforce the selected proof lifetime

Delegation §5 defines receiver proof validity as nbf <= lower, upper < exp, and nbf <= iat < exp. It then instructs issuers to use nbf=t−5, exp=t+25 for delivery or exp=t+295 for approval. It never explicitly requires the **receiver** to reject a signed token that fails those profile-specific relations. A valid configured signer emitting iat=1000, nbf=995, exp=100000 passes the listed receiver checks at [1001,1003], despite the chosen 30/300-second profile. The approval type's subject and audience checks do not repair a lifetime violation.

Fix: state the receiver's exact time-shape predicates for each proof type using checked arithmetic, including maximum span and the selected iat/nbf/exp relationship. If the fixed profiles require exact nbf=iat−5 and exp=iat+25/+295, enforce both equalities. If shorter lifetimes are intentionally admitted, specify their exact allowed range instead of relying on issuer behavior. Apply the predicates at initial verification and approval spending. Reject non-integer/negative/out-of-range time encodings under explicit bounds.

Vectors: valid delivery and approval windows; valid signature with overly long exp; nbf moved far into the past while iat is recent; incorrect profile window; equality at expiry; arithmetic underflow/overflow. Each malformed signed window must refuse before new nonce entry or approval spend/provider dispatch.

## FBR-02 — P1: nonce-store suspension can admit a proof after its validity or key authority ends

Delegation §4.1 verifies time/key/projection, then awaits durable nonce consumption, then constructs authenticated context and decodes application input. §5 says expiry after **admission** does not erase an already-running attempt, but the admission here happens only after the consume acknowledgement. A delivery can be verified near exp, wait on the durable store past exp (or key/projection revocation), then gain authenticated entry. A 40-second generic execution deadline can still be live after the roughly 25-second delivery expiry, so the signed deadline does not necessarily refuse this late entry. Approval spend has an explicit recheck; delivery entry does not.

Fix: define the delivery admission serialization point and recheck trusted clock validity, current verification-key/projection admission and remaining signed deadline after consume acknowledgement **before** granting authenticated entry. Alternatively couple the decision atomically to those facts with a specified trusted observation. If a nonce was consumed but the final check fails, keep it consumed and refuse; do not roll it back or replay the transport. Treat ambiguous consume acknowledgement as unavailable with no entry. The later rule permitting an admitted invocation to continue after proof expiry applies only once this final entry decision succeeds, with ongoing policy/deadline constraints unchanged.

Vectors: nonce store suspends across exp equality; across signing-key revocation; across loss of bounded clock certainty; across execution deadline. All produce no authenticated entry, approval spend or provider dispatch, while preserving an acknowledged nonce receipt. A valid post-ack entry followed by later token expiry may continue only within current authority and original execution budget.

## Foundation requirements otherwise addressed

- FB-01/02: canonical subjects contain the executing receiver/operation/connection and effect-relevant revisions, trusted scope/origin/route and input digest. The admitted preparation read exposes the exact subject without provider credentials, business reservation or approval spending. Gateway alias/full projection freshness is explicitly separate; route/leaf revision changes cannot preserve the old subject or silently create another key namespace.
- FB-03: gateway assertions are explicitly scoped trust; leaf policy independently checks current authority and receiver-configured issuer projection. Qualified caller mapping is injective. Optional realm/executor absence is preserved, and body assertions cannot manufacture admission.
- FB-04: distinct compact-JWS types and audiences separate delivery from issuer approval. Exact raw body/method/receiver/path are authenticated; GET describe has an empty body and private nonce correlation while its public request_id remains null. Strict duplicate/unknown/header/encoding rules and the correlated TLS response path address the prior ambiguity.
- FB-05: nonce receipt and approval redemption are different immutable facts and owners. Only the executing leaf spends approval and owns the business attempt; a fresh transport proof may observe an existing key without another spend. The gateway never creates a second business AttemptRecord or redeems the proof.
- FB-06: fixed no-expiry-skew interval intent, key independence, stable nonce namespaces, retention through safe expiry, no premature eviction and no divergent-history failover are stated. The two remaining receiver predicates above are necessary to make that intent enforceable.
- FB-07/08: spend precedes the separate durable dispatch gate, ambiguous acknowledgements give no permission, and gateway loss preserves conservative E02 outcomes. Current result-access denial prevents replay existence or original identity disclosure. A known applied/refused result survives later audit failure; no signature refresh or route change permits automatic resend.
- ESS models subject/proof values and acknowledged receipts separately from the attempt. Its explicit UNMAPPED predicates avoid claiming cryptographic, temporal or storage enforcement from declaration validation.

## Standards check

I independently opened [RFC 9864 §2.2](https://www.rfc-editor.org/rfc/rfc9864.html#section-2.2), [RFC 7515 §3.1](https://www.rfc-editor.org/rfc/rfc7515.html#section-3.1) and [RFC 8037](https://www.rfc-editor.org/rfc/rfc8037.html). The fully specified JOSE identifier Ed25519 is defined by RFC 9864; choosing it explicitly for the new type rather than silently accepting the predecessor's polymorphic EdDSA identifier is valid. RFC framing and cryptographic primitives do not by themselves validate the application subject, current policy, nonce admission or approval-spend semantics. The known Connectors fetch-capability gap had already been reported by root; this was direct read-only primary-standard verification, not a provider or credential operation.

The proposal remains unimplemented and unadvertised. Approval of the final semantics will still require the corrected byte/time/failure vectors and a clear separation between cryptographic/example checks, ESS compilation and runtime conformance.

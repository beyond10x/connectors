---
format: aep.planning-md/1
id: verification-report:findings-federation-binding-r2-20260908
kind: verification-report
status: draft
title: Structured transcription of review-result:federation-binding-r2-20260908
tags:
- review-findings-supplement
refs:
- provider: review-body-sha256
  reference: a8e3fa5a946de6d8987c1a6b913e95fdd33728ab8f35089c0e777292d4a38eae
relations:
- verifies: review-result:federation-binding-r2-20260908
- derived_from: specification:review-findings-toolchain-upgrade-20260911
revision: 1
---
# Structured transcription of review-result:federation-binding-r2-20260908

This supplements [the immutable original](../review-result/federation-binding-r2-20260908.md).
It records no new critic run, approval, review outcome or current implementation
finding. Source binding uses the SHA-256 of the exact original body returned by AEP.
The original review's scope, verdict, source snapshot and historical date remain authoritative.

## Original conclusion

The original report's verdict and scope remain authoritative; no new verdict is assigned.

## Transcription method

2 findings remain in the scope of this report's final stated conclusion.
Closed items in a same-pass disposition and verification/command tables are excluded.
Each nonempty message reproduces a source section or table row verbatim, including
its original identifier and citations. P0/P1, major and blocking map to blocker;
P2, minor and should-fix map to warning; P3 and nit map to note. Ungraded observations
remain unspecified. No per-finding verdict or introduced/pre-existing classification
is invented. The file is the first explicit source citation; when only shorthand
citations are present, legacy-source-excerpt identifies the original report itself.
The full citation context remains in the message and original. This transcription
makes no claim that paraphrased findings across different historical rounds have
identical comparison signatures.

## Findings

```findings
[
  {
    "file": ".engineering/planning/review-result/federation-binding-r2-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "blocker",
    "message": "## FBR-01 — P1: receivers do not explicitly enforce the selected proof lifetime\n\nDelegation §5 defines receiver proof validity as nbf <= lower, upper < exp, and nbf <= iat < exp. It then instructs issuers to use nbf=t−5, exp=t+25 for delivery or exp=t+295 for approval. It never explicitly requires the **receiver** to reject a signed token that fails those profile-specific relations. A valid configured signer emitting iat=1000, nbf=995, exp=100000 passes the listed receiver checks at [1001,1003], despite the chosen 30/300-second profile. The approval type's subject and audience checks do not repair a lifetime violation.\n\nFix: state the receiver's exact time-shape predicates for each proof type using checked arithmetic, including maximum span and the selected iat/nbf/exp relationship. If the fixed profiles require exact nbf=iat−5 and exp=iat+25/+295, enforce both equalities. If shorter lifetimes are intentionally admitted, specify their exact allowed range instead of relying on issuer behavior. Apply the predicates at initial verification and approval spending. Reject non-integer/negative/out-of-range time encodings under explicit bounds.\n\nVectors: valid delivery and approval windows; valid signature with overly long exp; nbf moved far into the past while iat is recent; incorrect profile window; equality at expiry; arithmetic underflow/overflow. Each malformed signed window must refuse before new nonce entry or approval spend/provider dispatch."
  },
  {
    "file": ".engineering/planning/review-result/federation-binding-r2-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "blocker",
    "message": "## FBR-02 — P1: nonce-store suspension can admit a proof after its validity or key authority ends\n\nDelegation §4.1 verifies time/key/projection, then awaits durable nonce consumption, then constructs authenticated context and decodes application input. §5 says expiry after **admission** does not erase an already-running attempt, but the admission here happens only after the consume acknowledgement. A delivery can be verified near exp, wait on the durable store past exp (or key/projection revocation), then gain authenticated entry. A 40-second generic execution deadline can still be live after the roughly 25-second delivery expiry, so the signed deadline does not necessarily refuse this late entry. Approval spend has an explicit recheck; delivery entry does not.\n\nFix: define the delivery admission serialization point and recheck trusted clock validity, current verification-key/projection admission and remaining signed deadline after consume acknowledgement **before** granting authenticated entry. Alternatively couple the decision atomically to those facts with a specified trusted observation. If a nonce was consumed but the final check fails, keep it consumed and refuse; do not roll it back or replay the transport. Treat ambiguous consume acknowledgement as unavailable with no entry. The later rule permitting an admitted invocation to continue after proof expiry applies only once this final entry decision succeeds, with ongoing policy/deadline constraints unchanged.\n\nVectors: nonce store suspends across exp equality; across signing-key revocation; across loss of bounded clock certainty; across execution deadline. All produce no authenticated entry, approval spend or provider dispatch, while preserving an acknowledged nonce receipt. A valid post-ack entry followed by later token expiry may continue only within current authority and original execution budget."
  }
]
```


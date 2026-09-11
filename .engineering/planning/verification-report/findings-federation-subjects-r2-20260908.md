---
format: aep.planning-md/1
id: verification-report:findings-federation-subjects-r2-20260908
kind: verification-report
status: draft
title: Structured transcription of review-result:federation-subjects-r2-20260908
tags:
- review-findings-supplement
refs:
- provider: review-body-sha256
  reference: 2add5b7d2e8b095a02eb8aa331fc538a28c349b50d1788e13215df3474cfbcf8
relations:
- verifies: review-result:federation-subjects-r2-20260908
- derived_from: specification:review-findings-toolchain-upgrade-20260911
revision: 1
---
# Structured transcription of review-result:federation-subjects-r2-20260908

This supplements [the immutable original](../review-result/federation-subjects-r2-20260908.md).
It records no new critic run, approval, review outcome or current implementation
finding. Source binding uses the SHA-256 of the exact original body returned by AEP.
The original review's scope, verdict, source snapshot and historical date remain authoritative.

## Original conclusion

Verdict: **ship the proposed specification with one verifier-rule fix**. There is **one remaining P1**, no P0. The draft resolves the original A01–A08 ownership and trace decisions. No runtime implementation is reviewed or approved by this verdict.

## Transcription method

1 findings remain in the scope of this report's final stated conclusion.
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
    "file": "contracts/service/delegation.md",
    "category": "legacy-review",
    "severity": "blocker",
    "message": "### F03-A-R2-01 — P1 — Make maximum proof lifetime a receiver predicate\n\n**Source:** `contracts/service/delegation.md:145`; related bounded approval policy at line 85 and receiver verification at lines 133, 141.\n\nThe issuer construction gives a delivery span of 30 seconds and an approval span of 300 seconds, but does not explicitly assign `iat = t`. The receiver's listed predicates are only `nbf <= lower`, `upper < exp`, and `nbf <= iat < exp`. They do not require the claimed interval to satisfy the selected profile's construction or maximum span.\n\nA correctly signed token with `nbf = 995`, `iat = 1000`, and `exp = 1000000`, observed under `[lower, upper] = [998, 1002]`, satisfies every listed receiver predicate. It violates either selected issuer lifetime but could pass a verifier implemented directly from the receiver rules. A similarly malformed `iat` can be far in the future while still between nbf and exp. Signature validity establishes the configured issuer, not conformance to bounded claim semantics. This matters to short-lived delegated entry, approval freshness, key overlap and nonce capacity/retention assumptions.\n\n**Required correction:** explicitly assign issuer `iat = t`; require receivers to validate the selected proof type's exact `nbf/iat/exp` relationship or its stated bounded equivalent before entry/spend. Retain checked arithmetic and conservative current-time checks. Do not rely on the issuer construction prose alone to reject an overlong signed proof.\n\n**Verification:** add valid-boundary and correctly signed invalid vectors for overlong delivery, overlong approval, inconsistent iat/nbf, future iat with an otherwise currently valid interval, and expiry equality. Distinguish malformed lifetime from an ordinary valid profile token observed outside its interval.",
    "line": 145
  }
]
```


---
format: aep.planning-md/1
id: verification-report:findings-auth-access-b-r2-20260908
kind: verification-report
status: draft
title: Structured transcription of review-result:auth-access-b-r2-20260908
tags:
- review-findings-supplement
refs:
- provider: review-body-sha256
  reference: 79e818709882fde48da5024bf4a5db1f9e2774ceceae036aed4b39775f04eac9
relations:
- verifies: review-result:auth-access-b-r2-20260908
- derived_from: specification:review-findings-toolchain-upgrade-20260911
revision: 1
---
# Structured transcription of review-result:auth-access-b-r2-20260908

This supplements [the immutable original](../review-result/auth-access-b-r2-20260908.md).
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
    "file": "contracts/auth/capability/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "## AP-B-R01 — normative 401 retry rule still grants an unselected behavior (P2)\n\n`contracts/auth/capability/v1alpha1/semantics.md:73` says a selected refresh-capable profile's 401 \"triggers at most one coordinated refresh ... and one re-dispatch for reads\". Its corrected scenario at line 95 instead requires a separately permitted re-dispatch and leaves exact read-retry support to its owner story. `contracts/auth/acquisition/v1alpha1/semantics.md:120` explicitly leaves the read-retry contract unsettled. The normative rule can therefore be read as selecting automatic business-read retry merely from refresh capability, before that owner's prerequisites exist. A fresh credential generation also invalidates existing permission evidence under evidence §4.4.\n\nCorrection: qualify the normative bullet itself: read re-dispatch is at most once and only under an explicitly selected read-retry binding, with fresh current-generation admission and the original remaining shared budget/deadline. Without that separately supported binding, no automatic re-dispatch. Do not implement or invent that owner's full retry protocol here.",
    "line": 73
  },
  {
    "file": "contracts/auth/acquisition/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "## AP-B-R02 — authorization-code client-auth declaration is required by evidence but absent from its matrix (P2)\n\n`contracts/auth/acquisition/v1alpha1/semantics.md:76` requires sourced endpoints, an explicit PKCE choice, registration kind, callback binding and token interpretation for authorization code, but does not require the explicit client-authentication choice. The adjacent client-credentials row does. `docs/evidence/auth-profile-budget-20260908/traces.md:46` Q07 claims a missing sourced PKCE/client-auth choice must refuse. Registration kind alone does not specify the authentication method used at the token endpoint, and PKCE is a different proof.\n\nCorrection: require the reviewed, sourced client-auth method/policy explicitly in the authorization-code row. Keep its actual vendor behavior and future reader binding as advertisement prerequisites. A generic ESS shape need not execute this predicate.",
    "line": 76
  }
]
```


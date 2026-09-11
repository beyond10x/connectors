---
format: aep.planning-md/1
id: verification-report:findings-federation-subjects-r3-20260908
kind: verification-report
status: draft
title: Structured transcription of review-result:federation-subjects-r3-20260908
tags:
- review-findings-supplement
refs:
- provider: review-body-sha256
  reference: a2d75cd5e3a435fd443c88c9f0efcf6eb24efff83460be8a3a7f367461fe1892
relations:
- verifies: review-result:federation-subjects-r3-20260908
- derived_from: specification:review-findings-toolchain-upgrade-20260911
revision: 1
---
# Structured transcription of review-result:federation-subjects-r3-20260908

This supplements [the immutable original](../review-result/federation-subjects-r3-20260908.md).
It records no new critic run, approval, review outcome or current implementation
finding. Source binding uses the SHA-256 of the exact original body returned by AEP.
The original review's scope, verdict, source snapshot and historical date remain authoritative.

## Original conclusion

The original report's verdict and scope remain authoritative; no new verdict is assigned.

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
    "file": "contracts/operations/v1alpha1/scenarios/idempotency-namespace-values.yaml",
    "category": "legacy-review",
    "severity": "warning",
    "message": "### F03-A-R3-01 — P2 — Federated positive reservation inherits a direct null route\n\n**Source:** `contracts/operations/v1alpha1/scenarios/idempotency-namespace-values.yaml:195–206`, inheriting the anchor at lines 58–68. Normative requirements: `contracts/service/delegation.md:51,64` and the operations fingerprint definition.\n\nThe positive federated reservation names origin `{kind: federated, authority_ref: trusted-gateway}` but reuses `fingerprint: *id001`. That anchor contains `route: null`. F03 requires a non-null route with gateway identity matching the admitted federated origin. The fixture says its callers/origins were separately admitted and expects successful reservation, so this is an inconsistent positive example, not an intentionally rejected adversarial value.\n\nESS authoring still accepts it because cross-value origin/route admission is explicitly outside the generic type checker. Therefore a passing authored/synthesized scenario count does not correct the semantic inconsistency.\n\n**Required correction:** retain the shared null route for the three direct-origin examples; give the federated candidate a full fingerprint with `{gateway_instance: trusted-gateway, route_id, route_revision}`. Regenerate the retained operations-authored output and refresh the relevant synthesis evidence. Do not introduce a separate business key or origin namespace to make the example pass.",
    "line": 195
  }
]
```


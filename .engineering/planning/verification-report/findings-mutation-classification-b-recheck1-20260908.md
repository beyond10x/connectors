---
format: aep.planning-md/1
id: verification-report:findings-mutation-classification-b-recheck1-20260908
kind: verification-report
status: draft
title: Structured transcription of review-result:mutation-classification-b-recheck1-20260908
tags:
- review-findings-supplement
refs:
- provider: review-body-sha256
  reference: 67c00761dd4bd9c551a852c33da91a400af63d6097723f88b032f5d7e4ae8c17
relations:
- verifies: review-result:mutation-classification-b-recheck1-20260908
- derived_from: specification:review-findings-toolchain-upgrade-20260911
revision: 1
---
# Structured transcription of review-result:mutation-classification-b-recheck1-20260908

This supplements [the immutable original](../review-result/mutation-classification-b-recheck1-20260908.md).
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
    "file": "docs/adapters/media-session.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "## MP-B-C01 — Known session termination is mislabeled as session loss\n\n**P2. Sole owner: story:contracts-mutation-classification.**\n\nThe new SIP table (`docs/adapters/media-session.md:85`) selects **session_lost for every terminal** observed after readiness and before result encoding. The verification record's MC18 repeats it. That trigger includes confirmed local_close/remote_hangup, revoked and lease_expired, not only continuity loss.\n\nThe sessions owner explicitly distinguishes lost continuity from known terminal reasons and confirmed closed resource release (`contracts/sessions/v1alpha1/semantics.md:47`, §§4–4.1). The new delivery boundary should not discard a known terminal cause by presenting every such session as lost. Mutation applied knowledge is correctly retained and must remain so.\n\n**Required correction:** retain applied and withhold the usable handle; choose the applicable existing safe session error for the observed terminal fact (for example revoked, lease_expired or session_not_ready). Use session_lost only when continuity is actually lost. Preserve the first terminal and historical receipt, and update MC18. No new code, state, wire field, protocol or runtime test is required. The exact finding was frozen separately in `finding-MP-B-C01.md` before any source correction.",
    "line": 85
  }
]
```


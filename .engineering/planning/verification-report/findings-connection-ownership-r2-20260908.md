---
format: aep.planning-md/1
id: verification-report:findings-connection-ownership-r2-20260908
kind: verification-report
status: draft
title: Structured transcription of review-result:connection-ownership-r2-20260908
tags:
- review-findings-supplement
refs:
- provider: review-body-sha256
  reference: 430152edd861d6676ed5d238a6460784065c659a89dd0f6678bef28f51afc9b3
relations:
- verifies: review-result:connection-ownership-r2-20260908
- derived_from: specification:review-findings-toolchain-upgrade-20260911
revision: 1
---
# Structured transcription of review-result:connection-ownership-r2-20260908

This supplements [the immutable original](../review-result/connection-ownership-r2-20260908.md).
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
    "file": ".engineering/planning/review-result/connection-ownership-r2-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "### RCBR-05 — P2: optional Invocation.connection became “nullable” by implication\n\nManagement §1 calls the field “the existing nullable Invocation.connection selector” and says create/list reject a “non-null” selector. E02 compatibility §4 instead defines the field as an optional safe connection ref and permits omission only where the selected profile resolves one implicitly. The management instance/acquisition-scoped exception is intended, but the texts disagree both about omission and about present JSON null. The nullable F03 **signed claim** is a different object from the Invocation field.\n\nMinimal fix: specify a present, nonempty exact string for connection-scoped describe/revoke/repair; omit the Invocation field for instance/acquisition-scoped create/list/status, rejecting a supplied selector including JSON null. A protected completion is not an Invocation. Align compatibility §4's omission rule with those selected management scopes, while keeping implicit configured resolution only for profiles that actually define it. Reject duplicate body selectors that disagree. Do not silently broaden the wire to null or invent an implicit business connection. A root-level `connection:null` example should not be accepted merely because its F03 claim uses null."
  },
  {
    "file": ".engineering/planning/review-result/connection-ownership-r2-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "note",
    "message": "### RCBR-06 — P3: management says revoked records can be repaired immediately before prohibiting it\n\nManagement §1 says an admitted caller “must be able to inspect, repair or revoke pending, disabled, degraded and revoked records,” then says revoked records cannot be repaired or re-enabled. The second rule and the new terminal state reduction agree; the first sentence accidentally makes a conflicting promise.\n\nMinimal fix: distinguish the actions directly: independently admitted inspection/local revoke can target non-ready records including revoked; repair may target only a non-revoked binding. The rule exempts management from provider readiness, not from action-specific lifecycle prerequisites."
  }
]
```


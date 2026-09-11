---
format: aep.planning-md/1
id: verification-report:findings-sessions-semantics-r1-20260908
kind: verification-report
status: draft
title: Structured transcription of review-result:sessions-semantics-r1-20260908
tags:
- review-findings-supplement
refs:
- provider: review-body-sha256
  reference: b8a098ed280935a545dd064e69593536e2e7b71c6361b56873eb9c97e6f8a683
relations:
- verifies: review-result:sessions-semantics-r1-20260908
- derived_from: specification:review-findings-toolchain-upgrade-20260911
revision: 1
---
# Structured transcription of review-result:sessions-semantics-r1-20260908

This supplements [the immutable original](../review-result/sessions-semantics-r1-20260908.md).
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
    "file": "contracts/sessions/v1alpha1/scenarios/first-terminal-not-replaced.yaml",
    "category": "legacy-review",
    "severity": "blocker",
    "message": "## Major R1 — successful authored close examples extend cutoff past existing lease expiry\n\nLocations:\n\n- `contracts/sessions/v1alpha1/scenarios/first-terminal-not-replaced.yaml:62`\n- `contracts/sessions/v1alpha1/scenarios/media-overload-shared-reason.yaml:61`\n- `contracts/sessions/v1alpha1/scenarios/revocation-refuses-data-and-renewal.yaml:71`\n- `contracts/sessions/v1alpha1/scenarios/revocation-beats-renewal.yaml:81`\n\nEach scenario successfully establishes the live lease with effective expiry `12:00:04Z`, with no later successful renewal. The first two successful `BeginClose` decisions nevertheless publish `cutoff_due_at: 12:00:05Z`; the other two publish `12:00:06Z`. These are not adversarial inputs expected to be refused: they are accepted close decisions and emit `ClosingBegun`.\n\nThe normative contract requires cutoff at effective expiry without any grace (`contracts/sessions/v1alpha1/semantics.md:102`, `:110`), and the model comments explicitly say earlier expiry/drain deadlines dominate (`ess/domains/sessions.yaml:175`). A later ordinary close ceiling cannot supersede an already earlier deadline. The textual audit therefore currently accepts examples inconsistent with its own chosen rule. ESS compilation does not catch this because the specification explicitly leaves timestamp comparisons unmapped; that disclosure is correct but does not make the example deadlines coherent.\n\nMinimal fix: set each successful close cutoff to the earliest applicable lease expiry, drain deadline and terminal ceiling. In these four examples that is at most `12:00:04Z`. Keep intentionally rejected later close inputs in the first-terminal example clearly identified as adversarial observations. Re-run the explicit session author/synthesis checks after correcting the examples, and audit the resulting supplied deadlines against the prose.",
    "line": 62
  },
  {
    "file": "contracts/sessions/v1alpha1/scenarios/revocation-beats-renewal.yaml",
    "category": "legacy-review",
    "severity": "warning",
    "message": "## Minor R2 — the revocation-race example shifts the authority time after observing revocation\n\nLocation: `contracts/sessions/v1alpha1/scenarios/revocation-beats-renewal.yaml:58`, with the later terminal record at `:80` and teardown deadline at `:82`.\n\nThe scenario supplies a trusted `revoked` authority decision at `12:00:03Z`, then records the host's terminal acceptance at `12:00:04Z` and gives teardown until `12:00:09Z`. Its summary explicitly describes a revocation already known before the local closing transition. The prose instead defines the bounds from authoritative acceptance and says the host serializes revocation and renewal (`contracts/sessions/v1alpha1/semantics.md:97`, `:108`). As written, it is unclear whether the later lifecycle bookkeeping is incorrectly allowed to start a fresh terminal timer.\n\nMinimal fix: distinguish authoritative acceptance from the later modeled lifecycle observation. For example, preserve an authoritative `accepted_at` of `12:00:03Z` (or earlier), cap teardown at `12:00:08Z`, and explain that the `BeginClose` act at `12:00:04Z` records that existing fact without renewing its deadlines. Alternatively use a scenario ordering that has no earlier trusted revocation observation. Do not imply that observing an already committed revocation lets a later local transition reset the clock.",
    "line": 58
  }
]
```


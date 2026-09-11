---
format: aep.planning-md/1
id: verification-report:findings-discovery-b-recheck1-20260908
kind: verification-report
status: draft
title: Structured transcription of review-result:discovery-b-recheck1-20260908
tags:
- review-findings-supplement
refs:
- provider: review-body-sha256
  reference: 22654ee016a8af805de2d1f9b8c999b2b7156adbb9662c8fcffb60aeac924e83
relations:
- verifies: review-result:discovery-b-recheck1-20260908
- derived_from: specification:review-findings-toolchain-upgrade-20260911
revision: 1
---
# Structured transcription of review-result:discovery-b-recheck1-20260908

This supplements [the immutable original](../review-result/discovery-b-recheck1-20260908.md).
It records no new critic run, approval, review outcome or current implementation
finding. Source binding uses the SHA-256 of the exact original body returned by AEP.
The original review's scope, verdict, source snapshot and historical date remain authoritative.

## Original conclusion

Review limitations: no provider calls, runtime tests, reducers, gate, schema generation or planning changes were performed. Shape/trace evidence has not yet been supplied for final confirmation. No other reviewer report was inspected. No verdict on E07/E14/E32 or implementation advertisement is implied.

## Transcription method

3 findings remain in the scope of this report's final stated conclusion.
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
    "file": ".engineering/planning/review-result/discovery-b-recheck1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "## DC-B-R01 — partial merge can downgrade a terminal withdrawn row (P2)\n\nResource discovery §4.2 table at lines110–112 says denied/failed/capped partitions retain their prior/unobserved rows as stale. §4.3 line123 explicitly makes a positively withdrawn incarnation terminal and requires a new id on reappearance. The general merge rule does not exclude previously withdrawn rows, so a complete deletion followed by a denied or capped scan can change withdrawn back to stale. Subsequent exact re-observation would then appear eligible for the same-id stale recovery path, contradicting terminal withdrawal.\n\nCorrection: make merge precedence explicit: terminal withdrawn incarnations remain withdrawn until historical eviction; failed/incomplete scans cannot demote them to stale or observed. Only nonterminal previously positive/stale observations use stale retention. Reappearance always gets a new incarnation/id, even if a later scan lacks complete coverage. Add a trace for complete withdrawal → denied/capped scan → same provider identity reappears. Also qualify resource scenario line151: type change yields withdrawn only when complete comparable coverage proves old-incarnation absence; otherwise stale/ineligible is the selected rule at123."
  },
  {
    "file": ".engineering/planning/review-result/discovery-b-recheck1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "## DC-B-R02 — publication deadline conflicts with partial output after deadline exhaustion (P2)\n\nResource discovery line112 selects a publishable partial view after an object/byte/call/**deadline** cap. Publication line127 captures the original deadline and rechecks the captured facts at the atomic publication point; ESS discovery.yaml line61 exposes deadline_current. Limits142 gives a shared original provider deadline. The text does not distinguish a provider-send/collection deadline from the enclosing execution/publication deadline. If they are the same deadline, a scan ending because it expired cannot satisfy the publication gate, so the promised partial publication/result is impossible.\n\nCorrection: explicitly choose the two-clock/budget relationship or remove deadline-expired partial success. For example, provider collection can end capped at its original provider deadline with no further provider sends, while a still-live enclosing operation deadline admits bounded metadata publication/result delivery; once the operation/publication deadline expires, refuse and do not publish this attempt. Both original deadlines must be pinned, never reset on pages/subwork, with current authority and publication fencing still checked. Alternatively select no publication after the single deadline and state that exception in the table. Add separate traces for provider deadline exhausted while publication remains admitted versus enclosing deadline exhausted before publication. This is a semantic distinction, not a request for clock/runtime implementation."
  },
  {
    "file": ".engineering/planning/review-result/discovery-b-recheck1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "## DC-B-R03 — first Service profile adds a cluster-wide target not defined by F08 (P2)\n\nResource discovery line77 and Kubernetes adapter's final coverage paragraph permit one explicitly admitted cluster-wide Service-list partition. Existing F08 auth evidence §4.4 (line124 in the frozen auth evidence) instead says empty namespace means **cluster scope**, while a namespaced list carries its actual namespace; exact configured resource kinds multiply namespace targets. Resource discovery itself limits this profile to core/v1 Services and per-namespace SSAR at92. A single all-namespaces Service request is neither a cluster-scoped resource nor one exact namespaced list under that prerequisite. Treating it as one cluster partition would also sidestep the selected finite namespace fan-out without a corresponding permission-target rule.\n\nCorrection: keep the first Service profile on its finite admitted per-namespace partitions. Mark cluster partition vocabulary as reserved/nonselectable for that profile until a separately reviewed all-namespaces/cluster-scoped binding defines exact authorization, membership, bounds and compatibility. If cluster-wide Service listing is intentionally selected now, make its exact distinct F08 target semantics explicit and coordinate that profile change; do not silently infer it from namespace:null/empty string. E07/E14/E32 remain separately owned and are not closed by generic coverage vocabulary. Include a rejection trace for an unsupported cluster-wide Service selection."
  }
]
```


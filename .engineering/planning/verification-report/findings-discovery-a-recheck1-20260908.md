---
format: aep.planning-md/1
id: verification-report:findings-discovery-a-recheck1-20260908
kind: verification-report
status: draft
title: Structured transcription of review-result:discovery-a-recheck1-20260908
tags:
- review-findings-supplement
refs:
- provider: review-body-sha256
  reference: 8cf81b6ef2cf4195ee5ea24ce57afbd114200e285fba9a39387daa760895ec92
relations:
- verifies: review-result:discovery-a-recheck1-20260908
- derived_from: specification:review-findings-toolchain-upgrade-20260911
revision: 1
---
# Structured transcription of review-result:discovery-a-recheck1-20260908

This supplements [the immutable original](../review-result/discovery-a-recheck1-20260908.md).
It records no new critic run, approval, review outcome or current implementation
finding. Source binding uses the SHA-256 of the exact original body returned by AEP.
The original review's scope, verdict, source snapshot and historical date remain authoritative.

## Original conclusion

**Verdict: needs revision. Residual findings: P0 0 / P1 0 / P2 3 / P3 0.** This is the intermediate normative review; the final typed/textual evidence packet is not yet ready. Initial findings DC-A-01–08 have substantive responses, with the remaining contradictions below. E07/E14/E32 remain explicitly outside closure.

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
    "file": ".engineering/planning/review-result/discovery-a-recheck1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "## DC-A-R2-01 — P2 — Downstream rules still prohibit the selected per-partition partial behavior\n\n**Sources:** resources:109–115, versus auth/evidence:132, service compatibility:155, Kubernetes adapter:69; Grafana adapter:134.\n\nThe new discovery owner explicitly permits a complete comparable partition to confirm its own absent rows even when another partition makes the overall view incomplete. The unchanged auth/evidence wording prohibits withdrawing unseen resources from an incomplete observation, the compatibility summary says incomplete coverage cannot establish discovery withdrawal, and Kubernetes repeats the blanket prohibition. A complete a plus denied b scan therefore has inconsistent instructions about a's previously known absent object.\n\nThe new Grafana paragraph separately says cap/failure/filtered visibility retains only historical unknown evidence, while resources:112 permits trustworthy newly collected positives to be published as observed in a partial view. The adapter summary should not suppress that selected positive-evidence branch or imply stale history is fresh. The older resource conformance rename/type-change trace at :151 should also state the complete comparable scan prerequisite for the asserted withdrawal.\n\n**Required correction:** align auth/evidence, compatibility and adapter summaries with the selected distinction: incomplete partitions provide no absence; an independently complete, comparable partition may prove absence only inside its exact scope, even in a partial view. Permission coverage by itself still proves no provider exhaustion. Say that retained unobserved rows are historical while trustworthy current positives may remain observed under the owner's rule. Keep F08 unknown-authorization pre-resource refusal intact and leave E07/E14/E32 open."
  },
  {
    "file": ".engineering/planning/review-result/discovery-a-recheck1-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "## DC-A-R2-02 — P2 — The merge rule can downgrade terminal withdrawn evidence to stale\n\n**Sources:** resources:109–112,117,123,139–144.\n\nThe denied-subset row says to retain prior rows as stale, and the partial/failure row says to retain unobserved prior rows as stale. Those prior rows can already be withdrawn. Section 4.3 makes a positively withdrawn incarnation terminal and requires any later reappearance to receive a new id. The generic stale-retention rule can erase that stronger fact during an incomplete subsequent scan, or at least expose a misleading nonterminal classification.\n\n**Required correction:** state merge precedence explicitly: a retained withdrawn incarnation keeps its withdrawn classification/terminal fact through later denied, capped, unavailable or not_scanned partitions; it is never demoted to stale or renewed into observed. Only bounded eviction removes its row, and lost verifiable continuity still prohibits reuse of its old id. Fresh positives for a reappearing incarnation receive the new id even if the retained old row remains visible. Add a trace for complete withdrawal followed by denied/failed scan and then same-UID reappearance."
  },
  {
    "file": "ess/domains/discovery.yaml",
    "category": "legacy-review",
    "severity": "warning",
    "message": "## DC-A-R2-03 — P2 — Deadline-limited partial publication does not identify which deadline must remain current\n\n**Sources:** resources:112,127,131,142; ess/domains/discovery.yaml:52–62; service compatibility's separate provider/execution ceilings.\n\nThe scan table promises a partial classified view when a deadline cap stops provider work. Publication captures the original deadline and requires rechecking the captured facts, while PublicationFacts contains deadline_current. Scan work names the original provider deadline. If that same expired deadline must remain current at publication, the promised deadline-capped partial publication is impossible; if a different execution/finalization deadline is intended, it is not identified. An implementer could instead silently extend the expired provider budget to finish/publish.\n\n**Required correction:** distinguish the provider-work deadline from the outer execution/publication/response deadline. State exactly when provider sends stop, whether already trustworthy positives may publish while the outer deadline remains valid, and what happens once that final deadline expires. No extra provider send, hidden renewed budget or success after the selected outer boundary is permitted. Type/comment the publication deadline fact consistently and add both boundary traces. Alternatively select refusal rather than successful partial publication when the only supported deadline expires; the document must choose one coherent rule.",
    "line": 52
  }
]
```


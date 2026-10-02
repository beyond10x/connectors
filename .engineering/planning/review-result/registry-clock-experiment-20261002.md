---
format: aep.planning-md/3
id: review-result:registry-clock-experiment-20261002
kind: review-result
status: active
title: Independent review of the completed clock rejection experiment
relations:
- reviews: story:registry-clock-outside-shared-batches
revision: 1
---
approve
unit: registry-clock rejection evidence at 26929027f85b4653d81bd9cb290a7f472cfe84e8
verdict: nothing found — rejection experiment complete; candidate remains rejected
cases: executed 11→11, red 1 in retained baseline→candidate runs; review reruns 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: worktree-managed review lease metadata only
needs-coordinator: create and verify the promised recovery archive before publication

`git diff 26929027f85b4653d81bd9cb290a7f472cfe84e8 --stat` and `git status --short` both returned empty output. HEAD remains the exact base. This review added no cases, changed no files and ran no builds or probes.

The frozen report hashes to `99fe25a55f592f7381bd40e6fb430383ccad596de57cb41f1fd82e14e6b132f7`; every entry in `evidence-final.sha256` verified.

The retained results support the decision:

- **Invariants:** baseline 9 passed; candidate 8 passed and 1 failed. The decisive unchanged test returned `Err(MetadataUnavailable)` instead of `Err(ConcurrentRevision)`, exit 101. No stale successful response is claimed. Evidence: `.local/clock-experiment/candidate-same-millisecond.log:7`, baseline counterpart `:270`, report `:75`.
- **Both complete numeric matrices:** actual sizes 55/601/1203 appear in both invoke and batch logs. All thirty measured invokes report success, with no caught-panic output from the six warmups. Candidate wall medians are 356/4410/9227 ms; recomputation gives **25.918539×**, exceeding 2×. Evidence: `candidate-invoke.log:5`, `:22`, `:39`; both batch logs `:5`.
- **Fixture identity:** current saved databases, references and recorded sidecars verify against the complete creation manifest. Before-phase manifests match it exactly; retained creation/copy hashes and timestamps support the actual generation sequence, without inventing a pre-generation manifest. Evidence: `fixtures-complete-created.sha256`, `baseline-untouched-copies.sha256`, `baseline-1200-copy.sha256`, `fixture-1200-created.at`.
- **Source identity/restoration:** the one-condition transformation reproduces the recorded candidate source hash. Restored source and lock hashes verify; probe/test sources match the base. The retained candidate binary matches its manifest. The baseline binary identity is historical manifest evidence, since that executable was replaced. Bridge-source comparison against `b1f4327660` is empty.
- **Public draft:** copied logs equal originals after exactly the declared home-prefix substitution; exits and rejected patch match byte-for-byte. Local Markdown links resolve. Numeric claims and upstream limitations agree with the raw evidence. Archive creation remains the explicitly assigned publication prerequisite, not an action established by this review.

No revision findings. This approves recording the **completed rejection experiment**, not integrating the candidate or declaring production performance green. Entity Runtime #51 and the larger sustained-workload milestone remain unresolved.

Review lease `codex-cb26c-clock-evidence-review` was released successfully. Outside writes were limited to tool-managed lease metadata beneath `$HOME/.local/state/worktree`; the home prefix is redacted here. No worker scratch or source was written.

```findings
[]
```

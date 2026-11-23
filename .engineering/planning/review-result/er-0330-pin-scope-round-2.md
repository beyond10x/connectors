---
format: aep.planning-md/3
id: review-result:er-0330-pin-scope-round-2
kind: review-result
status: active
title: Scope critic, round 2, story:entity-runtime-0330-records-name-definition-by-digest
relations:
- reviews: story:entity-runtime-0330-records-name-definition-by-digest
revision: 1
---
verdict: needs-revision

1. The "third vs fifth" disagreement from round 1 finding 3 is only half fixed.
   - This story now says about 3× for the read workload and bounds it at "at most half" (story:28-34). `story:settled-read-uses-leave-the-store` still says "With `er.record/5` the bytes per read fall to an estimated fifth" (line 28 of its body, revision 1).
   - Change: in that sibling, replace "an estimated fifth" with "about a third (3×; story:entity-runtime-0330-records-name-definition-by-digest)". This does not block the pin story itself.

Findings 1, 2, 4 and 5 from round 1 are closed:
- This story owns the `blob_bytes_per_invoke` print (acceptance 3).
- `story:read-cost-test-reports-bytes-per-read` now has `depends_on` on this story and keeps only the bound, taken after the pin.
- Source states that the 650 MB store does not shrink and where the consumer's gain comes from.
- Acceptance 7 puts the "records already written keep their size" sentence in the CHANGELOG.
- The RSS line is gone.

What I read: the revised story, `story:read-cost-test-reports-bytes-per-read` and `story:settled-read-uses-leave-the-store`, all via `aep plan artifact show`.

Not established, and outside scope-critic judgement: the new "One-way for an existing store" section, question D4 and the Breaking entry trace to the Entity Runtime release note that 0.32.0 cannot open a 0.33.0 store. I did not count them as reach. Whether acceptance 6 can be run is for the acceptance critic.

```findings
- {file: .engineering/planning/story/settled-read-uses-leave-the-store.md, line: 28, category: scope, severity: warning, verdict: needs-revision, origin: introduced, message: "settled-read-uses-leave-the-store still says the bytes per read fall to an estimated fifth; the pin story now states about 3x for the read workload, so the two stories disagree"}
```

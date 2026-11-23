---
format: aep.planning-md/3
id: review-result:er-0330-pin-design-round-2
kind: review-result
status: active
title: Design critic, round 2, story:entity-runtime-0330-records-name-definition-by-digest
relations:
- reviews: story:entity-runtime-0330-records-name-definition-by-digest
revision: 1
---
verdict: approve

All four round-1 findings landed in the stored text:
- **Print and bound ownership:** the pin story owns the print (its acceptance 3, 4 and 8). The read-cost story is at revision 2, keeps only the bound, and carries `depends_on` to the pin story.
- **Test-function edge:** `story:settled-read-uses-leave-the-store` now has `depends_on` to both the pin story and the read-cost story. The graph shows both edges.
- **`LEVEL` stays 9:** the section "One-way for an existing store" (lines 65-85) states the decision with the code reasons. The deferred conditional is gone.
- **Checkpoint precedent:** the `informed_by decision-blocker:store-checkpoints-one-way` edge is in the graph. The body says why opt-in is impossible (lines 71-73), and acceptance 7 names the Breaking entry with the way back.

No cycle: the edges form a chain of read-cost after pin, then settled-read, plus the direct pin edge from settled-read. Each edge has a stated reason in "Shares files with" (lines 168-170).

What I read: the three stories in full, plus `aep plan artifact graph` filtered to their edges. `aep plan artifact validate` reported `valid`.

Not established, and none of it blocks:
- D4 is open. If the operator picks "level raise" or "writer switch first", the "One-way for an existing store" section and acceptance 6 and 7 change. The story is built on a branch on the default and its merge waits, so I did not treat this as a finding.
- Acceptance 6 and 7 say "0.75.0 and earlier" in the CHANGELOG sentence but run the 0.74.2 archive. Both builds pin Entity Runtime 0.32.0, so I did not treat this as a finding.
- I did not run any build or test. The earlier conclusion that no call site in `er.rs` must change still rests on reading the 0.32.0 and 0.33.0 sources only.

```findings
[]
```

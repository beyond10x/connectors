---
format: aep.planning-md/3
id: review-result:wave-d417-u1-adversary-2
kind: review-result
status: active
title: Adversary re-check of the cold owner start after D422
relations:
- reviews: story:cold-owner-start-verifies-once
revision: 1
---
**Verdict: F1 to F4 of `review-result:wave-d417-u1-adversary` closed; one leftover warning from the F1 fix.**

Adversary re-check (D422) on `e1b5d3c23`, branch `unit/d417-cold-owner-start`. The three cases of `23247269c`, red at
`e60844a1d`, pass alone; `cargo test -p connectors-host --lib checkpoint_tests` 8 passed and
`cargo test -p connectors --test cold_owner_start --test cold_owner_start_adversary --test cold_owner_start_security`
6 passed plus 1 ignored, 3 passed, 3 passed, exit 0 (`~/.cache/connectors-wave-20261103/u1-adv/r2.log`). Executed
20, red 0. The leftover has no case yet; the lead sent it back to the unit as correction round 3, inside the acceptance
(one cold start on a store of about 200 MB verifies once).

```findings
[
 {"file":"crates/connectors-host/src/local/metadata/er.rs","line":584,"category":"concurrency","severity":"warning","verdict":"CONFIRMED","origin":"undecided","message":"`if locked || ...` stops every locked open from waiting, including when the holder is an inspection that never relocks (owner maintenance's Metadata::inspect every 5 s, mutations.rs:207). So a read's locked opens in the owner (the read-use grant, transaction_admission at use_and_retirement.rs:229, and audit writes, audit.rs:61) verify the whole store again when they meet that 70 ms hold, as the 200 MB timeline already measured (u1/diag-timeline-200mb.log:314-318, about 4.7 s). Only a holder that will relock (an unlocked observation) needed to be excluded, not every holder."}
]
```

Not checked by the re-check: with F2's fix, the refusal code for an uncached description that may not launch now comes
from the owner rather than from the direct read.

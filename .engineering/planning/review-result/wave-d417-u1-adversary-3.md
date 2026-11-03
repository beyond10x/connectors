---
format: aep.planning-md/3
id: review-result:wave-d417-u1-adversary-3
kind: review-result
status: active
title: Adversary re-check of the cold owner start after round 3
relations:
- reviews: story:cold-owner-start-verifies-once
revision: 1
---
**Verdict: nothing found; the leftover of `review-result:wave-d417-u1-adversary-2` (er.rs:584) is closed.**

Adversary re-check on `bc8c9fdae` (round 3: `er::Hold` per checkout; a locked open skips the wait only while a holder
that may relock has the handle). `cargo test -p connectors-host --lib checkpoint_tests`: 9 passed, exit 0, 11.73 s
(`~/.cache/connectors-wave-20261103/u1-adv/r3b.log`), including
`a_locked_open_waits_for_the_handle_an_inspection_uses_instead_of_verifying_again` (red at `e1b5d3c23`) and
`a_locked_open_never_waits_for_a_handle_whose_user_waits_for_its_lock`. A first attempt failed all 9 in 0.00 s because
`.local/tmp` was missing after a `cargo clean` for disk; it was recreated and rerun. Thirteen inspect sites were checked:
no production path has an inspection whose callee takes the lifecycle lock; one added later would be slow, not a
deadlock.

```findings
[]
```

---
format: aep.planning-md/3
id: review-result:owner-cancel-adversary-r1-20261111
kind: review-result
status: active
title: Adversary pass 1 on owner-cancels-a-bounded-read
relations:
- reviews: story:owner-cancels-a-bounded-read
revision: 1
---
# Adversary pass 1: owner-cancels-a-bounded-read

Reviewed commit c2abf89b8b on impl/owner-cancels-a-bounded-read (base 8c47ef8581). Cases in
`apps/connectors/tests/owner_cancel_adversary.rs`: 6 cases, 4 green, 2 red, run with one test
thread through the build slot.

Held: a cancel from another session leaves the running read intact; a cancelled queued read is
never sent; the adapter of a cancelled read is reaped within 1 s with no zombie; the next job waits
no longer than a relaunch; the /1 and /2 paths are unchanged.

Not held: a read cancelled during its adapter launch still opens the stored credential after the
cancel; a cancelled read leaves its instance reported `failed`. A cancelled queued job keeps its
queue slot until the worker reaches it (read in the code, not tested: the no-load-tests rule
arrived before the case could run).

Coordinator decision on the third finding: the contract is reworded to state that the slot is held
until the worker reaches the job; freeing it at cancel time is filed as
story:owner-cancel-frees-queue-slot.

```findings
[
{"file":"contracts/cli/v1alpha1/owner.md","line":119,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"a read cancelled while its adapter launches still has its stored credential read after the cancel and after the server exited, because the worker checks the cancel only at pickup (supervisor.rs:1106) and before dispatch (supervisor.rs:1801), not before the launch and credential read at supervisor.rs:1283 and :1703"},
{"file":"crates/connectors-host/src/local/owner/lifecycle.rs","line":92,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"ending the adapter on cancel leaves the dead child recorded, so instances status reports failed for a deliberate kill, against semantics.md:1473 which reserves failed for unexpected termination"},
{"file":"crates/connectors-host/src/local/owner/supervisor.rs","line":702,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"a cancelled queued job keeps its slot in the 16-slot queue until the worker reaches it (supervisor.rs:1105), so cancelled jobs behind a long running job can get another read refused capacity, which is weaker than owner.md:118 \"frees its queue slot\""}
]
```

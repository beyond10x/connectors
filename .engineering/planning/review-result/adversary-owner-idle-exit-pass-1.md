---
format: aep.planning-md/3
id: review-result:adversary-owner-idle-exit-pass-1
kind: review-result
status: active
title: Adversary pass 1 on owner idle exit
relations:
- reviews: story:owner-idle-exit
revision: 1
---
unit: story:owner-idle-exit, uncommitted tree wave0929c-idle on 74f09e6d4
verdict: NEEDS-CHANGE
cases: executed 308→309, red 1
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: scratch/adversary/suite.log
needs-coordinator: none

Case apps/connectors/tests/owner_idle_exit_adversary.rs (red: 4 idle owners with a 30 s bound, none exited by 45 s).
Could not break: in-flight writes, captures and revalidations; a job in the grace window; recovery at retirement;
a child mid-start; a long read; the override in a release build or a CLI-started owner; socket removed before the
lock is released.

Coordinator routing: the blocker to the implementor (a sweep is work only while it recovers something); the note
decided (recovery that settles nothing does not reset the idle clock; pending attempts resume on the next owner).

```findings
[
  {"file": "crates/connectors-host/src/local/owner/maintenance.rs", "line": 51, "category": "acceptance", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "every empty 5 s recovery sweep marks itself busy and the 10 ms idle poll sees it, so with the 600 s bound an owner with no client, no child and nothing pending effectively never exits"},
  {"file": "crates/connectors-host/src/local/owner/transport.rs", "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "a pending attempt that can never settle requeues recovery every 5 s and keeps the owner alive indefinitely"}
]
```

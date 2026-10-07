---
format: aep.planning-md/3
id: review-result:adversary-entity-runtime-029-pin-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the Entity Runtime 0.29.0 pin
relations:
- reviews: story:entity-runtime-029-pin
revision: 1
---
unit: U1 story:entity-runtime-029-pin
verdict: green
cases: executed 892→892, red 0
origin: introduced 2, pre-existing 0, undecided 0
wrote-outside-worktree: ~/.cache/u1adv-j (deleted)
needs-coordinator: no

No failing case against 42d2885ab; two notes, neither holds the unit.

Probes run and deleted: an in-place restore of the store under an idle verified handle (reads and the next write matched a fresh process); stores written by 0.29.0 opened by the 0.26.0-built test binary (both tests passed; the 0.26.0 fixture after a 0.29.0 write passed write-and-replay); installed connectors 0.31.0 and 0.28.0 reported metadata ready on a store the new build created.

Could not break: checkpoints are installed only by the explicit enable call (entity-eventlog sync.rs:957), which no runtime call of connectors reaches; a checkpoint is written only by a drain shutdown (sync.rs:2247) and connectors uses CancelQueued (er.rs:530); the new tail-loss refusal falls back to a fresh open; BridgeOperationIdentity::Administration is never matched; read-bound accounting is unchanged for whole-model handles; no new third-party crates; the fixture README's counts and hashes hold; one runtime copy each of eventlog-core and entity-core 0.29.0, plus entity-core 0.24.1 from ESS.

| # | file:line | verdict / origin | finding |
|---|---|---|---|
| F1 | crates/connectors-host/tests/fixtures/metadata-store-er-0.26.0/README.md:16 | CONFIRMED / introduced | the README lists the fixture's SHA-256 values and forbids regenerating it, but no test checks them; the hashes match today |
| F2 | website/docs/introduction/status.md:21 | CONFIRMED / introduced | the page says local metadata runs on Entity Runtime 0.26.0, false once this pin ships; the release step's website update covers it |

```findings
[
  {"file": "crates/connectors-host/tests/fixtures/metadata-store-er-0.26.0/README.md", "line": 16, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "F1 No test checks the README's SHA-256 values, so regenerating the fixture with a newer build, which the README forbids, leaves the suite green."},
  {"file": "website/docs/introduction/status.md", "line": 21, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "F2 The status page says local metadata runs on Entity Runtime 0.26.0, which is false once the 0.29.0 pin ships; the release step's website update must change it."}
]
```

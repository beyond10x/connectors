---
format: aep.planning-md/3
id: review-result:security-metadata-replay-pass-1
kind: review-result
status: active
title: Security review of the metadata replay bound
relations:
- reviews: story:metadata-open-is-not-a-full-replay
revision: 1
---
unit: story:metadata-open-is-not-a-full-replay, uncommitted tree wave0929b-replay on 1d4df135b
verdict: INFEASIBLE (invariant 3 broken for file identity; invariants 1, 2, 4, 5, 6 hold)
cases: executed 270→272, red 0
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: scratch/security logs
needs-coordinator: keep the `#[cfg(test)] mod security_replay_tests;` line in local/mod.rs

Invariants: 1 integrity holds (cached reopen refuses whenever a fresh open refuses, every event); 2 lock holds with a
note (the pooled facade's SQLite handle outlives the lifecycle lock); 3 isolation broken for inode (path-only key);
4 catch-up equals replay holds; 5 secrets hold with a note (baseline rows and eventlog's verified-blob cache live
for the process); 6 -wal/-shm hold (0600, WAL crash-safe).

Cases in crates/connectors-host/src/local/security_replay_tests.rs:
a_held_authority_refuses_every_in_place_alteration_a_fresh_open_refuses,
an_owner_batch_over_a_subject_another_process_wrote_equals_a_fresh_replay (both green).

Coordinator routing: warning fixed with the (dev, ino) key; the note by rewriting the drop-order comment; the mod.rs
line kept.

```findings
[
  {"file": "crates/connectors-host/src/local/metadata/er.rs", "line": 121, "category": "integrity", "severity": "warning", "verdict": "INFEASIBLE", "origin": "introduced", "message": "The held-authority pool is keyed by path, authority, source level and process but not by file identity, so after the store is replaced at its path a cached facade writes into the moved-aside file; no workflow found that replaces a live store."},
  {"file": "crates/connectors-host/src/local/metadata.rs", "line": 180, "category": "judgement", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "The pooled ER facade's SQLite handle outlives _lifecycle_lock, so its last-close sidecar retirement can run unlocked on IDLE_LIMIT eviction, contrary to the drop-order comment; only reachable with more than four stores in one process."}
]
```

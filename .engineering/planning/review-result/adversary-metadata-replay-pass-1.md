---
format: aep.planning-md/3
id: review-result:adversary-metadata-replay-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the metadata replay bound
relations:
- reviews: story:metadata-open-is-not-a-full-replay
revision: 1
---
unit: story:metadata-open-is-not-a-full-replay, uncommitted tree wave0929b-replay on 1d4df135b
verdict: CONFIRMED (the one red case builds a state nobody was shown to reach: INFEASIBLE)
cases: executed 268→272, red 1
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: scratch/adversary logs
needs-coordinator: whether a replaced state directory under a running process is supported

Cases in crates/connectors-host/tests/metadata_reopen_adversary.rs: a_write_after_the_store_is_replaced_at_its_path_lands_in_the_new_store
(red: the write went to the moved-away store, left 33 right 27); a_reopen_refuses_every_altered_event_a_full_replay_refuses,
a_reopen_refuses_every_altered_blob_a_full_replay_refuses, concurrent_writers_leave_a_reopen_equal_to_a_full_replay (green).

Coordinator routing: finding 1 fixed by keying the pool on the file's (dev, ino); finding 2 by rewriting the comments.
Returned to the implementor.

```findings
[
  {"file": "crates/connectors-host/src/local/metadata/er.rs", "line": 497, "category": "concurrency", "severity": "warning", "verdict": "INFEASIBLE", "origin": "introduced", "message": "The pooled authority is matched by path only, so after the state directory is replaced the reopen writes to the moved-away store's open file instead of the store at the path; no workflow found that reaches it."},
  {"file": "crates/connectors-host/src/local/metadata.rs", "line": 919, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The lifecycle-lock comment still says no handle is kept across provider work and that sidecars are retired at last close under the lock, but the pool now keeps an Eventlog SQLite handle for the life of the process."}
]
```

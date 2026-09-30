---
format: aep.planning-md/3
id: review-result:adversary-failed-connect-pass-1
kind: review-result
status: active
title: Adversary pass 1 on failed connect reports its cause
relations:
- reviews: story:failed-connect-reports-its-cause
revision: 1
---
unit: story:failed-connect-reports-its-cause, uncommitted tree wave0930b-connect on e9af199d8
verdict: NEEDS-CHANGE
cases: executed 348→356, red 4
origin: introduced 1 / pre-existing 1 / undecided 0
wrote-outside-worktree: scratch/adversary (base snapshot and its build dir, suite.log)
needs-coordinator: whether the malformed-reply defect is fixed in this unit

Cases in apps/connectors/tests/failed_connect_adversary.rs against a stand-in owner socket: connect with a reply that
is valid JSON but not a reply, non-JSON bytes, a failed reply with an unknown field, revalidate with an unknown reply
kind (red; same at base); replies cut mid-frame and no reply until the deadline (green).
Could not break: a lost reply after publication, Reply::Failed parsing, repair on the same path, the acquisition rule,
status and CLI answer agreeing.

Coordinator routing: both fixed in this unit (every unreadable reply is outcome_unknown; owner-side a revalidate the
pool stopped waiting for answers outcome_unknown).

```findings
[
  {"file": "crates/connectors-host/src/local/owner/transport.rs", "line": 88, "category": "acceptance", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "pre-existing", "message": "A malformed owner reply reaches the caller as definite service_failure at dispatch/retry_explicitly instead of outcome_unknown, inviting a duplicate connect."},
  {"file": "crates/connectors-host/src/local/owner/supervisor.rs", "line": 267, "category": "concurrency", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "pool.run's recv_timeout/Disconnected on Task::Revalidate can fire after finish_revalidation committed, and the owner's Failed{Timeout|Unavailable} is now reported as a definite dispatch failure."}
]
```

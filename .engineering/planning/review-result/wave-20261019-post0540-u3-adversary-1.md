---
format: aep.planning-md/3
id: review-result:wave-20261019-post0540-u3-adversary-1
kind: review-result
status: active
title: 'U3 adversary pass 1: owner-request-waits-outside-the-pool-lock'
relations:
- reviews: story:owner-request-waits-outside-the-pool-lock
revision: 1
---
unit: story:owner-request-waits-outside-the-pool-lock (findings cover commit 8c54e452c plus this pass's uncommitted test files)
verdict: red
cases: executed 634→638, red 2
origin: introduced 1, pre-existing 1, undecided 0
wrote-outside-worktree: ~/.cache/connectors-wave-20261019b/u3/adversary-1/{suite.log, suite-all.log, suite-without-mine.log, tree/ (a copy of the commit with its own target/, 781M)}
needs-coordinator: yes. The 2 red cases are pre-existing and INFEASIBLE, so they should not send this unit back. Decide whether `adversary_lock_order_tests.rs` stays in the tree or goes to its own story.

The two red cases do not point at a defect in this unit's code. The one finding that is the unit's own is a test gap, and the green case now in the tree closes it. Nothing in the implementation needs to change.

**1. Diff stat**
```
 crates/connectors-host/src/local/owner/supervisor.rs | 5 +++++
 1 file changed, 5 insertions(+)
?? crates/connectors-host/src/local/owner/supervisor/   (adversary_tests.rs, adversary_lock_order_tests.rs)
```
The 5 lines in `supervisor.rs` are only `#[cfg(test)] mod adversary_lock_order_tests;` and `#[cfg(test)] mod adversary_tests;` at the end of the file, the only way a new file can reach the private `Pool` and `Worker` internals.

**2. Cases added (each run alone first)**

| Test | Asserts | Now |
|---|---|---|
| `adversary_tests::a_request_waiting_on_its_instance_is_sent_once_the_lock_is_released` | a request waiting on its instance's control lock is answered `Ok` within 250 ms of the lock being freed, and `queued` goes back to 0 | green |
| `adversary_tests::the_queued_count_is_returned_on_every_way_out_of_send` | `queued` goes back to 0 or stays exact on timeout, `Capacity`, `Unavailable` and stop | green |
| `adversary_lock_order_tests::quiet_waiting_on_a_held_control_lock_delays_no_other_instance` | while `quiet()` waits on an idle instance's held control lock, a request for another instance still answers by its 300 ms deadline | red |
| `adversary_lock_order_tests::working_waiting_on_a_held_control_lock_delays_no_other_instance` | the same, for `working()` | red |

Red output from the solo run:
```
panicked at .../adversary_lock_order_tests.rs:118:5:
assertion `left == right` failed: a request for another instance was still unanswered 1 s on, behind `quiet`
  left: None
 right: Some(None)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 570 filtered out; finished in 1.00s
```

- Mutant probe (scratch copy): `lock_until` calling `try_lock` once and then only sleeping until the deadline. Every request arriving during a write's commit times out; both of the unit's `pool_lock_tests` stay ok; `a_request_waiting_on_its_instance_is_sent_once_the_lock_is_released` goes red.
- Origin check: base `supervisor.rs` (52769afb3) with the lock-order file added is red the same way in both tests, so that finding is pre-existing.

**3. Suite**
- `cargo test --locked -j 8 -p connectors-host --no-fail-fast` → exit 101, executed 638 (636 passed, 2 failed, 34 ignored); the only failures are the 2 lock-order cases.
- The same with `-- --skip supervisor::adversary_` → exit 0, executed 634.
- clippy and fmt → exit 0.

**4. Findings**

| file:line | verdict | origin | measured | what reaches it |
|---|---|---|---|---|
| supervisor.rs:320 | CONFIRMED | introduced | The unit's tests never show a waiting request going ahead once the lock is free; the mutant passes both. | Every request that arrives during a write's native commit. Fix: keep `adversary_tests.rs`. |
| supervisor.rs:764, :789 | INFEASIBLE | pre-existing | `working()` and `quiet()` still lock each worker's control lock while holding `workers`; a control lock held on an idle worker stalls every other instance's `send`. | A long control hold only happens on a `busy` worker, and both functions skip a busy worker before they lock. Other holders hold it for microseconds. The idle loop calls `working()` every 10 ms only when there are no clients (transport.rs:1366). |

**5. Attacked and could not break:** deadline precision; `queued` on every exit path; stop() during a waiting send; epoch change between admit and lock; worker restarted while a request waits; shutdown while a request holds a cloned sender; custody `pool.run` from a worker thread (lock order cannot invert). Starvation and CPU from 5 ms polling: not measured.

```findings
[
  {"file": "crates/connectors-host/src/local/owner/supervisor.rs", "line": 320, "category": "mutant", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "the unit's tests stay green when lock_until never retries try_lock after the first WouldBlock (every request behind a write would time out); adversary_tests::a_request_waiting_on_its_instance_is_sent_once_the_lock_is_released catches it"},
  {"file": "crates/connectors-host/src/local/owner/supervisor.rs", "line": 764, "category": "concurrency", "severity": "note", "verdict": "INFEASIBLE", "origin": "pre-existing", "message": "working() and quiet() lock an idle worker's control lock while holding workers, so a held control lock stalls other instances' requests (adversary_lock_order_tests::quiet_waiting_on_a_held_control_lock_delays_no_other_instance, ::working_waiting_on_a_held_control_lock_delays_no_other_instance); no long hold on a non-busy worker was found"}
]
```

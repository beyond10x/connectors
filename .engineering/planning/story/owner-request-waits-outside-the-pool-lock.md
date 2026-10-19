---
format: aep.planning-md/3
id: story:owner-request-waits-outside-the-pool-lock
kind: story
status: draft
title: A request behind an instance's in-flight write never stalls another instance or overruns its own deadline
summary: Pool::send takes an instance's control lock while it holds the workers mutex
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: crates/connectors-host/src/local/owner/mutation/execution.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/supervisor.rs
revision: 3
---
## Outcome

A request for an instance whose write is in its native commit waits only on that instance and
answers `timeout` by its own deadline; a request for any other instance is not delayed by it.

## Why later

The review of release 0.54.0 (2026-10-19, lane r7) found that `Pool::send`
(`crates/connectors-host/src/local/owner/supervisor.rs:507-555`) checks the deadline, takes the
`workers` mutex, and then takes the instance's control lock while still holding `workers`. A write
holds that control lock through `native.commit()`
(`crates/connectors-host/src/local/owner/mutation/execution.rs:433-526`), for up to the write's
deadline (20 s, `mutation.rs:250`). A second request for that instance then answers after its own
deadline, and every other instance's `send`, `run`, `recover` and `shutdown` waits on `workers`.
The verifier reproduced it: with instance A's control lock held for 3 s, a request for instance B
took 3.39 s (`verify_r7_tests.rs:103` in the scratch tree, log `r7-verify-1.log`). The lock order is
the same at `c15004d81`, so it predates 0.54.0; no wrong write follows, because the worker rechecks
the deadline before dispatch (`supervisor.rs:894`). Changing the lock order of the owner's pool is
its own story.

## Acceptance

- A test holds instance A's control lock and shows a request for instance B answers in under
  100 ms, and a second request for A answers `timeout` by its own deadline.
- The pool clones the worker's sender, control and queue under `workers` and releases `workers`
  before it waits on the control lock; the deadline and `stopped` are re-checked after the wait.
- The existing supervisor, lease and revalidation-join suites pass unchanged.

---
format: aep.planning-md/3
id: story:journey-owners-are-stopped
kind: story
status: implemented
title: Journey tests stop the owners they start, and an owner never outlives its directory
relations:
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-17T02:29:03Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-17T02:29:04Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-17T02:29:04Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Observed
Every ignored CLI journey starts a `__connectors-owner` through the CLI
(`crates/connectors-host/src/local/owner/transport.rs:993`) and nothing stops it when the test
ends. The owners reparent to PID 1 and run until their idle retirement, or for ever when their
state directory was deleted. One development host held 146 such owners (143 orphaned, 121 older
than an hour, 771 MiB RSS). Issue: https://github.com/beyond10x/connectors/issues/175

## Acceptance
- After a journey test, pass or fail (panic included), no owner it started is alive: a failing
  test proves it first (it starts a journey that panics and finds its owner alive), then the
  harness stops the owner it started by PID and reaps it (a drop guard).
- An owner exits when its socket directory or state directory goes away, and a test owner exits
  when the process that started it is gone; each has a test that removes the directory or ends
  the parent and finds the owner gone within a bounded time.
- A production owner started by the CLI keeps its current lifetime (idle retirement), shown by
  the existing persistence journeys.
- Spec first: the owner's lifetime rules are stated in the owner's ESS/contract before the code.

---
format: aep.planning-md/3
id: story:owner-cancel-timing-test-under-load
kind: story
status: draft
title: The owner-cancel timing test passes on a loaded runner
relations:
- decomposes: epic:review-20261123-findings
revision: 1
---
## Observed

Pull request #276, gate round 2 (2026-11-23): `connectors::owner_cancel_adversary
the_call_after_a_cancelled_read_waits_no_longer_than_a_relaunch` failed at 2.96 s against a
2.19 s bound; it passed in round 1 at 2.89 s and on the rerun. The change did not touch that code.

## Acceptance

- The test asserts on events (the relaunch happened or not), not on elapsed time against a bound
  that a loaded runner crosses; ten runs under build-pool load pass.

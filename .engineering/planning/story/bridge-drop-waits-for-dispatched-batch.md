---
format: aep.planning-md/3
id: story:bridge-drop-waits-for-dispatched-batch
kind: story
status: draft
title: A refused metadata handle does not leave a batch running after its lock is released
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:metadata-invoke-cost-flat-in-store-size
revision: 1
---
## Observed (2026-10-02, wave 20261002a store-cost unit)

After a bridge call answers `OutcomeUnknown`, the dropped `RecordedEventlogBridge` detaches its worker (`Drop` does not
join), and the abandoned batch can still commit after the handle's lifecycle lock is released. With a host change that
skipped the reopen verification, the next invoke answered `MetadataUnavailable` in that window (reproduced with a
300 ms batch wait at 601 events). The unchanged host recovered, with one 70 s invoke; why is a hypothesis (it waits
behind that capture's `BEGIN IMMEDIATE`).

## Acceptance

- A refused handle stops or waits for its dispatched batch before its lifecycle lock is released (for example
  `shutdown(CancelQueued, wait)` on drop), shown by a test that times out a batch and then invokes again.

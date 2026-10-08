---
format: aep.planning-md/3
id: story:lift-expiry-batch-bound
kind: story
status: draft
title: Lift the 32-member expiry batch bound once Entity Runtime batches scale linearly
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

The expiry batch bound that story:metadata-invoke-cost-flat-in-store-size introduced (at most 32
`ExpireReadUse` members per batch) is raised or removed, because Entity Runtime executes a batch
in time linear in its members.

## Waits on

upstream-blocker:er-batch-cost-superlinear, which clears with the Entity Runtime release of
`story:batch-cost-grows-linearly-with-members` in https://github.com/beyond10x/entity-runtime.

## Acceptance

- The pin moves to that release (with or after story:entity-runtime-eventlog-081-pin).
- `first_owner_open_of_a_grown_store`, release build, with the bound removed: the catch-up batch
  of a 1,201-event store (396 members) and of a 6,000-event store finishes inside the 30 s bridge
  deadline; the times are recorded beside the 2026-10-08 numbers in the blocker.
- If it does not, the bound stays and the measured largest safe size replaces 32.

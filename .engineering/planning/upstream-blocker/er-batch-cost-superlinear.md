---
format: aep.planning-md/3
id: upstream-blocker:er-batch-cost-superlinear
kind: upstream-blocker
status: cleared
title: Entity Runtime batch execution time grows faster than its members (0.29.0)
relations:
- blocks: story:lift-expiry-batch-bound
revision: 4
transitions:
- {from: "open", to: "cleared", at: "2026-10-08T07:53:30Z", actor: "human:timo", revision: 4}
---
## What blocks

Entity Runtime 0.29.0 `RecordedProviderFacade::execute_batch` (entity-eventlog
`adapter.rs:1827`, `BatchReadStore::for_batch`, `Executor::batch`) takes time that grows faster
than the number of batch members. Measured 2026-10-08 on the release build of
`first_owner_open_of_a_grown_store` (`crates/connectors-host/src/local/registry/store_cost_tests.rs`),
batch deadline raised to 900 s:

| store | members | time |
|---|---|---|
| 601 events | 196 (195 `ExpireReadUse`, 1 `CaptureReadUse`) | 21.4–28.2 s |
| 1,201 events | 396 (395 `ExpireReadUse`, 1 `CaptureReadUse`) | 96.8–117.7 s |
| warm handle | 1–3 | 40–55 ms |

At the 30 s bridge deadline the batch answers `OutcomeUnknown`, and the next open waits on the
retiring handle's lock and answers `MetadataUnavailable`. That is what failed every invoke at
1,201 and 6,000 events, on 0.32.0 and on the checkpoint branch alike. Member count and store
size grew together in this harness; which drives the cost is not separated.

## Workaround in this repository

story:metadata-invoke-cost-flat-in-store-size applies expiries in bounded batches.

## Clears when

An Entity Runtime release executes a batch in time linear in its members, and connectors pins it;
the bound may then be raised or removed.

## Upstream

Entity Runtime tracks the fix as `story:batch-cost-grows-linearly-with-members` (draft) in
https://github.com/beyond10x/entity-runtime, after its 0.30.1 release (Eventlog 0.8.1,
https://github.com/beyond10x/entity-runtime/pull/71). story:lift-expiry-batch-bound lifts the
bound when that release is pinned.

## Cleared

Cleared by Entity Runtime 0.30.2 (released 2026-10-08, `story:batch-cost-grows-linearly-with-members`),
pinned by connectors 0.33.0. It holds for SQLite stores; the File and PostgreSQL providers remain
superlinear.

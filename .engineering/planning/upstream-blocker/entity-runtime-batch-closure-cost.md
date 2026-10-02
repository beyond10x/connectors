---
format: aep.planning-md/3
id: upstream-blocker:entity-runtime-batch-closure-cost
kind: upstream-blocker
status: open
title: Entity Runtime 0.25.1 batch execution and facade reads cost the whole store
refs:
- provider: github
  reference: beyond10x/entity-runtime#51
relations:
- blocks: story:metadata-invoke-cost-flat-in-store-size
revision: 1
---
Filed upstream: https://github.com/beyond10x/entity-runtime/issues/51 (2026-10-02, by the bot).

Measured by the wave 20261002a unit (release build): per-invoke time 578 ms at 55 events, 12.4 s at 601, 40.2 s at
1,203; `RecordedProviderFacade::execute_batch` is 62-87% of it, host-only work 0.4%. A batch on the registry clock
subject costs 62 ms at 55 events and 5.7 s at 1,203; a batch on a subject in no earlier batch stays 26-55 ms. The
scoped read follows every subject that shares a batch with a record read, and the registry clock is in every registry
batch. Cleared when an Entity Runtime release carries subject-bounded batch execution and subject-scoped facade
reads, and connectors pins it.

---
format: aep.planning-md/3
id: upstream-blocker:entity-runtime-open-verifies-whole-store
kind: upstream-blocker
status: open
title: Entity Runtime 0.26.0 verifies the whole store on every open
refs:
- provider: github
  reference: beyond10x/entity-runtime#55
relations:
- blocks: story:metadata-invoke-cost-flat-in-store-size
revision: 1
---
Filed upstream: https://github.com/beyond10x/entity-runtime/issues/55 (2026-10-06, by the bot).

Entity Runtime 0.26.0 (adopted in beyond10x/connectors#96) made batch execution and reads inside
one provider handle flat, but every open still verifies the complete store
(`RecordedProviderFacade::start_with_read_policy`: "opening always verifies the whole authority"),
and connectors' CLI opens the store in each short-lived command process.
`read_invoke_cost_by_store_size` (release build): `er.start` 75 / 492 / 963 ms per invoke at 55 /
601 / 1,203 events. On an operator store of 2,623 events (74 MB) `connections list` took 4.1 s and
`operations describe` 2.9 s (beyond10x/connectors#101).

Cleared when an Entity Runtime release opens a store without verifying its whole history (a
persisted verified checkpoint plus the appended suffix) and connectors can adopt it.

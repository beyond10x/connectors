---
format: aep.planning-md/3
id: upstream-blocker:er-model-record-copies
kind: upstream-blocker
status: open
title: Entity Runtime keeps each record three times with its own decoded definition (entity-runtime#59)
refs:
- provider: github
  reference: beyond10x/entity-runtime#59
relations:
- blocks: story:owner-memory-bounded
revision: 2
---
## What blocks

story:owner-memory-bounded acceptance 3 (peak RSS at 600 events at most 506 MB, at 1,200 at most
twice that) is below the live heap of the two verified Entity Runtime models the owner and the CLI
hold at the peak: 492 MB at 601 events (massif, 2026-10-06; profile on
https://github.com/beyond10x/connectors/issues/103#issuecomment-6020465666). Measured after the
connectors-side fix: 645 MB at 601 events, 1,371 MB at 1,201 (ratio 2.13).

## Clears when

An Entity Runtime release keeps one copy per committed record and one decoded definition per
distinct definition (https://github.com/beyond10x/entity-runtime/issues/59), or stops modelling the
whole store on every open (https://github.com/beyond10x/entity-runtime/issues/55), and connectors
pins it; then the story re-measures with `read_invoke_cost_by_store_size`.

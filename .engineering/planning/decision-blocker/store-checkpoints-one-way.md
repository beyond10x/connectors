---
format: aep.planning-md/3
id: decision-blocker:store-checkpoints-one-way
kind: decision-blocker
status: open
title: Nobody has decided whether the host enables one-way store checkpoints by itself
relations:
- blocks: story:metadata-invoke-cost-flat-in-store-size
revision: 1
---
## The question

`story:metadata-invoke-cost-flat-in-store-size` (#101) needs Entity Runtime 0.29.0's durable open
checkpoints: a tracked open then starts from a persisted checkpoint instead of verifying the
whole history. Enabling them on a store is one-way: it installs Eventlog's triggers and
continuity tables, and Entity Runtime 0.28.0 and earlier, so every connectors release up to and
including 0.32.0, refuse to open a store that carries them. An operator who upgrades and then
rolls back can no longer open the store with the older binary.

## Options

| option | what happens | cost |
|---|---|---|
| A | the host enables checkpoints on every store when it first opens it | every operator gets #101's fix at once; a store opened once by the new release cannot be opened by 0.32.0 or earlier |
| B | an explicit command enables them on one store; nothing changes until it is run | rollback stays possible until the operator chooses; #101 stays unfixed for anyone who does not run it |
| C | new stores are created with checkpoints; an existing store is enabled by the explicit command | new installs are fast with no action; existing stores keep their rollback until the operator chooses |

## What would clear it

A decision naming A, B or C, which the story's acceptance and CHANGELOG then state.

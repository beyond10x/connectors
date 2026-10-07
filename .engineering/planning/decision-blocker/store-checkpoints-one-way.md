---
format: aep.planning-md/3
id: decision-blocker:store-checkpoints-one-way
kind: decision-blocker
status: cleared
title: Nobody has decided whether the host enables one-way store checkpoints by itself
relations:
- blocks: story:metadata-invoke-cost-flat-in-store-size
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-07T17:15:32Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"approval":1}}}
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

## Decision — 2026-10-07

Decided 2026-10-07: option C. New stores are created with durable open checkpoints; an existing
store is enabled only by an explicit command its owner runs. Option A was refused because it
would change an existing user's store irreversibly without an act of its owner; B would leave
new installs without the fix.

Conditions the story carries:

1. The explicit command states that the change is one-way (connectors 0.32.0 and earlier can no
   longer open the store) and refuses to run without a confirming flag.
2. The release notes name it under Breaking or Migration.
3. Both paths are tested: a new store created with checkpoints, and an existing store enabled
   by the command.
4. A long-running consumer instance that started before this change is not enabled before
   2026-10-14.

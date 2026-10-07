---
format: aep.planning-md/3
id: specification:wave-20261007b-entity-runtime-029
kind: specification
status: approved
title: 'Wave 20261007b: Entity Runtime 0.29.0'
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:entity-runtime-029-pin
revision: 3
transitions:
- {from: "draft", to: "in_review", at: "2026-10-07T08:02:01Z", actor: "human:timo", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-07T08:02:01Z", actor: "human:timo", revision: 3}
---
## Wave 20261007b: Entity Runtime 0.29.0

Opened 2026-10-07, `aep:implementing` 0.20.1 wave mode, on the operator's standing approval of
waves (2026-10-05) and the delivery shape set on 2026-10-07: one integration branch, one pull
request.

## Integration

| | |
|---|---|
| integration branch | `wave/20261007b`, stacked on `wave/20261007a` (`07886629c`), which waits on a Gates fix before it can be pushed |
| integration tree | managed `conn-w20261007b` |
| pull request | one, after `wave/20261007a`'s pull request merges, so the two land in order |

## Units

| unit | story | serves | branch | worktree | stage |
|---|---|---|---|---|---|
| U1 | `story:entity-runtime-029-pin` | `vision:independent-contract-adapters` | `unit/entity-runtime-029-pin` | `conn-u-er029` | planned |

N is one: the stories the new pin unblocks (`story:metadata-invoke-cost-flat-in-store-size`,
#101, which enables durable open checkpoints, one-way; `story:owner-memory-bounded`, #103) go
into the wave after release 0.32.0.

Dispatch: `aep:implementor`, then one `aep:adversary` pass (a storage dependency).

## Pre-flight

| check | value |
|---|---|
| free disk on `/` | 38,205 MiB (`df -m /`, 2026-10-07); a full gate needs a build slot below 30 GiB |
| one measured build | 8,139 MiB (`gate --msrv`, one connectors tree) |
| units at once | 1; the unit tree's cache is discarded before the integration gate |

## Commits this approval covers

The opening planning commit; one unit commit through `b10x-gates bot`; its merge into
`wave/20261007b`; the closing planning commit; the one pull request into `main` and its merge.
Not a release.

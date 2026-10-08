---
format: aep.planning-md/3
id: specification:wave-20261007c-store-cost-memory
kind: specification
status: approved
title: 'Wave 20261007c: store cost (#101) and owner memory (#103)'
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:owner-memory-bounded
- informed_by: story:metadata-invoke-cost-flat-in-store-size
revision: 4
transitions:
- {from: "draft", to: "in_review", at: "2026-10-07T17:16:09Z", actor: "human:timo", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-07T17:16:10Z", actor: "human:timo", revision: 3}
---
## Wave 20261007c: store cost (#101) and owner memory (#103)

Opened 2026-10-07, `aep:implementing` 0.20.1 wave mode, on the operator's standing approval of
waves and the delivery shape set on 2026-10-07: one integration branch, one pull request.

## Integration

| | |
|---|---|
| integration branch | `wave/20261007c` from `main` at `e1a377261a` (release 0.32.0) |
| integration tree | managed `conn-w20261007c` |
| pull request | one, after the full gate on the integration branch |

## Units

They were planned one after the other: both stories' `## Scope` name
`crates/connectors-host/src/local/metadata/er.rs` and
`crates/connectors-host/src/local/registry/store_cost_tests.rs`.

Resumed 2026-10-08 by a new coordinator session. Entity Runtime 0.29.0 (in `main` since 0.32.0)
carries the upstream fixes U1 waited on (entity-runtime#59 in 0.28.0, entity-runtime#55 in
0.29.0), so U1 starts with a measurement on the wave base. U2 is dispatched at the same time;
U1 changes, if any, are expected in the peak-RSS reporting of `store_cost_tests.rs`, and the
coordinator resolves any overlap at merge. U2 also moves the ESS pin from 0.55.0 to 0.56.0
(newest release), because its specification is validated and regenerated with the newest `ess`.

| unit | story | issue | branch | worktree | build dir | scratch | stage |
|---|---|---|---|---|---|---|---|
| U1 | `story:owner-memory-bounded` | #103 | `unit/owner-memory-bounded-20261007` | managed `conn-u1-mem` | `conn-u1-mem/target` | `conn-u1-mem/.local` | baseline measurement on `91e6baae33` |
| U2 | `story:metadata-invoke-cost-flat-in-store-size` | #101 | `unit/store-checkpoints` (head `b0f9d7632a`, specification) | managed `conn-u2-ckpt` | `conn-u2-ckpt/target` | `conn-u2-ckpt/.local` | implementor dispatched (`aep:implementor`) |

Briefs: `conn-w20261007c/.local/wave/<unit>/brief.md` (git-ignored).

U2 follows `decision-blocker:store-checkpoints-one-way` (cleared, option C) and
`decision-blocker:checkpoint-offline-edit-detection` (cleared, option B).

Dispatch: `aep:implementor` per unit; `aep:adversary` per unit (storage and memory changes).

## Pre-flight

| check | value |
|---|---|
| free disk on `/` | 17,210 MiB (2026-10-07T17:15Z); builds wait for a build slot below 30 GiB |
| one measured build | release build of `connectors-host` tests, estimated 8–10 GB |
| units at once | 1 build at a time |

## Commits this approval covers

The planning commits on `wave/20261007c`; one or more commits per unit through `b10x-gates bot`;
their merges into `wave/20261007c`; the closing planning commit; one pull request into `main`
and its merge. Not a release.

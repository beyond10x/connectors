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
revision: 3
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

They run one after the other: both change `crates/connectors-host/src/local/metadata/er.rs` and
`crates/connectors-host/src/local/registry/store_cost_tests.rs` (both stories' `## Scope`).

| unit | story | issue | branch | worktree | stage |
|---|---|---|---|---|---|
| U1 | `story:owner-memory-bounded` | #103 | `unit/owner-memory-bounded-20261007` | `conn-u1-mem` | brief written; release build waits for a build slot |
| U2 | `story:metadata-invoke-cost-flat-in-store-size` | #101 | `unit/store-checkpoints` | `conn-u2-ckpt` | specification being drafted (no build); implementation after U1 merges |

U2 follows `decision-blocker:store-checkpoints-one-way` (cleared): new stores with checkpoints, an
existing store by an explicit, confirmed command.

Dispatch: `ess:author` for U2's specification; `aep:implementor` per unit; `aep:adversary` per
unit (storage and memory changes).

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

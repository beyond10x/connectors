---
format: aep.planning-md/3
id: specification:wave-20261007c-store-cost-memory
kind: specification
status: implemented
title: 'Wave 20261007c: store cost (#101) and owner memory (#103)'
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:owner-memory-bounded
- informed_by: story:metadata-invoke-cost-flat-in-store-size
- informed_by: story:entity-runtime-eventlog-081-pin
revision: 7
transitions:
- {from: "draft", to: "in_review", at: "2026-10-07T17:16:09Z", actor: "human:timo", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-07T17:16:10Z", actor: "human:timo", revision: 3}
- {from: "approved", to: "implemented", at: "2026-10-08T06:30:15Z", actor: "human:timo", revision: 7}
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

## U3: Entity Runtime 0.30.1

Added 2026-10-08: `story:entity-runtime-eventlog-081-pin` (#101 and #103 context), Entity Runtime
0.29.0 → 0.30.1 and Eventlog 0.8.0 → 0.8.1. It changes `Cargo.toml` and `Cargo.lock`, which U2
also changes (ESS 0.56.0), so it runs on the integration branch after U2 merges, in
`conn-w20261007c`, by the coordinator. Acceptance as in the story; its cost measurement runs on
the same machine as U2's.

## Results

Release build of `read_invoke_cost_by_store_size` (one machine, 2026-10-08), stores grown by read
invokes. Logs: `conn-w20261007c/.local/wave/{u1,u2,u3}` (git-ignored).

| run | events | invokes failed | median | peak RSS |
|---|---|---|---|---|
| base `91e6baae33` (0.32.0, Entity Runtime 0.29.0) | 601 | 0 of 5 | 1,211 ms | 377 MB |
| base | 1,201 | 3 of 5 (`OutcomeUnknown`, then `MetadataUnavailable`) | 24,711 ms | 608 MB |
| U2 `a6a59980e4` (Entity Runtime 0.29.0) | 601 | 0 of 5 | 229 ms | 329 MB |
| U2 | 1,201 | 0 of 5 | 341 ms | 560 MB |
| U2 | 6,000 | 0 of 5 | 255 ms | 2,314 MB |
| U2, 700 invokes | 601 | 0 of 700 | 278 ms | 387 MB |
| U3 `4f712a6e73` (Entity Runtime 0.30.1) | 601 | 0 of 5 | 127 ms | 329 MB |
| U3 | 1,201 | 0 of 5 | 128 ms | 560 MB |

- #101: median at 6,000 events is 1.11x the median at 600 (limit 2x); 700 invokes with no
  `timeout` and no `outcome_unknown`.
- #103: peak RSS 329 MB at 601 events (limit 506) and 560 MB at 1,201 (limit 2x 329 = 658).
- U2 took three correction rounds: the base failure at 1,201 events was one 396-member expiry
  batch inside Entity Runtime (`upstream-blocker:er-batch-cost-superlinear`); CLI reads now go
  through the owner; adversary pass 1 (`review-result:adversary-store-checkpoints-pass-1`).
- U1 changed no code: the targets hold on the base after Entity Runtime 0.28.0's record fix.
- Trees: `conn-u1-mem` and `conn-u2-ckpt` finished and removed by `worktree gc` (archives under
  the worktree archive directory).
- Package tests on the integration branch at `4f712a6e73`: connectors-host 396 passed, 0 failed,
  33 ignored; connectors 122 passed, 0 failed, 6 ignored; clippy `-D warnings`, fmt,
  `connectors-build metadata-entities --check` and `cli --check` clean. The pull request's CI
  is the full gate.

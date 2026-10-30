---
format: aep.planning-md/3
id: task:gate-pins-current-aep
kind: task
status: draft
title: The repository gate pins the current AEP release
relations:
- informed_by: story:connectors-do-short-invocation
revision: 2
---
## Problem

`connectors-build gate` requires AEP 0.71.3 and refuses any other `aep`. This machine's `aep` is
now 0.78.0, so on 2026-10-30 the D218 gate ran only with `CONNECTORS_AEP` pointed at a
downloaded 0.71.3.

## Outcome

The gate pins the current AEP release, and that release reads this store.

## Acceptance

- `cargo run --locked -p connectors-build -- gate` finds the pinned release on a machine where the
  current AEP is installed, without `CONNECTORS_AEP`.
- The store validates under that release.

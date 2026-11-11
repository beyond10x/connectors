---
format: aep.planning-md/3
id: story:gate-entity-runtime-metadata-drift
kind: story
status: archived
title: Gate refuses Entity Runtime metadata drift from ess/domains/cli.yaml
tags:
- from-finding
revision: 2
transitions:
- {from: "draft", to: "archived", at: "2026-11-11T08:55:50Z", actor: "human:timo", revision: 2}
---
## Finding

At 47166ae5d8 (on `origin/wave/zero-issues-b`, not on `main`) the repository gate
`cargo run --locked -p connectors-build -- gate` exits 1 at its first step:
`Error: "local metadata Entity Runtime definition drift"`. `ess/domains/cli.yaml` changed without
regenerating the Entity Runtime definitions, the operation types and the `cli.yaml` citations; only
`source_digest` and `synthesis_digest` differ. Nine `connectors-build` metadata tests fail on the
same drift (`metadata_conformance`, `cli_spec_mapping_adversary`, the fixture adversary and the
oauth2 adversary).

Measured by the Grafana catalog wave (session 2c881569), which changed nothing under `ess/`.

## Outcome

The committed generated metadata matches `ess/domains/cli.yaml` and the gate's first step passes.

## Acceptance

- `cargo run --locked -p connectors-build -- gate` passes its metadata step on the branch that
  carries the change.
- `cargo test -p connectors-build --tests` passes.

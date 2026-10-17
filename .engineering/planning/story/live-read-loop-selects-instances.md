---
format: aep.planning-md/3
id: story:live-read-loop-selects-instances
kind: story
status: draft
title: The live read loop selects with connector and instance
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Context

The live read loop (`crates/connectors-build/src/live_read_loop.rs:60`) reads a file whose entries
select what to read through a field named `adapter`, which it passes to the CLI as `--adapter`.
That flag is deprecated by epic:connector-vocabulary, so every live read loop run prints a
deprecation line, and `docs/development.md` has to write the file through `jq` because a JSON
line with that key fails the vocabulary check. Found by the repository-notes unit on 2026-10-17.

## Change

- The read-loop file selects with `connector` and `instance` and passes `--connector` and
  `--instance`; a file that still has `adapter` keeps working and is reported as deprecated.
- `docs/development.md` shows the file as JSON again.

## Scope

- `crates/connectors-build/src/live_read_loop.rs`
- `docs/development.md`

## Acceptance

A read-loop run over a file using `connector` and `instance` prints no deprecation line, and one
over a file using `adapter` prints exactly one.

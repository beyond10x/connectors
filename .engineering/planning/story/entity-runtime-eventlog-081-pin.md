---
format: aep.planning-md/3
id: story:entity-runtime-eventlog-081-pin
kind: story
status: draft
title: Entity Runtime on Eventlog 0.8.1
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

Connectors builds on the Entity Runtime release that builds against Eventlog 0.8.1, so one
Eventlog (0.8.1) is linked. Eventlog 0.8.1 (released 2026-10-08) finds a stream's head, window
and receipts through a per-stream index instead of reading every event, and stops replay from
rescanning the store per event (https://github.com/beyond10x/eventlog/issues/42).

## Waits on

The Entity Runtime story `entity-runtime-builds-against-eventlog-0-8-1` in
https://github.com/beyond10x/entity-runtime, and that release's tag on GitHub. The pin does not
move before the tag exists.

## Work

- Spec first: the pin is not a model change. The ESS specification is validated with the newest
  `ess`, and generated outputs that depend on the runtime are regenerated through the
  repository's tasks; none is edited by hand.
- `Cargo.toml`: the four `entity-*` crates from tag 0.29.0 to the new release; `eventlog-core`
  from tag 0.8.0 to 0.8.1; `Cargo.lock` follows.
- Adapt any call site the release changed.
- `docs/development.md` names the new pins; `CHANGELOG.md` Unreleased records the move.

## Acceptance

- The package tests of `connectors-host` and `connectors` pass on the new pins, and the pull
  request's full gate passes.
- A store written by the previous pin opens and reads under the new one
  (`crates/connectors-host/tests/metadata_store_previous_pin.rs`).
- `read_invoke_cost_by_store_size`, release build, at 600 and 1,200 events: medians recorded
  beside the previous pin's on the same machine.

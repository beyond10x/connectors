---
format: aep.planning-md/3
id: story:entity-runtime-eventlog-081-pin
kind: story
status: implemented
title: Entity Runtime on Eventlog 0.8.1
relations:
- serves: vision:independent-contract-adapters
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T04:08:46Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-08T04:08:46Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-08T06:29:55Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Outcome

Connectors builds on Entity Runtime 0.30.1, the release that builds against Eventlog 0.8.1, so one
Eventlog (0.8.1) is linked. Eventlog 0.8.1 (released 2026-10-08) finds a stream's head, window
and receipts through a per-stream index instead of reading every event, and stops replay from
rescanning the store per event (https://github.com/beyond10x/eventlog/issues/42).

## Waits on

The Entity Runtime story `entity-runtime-builds-against-eventlog-0-8-1` in
https://github.com/beyond10x/entity-runtime, released as 0.30.1 on 2026-10-08 (https://github.com/beyond10x/entity-runtime/releases/tag/0.30.1);
upstream-blocker:entity-runtime-on-eventlog-081 is cleared.

## Work

- Spec first: the pin is not a model change. The ESS specification is validated with the newest
  `ess`, and generated outputs that depend on the runtime are regenerated through the
  repository's tasks; none is edited by hand.
- `Cargo.toml`: the four `entity-*` crates from tag 0.29.0 to 0.30.1; `eventlog-core`
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

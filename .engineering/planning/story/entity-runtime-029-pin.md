---
format: aep.planning-md/3
id: story:entity-runtime-029-pin
kind: story
status: implemented
title: Entity Runtime 0.29.0 and Eventlog 0.8.0
relations:
- serves: vision:independent-contract-adapters
- informed_by: upstream-blocker:er-model-record-copies
- informed_by: upstream-blocker:entity-runtime-open-verifies-whole-store
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: inferred
  path: crates/connectors-host/src/local
- confidence: inferred
  path: crates/connectors-host/src/local/metadata/entity-runtime-definitions.json
- confidence: cited
  path: docs/development.md
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-10-07T08:01:40Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-07T08:01:40Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-07T11:03:14Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Outcome

Connectors builds on Entity Runtime 0.29.0 and Eventlog 0.8.0, the releases that close the two
upstream defects the store-cost stories wait on: a verified model holds each record once
(entity-runtime#59, shipped in 0.28.0) and a tracked SQLite open can start from a persisted
checkpoint instead of verifying the whole history (entity-runtime#55, 0.29.0).

## Work

- Spec first: the pin is not a model change. The ESS specification is validated with the
  newest `ess`, and generated outputs that depend on the runtime are regenerated through the
  repository's tasks (`connectors-build metadata-entities`, `cli`, `docs`); none is edited by hand.
- `Cargo.toml`: the four `entity-*` crates from tag 0.26.0 to 0.29.0; `eventlog-core` from rev
  `6983cc25` to Eventlog's `0.8.0` tag, so one Eventlog is linked; `Cargo.lock` follows.
- Adapt the call sites the releases changed, if any (0.28.0 shares one copy of each record
  across indexes; 0.29.0 adds `BridgeOperationIdentity::Administration` and checkpoint APIs).
- Do not enable durable open checkpoints: enabling is one-way for Entity Runtime 0.28.0 and
  earlier and belongs to `story:metadata-invoke-cost-flat-in-store-size`.
- `docs/development.md` names the new pins; `CHANGELOG.md` Unreleased records the move and
  that stores written by this version stay readable by 0.31.0 because no checkpoint is enabled.

## Acceptance

- The full gate with `--msrv` passes on the integration branch with the new pins.
- A store written by the previous pin opens and reads under the new one (an existing
  long-lived fixture or a test that writes with the old format is enough; say which).
- `cargo tree` shows one `eventlog-core` and one `entity-core` for the runtime, besides the
  `entity-core` that `ess-entity-runtime` brings.
- `upstream-blocker:er-model-record-copies` and
  `upstream-blocker:entity-runtime-open-verifies-whole-store` are cleared by the coordinator
  once this lands.

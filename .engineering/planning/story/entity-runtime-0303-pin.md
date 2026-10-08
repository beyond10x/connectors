---
format: aep.planning-md/3
id: story:entity-runtime-0303-pin
kind: story
status: implemented
title: Entity Runtime 0.30.3 and Eventlog 0.8.3
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: crates/connectors-host/src/local/metadata/er.rs
- confidence: cited
  path: docs/development.md
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T18:06:44Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-08T18:06:44Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-08T18:46:55Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

Connectors builds on Entity Runtime 0.30.3, which depends on Eventlog 0.8.3. With the `file`
provider a recorded batch now costs time linear in its members (the handle hashes each batch blob
once instead of once per member read); PostgreSQL handles make the same change. No API or on-disk
format change (https://github.com/beyond10x/entity-runtime/releases/tag/0.30.3).

## Work

- Spec first: the pin is not a model change. The ESS specification is validated with the newest
  `ess`, and generated outputs that depend on the runtime are regenerated through the
  repository's tasks; none is edited by hand.
- `Cargo.toml`: the four `entity-*` crates from tag 0.30.2 to 0.30.3; `eventlog-core` from tag
  0.8.1 to 0.8.3; `Cargo.lock` follows, with one Eventlog linked.
- The comment above `EXPIRY_BATCH` in `crates/connectors-host/src/local/metadata/er.rs` names
  0.30.3; the bound itself stays 128 (every store this host opens is SQLite).
- `docs/development.md` names the new pins; `CHANGELOG.md` Unreleased records the move.

## Acceptance

- The package tests of `connectors-host` and `connectors` pass on the new pins, and the pull
  request's full gate passes.
- A store written by the previous pin opens and reads under the new one
  (`crates/connectors-host/tests/metadata_store_previous_pin.rs`, fixture re-recorded from 0.30.2
  only if the test needs a new one).
- `cargo tree -i eventlog-core` shows exactly one version, 0.8.3.

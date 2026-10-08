---
format: aep.planning-md/3
id: story:aep-pin-0690
kind: story
status: active
title: Plan with AEP 0.69.0
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: crates/connectors-build/aep-toolchain.json
- confidence: cited
  path: crates/connectors-build/src/aep_toolchain.rs
- confidence: cited
  path: docs/development.md
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T07:53:31Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-08T07:53:31Z", actor: "human:timo", revision: 3}
---
## Outcome

The repository plans with the newest AEP release, 0.69.0, so `connectors-build gate` accepts the `aep`
that `b10x upgrade` installs.

## Source

`crates/connectors-build/aep-toolchain.json` pins 0.68.0; the machine `aep` is 0.69.0 (released
2026-10-07 22:42Z). The gate refuses it and tells the operator to run `b10x upgrade`, which installs
0.69.0 again, so two sessions downloaded 0.68.0 by hand. The store validates unchanged under 0.69.0.

## Acceptance

- The pin is 0.69.0; the planning workflow and the Rust gate install it; the development guide names it.
- `aep plan artifact validate` with 0.69.0 answers `valid` and changes no file.
- The refusal for a mismatched `aep` names the pinned version and the exact command that installs it.

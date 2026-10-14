---
format: aep.planning-md/3
id: story:aep-pin-0713
kind: story
status: draft
title: Plan with AEP 0.71.3
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: .engineering/project.yaml
- confidence: cited
  path: .github/workflows/planning.yml
- confidence: cited
  path: .github/workflows/rust-gate.yml
- confidence: inferred
  path: CHANGELOG.md
- confidence: cited
  path: crates/connectors-build/aep-toolchain.json
- confidence: cited
  path: crates/connectors-build/src/aep_toolchain.rs
- confidence: cited
  path: crates/connectors-build/src/gate.rs
- confidence: cited
  path: docs/development.md
revision: 3
---
## Outcome

The repository plans and validates with the newest AEP release: `crates/connectors-build/aep-toolchain.json` names it, the planning store validates under it, and the gate resolves it.

## Today

The pin is 0.69.1 (`crates/connectors-build/aep-toolchain.json`); the newest release on the machine is 0.71.3 (`gh release list -R beyond10x/aep`, 2026-10-14; the machine runs 0.71.2).

## Acceptance

- `aep plan artifact validate` under the new release reports the store valid, and any migration the release asks for is applied through its own command.
- `connectors-build ignored`/gate tests that read the pin pass; `docs/development.md` names the new version.

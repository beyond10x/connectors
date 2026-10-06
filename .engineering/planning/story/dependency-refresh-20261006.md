---
format: aep.planning-md/3
id: story:dependency-refresh-20261006
kind: story
status: implemented
title: 'Dependencies at their newest releases: crates.io, ESS 0.53.0, AEP 0.68.0'
relations:
- decomposes: epic:connector-probe-20261006
- supersedes: story:toolchain-pins-newest-release-20261001
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: .engineering/project.yaml
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: inferred
  path: apps/connectors-cli-contract
- confidence: inferred
  path: apps/connectors/tests/cli_surface_minor.rs
- confidence: cited
  path: crates/connectors-build/aep-toolchain.json
- confidence: inferred
  path: crates/connectors-host/src/local/metadata/entity-runtime-definitions.json
- confidence: inferred
  path: crates/connectors-spec/tests/adversary_ess_limit_notes.rs
- confidence: cited
  path: crates/connectors-spec/toolchain.json
- confidence: inferred
  path: docs/development.md
- confidence: inferred
  path: ess/domains
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-06T09:42:56Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-10-06T09:42:56Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-10-06T10:22:13Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Defect

On 2026-10-06 the gate requires ESS 0.52.0 (`crates/connectors-spec/toolchain.json`) and AEP
0.65.0 (`crates/connectors-build/aep-toolchain.json`) while ESS 0.53.0 and AEP 0.68.0 are released
(`gh release list`), and `b10x upgrade` installs the newest (operator rule 2026-10-05: always the
most recent ESS). `cargo update --dry-run` lists 18 compatible crates.io updates (among them tokio
1.53.1 -> 1.53.2, uuid 1.26.1 -> 1.27.0, jsonschema 0.58.2 -> 0.58.5); `cargo outdated -R -w`
reports no direct dependency behind a major version. Entity Runtime 0.26.0 is its newest release
and eventlog stays at the revision 0.26.0 builds on (`6983cc2`).

None of these is expected to change the store growth of #101: no released Entity Runtime opens a
store without verifying its whole history (beyond10x/entity-runtime#55).

## Acceptance

- `crates/connectors-spec/toolchain.json` names ESS 0.53.0, the ESS crates in `Cargo.toml` are at
  tag 0.53.0, and generated outputs and the metadata conformance suite are regenerated with it.
- `crates/connectors-build/aep-toolchain.json` names AEP 0.68.0 and the planning store validates
  under it.
- `Cargo.lock` carries the compatible crates.io updates (`cargo update`), and
  `cargo update --dry-run` lists none afterwards except ones refused with a stated reason.
- The gate with `--msrv` passes.

## Scope

Derived 2026-10-06 by `aep:story-scoper`. **Cited** = read from the story or the tree; **inferred** =
a reading that could be wrong (the fan-out is read from the ESS 0.52.0 bump `9cc4613`).

- **Files:** `Cargo.toml:25-31` (the seven `ess-*` crates at tag `0.52.0`), `Cargo.lock`,
  `crates/connectors-spec/toolchain.json`, `crates/connectors-build/aep-toolchain.json` — cited
- **Also likely:** `apps/connectors-cli-contract` (generated CLI, `connectors-build cli`),
  `crates/connectors-host/src/local/metadata/entity-runtime-definitions.json`, `ess/domains`,
  `crates/connectors-spec/tests/adversary_ess_limit_notes.rs`, `apps/connectors/tests/cli_surface_minor.rs`,
  `.engineering/project.yaml` (`protocols` pin `aep#665cd6e` = 0.65.0), `docs/development.md:57-63`
  ("Pinned tools", already stale on Entity Runtime 0.25.1) — inferred
- **Confidence:** high for the four pinned files; medium for the regenerated fan-out
- **Would collide with:** every unit that touches `Cargo.toml` or `Cargo.lock`, the generated CLI
  contract, `ess/` or the metadata definitions — schedule it alone or first
- **Safety fact:** ESS 0.53.0 pins `entity-core` 0.24.1 as 0.52.0 does, so the bump adds no second
  Entity Runtime core beside 0.26.0; CI reads both pins from the JSON files
  (`.github/workflows/rust-gate.yml:36,47`, `planning.yml:24`) — unproven

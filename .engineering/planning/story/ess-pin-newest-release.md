---
format: aep.planning-md/3
id: story:ess-pin-newest-release
kind: story
status: active
title: Move the ESS pin to the newest release
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: apps/connectors-cli-contract
- confidence: inferred
  path: crates/connectors-build/src/metadata_conformance.rs
- confidence: inferred
  path: crates/connectors-build/src/metadata_entities.rs
- confidence: cited
  path: crates/connectors-host/src/local/metadata/entity-runtime-definitions.json
- confidence: cited
  path: crates/connectors-spec/src/toolchain.rs
- confidence: cited
  path: crates/connectors-spec/toolchain.json
- confidence: cited
  path: docs/development.md
- confidence: inferred
  path: ess/domains/artifact_provenance.yaml
- confidence: inferred
  path: ess/domains/auth_bindings.yaml
- confidence: cited
  path: website/docs/introduction/status.md
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T21:31:58Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-09-29T21:31:58Z", actor: "human:timo", revision: 6}
---
## Problem

The repository pins ESS 0.40.0 (`crates/connectors-spec/toolchain.json`), and `ess` on PATH is 0.42.0 since 2026-09-29, observed by the owner-build-handshake implementor. The gate refuses a mismatched version, so a machine that stays current cannot run the gate without `CONNECTORS_ESS` pointing at an older release.

## Acceptance

- The ESS pin and the `ess-*` crates move to the newest ESS release; generated outputs are regenerated; the full gate passes with the plugin-installed `ess` on PATH.

## Decided for the wave (coordinator, 2026-09-29)

- Target ESS 0.44.0, the newest release (`ess --version`; GitHub release 0.44.0 published
  2026-09-29T18:24Z), not 0.42.0.
- Move `crates/connectors-spec/toolchain.json`, the seven `ess-*` git dependencies in `Cargo.toml` and
  `Cargo.lock` together; regenerate `entity-runtime-definitions.json` and the CLI contract and commit
  whatever they produce; update the ESS-LIMIT notes that name 0.40 only if 0.44 changes what they say
  (the CLI-type invariant refusal is unchanged in 0.44.0).
- The gate runs with `CONNECTORS_ESS` pointing at the 0.44.0 toolchain; the metadata conformance suite
  must stay all passed, 0 unsupported. Any new synthesis refusal or failing scenario is reported, not
  hidden.

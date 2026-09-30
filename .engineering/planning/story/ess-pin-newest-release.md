---
format: aep.planning-md/3
id: story:ess-pin-newest-release
kind: story
status: implemented
title: Move the ESS pin to the newest release
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: crates/connectors-build/src/metadata_conformance.rs
- confidence: cited
  path: crates/connectors-host/src/local/metadata/er.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/tests.rs
- confidence: cited
  path: crates/connectors-spec/src/toolchain.rs
- confidence: cited
  path: crates/connectors-spec/tests/adversary_ess_limit_notes.rs
- confidence: cited
  path: crates/connectors-spec/toolchain.json
- confidence: cited
  path: docs/development.md
- confidence: cited
  path: ess/domains/approval_issuers.yaml
- confidence: cited
  path: ess/domains/cli.yaml
- confidence: cited
  path: ess/domains/delegation.yaml
- confidence: cited
  path: ess/domains/execution_audit.yaml
- confidence: cited
  path: ess/domains/local_approval_policy.yaml
- confidence: cited
  path: ess/domains/mutations.yaml
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T21:31:58Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-09-29T21:31:58Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-09-30T00:49:52Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
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

---
format: aep.planning-md/3
id: story:ignored-suites-have-a-runner
kind: story
status: active
title: The 35 ignored suites have a command that runs them
relations:
- decomposes: epic:tech-debt-review-20260930
- serves: vision:independent-contract-adapters
- informed_by: specification:milestone-acceleration-20261002
scope:
- confidence: inferred
  path: crates/connectors-build/src/ignored.rs
- confidence: cited
  path: crates/connectors-build/src/main.rs
- confidence: cited
  path: docs/development.md
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-10-02T11:48:33Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-02T11:48:33Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"review_outcome":1}}}
---
## Problem
The 2026-09-30 review counted 35 ignored tests; that count is historical and newer store-cost probes are ignored too. Discover the current inventory. gate.rs has no classified ignored-suite runner.

## Acceptance

An ignored-suite-runner conformance result passes only when all mandatory inventory, execution, prerequisite, failure-propagation and helper-exclusion cases below pass.

## Boundary
Implement a Rust clap-derived connectors-build command. Classify disposable local custody/CLI fixtures separately from live-provider tests, timing probes and subprocess helpers. Default to disposable fixtures; live sandboxes and costly probes require explicit selection. Credential presence never grants authority. Unknown classifications fail inventory validation.
Report exact selected/executed/skipped/failed names and counts. A required release family refuses missing prerequisites rather than counting skips as acceptance. Existing gate behavior stays unchanged until explicit enrollment is reviewed. Retain one operator-host run and its missing prerequisites.

## Scope
Cited: crates/connectors-build/src/main.rs, docs/development.md. Inferred new module: crates/connectors-build/src/ignored.rs. Do not edit gate.rs, workflow YAML or inventoried tests.
Coordinate the inventory with the SQL fixture unit without editing its file. Coordinator owns store writes, evidence and release metadata. No runtime entity is introduced.

## Required conformance cases

The named tooling conformance cases ignored-inventory-accounted, missing-prerequisite-is-explicit, selected-test-failure-is-nonzero and subprocess-helper-never-top-level account for every current ignored test, execute selected available suites, name missing prerequisites, fail on selected-test failures and never invoke subprocess-only helpers at top level.

---
format: aep.planning-md/3
id: story:spec-models-operation-admission
kind: story
status: draft
title: 'The ESS spec models operation admission: absent, not granted, stale description'
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: apps/connectors-cli-contract
- confidence: inferred
  path: crates/connectors-build/src/metadata_conformance.rs
- confidence: inferred
  path: crates/connectors-build/src/metadata_entities.rs
- confidence: cited
  path: crates/connectors-host/src/local/metadata/entity-runtime-definitions.json
- confidence: inferred
  path: crates/connectors-host/src/local/metadata/er.rs
- confidence: cited
  path: ess/domains/declarations.yaml
revision: 3
---
## Source

ESS design review of the CLI domain (2026-09-30): the admission refusals for an operation exist only as
`FailureCode` variants (`ess/domains/cli.yaml:244-246`), so conformance cannot catch the 0.18.0 defect where an absent
operation answered `forbidden`/`stale_description`. A probe at `ess/15` under ESS 0.45 validated and synthesized all
four branches (absent → `not_found`, not granted → `forbidden`, revision mismatch → `stale_description`, else
admitted), with `unknown_instance` and `when_subject` guards.

## Acceptance

- `ess/domains/declarations.yaml` (`OperationDeclaration`) declares the admission command with the four outcomes in
  that precedence; `ess specify validate` passes and synthesis yields their scenarios.
- The metadata conformance target runs those scenarios against the host and they pass after
  story:absent-operation-reports-not-found; each one fails on the 0.18.0 behaviour (shown by a planted defect).
- `entity-runtime-definitions.json` and the CLI contract regenerate.

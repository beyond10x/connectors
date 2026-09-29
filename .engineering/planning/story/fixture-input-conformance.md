---
format: aep.planning-md/3
id: story:fixture-input-conformance
kind: story
status: active
title: Run the metadata authority suite with fixture inputs in the gate
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: cited
  path: crates/connectors-build/src/gate.rs
- confidence: cited
  path: crates/connectors-build/src/metadata_conformance.rs
- confidence: cited
  path: crates/connectors-host/src/local/metadata/entity-runtime-definitions.json
- confidence: cited
  path: ess/domains/credential_evidence.yaml
- confidence: cited
  path: ess/domains/delegation.yaml
- confidence: cited
  path: ess/domains/execution_audit.yaml
- confidence: cited
  path: ess/domains/idempotency.yaml
- confidence: cited
  path: ess/domains/mutations.yaml
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T11:07:12Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-09-29T11:07:12Z", actor: "human:timo", revision: 5}
---
## Scope
Integrate the fixture-input work retained in managed worktree wt-d90bbee1cfa0 (patch ~/.cache/claude-tmp/fixA.BmI5/fixture-inputs-on-integration.patch): fixture_inputs on PrepareAttempt, ReserveKey, RecordApprovalRedemption, AcknowledgeAnchor; the runner's fixture provider; view fields killing 5 survivors. Restore the gate step running `connectors-build metadata-conformance`.

## Acceptance
- Baseline 285/285 passed, 0 unsupported; mutation audit 176 killed, 0 survived (measured in the worktree).
- Decide whether a kernel invariant violation counts as failed rather than unsupported.

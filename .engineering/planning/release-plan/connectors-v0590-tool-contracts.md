---
format: aep.planning-md/3
id: release-plan:connectors-v0590-tool-contracts
kind: release-plan
status: active
title: 'Release 0.59.0: tool contracts for admitted operations (ADR 0103)'
relations:
- serves: vision:independent-contract-adapters
revision: 2
transitions:
- {from: "draft", to: "active", at: "2026-10-26T16:50:10Z", actor: "human:timo", revision: 2, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

Minor release 0.59.0 of wave 20261026-tool-contracts (`approval-record:wave-20261026-tool-contracts`)
and the "Text file busy" fix of the list tests (#208):
- Connectors publishes a `tool-contract/1` document per instance for its admitted operations
  (contract `tools/v1alpha1`, ADR 0103), with profiles authored beside the adapter and every
  refusal named.
- The Tavily adapter's contract is generated, drift-checked by the gate and held to a
  `tool-conformance/1` suite through the adapter's own invoke.
- Model domain `connectors.tool_contracts` with authored scenarios.

Minor rather than patch: a new crate and command, a new contract and a new model domain. Nothing a
consumer already uses changes; the `connectors` CLI is unchanged apart from its version.

Approved by the directing session `b10x-chef-orchestrator` under its grant (dispatch-queue row 12,
2026-10-26): implement one wave of ADR 0103 in connectors and "release per AGENTS.md if the release
process allows".

## Scope

| story | status |
|---|---|
| story:tool-contract-model | implemented |
| story:connector-projection | implemented |
| story:tool-conformance-over-invocation | implemented |
| story:list-tests-copied-cli-busy | implemented (#208) |

`story:tool-contracts-for-catalog-connectors` and `story:tool-contract-serving` stay draft and are
not in this release.

## Limitations

These are the limitations the CHANGELOG 0.59.0 section states:
- Only the Tavily adapter publishes a contract.
- No tool host is served the contract yet; the conformance suite calls the adapter directly, not
  the owner's governed invocation.

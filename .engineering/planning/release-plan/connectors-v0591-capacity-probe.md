---
format: aep.planning-md/3
id: release-plan:connectors-v0591-capacity-probe
kind: release-plan
status: active
title: 'Release 0.59.1: the owner-capacity test fix and the ESS 0.75.0 pin'
relations:
- serves: vision:independent-contract-adapters
revision: 2
transitions:
- {from: "draft", to: "active", at: "2026-10-29T14:54:51Z", actor: "human:timo", revision: 2, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

Patch release 0.59.1 of wave D210 (`approval-record:wave-d210-capacity-probe`):
- The two owner-capacity tests of the local CLI hold the owner's startup channel while they fill its 32 client slots, so they no longer fail on a loaded or single-CPU host (`story:local-cli-capacity-probe-flake`).
- The ESS 0.75.0 pin, already on `main` since #216, is released. The pin is not moved (D198).

Patch rather than minor: the CLI, the owner, the contracts and the models are unchanged apart from the version.

Approved by the directing session `b10x-chef-orchestrator` under its grant, item D210 (2026-10-29): "a connectors wave for story:local-cli-capacity-probe-flake and its release".

## Scope

| story | status |
|---|---|
| story:local-cli-capacity-probe-flake | implemented |

## Limitations

None beyond those of 0.59.0.

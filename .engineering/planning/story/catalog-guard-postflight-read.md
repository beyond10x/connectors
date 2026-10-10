---
format: aep.planning-md/3
id: story:catalog-guard-postflight-read
kind: story
status: implemented
title: A catalog guard proves a write that answers without a body by a read after it
relations:
- serves: vision:independent-contract-adapters
- decomposes: epic:fluxplane-plugin-parity
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-10T02:00:36Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-10T02:00:36Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-10T03:47:22Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

A catalog guard can prove a write whose answer carries no body. A postflight may declare a read
the engine issues after the write is dispatched, with values taken from the write's inputs; its
checks then read that answer instead of the write's own.

## Why

Jira `doTransition` answers 204 with no content. A postflight today compares only the write's own
answer (`adapters/catalog/src/lib.rs`, `Prepared::execute`), so a guarded transition either
reports the effect as unknown on every success or proves nothing. `story:parity-jira-transitions`
shipped the `issue.transitions` read and left the run unselected for this reason.

A second gap for the same operation: a guard has one preflight read, and a check cannot select an
array element by a member's value, so one guard cannot check both the issue's status (`getIssue`)
and that the transition is available (`getTransitions?transitionId=`).

## Acceptance

- Spec first: the postflight read is modelled in the catalog ESS model and the selection format,
  validated with the newest `ess`, before the engine changes.
- A guarded `doTransition` selection answers through `connectors operations invoke` against a
  fixture: the preflight refuses a wrong current status before the POST, and the postflight read
  proves the target status, or reports the effect as unknown by name when the re-read disagrees.
- A write whose answer has a body keeps its current postflight behaviour, byte for byte.
- `jira.issue.transition.run` moves to covered on the parity page.

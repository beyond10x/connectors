---
format: aep.planning-md/3
id: story:parity-jira-transitions
kind: story
status: implemented
title: Jira transition list and run
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-09T22:50:36Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-09T22:50:37Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-10T03:47:23Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

Jira transition list and run — parity unit U07 of `docs/fluxplane-plugin-parity.md`.

## Operations

`jira.issue.transition.list`, `jira.issue.transition.run`

242 calls since 2026-09-09 (declared and mapped undeclared names), provider `jira`, planned wave W3.

## Surface

catalog `operations.json` selection: `getTransitions`, `doTransition`; a run by name or a walk to a status is composition outside the catalog.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives (the parity page names the gap per operation).
- The parity page row of each operation moves to covered, with the Connectors operation named.

## Wave 20261010a (2026-10-10)

- Delivered: `issue.transitions` (`getTransitions`), a read; `jira.issue.transition.list` covered.
- Not delivered: `doTransition`. It answers 204 with no body, and a catalog postflight reads only the write's own answer; a guard that also checks availability needs a second preflight read. Waits for `story:catalog-guard-postflight-read`.
- The story stays active until the run is selected.

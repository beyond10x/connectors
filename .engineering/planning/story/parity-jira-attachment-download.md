---
format: aep.planning-md/3
id: story:parity-jira-attachment-download
kind: story
status: implemented
title: Jira attachment download
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
- depends_on: story:catalog-binary-responses
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-10T14:23:55Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-10T14:23:55Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-10T16:33:07Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Outcome

Jira attachment download — parity unit U23 of `docs/fluxplane-plugin-parity.md`.

## Operations

`jira.issue.attachment.get`

2 calls since 2026-09-09 (declared and mapped undeclared names), provider `jira`, planned wave W5.

## Surface

catalog engine change: binary responses.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives (the parity page names the gap per operation).
- The parity page row of each operation moves to covered, with the Connectors operation named.

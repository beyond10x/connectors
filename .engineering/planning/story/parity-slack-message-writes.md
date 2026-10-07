---
format: aep.planning-md/3
id: story:parity-slack-message-writes
kind: story
status: draft
title: Slack message send, edit and delete
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
- depends_on: story:catalog-slack-reads
revision: 1
---
## Outcome

Slack message send, edit and delete — parity unit U08 of `docs/fluxplane-plugin-parity.md`.

## Operations

`slack.message.send`, `slack.message.edit`, `slack.message.delete`

162 calls since 2026-09-09 (declared and mapped undeclared names), provider `slack`, planned wave W2.

## Surface

the Slack catalog provider.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives (the parity page names the gap per operation).
- The parity page row of each operation moves to covered, with the Connectors operation named.

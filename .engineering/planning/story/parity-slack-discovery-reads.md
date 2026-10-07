---
format: aep.planning-md/3
id: story:parity-slack-discovery-reads
kind: story
status: draft
title: Slack search, channels, users and workspace info
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
- depends_on: story:catalog-slack-reads
revision: 1
---
## Outcome

Slack search, channels, users and workspace info — parity unit U02 of `docs/fluxplane-plugin-parity.md`.

## Operations

`slack.search`, `slack.user.list`, `slack.channel.list`, `slack.info`, `slack.test`, `slack.emoji.list` (`slack.thread` and `slack.message.list` are `story:catalog-slack-reads`)

512 calls since 2026-09-09 (declared and mapped undeclared names), provider `slack`, planned wave W1.

## Surface

catalog provider after `story:catalog-slack-reads` (threads and history); a user-token profile for search.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives (the parity page names the gap per operation).
- The parity page row of each operation moves to covered, with the Connectors operation named.

## Note

The 512 calls include the 2 operations `story:catalog-slack-reads` delivers.

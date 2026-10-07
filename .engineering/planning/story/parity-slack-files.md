---
format: aep.planning-md/3
id: story:parity-slack-files
kind: story
status: draft
title: Slack file list, info, download, upload and delete
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
- depends_on: story:catalog-slack-reads
revision: 1
---
## Outcome

Slack file list, info, download, upload and delete — parity unit U13 of `docs/fluxplane-plugin-parity.md`.

## Operations

`slack.file.upload`, `slack.file.download`, `slack.file.delete`, `slack.file.info`, `slack.file.list`

52 calls since 2026-09-09 (declared and mapped undeclared names), provider `slack`, planned wave W4.

## Surface

catalog engine change: binary download and a multi-step external upload.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives (the parity page names the gap per operation).
- The parity page row of each operation moves to covered, with the Connectors operation named.

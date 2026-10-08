---
format: aep.planning-md/3
id: story:parity-opsgenie-alerts
kind: story
status: draft
title: Opsgenie alerts and on-call
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

Opsgenie alerts and on-call reads and alert actions, so Connectors covers the fluxplane `opsgenie` plugin (`docs/fluxplane-plugin-parity.md`, section `opsgenie`).

## Operations

`opsgenie.alert.list`, `opsgenie.alert.get`, `opsgenie.alert.ack`, `opsgenie.alert.close`, `opsgenie.alert.note`, `opsgenie.oncall`, `opsgenie.schedule.list`, `opsgenie.test`

0 calls in Claude Code and Codex session transcripts from 2026-09-09 to 2026-10-08 (recount of
2026-10-08: `fluxplane-plugin operation invoke|call opsgenie …` sites in Bash tool calls, each tool call
counted once).

## Surface

A new catalog provider over Opsgenie's published OpenAPI document, if one can be pinned (not checked); otherwise a native adapter. Writes (ack, close, note) need guards and an approval.

## Scheduling

Ordered last by call count. Not placed in a wave until a session uses the plugin or the operator
asks for it.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives.
- The parity page row of each operation moves to covered, with the Connectors operation named.

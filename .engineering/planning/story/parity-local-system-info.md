---
format: aep.planning-md/3
id: story:parity-local-system-info
kind: story
status: draft
title: Local system information
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

Local system information, so Connectors covers the fluxplane `system` plugin (`docs/fluxplane-plugin-parity.md`, section `system`).

## Operations

`system.info`

0 calls in Claude Code and Codex session transcripts from 2026-09-09 to 2026-10-08 (recount of
2026-10-08: `fluxplane-plugin operation invoke|call system …` sites in Bash tool calls, each tool call
counted once).

## Surface

A local operation with no provider. May be served by the agent harness instead; that decision comes before modelling.

## Scheduling

Ordered last by call count. Not placed in a wave until a session uses the plugin or the operator
asks for it.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives.
- The parity page row of each operation moves to covered, with the Connectors operation named.

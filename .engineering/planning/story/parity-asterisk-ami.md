---
format: aep.planning-md/3
id: story:parity-asterisk-ami
kind: story
status: draft
title: Asterisk Manager Interface reads and call control
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

Asterisk Manager Interface reads and call control, so Connectors covers the fluxplane `asterisk` plugin (`docs/fluxplane-plugin-parity.md`, section `asterisk`).

## Operations

`asterisk.ami.ping`, `asterisk.channel.list`, `asterisk.peer.list`, `asterisk.devicestate.list`, `asterisk.queue.status`, `asterisk.command`, `asterisk.call.originate`, `asterisk.channel.hangup`

0 calls in Claude Code and Codex session transcripts from 2026-09-09 to 2026-10-08 (recount of
2026-10-08: `fluxplane-plugin operation invoke|call asterisk …` sites in Bash tool calls, each tool call
counted once).

## Surface

A new native adapter over the AMI TCP protocol (no HTTP document). `asterisk.command` runs arbitrary CLI commands and needs a disclosure and approval decision before it is modelled.

## Scheduling

Ordered last by call count. Not placed in a wave until a session uses the plugin or the operator
asks for it.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives.
- The parity page row of each operation moves to covered, with the Connectors operation named.

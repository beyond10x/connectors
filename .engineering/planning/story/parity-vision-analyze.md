---
format: aep.planning-md/3
id: story:parity-vision-analyze
kind: story
status: draft
title: Image analysis through a vision provider
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

Image analysis through a configured vision provider, so Connectors covers the fluxplane `vision` plugin (`docs/fluxplane-plugin-parity.md`, section `vision`).

## Operations

`vision.analyze`, `vision.provider.list`

0 calls in Claude Code and Codex session transcripts from 2026-09-09 to 2026-10-08 (recount of
2026-10-08: `fluxplane-plugin operation invoke|call vision …` sites in Bash tool calls, each tool call
counted once).

## Surface

Composition over provider operations (for example `openai.vision.analyze`), as `websearch.search` is over Tavily; depends on a provider that offers image analysis.

## Scheduling

Ordered last by call count. Not placed in a wave until a session uses the plugin or the operator
asks for it.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives.
- The parity page row of each operation moves to covered, with the Connectors operation named.

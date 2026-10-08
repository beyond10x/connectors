---
format: aep.planning-md/3
id: story:parity-duckduckgo-search
kind: story
status: draft
title: DuckDuckGo web search
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

Web search through DuckDuckGo, so Connectors covers the fluxplane `duckduckgo` plugin (`docs/fluxplane-plugin-parity.md`, section `duckduckgo`).

## Operations

`duckduckgo.search`

0 calls in Claude Code and Codex session transcripts from 2026-09-09 to 2026-10-08 (recount of
2026-10-08: `fluxplane-plugin operation invoke|call duckduckgo …` sites in Bash tool calls, each tool call
counted once).

## Surface

A second `websearch` family adapter beside Tavily. DuckDuckGo publishes no search API document; the route (HTML endpoint or Instant Answer API) is decided before modelling.

## Scheduling

Ordered last by call count. Not placed in a wave until a session uses the plugin or the operator
asks for it.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives.
- The parity page row of each operation moves to covered, with the Connectors operation named.

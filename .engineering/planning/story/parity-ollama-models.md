---
format: aep.planning-md/3
id: story:parity-ollama-models
kind: story
status: draft
title: Ollama models and inference
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

Ollama model listing, inspection and inference, so Connectors covers the fluxplane `ollama` plugin (`docs/fluxplane-plugin-parity.md`, section `ollama`).

## Operations

`ollama.info`, `ollama.ps`, `ollama.model.list`, `ollama.model.show`, `ollama.chat`, `ollama.generate`, `ollama.embed`

0 calls in Claude Code and Codex session transcripts from 2026-09-09 to 2026-10-08 (recount of
2026-10-08: `fluxplane-plugin operation invoke|call ollama …` sites in Bash tool calls, each tool call
counted once).

## Surface

A catalog provider if an Ollama API document can be pinned (not checked); otherwise a native adapter over the local HTTP API.

## Scheduling

Ordered last by call count. Not placed in a wave until a session uses the plugin or the operator
asks for it.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives.
- The parity page row of each operation moves to covered, with the Connectors operation named.

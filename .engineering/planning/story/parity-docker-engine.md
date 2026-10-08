---
format: aep.planning-md/3
id: story:parity-docker-engine
kind: story
status: draft
title: Docker Engine containers, images, networks and volumes
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

Docker Engine containers, images, networks, volumes and system operations, so Connectors covers the fluxplane `docker` plugin (`docs/fluxplane-plugin-parity.md`, section `docker`).

## Operations

44 operations: `docker.container.*` (17), `docker.image.*` (10), `docker.network.*` (6), `docker.volume.*` (5), `docker.context.*` (2), `docker.build_cache.prune`, `docker.events`, `docker.info`, `docker.system.df`, `docker.system.prune` (full list in the parity page)

0 calls in Claude Code and Codex session transcripts from 2026-09-09 to 2026-10-08 (recount of
2026-10-08: `fluxplane-plugin operation invoke|call docker …` sites in Bash tool calls, each tool call
counted once).

## Surface

A catalog provider over the Docker Engine API OpenAPI document on the local socket, or a native adapter; `adapters/docker` holds a design and an ESS model. Needs a Unix-socket transport. Prune, remove, exec and push need guards and an approval.

## Scheduling

Ordered last by call count. Not placed in a wave until a session uses the plugin or the operator
asks for it.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives.
- The parity page row of each operation moves to covered, with the Connectors operation named.

---
id: S-055
title: "A self-service scope subset needs no operator"
pillar: Platform
status: ready
priority: 3
design: ../design/14-mcp-transport-for-the-hosted-connectors-server.md
epic: mcp-entry
areas: [identity]
---

# A self-service scope subset needs no operator

## Goal

This story's change lands in the separate beyond10x/identity repository (from `a664b99b`); it
is tracked here so the board shows the end-to-end dependency. Identity's
`admitted_connector_scope` compares the whole canonical scope string against four single-scope
literals, so any multi-scope request demands the `operator` group — an `sre`/`dev` principal
cannot hold `connectors.catalog.read connectors.invoke` in one token, making the MCP endpoint
operator-only in practice. Admit any subset of the four self-service scopes
(`connectors.catalog.read`, `connectors.connections.self`, `connectors.events.self`,
`connectors.invoke`) for any authenticated principal; any other member in the set still
demands `operator`.

## Acceptance

- In beyond10x/identity: a member principal mints `connectors.catalog.read connectors.invoke`
  (and the full self-service four); `connectors.catalog.read connectors.events.read` still
  demands `operator`; operator behaviour is unchanged — all proven in the vocabulary test.
- Identity's `bash scripts/gate.sh` exits 0; the identity commit is referenced here on close.

## Progress

- 2026-08-24 — filed from design 14 (scope-subset gap found during MCP exploration).

---
id: S-055
title: "A self-service scope subset needs no operator"
pillar: Platform
status: done
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
- 2026-08-24 — landed in beyond10x/identity as `f8c02a1`, merged to identity main `9732617`
  after independent review (PASS, 0 blocking, 1 minor: assertions exercise the policy fn per the
  acceptance's own wording; handler-level /v1/access-token coverage is a pre-existing gap).

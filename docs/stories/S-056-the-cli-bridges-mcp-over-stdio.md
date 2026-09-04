---
id: S-056
title: "The CLI bridges MCP over stdio"
pillar: Platform
status: done
design: ../design/14-mcp-transport-for-the-hosted-connectors-server.md
epic: mcp-entry
areas: [config, integrations]
---

# The CLI bridges MCP over stdio

## Goal

Interactive MCP clients hold a static bearer, but identity access tokens live 300 seconds, so
the hosted `/mcp` endpoint alone serves scripted callers well and interactive ones poorly.
Add a `connectors mcp` subcommand: a stdio MCP server that proxies to the hosted `/mcp`
endpoint with automatic per-scope token refresh from the login session established by
`connectors login`. The behaviour belongs in a `connectors-client` module; the subcommand itself
stays a thin frontend.

## Acceptance

- A stdio MCP client completes initialize/tools-list/tools-call against a hosted deployment
  through the bridge, across a token expiry, without manual re-auth.
- The thin-CLI architecture fence stays green with a dated, bounded cap raise covering parser and
  dispatch declarations only.

## Progress

- 2026-08-24 — filed from design 14, deliberately deferred behind S-053/S-054/S-055.
- 2026-09-02 — `connectors mcp` now proxies bounded newline-delimited MCP frames over stdio while
  keeping stdout protocol-pure. A controlled Rust integration crosses initialize/list/invoke,
  verifies least-privilege catalog versus invoke tokens, advances through the refresh margin and
  proves that neither the Identity session nor access tokens reach the MCP caller.

## Superseded by

`story:the-cli-bridges-mcp-over-stdio` in the AEP planning store, at
`.engineering/planning/story/the-cli-bridges-mcp-over-stdio.md`, now carries this story. Migrated 2026-09-04 by the
`aep-planning:story-migration` skill; this file is kept as the source it was migrated from and is
not the place the rung is recorded any more. The artifact quotes this file's `## Acceptance`
verbatim and names it in its own `## Provenance`.

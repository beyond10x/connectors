---
id: S-056
title: "The CLI bridges MCP over stdio"
pillar: Platform
status: backlog
design: ../design/14-mcp-transport-for-the-hosted-connectors-server.md
epic: mcp-entry
areas: [config, integrations]
---

# The CLI bridges MCP over stdio

## Goal

Interactive MCP clients hold a static bearer, but identity access tokens live 300 seconds, so
the hosted `/mcp` endpoint alone serves scripted callers well and interactive ones poorly.
Add a `connectors mcp` subcommand: a stdio MCP server that proxies to the hosted `/mcp`
endpoint with automatic identity login and per-scope token refresh. The behaviour belongs in
a `connectors-client` module (the thin-CLI fence caps the binary at 856 production lines and
pins its dependency list); the subcommand itself stays a thin frontend.

## Acceptance

- A stdio MCP client completes initialize/tools-list/tools-call against a hosted deployment
  through the bridge, across a token expiry, without manual re-auth.
- The thin-CLI architecture fence stays green without a cap raise beyond the reviewed
  convention.

## Progress

- 2026-08-24 — filed from design 14, deliberately deferred behind S-053/S-054/S-055.

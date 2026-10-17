---
title: MCP
sidebar_position: 21
description: MCP has specified contracts and a selected command spelling, and no runtime yet; nothing on this page can be configured today.
lede: MCP is planned, not shipped. Each step below says what is selected and what must land before it runs.
source: docs/local-mcp-cli.md, apps/connectors/spec/compatibility.json, adapters/mcp/contracts/server/v1alpha1/semantics.md, adapters/mcp/contracts/client/v1alpha1/auth.md
connectors: "mcp"
---

# MCP

:::caution[Planned]

MCP does not run. The CLI has no `server` command, no MCP instance configuration, no MCP auth
profile and no MCP operation binding, and the compatibility metadata says `mcp: deferred`. This
page keeps the onboarding steps in the same order as every other connector's so that each says
what exists and what does not.

:::

The words connector, instance, connection and operation are those of the
[README vocabulary](https://github.com/beyond10x/connectors#vocabulary); the
[MCP guide](https://github.com/beyond10x/connectors/blob/main/docs/local-mcp-cli.md) records the
selected intent in full.

## What it reads and writes

Nothing yet. Once a binding exists, an MCP server's tools, resources and prompts are to be
projected as operations; mutation advertisement stays withheld until the mutation and replay
binding, with its approval source and business key, exists.

## Prerequisites at the provider

None can be stated: which provider connection an inbound MCP caller may use, and whether
Connectors spawns an MCP server as a child process, are both undecided.

## Enable the connector and add an instance

Not possible yet. An MCP instance would take the pinned form `[connectors.mcp.instances.<instance>]`
of the other connectors served by their own executables, but none can be configured; do not add
one to a working configuration.

## Connect

Not available. The generic `connections connect` grammar exists, but no MCP auth profile, browser
OAuth or refresh for an MCP server is implemented.

## A first read

Not available. The selected inbound launch, for a local MCP client that starts Connectors as its
child, is spelled:

```text
connectors --config CONFIG --state-dir STATE_DIR server --transport stdio
```

It is not implemented; do not substitute `serve`, which starts the federation service.

## A first guarded write

Not available: no MCP operation, read or write, is bound.

## Reconnect when the connection is Stale

Not available. The requirements are specified: a reconnect must keep the identity, and an
uncertain rotating refresh cannot be exchanged again.

## Limits

- No runtime, no configuration schema, no launch handler and no conformance scenarios.
- Two decision blockers withhold review of the MCP contracts; the
  [README](https://github.com/beyond10x/connectors#where-the-project-stands) names them.

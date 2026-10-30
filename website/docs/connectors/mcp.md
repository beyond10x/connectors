---
title: MCP
sidebar_position: 21
description: MCP as a connector has specified contracts and no runtime yet; the local MCP server offers admitted read-only tools on Linux.
lede: MCP is planned, not shipped. Each step below says what is selected and what must land before it runs.
source: docs/local-mcp-cli.md, adapters/mcp/contracts/server/v1alpha1/semantics.md, adapters/mcp/contracts/client/v1alpha1/auth.md
connectors: "mcp"
---

# MCP

:::caution[Planned]

MCP as a connector does not run. The CLI has no MCP instance configuration, no MCP auth profile
and no MCP operation binding, and the compatibility metadata says `mcp: deferred`. Its local MCP
server, `server --transport stdio`, is described under [A first read](#a-first-read). This
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

None can be stated until a binding exists. The two questions that withheld it are decided: an
inbound MCP caller is the configured owner, one principal, so no caller-to-connection assignment
is modelled; and Connectors owns one pinned, supervised outbound stdio server process per
session. Neither decision is implemented.

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

On Linux it runs a bounded slice: admitted read-only operations of configured instances as tools,
with no resources, prompts, mutations or outbound MCP
([stdio guide](https://github.com/beyond10x/connectors/blob/main/docs/local-mcp-stdio.md)). On
macOS and Windows it exits with `mcp_unavailable`. Reading from an MCP server as a connector is
not implemented.

## A first guarded write

Not available: no MCP operation, read or write, is bound.

## Reconnect when the connection is Stale

Not available. The requirements are specified: a reconnect must keep the identity, and an
uncertain rotating refresh cannot be exchanged again.

## Limits

- No runtime, no configuration schema, no launch handler and no conformance scenarios.
- The two decision blockers that withheld review of the MCP contracts, on caller-to-connection
  assignment and on outbound stdio process ownership, are cleared in the planning store
  (2026-10-06 and 2026-10-03); the contracts epic is still a draft.

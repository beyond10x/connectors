---
format: aep.planning-md/3
id: decision-blocker:mcp-cloud-placement-selection
kind: decision-blocker
status: open
title: Nobody has selected a multi-caller cloud placement for the inbound MCP server
relations:
- blocks: story:mcp-cloud-caller-isolation
revision: 1
---
## The question

Is `$BIN server` to be offered in a placement where more than one MCP caller reaches
one Connectors instance? Nobody has selected one. The local placement has one
principal, the owner UID, and `epic:mcp-contracts` says cloud serving is optional.

## What it stops

`story:mcp-cloud-caller-isolation`: the caller-to-connection assignment and its
isolation fixture only mean something once a multi-caller placement exists.

## What would clear it

A decision that selects a multi-caller cloud placement, naming who the callers are
and where caller identity comes from. Nothing waits on it for the local placement.

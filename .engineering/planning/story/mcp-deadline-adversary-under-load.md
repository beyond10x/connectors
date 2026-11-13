---
format: aep.planning-md/3
id: story:mcp-deadline-adversary-under-load
kind: story
status: draft
title: The MCP deadline adversary does not fail under build-pool load
summary: adversary_mcp_deadline failed twice under load with the server ended before its first ping
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
# The MCP deadline adversary does not fail under build-pool load

## Need

On 2026-11-13 a full `cargo test -p connectors` run under heavy build-pool load exited 101 in
`apps/connectors/tests/adversary_mcp_deadline.rs`: two cases failed with
`assertion ... failed: the server ended` at line 435, the MCP server ending before it answered its
first ping. The binary alone passed (3 passed) and a full rerun was green.

## Acceptance

- The cause is found (a startup deadline shorter than a loaded machine needs, or a real early exit).
- The cases pass under the same load, or the deadline they hold is stated and justified.

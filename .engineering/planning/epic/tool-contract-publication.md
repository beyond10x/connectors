---
format: aep.planning-md/3
id: epic:tool-contract-publication
kind: epic
status: active
title: Connectors publishes tool-contract/1 documents for its admitted operations
summary: 'ADR 0103: a tool contract per admitted operation with the profile authored beside the adapter, held to tool-conformance/1'
refs:
- provider: atlas
  reference: adr-0103
relations:
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-26T15:33:28Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-26T15:33:32Z", actor: "human:timo", revision: 3}
---
## Outcome

For each admitted operation Connectors publishes a `tool-contract/1` document with the operation's
full profile (effects, consequences, risk, idempotency, determinism, bounds, subjects, placement,
capability, output), and the implementation behind it passes `tool-conformance/1`. A host offers
connector operations as tools from that document instead of compiling them itself. Credentials,
connection choice, grants and approval evidence stay in Connectors' governed invocation; a published
contract names none of them.

## Decision this carries out

Atlas ADR 0103, *Connectors implements the tool contract; tools does not read connectors' descriptors*
(accepted by the operator 2026-10-21, "approve adr 0103"), decisions 1, 2, 4 and 5:

1. Connectors publishes the contract; the profile is authored in Connectors beside the adapter that
   knows what the operation does. A provider profile name such as `tavily/2026-10` is not a profile.
2. A tool contract names no credential and no connection secret.
4. Connectors depends on `b10x-tool-contract` (tools, Apache-2.0) by git pin on a tools tag; the
   tools workspace sets `publish = false`.
5. `story:connector-projection` moves here from tools. What it projected on the host side (the profile,
   the input fields the connection fills, its `omit`) moves into Connectors' operation definition.
   Which operations a run sees stays the harness's grant.

Decision 6 (`tool-meta` stays in tools) and 7 (devcenter maps the profile onto MCP) are other
repositories' plans.

## Stories

| story | what | wave |
|---|---|---|
| `story:tool-contract-model` | ESS domain `connectors.tool_contracts`, its component and authored scenarios | 20261026 |
| `story:connector-projection` | the profile declaration, `publish`, the generated Tavily contract and its drift check | 20261026 |
| `story:tool-conformance-over-invocation` | a `Toolset` over the adapter invocation path, held to `tool-conformance/1` | 20261026 |
| `story:tool-contracts-for-catalog-connectors` | profiles for the catalog connectors' operations | later |
| `story:tool-contract-serving` | a CLI and library surface that answers the contract of an instance's admitted operations | later |

## Not in this epic

- MCP-backed operations: whether they can be published at all waits on
  `decision-blocker:mcp-outbound-stdio-process-ownership`; which connection an inbound MCP caller's
  `tool.invoke` runs under waits on `decision-blocker:mcp-caller-connection-assignment` (ADR 0103,
  Consequences).
- Registry publication of `b10x-tool-contract` (ADR 0103 decision 4 leaves it undecided).

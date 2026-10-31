---
format: aep.planning-md/3
id: story:mcp-tool-call-shapes-its-result
kind: story
status: implemented
title: MCP tools/call takes fields, each and walk; outputSchema.result is any JSON
tags:
- d240
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
- depends_on: story:invoke-shapes-its-result
- depends_on: story:invoke-walks-declared-pages
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T11:46:45Z", actor: "human:timo", revision: 2, executor: "agent:conn-d240", correlation: "wave-d240-output-shaping"}
- {from: "proposed", to: "active", at: "2026-10-30T11:46:45Z", actor: "human:timo", revision: 3, executor: "agent:conn-d240", correlation: "wave-d240-output-shaping"}
- {from: "active", to: "implemented", at: "2026-10-30T13:57:12Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1,"review_outcome":1,"verification":2}}, executor: "agent:conn-d240", correlation: "wave-d240-output-shaping"}
---
## Problem

The local MCP server sends every tool result twice, as `structuredContent` and as the same JSON
in a text block (`adapters/mcp/contracts/server/v1alpha1/projection.md`); one page of 40 Zendesk
audits is 81,991 B, 2.14x the CLI envelope. `tools/call` takes only `{input}`
(`apps/connectors/src/mcp/projection.rs`), so a client cannot narrow it.

## Outcome

`tools/call` takes `arguments.fields`, `arguments.each` and `arguments.walk` (with
`arguments.max_pages`) beside `input`, with the CLI's semantics
(`story:invoke-shapes-its-result`, `story:invoke-walks-declared-pages`).

- Decision D240 c = A: every tool's advertised `outputSchema.result` is any JSON, and the
  operation's own output schema moves to the tool's `_meta`.
- A walk over MCP returns its records as an array in the result, never a file (the read-only
  server has no local write effect). Over the response bound: `complete:false` and `resume`.
- Malformed `fields` or `each`, `walk` on an operation without a declared walk: refused as the
  server refuses invalid arguments today.
- The approval-free read path is unchanged: the server still admits only reads.

## Contract

`adapters/mcp/contracts/server/v1alpha1/projection.md` and any ESS the server's tool projection
is generated from.

## Acceptance

Named tests in `apps/connectors/tests/mcp_tool_call_shapes.rs` (or the existing
`local_mcp_server.rs` harness): `fields` narrows `structuredContent` and the text copy; `each`;
`walk` over two pages returns the records and `end`; `outputSchema.result` is any JSON and the
operation schema is in `_meta`; malformed arguments refused.

## Implemented

Wave D240, merged into `wave/d240-output-shaping` at 9179b5e3d; release 0.63.0.

- `apps/connectors/src/mcp/projection.rs`: `tools/call` takes `fields`, `each`, `walk` and
  `max_pages`. `outputSchema.properties.result` is any JSON, and the operation's output schema
  moves to `_meta` (Breaking). A walk returns `items`, bounded by the response size and by the
  request deadline less a 5 s reserve. `max_pages` accepts an integral number such as `2.0`.
- Contract section "Shaping a tool result" in
  `adapters/mcp/contracts/server/v1alpha1/projection.md`, and `docs/local-mcp-stdio.md`.
- Tests: `apps/connectors/tests/mcp_tool_call_shapes.rs` (7), `mcp_tool_call_shapes_adversary.rs`
  (5).
- Review: `review-result:adversary-d240-s2-pass-1`, fixed.

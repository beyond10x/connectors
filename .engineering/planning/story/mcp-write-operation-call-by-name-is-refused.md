---
format: aep.planning-md/3
id: story:mcp-write-operation-call-by-name-is-refused
kind: story
status: active
title: A Write operation called by name over MCP is refused, held by a runtime case
tags:
- from-finding
relations:
- derived_from: review-result:connectors-wave-2026-11-04-u1-security
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: apps/connectors/tests/local_mcp_server_write_refusal.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-05T09:33:50Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-05T09:33:50Z", actor: "human:timo", revision: 4}
---
## Why

The security review of the MCP revision rule (review-result:connectors-wave-2026-11-04-u1-security) found
that the read-only guard keeping a `Write`-requirement or semantic-effect operation out of `tools/call`
(`apps/connectors/src/mcp/projection.rs:214-229`, `crates/connectors-host/src/local/owner/governed.rs:140`)
holds by reading only. No MCP-level case calls such an operation by name while its approval is not
required; the existing guarded case covers only approval required.

## Acceptance

- A runtime case in `apps/connectors/tests/` calls by name, over the stdio MCP server, an operation with
  a `Write` requirement and approval not required, and one with non-empty `semantic_effects`; each is
  refused `not_found` with zero provider requests, with and without a revision.

## Acceptance (2026-11-05)

Accepted by the directing session `b10x-chef-orchestrator`, whose grant makes the acceptance of this
story its own: the wave 2026-11-05 plan "is approved as sent: U1 and U2", with U2 this story, taken from
draft to active as a test-only story derived from
review-result:connectors-wave-2026-11-04-u1-security.

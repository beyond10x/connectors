---
format: aep.planning-md/3
id: story:mcp-write-operation-call-by-name-is-refused
kind: story
status: implemented
title: A Write operation called by name over MCP is refused, held by a runtime case
tags:
- from-finding
relations:
- derived_from: review-result:connectors-wave-2026-11-04-u1-security
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: apps/connectors/tests/local_mcp_server_write_refusal.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-05T09:33:50Z", actor: "human:operator", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-05T09:33:50Z", actor: "human:operator", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-05T14:19:42Z", actor: "human:operator", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
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

## Delivered (wave 2026-11-05, v0.67.0)

Unit commit 13b9a4938 and adversary cases c0acde398 were merged in 71bb4ef0b on `wave/2026-11-05-connectors`. The local staged gate is green at 925927b8f; the targeted test `local_mcp_server_write_refusal` has 6 passed.

The guards, as found:
- The Write case is refused first by the binding check at `apps/connectors/src/mcp/projection.rs:197`. A Write requirement exists only on `connectors-private/2`.
- The semantic-effects case is held by `projection.rs:214-221`.
- The Read-requirement clause at `:224-228` cannot be false, and stays as an extra layer.

The owner's bounded read does not recheck semantic effects; that is filed as story:owner-bounded-read-rechecks-semantic-effects.

Scope as landed:

| path | result |
|---|---|
| `apps/connectors/tests/local_mcp_server_write_refusal.rs` (new) | held |

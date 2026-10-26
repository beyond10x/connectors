---
format: aep.planning-md/3
id: decision-blocker:mcp-tool-call-without-revision
kind: decision-blocker
status: cleared
title: Nobody has decided whether an MCP tool call without a projection revision may run
relations:
- blocks: story:claude-connects-to-the-mcp-server
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "open", to: "cleared", at: "2026-10-23T20:45:35Z", actor: "human:timo", revision: 4}
---
## Question

May a `tools/call` with no `_meta["io.beyond10x.connectors/revision"]` run against the revision the server currently advertises? `adapters/mcp/contracts/server/v1alpha1/projection.md:125-126` says "Missing revision is malformed input", and `apps/connectors/src/mcp/projection.rs:319` refuses it with `-32602` "Missing projection revision".

A recorded real Claude Code 2.1.309 session (wave C, 2026-10-23) lists the tools, then calls one with no revision in `_meta`: its `tools/call` metadata holds only the protocol version, client info, client capabilities, its tool-use id and a progress token. It also opens with `server/discover` on revision `2026-07-28`, not `initialize` on `2025-11-25`. Against the current contract no tool call from that client can succeed, so story:claude-connects-to-the-mcp-server cannot meet its acceptance.

## Options

| option | what | cost |
|---|---|---|
| A | Amend the contract: a tool call with no revision binds to the revision the server currently advertises for that tool; a present but stale revision is still refused at its current position. One rule in `projection.md`, a row in the stale-ordering table (`projection.md:96-98`), the change in `projection.rs`, and conformance cases for absent, current and stale. | A client that does not echo the revision cannot detect a descriptor change between `tools/list` and `tools/call`; the server's `notifications/tools/list_changed` is its only signal. |
| B | Keep the contract: the story waits until MCP clients echo tool `_meta` in requests. MCP stays planned. | No MCP client observed so far can call a tool. |

Either option rewrites the story's acceptance from `initialize` on `2025-11-25` to `server/discover` on `2026-07-28`.

Recommended: A, because it keeps the stale-revision refusal for every client that sends one and admits the clients that exist.

Decider: operator

Loosening a safety rule of the documented v1alpha1 contract is outside the directing session's grant (directing session, 2026-10-23). Until it is decided, option B holds: the story waits.

## Decision

Decided 2026-10-23 by the operator, option A. The directing session `b10x-chef-orchestrator` relayed the answer verbatim with its date: "A A" (A for decision-blocker:custody-file-delivery-widens-custody, A for this blocker). Its instruction for this item:

> mcp-tool-call-without-revision, A. A call with no revision binds to the currently advertised revision, and a stale one is still refused. Change projection.md and projection.rs and add conformance cases for absent, current and stale. Rewrite the story's acceptance to server/discover on 2026-07-28.

What follows from it:

- `adapters/mcp/contracts/server/v1alpha1/projection.md`: a `tools/call` with no revision in `_meta` binds to the revision the server currently advertises for that tool; a present but stale revision is still refused.
- `apps/connectors/src/mcp/projection.rs` implements that rule.
- Conformance cases for an absent, a current and a stale revision.
- story:claude-connects-to-the-mcp-server gets its acceptance rewritten from `initialize` on `2025-11-25` to `server/discover` on `2026-07-28`, and is implemented in wave C.

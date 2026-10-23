---
format: aep.planning-md/3
id: story:owner-cancels-a-bounded-read
kind: story
status: draft
title: The local owner cancels a bounded read the MCP server abandons
relations:
- decomposes: epic:mcp-contracts
- informed_by: story:mcp-stdio-runtime-branch
- informed_by: review-result:w1022-u3-adversary-r2
revision: 1
---
## Problem

The local MCP stdio server (story:mcp-stdio-runtime-branch) cancels a request locally, but the
protected local owner has no per-request cancel. A read already sent to the owner runs to its
declared budget (execution 20,000 ms, provider 15,000 ms), so its credential read and provider call
can outlive a session that ended as `lost` and the server process itself.

Measured in wave 20261022, adversary pass 2:

- the owner request set (`crates/connectors-host/src/local/owner.rs:1022-1129`) has no per-request cancel;
  `Stop` (`:1117`, `local/owner/supervisor.rs:923-943`) stops the whole instance and leaves a lasting
  suppression;
- the instance worker runs one job at a time from a queue of 16 (`supervisor.rs:674`, `:735`,
  `:778`);
- the adapter side has `InvokeBounded` only (`crates/connectors-host/src/local/runtime/bounded.rs:131-149`);
  `cancel_write` covers writes (`runtime/writes.rs:78`) and `cancellation()` signals
  (`local/protected.rs:107`);
- the client blocks in `read_reply` (`crates/connectors-host/src/local/owner/transport.rs:93`, `:568`).

## Outcome

A cancelled or abandoned bounded read is cancelled end to end: the MCP server's cancel reaches the
owner, the owner cancels the instance job, and the adapter stops the provider call and the
credential read. No provider or credential read is open after the server process has exited.

## Acceptance

- `adversary_a_lost_session_leaves_no_provider_read_open`
  (`apps/connectors/tests/adversary_local_mcp_server.rs`) runs un-ignored and passes; its
  `ignored.rs` entry in the `deferred` family is removed.
- A cancelled call's owner job releases its queue slot before its budget elapses.
- `connectors-private/1`, `/2` and `/3` keep their wire behaviour; the cancel is a versioned
  addition reviewed against `adapters/mcp/contracts/server/v1alpha1/semantics.md` §3.4 and sessions
  §4.1.

## Not in scope

Cancelling writes (already covered by `cancel_write`), and any change to lease timing.

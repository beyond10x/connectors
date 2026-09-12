---
format: aep.planning-md/1
id: decision-blocker:mcp-outbound-stdio-process-ownership
kind: decision-blocker
status: open
title: Nobody has decided whether Connectors spawns an MCP server as a child process
relations:
- blocks: epic:mcp-contracts
withholds: review
revision: 1
---
## The relation nobody has decided

`McpServerBinding → supervised OS process`, for the **outbound stdio** transport:
when the local CLI connects to a selected MCP server over stdio, does Connectors
**spawn and own that server as a child process**, or does it only attach to
something an operator started?

| Field | What is undecided |
|---|---|
| Entities | `McpServerBinding` → the OS process implementing that server |
| Cardinality | one binding to one process, one binding to a process per session, or zero — no process at all, only an attached pair of pipes |
| Ownership | whether the Connectors owner supervises the process, and therefore whether it must pin, digest-verify and restart it |
| Lifecycle coupling | whether the process starts with the binding and dies with it, or outlives every session |

## Why nothing settles it

There is no process-execution capability for providers anywhere in this
repository, and one already-open blocker says the general question is unanswered.

`decision-blocker:helm-execution-family` (open) is titled "Nobody has decided
whether Connectors runs external provider binaries", and its evidence is exactly
what an outbound stdio MCP server would need: "There is no process-execution
capability anywhere in `crates/connectors-sdk/src`, `crates/connectors-client/src`
or any adapter: the only `Command::new` calls outside tests are
`crates/connectors-host/src/local/runtime/process.rs:42` and
`crates/connectors-host/src/local/owner/transport.rs:459`, which are the owner
spawning its own digest-verified adapter child, not a provider capability." That
first site was re-read for this blocker and still spawns
`/proc/self/fd/<executable fd>` — the owner's own verified adapter binary, not an
arbitrary provider command. A fresh `grep -rn 'Command::new' crates/connectors-sdk/src
crates/connectors-client/src adapters/*/src` returns nothing.

That blocker states its own reach: "C16 (exec, copy, port forward) and the Docker
workflows in C17 would land on the same family, so the answer here decides more
than Helm." An outbound stdio MCP server is another member of the same family, and
it is not covered by that blocker's `blocks` edges, which name
`initiative:complete-local-connectors` rather than `epic:mcp-contracts`. This
record carries the MCP instance of the question so the epic can see it; it does
not restate or narrow the Helm blocker, and it is not cleared by clearing that one
unless the answer given there covers a binary the repository does not pin.

`initiative:complete-local-connectors:32` mentions "consumer-owned stdio" as a
seam to be added to the sibling `../mcp` library. That is a statement about which
side of that library owns the transport plumbing, not a decision that Connectors
executes an unpinned third-party binary on a user's machine.

## What this stops

`epic:mcp-contracts` requires that transport profiles be selected at authoring and
that "local stdio and deployed HTTP profiles" be evaluated explicitly.

Not drafted because of this blocker: a story specifying the outbound stdio
transport profile — its framing, the server process's lifecycle, and what the
owner pins and verifies before executing it.

The outbound Streamable HTTP profile, which attaches to an endpoint and executes
nothing, is drafted and is not behind this question. The **inbound** stdio
direction is also not behind it: there the MCP client spawns `$BIN server`, so
Connectors is the child rather than the parent, and no provider binary is
executed by this repository.

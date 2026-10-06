---
format: aep.planning-md/3
id: decision-blocker:mcp-outbound-stdio-process-ownership
kind: decision-blocker
status: cleared
title: Nobody has decided whether Connectors spawns an MCP server as a child process
relations:
- blocks: epic:mcp-contracts
withholds: review
revision: 4
transitions:
- {from: "open", to: "cleared", at: "2026-10-03T14:52:29Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"approval":1}}}
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

## Operator decision — 2026-10-03

The operator answered on 2026-10-03: "Yes: own a pinned process per session
(recommended)" to whether Connectors should launch one explicitly configured,
pinned outbound MCP server process per session and supervise its cleanup.

Decision: Connectors owns the outbound stdio server process. Each stdio session
selects an explicit pinned executable configuration and has its own supervised
process; sessions do not share one process or attach implicitly to an arbitrary
operator-started process. Pin verification and bounded cleanup are required parts
of the binding. This resolves the ownership/cardinality choice that this blocker
withheld; it is not evidence that a process binding, framing, restart policy,
credential delivery or runtime conformance already exists.

Next: model the session/process/binding relations in native ESS, specify the
versioned outbound stdio lifecycle and admission/cleanup cases, then record its
implementation story. Reuse the shared bounded-process capability where its
semantics apply, retaining native MCP protocol behavior in the adapter. No
shell-script execution, automatic business retry or implicit unpinned launch is
authorized by the process ownership decision.

## Model reconciliation scope — 2026-10-03

The operator has cleared both process ownership decisions. Reconcile native MCP
stdio selection and the relation census with that answer; retain the full Helm
scope in its owning initiative. First model the MCP stdio process as a session-owned
record with a reference to its explicitly selected server binding and the configured
executable path/digest. A session owns at most one such process; a successfully
launched stdio session has one, and HTTP sessions have none. Ending the owning
session requires bounded process cleanup. This is a declared ownership/pin model,
not implemented launch behavior, an OS PID identity or a second credential owner.

Keep unresolved HTTP/durable-session and caller/Connection relationships explicit.
The native model must not import shared domains. The general bounded process port,
Helm command construction, filesystem/credential admission and guarded rollback
still need their own shared/native typed models and executable acceptance before
implementation claims. Do not create their implementation stories ahead of those
models. Update the census/selection checks to distinguish a resolved decision from
an open blocker instead of requiring the decision to remain open forever.

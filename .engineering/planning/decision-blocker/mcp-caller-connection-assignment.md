---
format: aep.planning-md/1
id: decision-blocker:mcp-caller-connection-assignment
kind: decision-blocker
status: open
title: Nobody has decided which provider connection an inbound MCP caller may use
relations:
- blocks: epic:mcp-contracts
withholds: review
revision: 1
---
## The relation nobody has decided

`McpCaller → connectors.auth_bindings.Connection`, in the direction an inbound
invocation runs: when a caller reaches the `$BIN server` inbound MCP binding and
invokes an advertised Connectors capability, **which connection's provider
credential carries out that invocation?**

| Field | What is undecided |
|---|---|
| Entities | `McpCaller` (an authenticated inbound MCP client) → `connectors.auth_bindings.Connection` |
| Cardinality | one caller to one connection, one caller to many, or many callers to one shared connection — all three are consistent with the epic's text |
| Ownership | whether a connection belongs to the caller that created it, or to the configured instance that every caller shares |
| Lifecycle coupling | whether revoking a caller revokes its connections, and whether a connection outlives the caller |

## Why nothing settles it

`ess/domains/declarations.yaml:180-181` declines the question in an `ess/1`
document, in as many words:

> `# UNMAPPED: multi-tenant SaaS ownership/assignment cardinality remains outside`
> `# this local, explicitly configured service slice. No tenant is caller-selected.`

Two more shared domains decline the same thing for their own records.
`ess/domains/mutations.yaml:110` — "UNMAPPED: admission principal/tenant entities
remain outside this binding" — while `connectors.mutations.AttemptRecord` does
carry a `connection` relation to exactly one `connectors.auth_bindings.Connection`
(`ess/domains/mutations.yaml:95-98`, `kind: references`, `cardinality: one`, `via:
connection_ref`). `ess/domains/idempotency.yaml:72-73` — "UNMAPPED: namespace
authority/issuer/tenant configuration remains outside this selected binding" —
while `connectors.idempotency.KeyReservation.attempt` binds one reservation to one
attempt (`ess/domains/idempotency.yaml:65-68`).

So the store is in a precise state: every durable record downstream of an
invocation names **one** connection, and nothing anywhere names the principal that
selected it. In the single-owner local placement the gap is harmless, because
`contracts/cli/v1alpha1/semantics.md` section 1 admits exactly one principal —
"the current effective Linux UID as the configured owner, verified again at a
protected Unix-domain socket using kernel peer credentials" — enforced at
`crates/connectors-host/src/local/owner/transport.rs:413`, which refuses a socket
whose `uid()` is not the process's own. With one principal there is nothing to
assign.

The cloud placement `epic:mcp-contracts` asks for has more than one principal, and
then the assignment is the whole of what "caller isolation" means.

## What this stops

`epic:mcp-contracts` acceptance criterion 3 requires that "A fixture verifies
caller isolation and no provider-secret disclosure". The second half is
specifiable now and is drafted. The first half is not: a caller-isolation fixture
has to assert which connections caller A may reach and which are caller B's, and
that assertion is this undecided relation.

Not drafted because of this blocker: a story specifying caller-to-connection
assignment and the caller-isolation fixture for the multi-caller cloud placement.

The epic's own boundary is why this is a question for the operator and not for a
contract author: "Cloud serving is an optional placement; it must not impose a
hosted identity service or federation on local use." Answering it by adopting the
proposed `service/v1alpha2` verified caller context would import a dependency
`review-result:concept-stack-integration-20260908` gap G2 records this repository
does not have ("no Identity dependency (`Cargo.lock`: 0 hits for
`identity-client`)"); answering it with an MCP-owned caller identity would make
MCP the owner of a fact the epic says to leave with an established owner.

No answer is assumed here, and no story carries either option.

---
format: aep.planning-md/1
id: story:mcp-inbound-mutation-replay
kind: story
status: draft
title: Bind inbound MCP mutating invocations to the existing attempt and idempotency owners
relations:
- decomposes: epic:mcp-contracts
- serves: vision:independent-contract-adapters
- depends_on: story:mcp-inbound-capability-projection
scope:
- confidence: inferred
  path: adapters/mcp/contracts/server/v1alpha1/mutations.md
- confidence: inferred
  path: adapters/mcp/contracts/server/v1alpha1/scenarios
revision: 2
---
## Acceptance

`adapters/mcp/contracts/server/v1alpha1/mutations.md` specifies how a mutating
invocation arriving over the inbound MCP binding binds to the existing attempt and
idempotency owners, such that a repeated MCP request whose first outcome was never
observed is answered from the recorded attempt rather than dispatched a second
time, and a lost reply is reported as an unknown outcome rather than as a failure.

## Scope

- `adapters/mcp/contracts/server/v1alpha1/mutations.md` — new. No other story
  writes this file.
- `adapters/mcp/contracts/server/v1alpha1/scenarios/` — shared directory, also
  written by `story:mcp-inbound-local-binding` and
  `story:mcp-inbound-capability-projection`; this story adds only mutation and
  lost-reply scenario files.

## Domain relations

Both relations this document rests on are declared, and it adds neither.

- **`connectors.idempotency.KeyReservation → connectors.mutations.AttemptRecord`,
  many reservations to one attempt, `references` rather than `owns` — inferable
  from `ess/domains/idempotency.yaml:65-69`** (`name: attempt`, `kind:
  references`, `cardinality: one`, `via: attempt_id`). The file states the
  consequence in its own comment at lines 70-71: "References, not owns: expiry of
  the replay reservation cannot delete audit evidence. Each key generation is
  bound to one immutable attempt identity." An MCP request identifier is therefore
  a key into a reservation, never a new ledger.
- **`connectors.mutations.AttemptRecord → connectors.auth_bindings.Connection`,
  one attempt to one connection — inferable from
  `ess/domains/mutations.yaml:95-99`** (`name: connection`, `kind: references`,
  `cardinality: one`, `via: connection_ref`). Every durable record of an
  invocation already names exactly one connection; an inbound MCP mutation
  produces one of these or it produces no durable record at all.

The attempt lifecycle supplies the non-duplication guarantee directly:
`ess/domains/mutations.yaml:111-120` declares `lose_outcome` from `Dispatching` to
`Indeterminate`, and `Indeterminate` is terminal with no transition out. There is
no state in which an uncertain effect becomes certain by being asked again.

## What this story must establish

`epic:mcp-contracts` acceptance criterion 5: "Cancellation, malformed input,
version/capability mismatch, partial output, lost replies and mutation uncertainty
have reviewed scenarios. A repeated MCP request cannot automatically duplicate an
uncertain business effect." This story owns the inbound **lost replies** and
**mutation uncertainty** halves; cancellation, malformed input, mismatch and
partial output for this direction belong to
`story:mcp-inbound-local-binding` and `story:mcp-inbound-capability-projection`,
and their outbound counterparts to `story:mcp-outbound-invocation-results`.

The established owners, all documented:
`contracts/operations/v1alpha1/semantics.md` owns the `mutation` profile —
"effects, idempotency, approval binding, durable attempt record, outcome-unknown"
(`contracts/README.md`, index table) — and `contracts/service/local-mutations.md`
owns the local approval coordinator, which
`contracts/cli/v1alpha1/semantics.md` records as adding "policy publication, exact
subject preparation and protected issuance" while stating that "Provider write
dispatch remains unfinished". This document maps MCP onto those; it does not
restate them and it does not add a second approval format.

`ess/domains/service_wire.yaml:14-16` already carries `outcome_unknown`,
`idempotency_conflict`, `approval_required`, `approval_refused` and
`approval_replayed` as distinct `ErrorCode` variants. Each needs a stated MCP
observable, and none may be collapsed into a generic protocol error.

## What it does not cover

Which connection a caller's mutation runs against in a multi-caller placement —
`decision-blocker:mcp-caller-connection-assignment`. Outbound mutation uncertainty
— `story:mcp-outbound-invocation-results`. No mutation is performed; this is an
authored document and its scenarios.

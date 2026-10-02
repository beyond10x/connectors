---
format: aep.planning-md/3
id: story:mcp-inbound-mutation-replay
kind: story
status: active
title: Bind inbound MCP mutating invocations to the existing attempt and idempotency owners
relations:
- decomposes: epic:mcp-contracts
- serves: vision:independent-contract-adapters
- depends_on: story:mcp-inbound-capability-projection
scope:
- confidence: inferred
  path: adapters/mcp/contracts/server/v1alpha1/mutation-cases.json
- confidence: cited
  path: adapters/mcp/contracts/server/v1alpha1/mutations.md
- confidence: inferred
  path: crates/connectors-build/tests/mcp_inbound_mutation_replay.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-02T22:44:32Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-10-02T22:44:32Z", actor: "human:timo", revision: 7}
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
  bound to one immutable attempt identity." An MCP request identifier is correlation only; an explicit business replay key
  uses the existing qualified reservation namespace, never a new ledger.
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

## Scope — confirmed 2026-10-03

Read-only aep:story-scoper inspected integration a2955675 and its reviewed candidate.
Cited: adapters/mcp/contracts/server/v1alpha1/mutations.md, named in acceptance.
Inferred: adapters/mcp/contracts/server/v1alpha1/mutation-cases.json and
crates/connectors-build/tests/mcp_inbound_mutation_replay.rs, exact authored case
inventory and Rust guard. These replace the broad scenarios directory allocation.
Confidence high for the document and owners; additional file allocation is inferred.
Only these three files are writable. No shared models, projection, host runtime,
manifest or gate changes. Inbound YAML is collected as ESS session traces
(gate.rs:250-268); a fake session command cannot prove mutation replay.

The reviewed projection story must reach implemented after its integration gate
before this unit starts. Mutation advertisement stays withheld until an actual
complete executable binding exists; this document does not establish that binding.
No new entity, caller assignment, cross-root relation or deletion cascade is added.

## Source corrections and precise acceptance

The earlier inference that an MCP request identifier is automatically a reservation
key is incorrect. operations/v1alpha1/semantics.md sections5.1-5.2 exclude correlation
IDs from replay fingerprints; local-mutations gives each invocation a fresh host
request identity. Preserve MCP correlation, host request, attempt and explicit caller
business-key identities separately. Explicit keyed replay uses receiver_instance,
admitted_authority, trusted_origin and the opaque key, existing mutation-key/v1 and
mutation-request/v2 qualification, canonical input and nullable route. Missing keyed
input refuses; none/natural operations gain no keyed replay by accepting a key.

Terminal Indeterminate alone does not establish non-duplication. Authoritative key
uniqueness, atomic reservation/preparation, one-shot dispatch winner and conservative
recovery also do. Current policy and target/result authority precede descriptor
revision, private lookup/enablement, input validation and then key inspection.
Repeat disclosure admission before replay or completing a wait. Exact retained matches
observe the original without fresh provider preflight, custody access, proof spend
or dispatch. After a miss, preserve the authoritative winner recheck before approval/
preflight refusal. Protected proof bytes never become ordinary MCP arguments.

Known-result retention is fixed at settlement plus86400seconds; quarantine never
expires automatically. New keys or correlation IDs do not resolve earlier unknown
outcomes. Distinguish absent caller reply, live host knowledge and durable recovery
knowledge. Preserve the five named replay/approval/uncertainty errors through the
reviewed projection mapping, plus applied-with-error and audit limitations.

The prior blanket statement that provider write dispatch is unfinished is historical:
local-mutations now describes guarded GitLab private-protocol-two dispatch. This is
not an MCP binding. The current story remains authored mapping with separate document
checks, never actual mutation or no-duplicate-effect conformance evidence.

---
format: aep.planning-md/1
id: story:mcp-outbound-invocation-results
kind: story
status: draft
title: Specify outbound MCP invocation results, error preservation, bounds and unknown outcomes
relations:
- decomposes: epic:mcp-contracts
- serves: vision:independent-contract-adapters
- depends_on: story:mcp-outbound-connection-lifecycle
scope:
- confidence: inferred
  path: adapters/mcp/contracts/client/v1alpha1/invocation.md
- confidence: inferred
  path: adapters/mcp/contracts/client/v1alpha1/scenarios
revision: 2
---
## Acceptance

`adapters/mcp/contracts/client/v1alpha1/invocation.md` specifies what invoking a
selected remote capability returns and refuses — result content and structured
output, output byte bounds, the preservation of a provider error as distinct from
a protocol error, malformed input, partial output, and an outcome that was never
observed — such that a remote tool annotation cannot make an invocation a local
write, and an unobserved outcome is reported as unknown rather than retried.

## Scope

- `adapters/mcp/contracts/client/v1alpha1/invocation.md` — new. No other story
  writes this file.
- `adapters/mcp/contracts/client/v1alpha1/scenarios/` — shared directory, also
  written by `story:mcp-outbound-connection-lifecycle` and
  `story:mcp-outbound-auth-lifecycle`; this story adds only result and error
  scenario files.

## Domain relations

None it states. The invocation semantics here are per-session values; whether an
invocation leaves a durable record outbound is
`story:mcp-domain-model`'s `McpServerBinding → McpCapabilitySnapshot` marker,
still `UNMAPPED:`.

The one relation the document explicitly refuses to create is worth naming,
because refusing it is the point: a remote MCP tool annotation establishes no edge
into this repository's authority model. The epic states it flatly — "Tool
annotations or remote descriptions cannot establish local write authority" — and
local write authority has its own owner,
`ess/domains/local_approval_policy.yaml`, summarised there as "Proposed serialized
local owner policy for approval issuance and write admission". Remote metadata is
input to a display, never input to that policy.

## What this story must establish

The epic's outbound deliverable: "invoking selected tools and accessing other
selected MCP capabilities; truthful results, errors, limits and shutdown", and its
capability deliverable: "Preserve provider errors versus protocol errors, result
content/structured output and output bounds."

**Bounds have an existing shape to reuse.**
`ess/domains/service_wire.yaml:52-59` declares
`connectors.service_wire.OperationLimits` with `request_bytes`, `result_bytes`,
`execution_ms`, `provider_ms` and `connect_ms`. An MCP output bound that invents a
sixth axis, or silently truncates without saying so, contradicts the repository's
own limit vocabulary. `ess/domains/transport.yaml:12` carries the matching rule
for a bounded read — "complete means EOF was actually observed, not that
Content-Length matched" — which is exactly the partial-output distinction
acceptance criterion 5 asks for.

**Provider error versus protocol error is a distinction the store already draws.**
`ess/domains/service_wire.yaml:20-24` declares
`connectors.service_wire.DiagnosticCause` as a `BaseErrorCode` plus a
`CauseStage`, and `CauseStage` (`ess/domains/service_wire.yaml:17-19`) separates
`dispatch`, `response` and `observation` as distinct stages. An MCP error surface that maps a remote server's business failure
and a JSON-RPC framing failure to one code loses a distinction this repository
already has.

**An unobserved outcome is never retried.**
`ess/domains/mutations.yaml:111-120` declares
`connectors.mutations.AttemptRecord`'s lifecycle with `lose_outcome` from
`Dispatching` to a terminal `Indeterminate` — a state with no transition out.
`contracts/service/compatibility.md` section 2 states the client-side rule in
prose: "A new mutation client that has sent an invocation and lacks a valid
definitive response reports outcome uncertainty and never retries automatically."
Outbound MCP invocation inherits both. This is the outbound half of the epic's
acceptance criterion 5; the inbound half is
`story:mcp-inbound-mutation-replay`.

## What it does not cover

Which capabilities are selected at all — `story:mcp-profile-selection-matrix`.
Composition, where an invocation is itself re-exposed inbound —
`story:mcp-composition-provenance`. Nothing inbound. No server is contacted.

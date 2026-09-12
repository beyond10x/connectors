---
format: aep.planning-md/1
id: story:mcp-outbound-connection-lifecycle
kind: story
status: draft
title: Specify the outbound MCP connection lifecycle over the selected HTTP transport
relations:
- decomposes: epic:mcp-contracts
- serves: vision:independent-contract-adapters
- depends_on: story:mcp-profile-selection-matrix
scope:
- confidence: inferred
  path: adapters/mcp/contracts/client/v1alpha1/scenarios
- confidence: inferred
  path: adapters/mcp/contracts/client/v1alpha1/semantics.md
revision: 2
---
## Acceptance

`adapters/mcp/contracts/client/v1alpha1/semantics.md` and its scenarios specify
the complete outbound Streamable HTTP connection lifecycle — how a server is
discovered and explicitly selected, what `initialize` negotiates, what a protocol
version or capability mismatch refuses, and how framing, streaming, progress,
cancellation, session loss and shutdown behave — such that every one of those
states has a named observable outcome and a scenario file, and none of them is
described as "the client retries".

## Scope

- `adapters/mcp/contracts/client/v1alpha1/semantics.md` — new; the outbound base
  document. No other story writes this file.
- `adapters/mcp/contracts/client/v1alpha1/scenarios/` — new; the lifecycle
  scenarios. `story:mcp-outbound-auth-lifecycle` and
  `story:mcp-outbound-invocation-results` each add files to this same directory
  under their own names, and say so in their own bodies; no story edits another's
  scenario file.

## Domain relations

The selection of a server rests on `McpServerBinding → McpOutboundSession`, whose
cardinality and lifecycle coupling this story does **not** state, because
`story:mcp-domain-model` carries it as an `UNMAPPED:` marker — no `ess/1`
document declares an MCP noun, and guardrail 7 keeps the marker rather than a
guess. This document therefore specifies what a *live* session does and refers to
that marker for what persists; a reviewer can check that no sentence here implies
a durable binding record.

## What this story must establish

`epic:mcp-contracts` names these outbound outcomes: "discovery and explicit
selection of a server; ... initialization/version and capability negotiation" and,
under its transport deliverable, "specify framing, streaming, cancellation,
progress, connection/session loss and version mismatch for the selected profiles."

**Explicit selection, never discovery-driven selection.** The epic's composition
deliverable states the rule this document must not violate even in its selection
section: "prohibit implicit credential forwarding, account fallback or authority
escalation from discovered metadata." The repository already holds the same line
for its own discovery families —
`contracts/service/compatibility.md` section 2: "a discovery observation cannot
switch the endpoint, credential, version or profile."

**Mismatch refuses; it does not degrade.** The same section supplies the shape:
"Unknown/missing routes or incompatible descriptors stop selection; the client
does not try a second route/version/credential or resend the request. A raw HTTP
404 or malformed response is not renamed into proof that a particular version is
unsupported." The MCP version-mismatch rule must be at least this strict against
the revision pinned by `story:mcp-specification-pin`.

**Loss is a distinct outcome from failure.** `ess/domains/service_wire.yaml:14-16`
already declares `connectors.service_wire.ErrorCode`, whose variant list contains
`unavailable` and `session_lost` as separate values; an MCP session-loss outcome that maps onto
neither, or onto both, is a defect this document must resolve.

## What it does not cover

- **Outbound stdio.** Held by `decision-blocker:mcp-outbound-stdio-process-ownership`:
  nobody has decided whether Connectors spawns an MCP server as a child process,
  and a stdio framing specification that did not answer that would be answering it
  silently. This document specifies Streamable HTTP only and says so in its status
  line.
- Authentication of the connection — `story:mcp-outbound-auth-lifecycle`.
- What an invocation returns — `story:mcp-outbound-invocation-results`.
- Anything inbound. This document states no `$BIN server` behaviour.

---
format: aep.planning-md/1
id: story:mcp-composition-provenance
kind: story
status: draft
title: Specify composition when an inbound MCP capability is backed by an outbound MCP server
relations:
- decomposes: epic:mcp-contracts
- serves: vision:independent-contract-adapters
- depends_on: story:mcp-outbound-invocation-results
- depends_on: story:mcp-inbound-capability-projection
scope:
- confidence: inferred
  path: adapters/mcp/contracts/composition/v1alpha1/scenarios
- confidence: inferred
  path: adapters/mcp/contracts/composition/v1alpha1/semantics.md
revision: 2
---
## Acceptance

`adapters/mcp/contracts/composition/v1alpha1/semantics.md` specifies what happens
when a capability the inbound binding advertises is itself backed by an outbound
MCP server — what provenance the caller sees, which limits apply, and what is
admitted — such that no credential is forwarded between the two hops, no account
is substituted when one fails, and nothing a remote server said about itself
raises what the composed capability is allowed to do.

## Scope

- `adapters/mcp/contracts/composition/v1alpha1/semantics.md` — new. No other
  story writes this file.
- `adapters/mcp/contracts/composition/v1alpha1/scenarios/` — new; the forwarding,
  fallback and escalation refusals.

## Domain relations

This document states no new relation. It is the one place in the decomposition
where both directions appear, and it appears there only to specify the boundary
*between* them — which is why it must not describe either direction's behaviour:
outbound belongs to `story:mcp-outbound-invocation-results` and inbound to
`story:mcp-inbound-capability-projection`, and this document cites both rather
than restating either.

The relation it is forbidden to create is the point of the document: a remote
server's advertised metadata creates no edge into this repository's authority
model. The epic states it — "prohibit implicit credential forwarding, account
fallback or authority escalation from discovered metadata" — and the owner of
local write authority is `ess/domains/local_approval_policy.yaml`, whose own
header says "Metadata alone grants no use; the authenticated owner, current
policy/key leases and target checks are required"
(`ess/domains/local_approval_policy.yaml:3-4`).

## What this story must establish

The epic's composition deliverable in full: "Describe composition when a local or
deployed MCP server exposes an outbound MCP-backed capability. Preserve
caller/target provenance, limits and admission through the mapping; prohibit
implicit credential forwarding, account fallback or authority escalation from
discovered metadata."

**The repository already has a one-hop composition family to imitate.**
`contracts/discovery/mediated_route/v1alpha1/semantics.md` is indexed in
`contracts/README.md` as "one-hop forwarding of a child adapter's HTTP through a
parent connection's admitted binding", and its ESS carrier is visible in
`ess/domains/auth_bindings.yaml:95` — a `Connection`'s `mediated_route` field of
type `Optional<connectors.discovery.MediatedRouteBinding>` — with the file's own
comment at lines 121-122 stating the constraint that matters here: "Fixed mediated route and
parent are present together only for a via binding; parent identity is this field,
not duplicated route authority." An MCP composition is the same shape and must not
be looser.

`story:contracts-host-composition` ("Assign mediated adapter wiring to an explicit
composition owner", implemented) established that composition has a named owner
rather than being an emergent property of two adapters being configured together.
This document names that owner for the MCP case.

**Provenance must survive the hop.** `contracts/README.md` records provenance as a
guarantee of the reads families — "bounded pages, cursors, provenance,
completeness" for `datasource.records/v1alpha1` — and
`ess/domains/service_wire.yaml:41-46` shows the safe projection shape:
`connectors.service_wire.SourceAudit` is an instance-qualified opaque correlation
value, not the record. A composed result that loses which hop produced it, or
that discloses the upstream binding to do so, fails one of the two.

## What it does not cover

Whether a composed capability may be exposed at all in a multi-caller placement —
that reduces to `decision-blocker:mcp-caller-connection-assignment`, since a
composed hop runs against some connection and nothing assigns one to a caller.
This document specifies the single-principal local composition and says so.

No composition is configured or exercised; this is an authored document and its
scenarios.

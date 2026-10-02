---
format: aep.planning-md/3
id: story:mcp-composition-provenance
kind: story
status: draft
title: Specify composition when an inbound MCP capability is backed by an outbound MCP server
relations:
- decomposes: epic:mcp-contracts
- serves: vision:independent-contract-adapters
- depends_on: story:mcp-outbound-invocation-results
- depends_on: story:mcp-inbound-capability-projection
- depends_on: story:mcp-outbound-auth-lifecycle
- depends_on: story:mcp-inbound-mutation-replay
scope:
- confidence: inferred
  path: adapters/mcp/contracts/composition/v1alpha1/composition-cases.json
- confidence: cited
  path: adapters/mcp/contracts/composition/v1alpha1/semantics.md
- confidence: inferred
  path: crates/connectors-build/tests/mcp_composition_provenance.rs
revision: 5
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
not duplicated route authority." This is a precedent for explicit boundaries, not proof that MCP operation
composition has the same transport shape or credential relation.

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

## Scope — confirmed 2026-10-03

Cited: adapters/mcp/contracts/composition/v1alpha1/semantics.md.
Inferred: adapters/mcp/contracts/composition/v1alpha1/composition-cases.json and
crates/connectors-build/tests/mcp_composition_provenance.rs. Replace the old broad
scenario directory allocation. Confidence high for primary contract, medium for
case/guard allocation. Exact three paths are the only collisions; source-inspected
safety at level2, not runtime proof. Existing serde dependencies and gate discovery
suffice. Single-owner local placement and eight unresolved relation markers remain.

## Source corrections before implementation

Mediated HTTP routing is not the same semantic shape as MCP operation composition:
contracts/discovery/mediated_route/v1alpha1/semantics.md section4 requires transport-
only substitution, material-free mediated child, one hop and private same-process
ports. MCP projection crosses protocol/operation boundaries and can authenticate
outbound independently. Reuse explicit ownership, independent admission and no
fallback principles; do not infer mediated_route, parent_connection_ref, GET-only
access, process placement or credential custody from the analogy.

SourceAudit is an instance-qualified opaque correlation, not a general provenance
chain or lookup authority (service_wire.yaml:41; compatibility.md:80-82). Only an
acknowledged trustworthy downstream observation supplies it. Remote MCP metadata or
self-reported identity cannot manufacture audit evidence; preserve admitted local
identity and keep remote content untrusted. Existing federation is an analogy only.

Auth and mutation contracts must finish before composition describes their behavior;
this story now depends on both. Mutation advertisement remains unavailable until its
actual executable binding exists. Cases cover threefamilies, independent revision
pairs, fixedtarget/account selection, credential locality, independent admission,
safeprovenance, wrapperbyteoverhead, remainingdeadline, partial/unknown results,
metadata-no-authority, missingmutationbinding and unresolved durable ownership.
Document guards derive applicable vocabulary from authoritative owners, reject
missing/duplicate/contradictory copied cases, and claim no network/runtime conformance.

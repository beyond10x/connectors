---
format: aep.planning-md/3
id: story:mcp-inbound-capability-projection
kind: story
status: draft
title: Specify which Connectors capabilities the inbound MCP binding advertises and how results map
relations:
- decomposes: epic:mcp-contracts
- serves: vision:independent-contract-adapters
- depends_on: story:mcp-domain-model
- depends_on: story:mcp-inbound-local-binding
scope:
- confidence: inferred
  path: adapters/mcp/contracts/server/v1alpha1/projection.md
- confidence: inferred
  path: adapters/mcp/contracts/server/v1alpha1/scenarios
revision: 3
---
## Acceptance

`adapters/mcp/contracts/server/v1alpha1/projection.md` specifies exactly which
declared Connectors operations become advertised MCP capabilities and how each
result and error crosses that boundary, such that an operation the caller is not
admitted to is absent from discovery rather than present and refused, and no
advertised capability carries a guarantee weaker than the operation it projects.

## Scope

- `adapters/mcp/contracts/server/v1alpha1/projection.md` — new. No other story
  writes this file.
- `adapters/mcp/contracts/server/v1alpha1/scenarios/` — shared directory, also
  written by `story:mcp-inbound-local-binding` and
  `story:mcp-inbound-mutation-replay`; this story adds only projection and
  error-mapping scenario files.

## Domain relations

Each `McpAdvertisedCapability` projects exactly one already declared operation.
The scalar `operation_ref` and its stated-edge commentary in
`adapters/mcp/spec/ess/domains/state.yaml` own that fact. The independent native
ESS root deliberately does not declare a cross-root relation to
`connectors.declarations.OperationDeclaration`.

`ess/domains/declarations.yaml` declares that `AdapterSpecification` owns many
`OperationDeclaration` records. That is the existing declaration owner, a different
pair from the advertised-capability projection; it neither establishes this
projection's cardinality nor a durable deletion cascade. Specify withdrawal from
discovery when the declaration is no longer selected/admitted without inventing
persistent record deletion.

An `operation_id` alone is not a runtime operation identity. Preserve the instance
and adapter qualification required by the shared mutation model, and define an
unambiguous MCP advertised name and inverse selection. This story does not assign
another caller's provider connection. The existing single-owner local binding
admits the configured owner, while the eight unresolved MCP relation markers stay
intact. No `McpCaller` entity or hidden Connection assignment is introduced.

## What this story must establish

The epic's inbound deliverable: "discoverable Connectors capabilities and
invocations with their existing target, permission, mutation and
credential-locality guarantees."

**Visibility is a refusal precedence question the repository has already answered
once.** `contracts/service/compatibility.md` section 3 states the rule for the
legacy projection: "The legacy invoke path resolves only against that set and
revision. A guessed hidden operation is `not_found` before dispatch ... Do not
accept a mutation through a hidden name, alternate connection, omitted approval or
direct handler lookup. Projection strips new metadata only after proving its
absence loses no required meaning; otherwise the operation is unavailable on that
binding." An MCP projection is a second projection of the same kind and inherits
that discipline; the same section's "extended visibility/refusal precedence"
pointer names where the governed variant lives.

**Eligibility is not viability.** `ess/domains/connection_admission.yaml:41-51`
separates `EligibilityFacts` — which carries `operation_enabled`,
`profile_matches`, `scope`, `permission` and `verification` — from the
`ViabilityDecision` it embeds. A capability
listing that treats a reachable connection as an invocable operation erases that
separation.

**Errors map onto an existing closed vocabulary or they are new.**
`ess/domains/service_wire.yaml:14-16` declares
`connectors.service_wire.ErrorCode` as a closed enum of thirty variants. The
projection must state, per variant, what a caller observes, and must not invent a
thirty-first by naming it only in MCP terms.

**Credential locality is a guarantee, not a default.** The epic requires the
projected capability to keep "credential-locality guarantees", and
`initiative:complete-local-connectors:19` records what local means here: "The
admitted OS Secret Service collection owns credentials." No provider credential,
and no fact derived from one, crosses into an MCP payload.

## What it does not cover

**Which connection's credential a given caller's invocation uses.** That relation
is held by `decision-blocker:mcp-caller-connection-assignment`: nothing in the
store assigns a caller to a connection, and
`ess/domains/declarations.yaml:180-181` declines to. In the single-principal local
placement there is nothing to assign, which is why this document is drafted; it
states no rule for a second caller.

Mutating invocations and their replay semantics —
`story:mcp-inbound-mutation-replay`. The binding and its session lifecycle —
`story:mcp-inbound-local-binding`. Anything outbound.

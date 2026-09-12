---
format: aep.planning-md/1
id: story:mcp-domain-model
kind: story
status: active
title: Model the MCP nouns in an adapter-owned ESS root, with unreadable relations left UNMAPPED
relations:
- decomposes: epic:mcp-contracts
- serves: vision:independent-contract-adapters
- depends_on: story:mcp-specification-pin
scope:
- confidence: inferred
  path: adapters/mcp/spec/ess/domains/protocol.yaml
- confidence: inferred
  path: adapters/mcp/spec/ess/domains/state.yaml
- confidence: inferred
  path: adapters/mcp/spec/ess/system.yaml
revision: 4
---
## Acceptance

A new `connectors_mcp` ESS root at `adapters/mcp/spec/ess/` validates under the
pinned toolchain — `ess specify validate --path adapters/mcp/spec/ess` exits 0 —
declaring the MCP nouns this epic introduces, with the three credential kinds the
epic keeps distinct modelled as three distinct types, and with every relation the
model could not read from an existing `ess/1` document or the pinned specification
carried as an explicit `UNMAPPED:` comment rather than a chosen cardinality.

## Scope

- `adapters/mcp/spec/ess/system.yaml` — new
- `adapters/mcp/spec/ess/domains/protocol.yaml` — new; protocol values
- `adapters/mcp/spec/ess/domains/state.yaml` — new; the entities and their
  lifecycles

No shared `ess/` file is touched, and `ess/system.yaml` is not edited. That is not
a preference; `adapters/README.md:66-70` states the rule: "Each authored
native model lives in `spec/ess` with namespace `connectors_<owner>.<domain>`.
Shared ESS includes none of these roots and imports none of their types. Each
compiles independently. Native-to-shared projection is an explicit adapter binding
obligation, not a cross-root ESS import or arbitrary JSON bag." The boundary gate
enforces it mechanically: `adapters/README.md`'s `## Boundary gate` section says
the scan runs "against reviewed terminology plus every adapter directory name",
so creating `adapters/mcp/` makes `mcp` a term shared ESS may not carry.
`crates/connectors-build/ess-boundary.json` currently lists `sip` as its only
`shared_protocol_names` entry, which is what an exception would have to look like.

## Domain relations

This story is where the relation census lands, so it carries the whole of it.

**Relations it can cite, and therefore states:**

- `McpAdvertisedCapability → connectors.declarations.OperationDeclaration`,
  many advertised capabilities to the operations already declared, the
  `AdapterSpecification` owning both — inferable from
  `ess/domains/declarations.yaml:119-123`
  (`connectors.declarations.AdapterSpecification.operations`, `kind: owns`,
  `cardinality: many`, `via: adapter_id`). The epic requires the reuse rather than
  a second owner: "Reuse established owners instead of making MCP the universal
  internal contract." An advertised MCP capability is therefore a projection of an
  owned declaration and never a second record of it.
- `McpSession → connectors.auth_bindings.Connection` is **deliberately absent**,
  and that absence is itself cited: `ess/domains/sessions.yaml:89-91` says
  "UNMAPPED: a future selected session binding must join its one Connection and
  establishment authority, whose persistence model remains unselected. Sessions
  are outside the current three-adapter model milestone; no stub edge is implied."
  This model repeats that refusal rather than inventing the edge the shared model
  declined to declare.

**Relations no `ess/1` document and no code answers, carried as `UNMAPPED:`
markers under planning-skill guardrail 7, not guessed:**

| Relation | What is unreadable |
|---|---|
| `McpServerBinding → connectors.declarations.ServiceConfiguration` | whether a selected outbound server is a configured instance at all |
| `McpServerBinding → connectors.auth_bindings.Connection` | whether it is one `Connection` row, several, or none |
| `McpServerBinding → McpCapabilitySnapshot` | whether a discovered tool/resource/prompt set is durable state or a per-session value, and therefore whether a cardinality exists to declare |
| `McpOutboundSession → McpServerBinding` | whether a session may outlive a binding, and what a binding's removal does to a live session |
| `McpServerBinding → credential custody` | whether the outbound server credential is a `connectors.credentials.CredentialGeneration` under a `Connection` (`ess/domains/credentials.yaml:41-45`, `kind: references`, `cardinality: one`, `via: connection_ref`) or a custody record with no connection owner |
| `McpInboundSession → McpCaller` | whether a caller is an entity in this repository at all |

Two of the census entries are not `UNMAPPED:` markers but filed questions, because
they are decisions rather than modelling gaps:
`decision-blocker:mcp-caller-connection-assignment` holds `McpCaller →
connectors.auth_bindings.Connection`, and
`decision-blocker:mcp-outbound-stdio-process-ownership` holds `McpServerBinding →
supervised OS process`. This model must not answer either by writing a
cardinality; where their nouns appear it carries the marker and names the blocker.

## The three credential kinds

`epic:mcp-contracts` requires them kept apart: "Caller credentials for inbound
access, credentials for an outbound MCP server and underlying provider credentials
are distinct." Three distinct declared types, never one polymorphic credential
value, is how a later reviewer can see that a story did not quietly collapse them.
This is the one place in the decomposition where a single document may name both
directions, because keeping the three apart is exactly a statement about the
boundary between them; it declares no behaviour for either direction.

## Cross-root limits this model cannot escape

`adapters/README.md` forbids importing a shared type into this root, and a survey
of the six existing adapter ESS roots finds none doing it — `grep -rn
'connectors\.\(auth_bindings\|declarations\|credentials\|mutations\|service_wire\)'
adapters/*/spec/ess/domains/*.yaml` returns nothing (inferred from that grep over
`adapters/atlassian`, `docker`, `gitlab`, `grafana`, `kubernetes` and `loki`; no
`ess/1` document states the prohibition as a relation rule). So the MCP↔shared
relations above could not be declared here even if somebody decided them; the
projection is an adapter binding obligation recorded in prose in the contract
documents, and the markers say so.

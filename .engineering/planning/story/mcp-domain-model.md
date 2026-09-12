---
format: aep.planning-md/1
id: story:mcp-domain-model
kind: story
status: implemented
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
revision: 9
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
a preference; `adapters/README.md:67-71` states the rule: "Each authored
native model lives in `spec/ess` with namespace `connectors_<owner>.<domain>`.
Shared ESS includes none of these roots and imports none of their types. Each
compiles independently. Native-to-shared projection is an explicit adapter binding
obligation, not a cross-root ESS import or arbitrary JSON bag." The boundary gate
enforces it mechanically: `adapters/README.md`'s `## Boundary gate` section says
the scan runs "against reviewed terminology plus every adapter directory name",
so creating `adapters/mcp/` makes `mcp` a term shared ESS may not carry.
`crates/connectors-build/ess-boundary.json` currently lists `sip` as its only
`shared_protocol_names` entry, which is what an exception would have to look like.

*(This citation read `:66-70` at revisions 1 to 4, and it was right when it was
written. `story:mcp-specification-pin` — a sibling in this same decomposition —
added the MCP row to that table, which moved every following line down by one.
This is citation drift caused by a unit of the same epic rather than an error of
authorship, and it is the second such drift this epic has produced: the pin story
cited the initiative at `:35` for a sentence at `:33`. A `path:line` into a file
another story in the same wave will edit is a citation with a half-life.)*

## Domain relations

This story is where the relation census lands, so it carries the whole of it.

**Relations it can cite, and therefore states:**

- `McpAdvertisedCapability → connectors.declarations.OperationDeclaration`,
  **one** advertised capability to the one operation it projects. The
  `AdapterSpecification` owns the declaration, and an advertised MCP capability is
  a projection of an owned declaration and never a second record of it — which is
  what the epic asks for: "Reuse established owners instead of making MCP the
  universal internal contract."

  *(Corrected at revision 6. This entry cited
  `ess/domains/declarations.yaml:119-123` for a cardinality of `many`. The
  adversary pass over this story checked that block: it declares
  `AdapterSpecification.operations` — `AdapterSpecification owns many
  OperationDeclaration` — which is a different pair, so it answers this edge
  neither way and `many` was chosen rather than read. The cardinality that is
  readable is the one this repository's own invariant gives — **for a `references`
  relation**, which this edge is: a `cardinality: many` reference carries a
  `List<…>` via field and a `cardinality: one` carries a scalar, across
  `ess/domains/artifact_provenance.yaml:85-89`,
  `ess/domains/auth_bindings.yaml:102-107` and
  `ess/domains/credentials.yaml:40-45`. The carrier is scalar, so the edge is
  `one`.)*

  *(The `references` scope was added at revision 7 and it is load-bearing. The
  second adversary pass over this story enumerated all 40 `via` relations in
  shared ESS and found two that carry `cardinality: many` through a scalar:
  `ess/domains/declarations.yaml:119-123` and
  `ess/domains/discovery_state.yaml:85-89`. Both are `kind: owns`, where `via`
  names the child's back-reference to the owner's identity — `adapter_id` and
  `collection_ref`, both `String` — so the carrier says nothing about how many
  children there are. Stated unscoped at revision 6, the invariant condemned two
  correct shared declarations, one of them the very block this paragraph
  withdraws.)*

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
supervised OS process`.

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

## The marker this model still owes

A second `UNMAPPED:` marker is owed on `ServerCapabilityKind`, and this story owns
it.

`story:mcp-profile-selection-matrix` defers `capability:server/tasks` and
`capability:server/extensions` against the marker this model attaches to
`ClientCapabilityKind` — "whether `tasks` is modelled as a client capability of
this root or as an extension identifier". Its adversary pass 2 measured what that
costs: the marker poses only the client-side question, `ServerCapabilityKind`
omits `tasks` with no marker of its own, and the interop schema declares
`ServerCapabilities.tasks` with **different children** from the client one. So
answering the client-side question clears the client row and settles neither
server row, while both read as cleared.

The matrix has recorded the inheritance in a stated table with a check that
derives the set from this model and requires exact equality, so a future
cross-side deferral fails. That records the debt; it does not pay it.

The durable fix is a second marker on `ServerCapabilityKind` posing the
server-side question in its own words. That text is this story's, not the
matrix's, and it is filed here rather than written into the matrix so the model
stays the single place a reader looks for what this root could not read.

*Recorded 2026-09-12, after this story reached `implemented`. It is an obligation
this model carries, not a defect in the work that closed — the marker that exists
is correct for the side it names.*

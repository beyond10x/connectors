---
format: aep.planning-md/1
id: story:mcp-inbound-cloud-profile
kind: story
status: draft
title: Specify the cloud-capable inbound MCP profile and its no-secret-disclosure fixture
relations:
- decomposes: epic:mcp-contracts
- serves: vision:independent-contract-adapters
- depends_on: story:mcp-inbound-local-binding
scope:
- confidence: inferred
  path: adapters/mcp/contracts/server/v1alpha1/cloud-profile.md
- confidence: inferred
  path: adapters/mcp/contracts/server/v1alpha1/fixtures
revision: 2
---
## Acceptance

`adapters/mcp/contracts/server/v1alpha1/cloud-profile.md` specifies the
cloud-capable inbound placement — that inbound access is authenticated before any
capability is named, the transport and session lifetime, streaming behaviour and
every failure mode — together with a fixture proving that no provider secret, and
no value derived from one, appears in any MCP payload the profile emits.

## Scope

- `adapters/mcp/contracts/server/v1alpha1/cloud-profile.md` — new. No other story
  writes this file.
- `adapters/mcp/contracts/server/v1alpha1/fixtures/` — new; the
  no-secret-disclosure fixture inputs.

## Domain relations

None it states, and one it is forbidden to state. `McpCaller →
connectors.auth_bindings.Connection` is held by
`decision-blocker:mcp-caller-connection-assignment`, and this document must not
answer it. The consequence is visible in what this story covers and what it does
not: the profile specifies that inbound access is authenticated and that secrets
do not leave, both of which hold for any number of callers; it specifies nothing
about which caller reaches which connection, which is what caller isolation means.

The document's own status line says this, so a reader six months from now does not
mistake "authenticated" for "isolated".

## What this story must establish

`epic:mcp-contracts` requires "a cloud-capable server binding exposed by `$BIN
server`" with "caller authentication/authorization where required by the
placement", and acceptance criterion 3 reads: "The cloud server profile specifies
authenticated inbound access, transport/session lifetime, streaming and failure
behavior. A fixture verifies caller isolation and no provider-secret disclosure;
this epic does not authorize deployment."

**The local baseline is not allowed to move.** The epic's operator request is
explicit: "The local CLI with persistent local credentials remains the baseline.
Cloud serving is an optional placement; it must not impose a hosted identity
service or federation on local use." Anything this profile requires must be inert
when the placement is local, and the document states which requirement that is,
one by one.

**Specifying is not depending.** `review-result:concept-stack-integration-20260908`
gap G2 records the current fact: admission today is "One shared static bearer,
constant-time compare (`crates/connectors-host/src/server.rs:57-70`); no Identity
dependency (`Cargo.lock`: 0 hits for `identity-client`)". The proposed alternative
lives in `contracts/service/v1alpha2/semantics.md`, indexed in
`contracts/README.md` as "verified caller context, admission, policy, audit". This
document states the authentication requirement the placement imposes and names
which of these two could satisfy it; choosing between them is the caller-identity
half of the blocker and is not decided here.

**No secret leaves, and the fixture is what proves it.** The repository's existing
discipline is quotable: `ess/domains/credentials.yaml:36` marks a captured
credential snapshot "Private only: must not enter a public evidence projection or
diagnostic", and `ess/domains/service_wire.yaml:41-46` shows the shape of a safe
projection — an opaque instance-qualified correlation value rather than the record
itself. The fixture asserts the same property over MCP payloads: error content,
tool descriptions, resource bodies and progress notifications alike.

## What it does not cover

**Caller isolation.** Not drafted: the multi-caller assignment of callers to
connections, and the isolation fixture that would assert it, are held by
`decision-blocker:mcp-caller-connection-assignment`, because
`ess/domains/declarations.yaml:180-181` declines the cardinality
("multi-tenant SaaS ownership/assignment cardinality remains outside this local,
explicitly configured service slice. No tenant is caller-selected") and
`ess/domains/mutations.yaml:110` and `ess/domains/idempotency.yaml:72-73` decline
the principal for their own records. Acceptance criterion 3's isolation half is
therefore the blocker, not a gap in this decomposition.

No deployment is authorized, prepared or described as ready. The epic says so and
this document repeats it.

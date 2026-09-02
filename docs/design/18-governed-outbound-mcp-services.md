# 18 — Governed outbound MCP services

Status: accepted 2026-09-02. Implements the outbound half of the MCP foundation decision and
advances S-075 without claiming its OCI-pack loader.

## Decision

Connectors talks outward to an MCP server through a generic Integration adapter. That adapter is
not the place Harness gets MCP tools: Harness consumes the shared MCP client foundation directly,
because Harness already owns per-run publication, approval, hooks, deadlines and result bounds.
Routing Harness through Connectors would require a Connector deployment for a local stdio server
and would conflate Harness's run authority with durable Connection and Grant authority.

The two consumers therefore share protocol machinery, not policy:

- the public MCP foundation owns tools-only lifecycle negotiation, stdio/Streamable HTTP codecs,
  frozen snapshots, lossless results and named bounds;
- Harness owns a local reviewed run profile and composes admitted tools into its `ToolPort`;
- Connectors owns an outbound generated service whose reviewed profile and deployment join the
  existing catalog, Connection, Grant, approval, secret-custody and egress boundaries.

Connectors' existing hosted `/mcp` route remains a separate inbound transport. Its three meta-tools
enter existing admission seams; it is not used recursively for outbound MCP.

## Reviewed profile

`McpServiceProfile` is strict data (`deny_unknown_fields`) carrying:

- the exact `b10x.connector-mcp-service.v1` contract identity;
- stable service and Connection references plus local provider display facts;
- the complete `ToolSnapshot`, including the negotiated protocol, every raw descriptor and its
  SHA-256 digest;
- a one-to-one mapping of every remote tool name to a Connectors-owned operation reference, title,
  description and effect.

Preparation reconstructs the snapshot digest, connects through Connector egress, freezes the live
tool list, and requires byte-for-byte typed equality. A missing, added or changed descriptor—or a
protocol revision change—refuses before an activatable factory exists. Subsetting is deliberately
not implicit: every remote tool must receive a reviewed local mapping. A deployment may then hide a
mapped operation with `expose = false`, but a newly appearing remote tool never becomes available
by accident.

Server prose and annotations are retained for evidence only. They do not choose local names,
effects, risk, approval, idempotency, grants, audiences or exposure. The local profile owns the
first four service facts; the ordinary `ServiceDeployment` owns permanent provider identity and
every deployment policy fact.

## Activation and invocation

Registering or preparing a factory is inert. Binding requires every operation to carry exactly the
reviewed `mcp_connection` endpoint reference, the exact optional `bearer` credential reference, and
at least one Grant reference. `ServiceBundleBuilder` additionally requires the exact operation set
and rejects provider, authority and operation collisions.

Describe leases hash the frozen snapshot, stable caller-authority seed, operation and Connection.
Invoke refuses a stale lease or wrong Connection, validates input against the frozen JSON Schema,
and then calls the mapped remote name. The full MCP call result remains JSON; `isError` becomes a
typed Connector failure, and oversize results remain the existing named `result_too_large`
refusal. No remote diagnostic is copied into caller-visible errors.

## Transport and credentials

The shared client release is pinned by both exact version and git revision. Its injected
Streamable HTTP client is implemented by `integration-mcp`; the implementation itself owns no
socket. It submits every POST and session deletion through `EgressTransport` using the reviewed
endpoint binding, so the server implementation retains HTTPS, DNS-after-resolution, destination,
request and response bounds.

An optional bearer is addressed by `CredentialRef`, never stored in the profile or deployment.
The adapter fetches it from `SecretStore` for each exchange and drops the secret wrapper after the
bounded request has been assembled. OAuth acquisition and refresh are Connector-owned producers
into that store; this adapter is only the spender and does not read the MCP foundation's local XDG
OAuth state.

`EgressTransport` is a bounded exchange port, not a long-lived response-body capability. Outbound
MCP therefore supports tools-only request/response over JSON or bounded SSE POST responses and
explicitly declines the optional server-push GET stream. Adding durable server notifications would
require a supervised Connector channel design, not an ambient socket inside this adapter.

## Consequences

- One MCP codec and bounds implementation serves Harness and Connectors without sharing authority.
- Generic MCP does not bypass the catalog; its immutable profile is a small generated service
  contribution and its deployment overlay is mandatory.
- Local stdio remains appropriate for Harness. Hosted Connectors outbound MCP is Streamable HTTP;
  spawning an arbitrary deployment process would violate the runtime's socket/process boundary.
- This provides the verified dynamic-service join needed by external deployment packs, but S-075
  remains open until hosted configuration loads such profiles from immutable OCI digests with
  provenance and merge verification.

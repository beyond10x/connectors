---
format: aep.planning-md/1
id: epic:mcp-contracts
kind: epic
status: draft
title: Contractualize outbound and inbound MCP, including auth and local/cloud server bindings
relations:
- informed_by: specification:recent-agent-adapter-usage-20260909
- informed_by: specification:contract-driven-connectors-design
- informed_by: review-result:concept-stack-integration-20260908
revision: 1
---
## Operator request

Track full MCP contractualization in this repository. The local CLI must connect to MCP servers, including authentication. The CLI must also expose MCP to local clients and, when deployed in a cloud, through `$BIN server`. This is an explicit product requirement recorded on 2026-09-09, not an inference that any current adapter already implements MCP.

The local CLI with persistent local credentials remains the baseline. Cloud serving is an optional placement; it must not impose a hosted identity service or federation on local use. Outbound use of a remote MCP server and serving inbound MCP are separate directions with separate trust boundaries.

## Required contract deliverables

- Outbound MCP: discovery and explicit selection of a server; authenticated connection and persistent local credential use/repair; initialization/version and capability negotiation; invoking selected tools and accessing other selected MCP capabilities; truthful results, errors, limits and shutdown.
- Inbound MCP: an explicitly selected local-client binding and a cloud-capable server binding exposed by `$BIN server`; caller authentication/authorization where required by the placement; discoverable Connectors capabilities and invocations with their existing target, permission, mutation and credential-locality guarantees.
- Pin the authoritative MCP specification revision and select supported transport/capability profiles at authoring. Evaluate local stdio and deployed HTTP profiles explicitly; specify framing, streaming, cancellation, progress, connection/session loss and version mismatch for the selected profiles. Do not equate a protocol capability listing with implemented support.
- Specify tools, resources, prompts and optional reverse-direction capabilities separately: supported, explicitly refused or deferred, with reasons. Tool annotations or remote descriptions cannot establish local write authority. Preserve provider errors versus protocol errors, result content/structured output and output bounds.
- Specify auth for both directions, including protected credential entry/acquisition, persistence, expiry/refresh/repair, revocation and failure. Credential custody is replaceable and local by default. Caller credentials for inbound access, credentials for an outbound MCP server and underlying provider credentials are distinct.
- Define mappings to existing Connectors service, records, session/execution, auth, discovery and mutation contracts where their guarantees apply. Reuse established owners instead of making MCP the universal internal contract. Keep MCP wire/native details with the owning adapter/binding so it can be extracted independently.
- Describe composition when a local or deployed MCP server exposes an outbound MCP-backed capability. Preserve caller/target provenance, limits and admission through the mapping; prohibit implicit credential forwarding, account fallback or authority escalation from discovered metadata.
- Provide a human CLI journey and machine-readable discovery/error contract. `$BIN server` is the requested entry-point intent; finalize exact flags and transports during contract authoring rather than treating this tracking record as a implemented CLI promise.

## ESS and review workflow

Read the design handoff and reuse existing typed ownership/lifecycles. Author the selected textual profiles and applicable ESS values, entities, commands, events, relations and lifecycle scenarios before implementation decomposition. UNMAPPED: durable MCP-specific state, ownership, transport-profile selection and any unsupported ESS semantics must be resolved by that work, not guessed in this epic. No new domain/entity or executable adapter is introduced by this tracking action.

Review wire/protocol fidelity and security/credential boundaries, then prove mapping and runtime conformance separately. A schema compile or lifecycle simulation does not prove network interoperability, provider effects, persistent credentials or cloud authentication.

## Acceptance

1. Outbound local CLI connects to a selected MCP fixture, authenticates, invokes an advertised capability, exits/restarts and reuses its persisted credential binding. Missing, expired, revoked or unavailable credentials produce distinct safe outcomes.
2. A local MCP client connects to the selected `$BIN server` local binding, discovers only supported/admitted capabilities and receives correctly mapped results and errors. No cloud control plane is necessary.
3. The cloud server profile specifies authenticated inbound access, transport/session lifetime, streaming and failure behavior. A fixture verifies caller isolation and no provider-secret disclosure; this epic does not authorize deployment.
4. Selected protocol versions, transports and optional capabilities have a coverage matrix. Unsupported functionality is explicitly refused, not silently approximated.
5. Cancellation, malformed input, version/capability mismatch, partial output, lost replies and mutation uncertainty have reviewed scenarios. A repeated MCP request cannot automatically duplicate an uncertain business effect.
6. All selected contractual gaps have owners, applicable ESS declarations/scenarios and independent review evidence before runtime stories are scheduled. CLI/docs describe the final selected behavior for humans and agents.

## Scope and status

Backlog recording only. No MCP connection, server startup, cloud deployment, credential migration or provider operation is authorized by this record. Implementation remains future work after the contracts are selected. The broader recent-usage analysis continues independently.

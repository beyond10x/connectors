---
title: Commands and interfaces
description: Follow a governed invocation and understand the CLI, HTTP, local, MCP, and byte-plane interfaces.
sidebar_label: Commands and interfaces
sidebar_position: 3
b10x:
  documentType: architecture
  audiences: [developer, operator]
---

# Commands and interfaces

Callers discover operations, inspect their requirements, and request an invocation through a
particular Connection. Connectors returns a typed result or refusal. Transports expose these
capabilities in forms suitable for a terminal, application, or agent.

## Follow an invocation

```mermaid
flowchart TB
    discover[Search and describe operation] --> request[Operation, Connection, description reference, input]
    request --> admission[Validate caller and request authority]
    admission --> grant[Check current Grant and description]
    grant --> approval[Verify and redeem approval when required]
    approval --> dispatch[Record attempt and dispatch admitted request]
    credentials[Resolve provider credentials] --> dispatch
    dispatch --> result[Result or typed refusal, with audit reference]
```

This is the common authority path, not a promise that every transport supports every method.
Discovery is scoped to what the caller may see. `describe` supplies the input contract, available
Connections, effect and approval posture, and a description reference. The actual
[invocation request](../../crates/protocol/src/operation.rs) carries `operation_ref`,
`connection_ref`, `description_ref`, `input`, and optional `approval_evidence_ref`.

A description reference binds the call to previously described facts; stale catalog or authority
information can cause refusal. The [hosted enforcement path](../../crates/server/src/hosted/enforcement.rs)
and backend admission checks produce proof-bearing values before provider effects become reachable.
Credential lookup and network execution follow the relevant admission decisions.

An operation can finish in one request or establish a session. Session methods inspect, terminate,
reconcile, or signal an existing execution. A daemon restart can leave an old execution's outcome
unknown; a missing in-memory record does not prove that no effect occurred.

## The CLI at a glance

The [shipped parser](../../crates/connectors-cli/src/lib.rs) defines eight top-level groups:

| Group | Responsibility | Example |
|---|---|---|
| `setup` | Create configuration, connect a provider, install completions | `connectors setup connect slack` |
| `inspect` | Report configuration, provider, and credential readiness | `connectors inspect doctor` |
| `session` | Log in to or out of a hosted deployment | `connectors session logout` |
| `serve` | Run local, hosted, or MCP service entry points | `connectors serve local` |
| `connection` | List, discover, activate, and materialize Connections | `connectors connection list` |
| `event` | Search, receive, and replay events | `connectors event search` |
| `operation` | Search, describe, invoke, and signal operations | `connectors operation search` |
| `admin` | Inspect hosted Integrations and supply administrative credentials | `connectors admin integrations status` |

Run a command with `--help` for its arguments. Use the global `-o json` option for machine-readable
output. Shell completions come from the same parser:

```bash
connectors setup completions fish
```

Version 0.6.0 introduced this grouping. Bare `connectors serve` displays the group; running a local
service requires `connectors serve local`. Use the current paths in scripts. The
[specification status](specification.md) explains why the ESS outline is not the parser's implementation.

## Choose an interface

| Interface | Access | Boundary |
|---|---|---|
| Local CLI/client | Explicit local configuration and runtime, including socket access | Owner-bound deployment; selected one-shot operations work without a standing daemon |
| Hosted HTTP | Configured base, usually `/api/connectors/v1` | Identity-authenticated contracts with route-specific admission |
| Inbound MCP | Hosted `/mcp` or `connectors serve mcp` over stdio | Adapts admitted capabilities; discovery does not bypass grants or approval |
| Outbound MCP | Reviewed remote-tool snapshot mapped to local operations | Provider adapter with deployment policy, Connection-bound egress, grants, and custody |
| Direct byte planes | Dedicated admitted session paths | Voice/media or bounded Git bytes after control-plane admission |

HTTP exposes Operation, Connection, Catalog, Event, Datasource, and administrative contracts.
For hosted clients, `connectors session login` records deployment selection and login continuity.
Explicit local `--config` or `--state-root` options select local operation where supported. The
shared `--target` surface remains incomplete; see [deployment selection](deployment.md#select-the-deployment).

## The deployed HTTP reference

A hosted deployment exposes `{base_path}/docs` and `{base_path}/openapi.json` without login. They
contain its API version, authentication requirements, examples, and refusal codes. The served
document combines a committed route skeleton with schemas generated from Rust protocol types;
the HTML reference renders that document.

The [contract implementation](../../crates/server/src/hosted/docs.rs) and
[route/example tests](../../crates/server/src/hosted/tests/docs.rs) connect those surfaces.
Internal Git-fetch routes deliberately remain outside public OpenAPI and MCP.

[Previous: Connections and authority](authority.md) · **Next:** [Events and durable state](events.md)

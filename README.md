---
title: Connectors
description: Understand the system that connects agents to external services while keeping credentials, permissions, and execution under explicit control.
sidebar_label: Overview
sidebar_position: 1
b10x:
  documentType: overview
  audiences: [evaluator, developer, operator]
---

# Connectors

Connectors gives applications and agents a common way to discover external operations, connect
accounts, invoke permitted actions, and receive provider events. It keeps provider credentials
inside the service and checks each request against the caller's authority and selected Connection.

**Current status: pre-v1.** Personal-local and bounded hosted deployments are implemented. Available
operations depend on the deployment, provider adapter, Connection, and Grant. Full SaaS and
satellite federation remain outside the current support claim.

## See the system

```mermaid
flowchart TB
    caller[Application or agent] --> entry[CLI, HTTP, or MCP]
    catalog[Reviewed provider catalog] --> service[Connectors service]
    entry --> service
    identity[Identity: hosted caller authority] --> service
    service --> custody[Credential and state stores]
    service --> adapter[Admitted provider adapter]
    adapter --> provider[External service]
    provider --> events[Normalized, admitted events]
    events --> caller
```

The catalog describes what can be called. A deployment enables an Integration; authorization
creates a Connection. A Grant determines which operations or events a caller may use through that
Connection. Connectors binds credentials at the execution boundary and records the result.
Hosted identity comes from [Identity](https://beyond10x.github.io/docs/identity/).

## Understand the architecture

Read the handbook in order, or follow the question you have.

| Your question | Start here |
|---|---|
| What are the subsystems, and who owns what? | [System overview](docs/architecture/README.md) |
| How do accounts, permissions, and credentials fit together? | [Connections and authority](docs/architecture/authority.md) |
| What happens when a caller invokes an operation? | [Commands and interfaces](docs/architecture/interfaces.md) |
| How does a provider event reach a consumer? | [Events and durable state](docs/architecture/events.md) |
| What changes between local and hosted deployments? | [Deployment and runtime](docs/architecture/deployment.md) |
| What is specified, generated, and enforced today? | [Specification status](docs/architecture/specification.md) |

## Connect a provider

The provider guides explain the supported connection paths and their credential requirements.

| Provider | Guide |
|---|---|
| Slack | [Socket Mode, hosted organization access, and personal connections](docs/guides/connect-slack.md) |
| GitLab | [Personal and automation access](docs/guides/connect-gitlab.md) |
| Jira | [Cloud gateway access and hosted delegated OAuth](docs/guides/connect-jira.md) |
| Confluence | [Cloud gateway reads](docs/guides/connect-confluence.md) |
| Kubernetes | [Connect clusters and discover resources](docs/guides/connect-kubernetes.md) |
| Grafana | [Connect a configured Grafana instance](docs/guides/connect-grafana.md) |

For hosted operation, continue with [Integration administration](docs/guides/administer-hosted-integrations.md).
For installation, configuration, and prerequisites, see [Deployment and runtime](docs/architecture/deployment.md).

## Contracts and implementation

A hosted deployment serves its API reference at `{base_path}/docs` and its OpenAPI document at
`{base_path}/openapi.json`. The usual base path is `/api/connectors/v1`. These describe the deployed
binary; use them when implementing an HTTP client.

The handbook describes current behavior and links to its implementation. The
[domain design](docs/design/01-domain-model.md), [architecture design](docs/design/02-architecture.md),
and [outbound MCP design](docs/design/18-governed-outbound-mcp-services.md) retain detailed engineering
decisions and dated amendments. Contributors should read [AGENTS.md](AGENTS.md).

<!-- b10x-docs:start -->
## Documentation

[Connectors documentation](https://beyond10x.github.io/docs/connectors/) · [Start](https://beyond10x.github.io/) · [Ecosystem](https://beyond10x.github.io/ecosystem/) · [Impact](https://beyond10x.github.io/changes/) · [Releases](https://beyond10x.github.io/releases/)
<!-- b10x-docs:end -->

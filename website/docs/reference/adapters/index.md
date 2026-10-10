---
title: Adapters
sidebar_position: 4
description: Every adapter the repository documents, what runs today and what is a design or a typed specification only.
---

# Adapters

Each adapter owns its provider's behaviour, native contracts, specification and fixtures
([Adapters and ownership](../../concepts/adapters.md)). A native contract or a typed model is not
an installed capability: the runtime column says what runs.

## Runtime available

| Adapter | Runs as | Operations |
|---|---|---|
| [GitLab](./gitlab/index.md) | catalog provider | 18 reads and 4 approved writes from the pinned OpenAPI document, and the two merge-request feed reads |
| [Catalog provider](./catalog/index.md) | `connectors-catalog-provider` | Jira, Confluence, HubSpot, Zendesk, Google, Runpod and Slack, from pinned API documents |
| [Kubernetes](./kubernetes/index.md) | `connectors-kubernetes` | `resources.list`, `endpoints.discover`, optionally `hosts.discover` and the Helm release reads |
| [SQL (PostgreSQL and MySQL)](./sql/index.md) | `connectors-sql` | `query.read`, `schema.list`, `database.list`, `table.list`, `table.describe`, `index.list` |
| [Tavily](./tavily/index.md) | `connectors-tavily` | `websearch.search`, `websearch.fetch`, `websearch.crawl` |
| [Loki](./loki/index.md) | `connectors-loki` | `logs.query_range`, `logs.query_metric`, `logs.labels`, directly or through Grafana's data-source proxy |
| [Prometheus](./prometheus/index.md) | `connectors-prometheus` | `series.query`, `series.query_range`, `rules.list`, directly or through Grafana's data-source proxy |
| [Grafana](./grafana/index.md) | `connectors-grafana` | `datasources.list`; discovery and the mediated route are specified only |

## Specification or design only

| Adapter | What exists |
|---|---|
| [Atlassian](./atlassian/index.md) | native document contracts and an ESS model for Jira and Confluence; Jira and Confluence reads run through the catalog provider instead |
| [Docker](./docker/index.md) | native log and mutation contracts and an ESS model |
| [Alertmanager](./alertmanager/index.md) | a design |
| [SIP](./sip/index.md) | a native dial contract |
| [RTVBP](./rtvbp/index.md) | a native session contract |

MCP has specified invocation, projection, auth lifecycle, mutation replay and composition
contracts in the repository's `adapters/mcp/` directory and no runtime. Slack and WebRTC have no
native adapter documentation; Slack conversations are read through the catalog provider.

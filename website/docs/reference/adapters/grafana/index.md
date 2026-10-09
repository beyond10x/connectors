---
title: Grafana
sidebar_position: 9
description: Data-source records through a saved service-account connection, and Loki reads through Grafana's proxy.
---

# Grafana

List the data sources a Grafana service account can read, and reach a Loki data source through
Grafana.

## Current capabilities

Grafana runs as the native executable `connectors-grafana` under the local CLI, with one read
through a saved connection:

| Operation | What it reads |
|---|---|
| `datasources.list` | the data sources the token can read, one `GET /api/datasources`, unpaged, at most 1,000 records |

Each record carries the data source's `uid`, `name`, plugin `type`, `access` mode (`proxy` or
`direct`) and `is_default`, and nothing else: backend URLs, users, databases, settings and
secure fields never reach a result.

A connection uses the profile `grafana.service_account`, a Grafana service-account token.
Connect, repair and `connections revalidate` prove the token with `GET /api/datasources`; the
identity is the configured connection. The configuration is HTTPS-only, may carry the sub-path
Grafana is served under, and takes an optional private CA file.

## Loki through Grafana

A Loki data source is read through Grafana as a [Loki](../loki/index.md) connection whose
`base_url` is `https://<grafana>/api/datasources/proxy/uid/<uid>/`, with the Grafana token as
its bearer token. `logs.query_range`, `logs.query_metric` and `logs.labels` then answer through
Grafana's data-source proxy.

## Limits

- Grafana, not Connectors, decides which data sources the token reaches; there is no allowlist
  of types or uids yet.
- Dashboard reads, datasource discovery and the mediated route of the native design are not
  built. A discovered data source would be information, not authority to use its provider; a
  child adapter does not import Grafana implementation details.

The operator guide is `adapters/grafana/README.md` in the repository.

## Native contract reference

- [Grafana datasource discovery](./contracts/discovery.md)
- [Grafana mediated routes](./contracts/routes.md)
- The [typed native model](./model/index.md), generated from `adapters/grafana/spec/ess`

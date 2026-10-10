---
title: Prometheus
sidebar_position: 10
description: PromQL instant and range queries and rule reads through a saved bearer connection, directly or through Grafana.
---

# Prometheus

PromQL series semantics through a shared datasource boundary.

The native profiles keep PromQL semantics with the provider and specialize the shared
[time series contract](../../contracts/data/series.md): instant and range queries with time and
step bounds, series identity and partial results, and a records profile for rules.

## Current capabilities

Prometheus runs as the native executable `connectors-prometheus` under the local CLI, with three
reads, each one GET against Prometheus's HTTP API through a saved connection:

| Operation | What it reads |
|---|---|
| `series.query` | one PromQL expression at one instant (default now): a vector, a matrix, a scalar or a string, at most 500 series |
| `series.query_range` | one PromQL expression over at most seven days at an explicit step, at most 2,000 points per series |
| `rules.list` | the alerting and recording rules, with each alerting rule's state, at most 2,000 rules |

Sample values stay Prometheus's strings (`NaN`, `+Inf`). A provider warning makes the result
partial and is returned with it.

A connection uses the bearer profile `prometheus.bearer`. Connect, repair and
`connections revalidate` prove the token with `GET api/v1/status/buildinfo`, which runs no query;
Prometheus names no account, so the identity is the configured connection. The configuration is
HTTPS-only, may carry a path prefix, and takes an optional private CA file.

## Through Grafana

A Prometheus data source behind Grafana is read with the same three operations. The connection's
`base_url` is the data source's Grafana proxy path,
`https://<grafana>/api/datasources/proxy/uid/<uid>/`, and its `prometheus.bearer` token is a
Grafana service-account token. The probe and every read go below that prefix. The uid comes from
the [Grafana adapter's](../grafana/index.md) `datasources.list`.

## Limits

- No `X-Scope-OrgID` tenant header is sent, so a backend that requires one from the client is
  not reachable.
- No query `timeout` parameter is sent; the host's transport deadline bounds each call.
- Too many samples is answered by Prometheus as an execution error and reaches the caller as
  `upstream_protocol`, not as a capacity error.
- Every query reads everything the token reaches; a matcher scope is not supported.
- A Prometheus without authentication cannot be connected, because the local host admits no
  credential-less profile.

The operator guide is `adapters/prometheus/README.md` in the repository.

## Native contract reference

- [Prometheus PromQL](./contracts/series.md), whose §11 states the reads and the connection
- The [typed native model](./model/index.md), generated from `adapters/prometheus/spec/ess`

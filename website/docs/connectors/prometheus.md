---
title: Prometheus
sidebar_position: 20
description: Run PromQL instant and range queries and list rules against Prometheus with a bearer token, directly or through Grafana's data-source proxy.
lede: One Prometheus instance per deployment and tenant runs instant and range PromQL queries and lists alerting and recording rules.
source: adapters/prometheus/README.md, adapters/prometheus/contracts/series/v1alpha1/semantics.md, adapters/prometheus/tests/local_runtime.rs
connectors: "prometheus"
---

# Prometheus

The `prometheus` connector is served by its own executable, `connectors-prometheus`. The words
connector, instance, connection and operation are those of the
[README vocabulary](https://github.com/beyond10x/connectors#vocabulary); the
[full Prometheus guide](https://github.com/beyond10x/connectors/blob/main/adapters/prometheus/README.md)
has the bounds and refusals.

No declaration ships for it, so the instance is pinned: it names its executable, that
executable's SHA-256 and the configuration revision the executable prints. Commands that need a
Prometheus are shown without output.

## What it reads and writes

| Reads | Writes |
|---|---|
| `series.query`, `series.query_range`, `rules.list` | none |

## Prerequisites at the provider

A bearer token the Prometheus, or the gateway in front of it, accepts. A Prometheus without
authentication cannot be connected. To read through Grafana instead, use a Grafana
service-account token and the data source's proxy path as `base_url`. Build the executable:

```sh
cargo build --release --locked -p connectors -p connectors-prometheus
```

## Enable the connector and add an instance

Write an owner-only native file, `prometheus.json`; `query_scope` admits only the empty list:

```json
{
  "format": "connectors-prometheus-local/1",
  "instance": "prometheus-prod",
  "base_url": "https://prometheus.example.internal/",
  "query_scope": {"allowed_matchers": []}
}
```

Print its bootstrap, hash the executable, and pin both in the instance:

```sh
target/release/connectors-prometheus --local-config /absolute/path/prometheus.json --print-local-bootstrap
sha256sum target/release/connectors-prometheus
```

```toml
[connectors.prometheus]
enabled = true

[connectors.prometheus.instances.metrics]
alias = "metrics"
instance_id = "prometheus-prod"
adapter_id = "prometheus"
configuration_revision = "REPLACE_FROM_BOOTSTRAP"
protocol = "v1alpha1"
private_protocol = "connectors-private/1"
startup = "on-demand"
restart = "never"

[connectors.prometheus.instances.metrics.permissions]
profiles = ["prometheus.bearer"]
operations = ["series.query", "series.query_range", "rules.list"]

[connectors.prometheus.instances.metrics.executable]
path = "/absolute/path/target/release/connectors-prometheus"
sha256 = "REPLACE_WITH_EXECUTABLE_SHA256"
args = ["--local-config", "/absolute/path/prometheus.json"]
```

## Connect

```sh
connectors connections connect --connector prometheus --instance metrics \
  --profile prometheus.bearer --credential-prompt
```

Connecting proves the token with `GET api/v1/status/buildinfo`, which runs no query.

## A first read

```sh
connectors invoke prometheus/metrics series.query \
  --input-json '{"query":"up{job=\"api\"}"}'
```

An omitted `time_unix_s` is now. Sample values stay Prometheus's strings.

## A first guarded write

The connector has no write, so nothing here needs an approval.

## Reconnect when the connection is Stale

A token Prometheus refuses is reported as `unauthorized` when next used, and the connection turns
`stale`. Publish a new token:

```sh
connectors --output json connections list --connector prometheus --instance metrics \
  | jq -c '[.result.connections[] | {connection, revision, state}]'
connectors connections reconnect --connector prometheus --instance metrics \
  --expected-revision "<revision from the list>" --credential-prompt
```

## Limits

- A range spans at most seven days and 2,000 points per series; a wider one is refused before any
  request.
- No tenant header is sent; a token grants no scopes and exposes no expiry.
- Every query reads everything the token reaches; no query scope can be configured yet.

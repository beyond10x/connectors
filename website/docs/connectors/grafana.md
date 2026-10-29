---
title: Grafana
sidebar_position: 19
description: List Grafana data sources with a service-account token, and read Grafana IRM alert groups and incidents on a second instance.
lede: One Grafana instance lists the data sources a service account can read; a second instance with its own token reads IRM alert groups and incidents.
source: adapters/grafana/README.md, adapters/grafana/spec/ess/domains/records.yaml, adapters/grafana/spec/ess/domains/irm.yaml
connectors: "grafana"
---

# Grafana

The `grafana` connector is served by its own executable, `connectors-grafana`. The words
connector, instance, connection, auth profile and operation are those of the
[README vocabulary](https://github.com/beyond10x/connectors#vocabulary); the
[full Grafana guide](https://github.com/beyond10x/connectors/blob/main/adapters/grafana/README.md)
has the records, windows and refusals. To query a Loki or Prometheus data source through Grafana,
use a [Loki](./loki.md) or [Prometheus](./prometheus.md) instance on the data source's proxy path.

No declaration ships for it, so the instance is pinned: it names its executable, that
executable's SHA-256 and the configuration revision the executable prints. Commands that need a
Grafana are shown without output.

## What it reads and writes

| Profile | Reads | Writes |
|---|---|---|
| `grafana.service_account` | `datasources.list` | none |
| `grafana.irm_service_account` | `irm.alert_groups.list`, `irm.alert_group.get`, `irm.incidents.list`, `irm.incident.get` | none |

An instance holds one connection under one auth profile, so the IRM reads are a second instance.

## Prerequisites at the provider

A Grafana service account allowed to read data sources (`datasources:read`) and a token for it.
For IRM on a Grafana Cloud stack, a token whose role grants the IRM alert-group and incident reads,
and the stack's IRM API URL from IRM's settings. Build the executable:

```sh
cargo build --release --locked -p connectors -p connectors-grafana
```

## Enable the connector and add an instance

Write an owner-only native file, `grafana.json`; add `irm_api_url` for the IRM instance's own file:

```json
{
  "format": "connectors-grafana-local/1",
  "instance": "grafana-prod",
  "base_url": "https://grafana.example.internal/"
}
```

Print its bootstrap, hash the executable, and pin both in the instance:

```sh
target/release/connectors-grafana --local-config /absolute/path/grafana.json --print-local-bootstrap
sha256sum target/release/connectors-grafana
```

```toml
[connectors.grafana]
enabled = true

[connectors.grafana.instances.grafana]
alias = "grafana"
instance_id = "grafana-prod"
adapter_id = "grafana"
configuration_revision = "REPLACE_FROM_BOOTSTRAP"
protocol = "v1alpha1"
private_protocol = "connectors-private/1"
startup = "on-demand"
restart = "never"

[connectors.grafana.instances.grafana.permissions]
profiles = ["grafana.service_account"]
operations = ["datasources.list"]

[connectors.grafana.instances.grafana.executable]
path = "/absolute/path/target/release/connectors-grafana"
sha256 = "REPLACE_WITH_EXECUTABLE_SHA256"
args = ["--local-config", "/absolute/path/grafana.json"]
```

The IRM instance, `irm`, is the same table with its own `alias`, `instance_id`, revision and
native file, the profile `grafana.irm_service_account` and the four IRM reads.

## Connect

```sh
connectors connections connect --connector grafana --instance grafana \
  --profile grafana.service_account --credential-prompt
```

Connecting proves the token with `GET /api/datasources`. The identity is the configured instance,
not a Grafana account.

## A first read

```sh
connectors invoke grafana/grafana datasources.list --input-json '{}'
```

Each record carries `uid`, `name`, `type`, `access` and `is_default`, nothing else.

## A first guarded write

The connector has no write: nothing acknowledges, resolves or silences, so nothing here needs an
approval.

## Reconnect when the connection is Stale

A revoked or expired token is reported as `unauthorized` when next used, and the connection turns
`stale`. Publish a new token:

```sh
connectors --output json connections list --connector grafana --instance grafana \
  | jq -c '[.result.connections[] | {connection, revision, state}]'
connectors connections reconnect --connector grafana --instance grafana \
  --expected-revision "<revision from the list>" --credential-prompt
```

Any token Grafana accepts is the same identity: use one instance per Grafana organization.

## Limits

- At most 1,000 data sources, unpaged; `complete` is `false` when Grafana listed more.
- That an incident's OnCall reference is the alert group's id is inferred, not confirmed against a
  real stack.
- Dashboard reads, data-source discovery and the mediated route are not implemented.

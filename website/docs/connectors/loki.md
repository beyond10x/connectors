---
title: Loki
sidebar_position: 18
description: Query Loki log lines, metrics and labels with a bearer token, directly or through Grafana's data-source proxy.
lede: One Loki instance per deployment and tenant reads log lines, metric queries and labels, and the line and trace profiles you configure.
source: adapters/loki/README.md, adapters/loki/contracts/logs/v1alpha1/semantics.md, adapters/loki/tests/local_runtime.rs
connectors: "loki"
---

# Loki

The `loki` connector is served by its own executable, `connectors-loki`. The words connector,
instance, connection and operation are those of the
[README vocabulary](https://github.com/beyond10x/connectors#vocabulary); the
[full Loki guide](https://github.com/beyond10x/connectors/blob/main/adapters/loki/README.md) has
the profiles, bounds and refusals.

No declaration ships for it, so the instance is pinned: it names its executable, that
executable's SHA-256 and the configuration revision the executable prints. Commands that need a
Loki deployment are shown without output.

## What it reads and writes

| Reads | Writes |
|---|---|
| `logs.query_range`, `logs.query_metric`, `logs.labels`, and through profiles you configure, `logs.profile_lines` and `logs.call_trace` | none |

## Prerequisites at the provider

A bearer token the Loki deployment, or the gateway in front of it, accepts. No tenant header is
sent, so a multi-tenant Loki must derive the tenant from the token. To read through Grafana
instead, use a Grafana service-account token and the data source's proxy path as `base_url`.
Build the executable:

```sh
cargo build --release --locked -p connectors -p connectors-loki
```

## Enable the connector and add an instance

Write an owner-only native file, `loki.json`; `query_scope` admits only the empty list:

```json
{
  "format": "connectors-loki-local/1",
  "instance": "loki-prod",
  "base_url": "https://loki.example.internal/",
  "query_scope": {"required_equalities": []}
}
```

Print its bootstrap, hash the executable, and pin both in the instance:

```sh
target/release/connectors-loki --local-config /absolute/path/loki.json --print-local-bootstrap
sha256sum target/release/connectors-loki
```

```toml
[connectors.loki]
enabled = true

[connectors.loki.instances.logs]
alias = "logs"
instance_id = "loki-prod"
adapter_id = "loki"
configuration_revision = "REPLACE_FROM_BOOTSTRAP"
protocol = "v1alpha1"
private_protocol = "connectors-private/1"
startup = "on-demand"
restart = "never"

[connectors.loki.instances.logs.permissions]
profiles = ["loki.bearer"]
operations = ["logs.query_range", "logs.query_metric", "logs.labels", "logs.profile_lines", "logs.call_trace"]

[connectors.loki.instances.logs.executable]
path = "/absolute/path/target/release/connectors-loki"
sha256 = "REPLACE_WITH_EXECUTABLE_SHA256"
args = ["--local-config", "/absolute/path/loki.json"]
```

## Connect

```sh
connectors connections connect --connector loki --instance logs \
  --profile loki.bearer --credential-prompt
```

Connecting proves the token with `GET /loki/api/v1/labels`. The identity is the configured
instance, not a Loki account.

## A first read

```sh
connection=$(connectors --output json connections list --connector loki --instance logs \
  | jq -r '.result.connections[0].connection')
schema=$(connectors --output json operations describe --connector loki --instance logs \
  --operation logs.query_range | jq -r .result.schema)
revision=$(connectors --output json operations describe --connector loki --instance logs \
  --operation logs.query_range | jq -r .result.revision)
connectors operations invoke --connector loki --instance logs --connection "$connection" \
  --operation logs.query_range --schema "$schema" --revision "$revision" \
  --input-json '{"query":"{app=\"api\"} |= \"error\"","start_unix_ns":"1788822000000000000","end_unix_ns":"1788825600000000000","limit":100}'
```

Validation lasts 60 seconds; a scheduled caller runs `connections revalidate` before reading.

## A first guarded write

The connector has no write, so nothing here needs an approval.

## Reconnect when the connection is Stale

A token the deployment no longer accepts is reported as `unauthorized` when next used, and the
connection turns `stale`. Publish a new token:

```sh
connectors --output json connections list --connector loki --instance logs \
  | jq -c '[.result.connections[] | {connection, revision, state}]'
connectors connections reconnect --connector loki --instance logs \
  --expected-revision "<revision from the list>" --credential-prompt
```

Any token the deployment accepts is the same identity: reconnect only with a token of the same
tenant.

## Limits

- Every query reads the whole tenant the token reaches; no query scope can be configured yet.
- No tenant header is sent.
- A token grants no scopes and exposes no expiry.
- After rebuilding the executable, update `sha256` and stop the running process before the next
  command.

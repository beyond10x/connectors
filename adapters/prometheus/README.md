# Prometheus through the local CLI

The `prometheus` connector answers three PromQL reads, each one GET against Prometheus's HTTP
API: `series.query` (an instant query), `series.query_range` (a range query at an explicit
step) and `rules.list` (the alerting and recording rules, with each alerting rule's state).
The mapping, bounds and errors are in [the Prometheus profile](contracts/series/v1alpha1/semantics.md)
§§3–5 and 11; the connection is §11.4 and the Grafana placement §11.5. It requires Linux
x86_64 and the [qualified Secret Service binding](../../docs/local-secret-service.md), like
every saved credential.

Connector, instance and connection are the terms of the [vocabulary](../../README.md#vocabulary).

```sh
CARGO_BUILD_JOBS=2 cargo build --release --locked -p connectors -p connectors-prometheus
target/release/connectors --output json setup init
target/release/connectors --output json setup check
```

## Configure the native target

Create an owner-only native JSON file, in an admitted directory without symlinks:

```json
{
  "format": "connectors-prometheus-local/1",
  "instance": "prometheus-prod",
  "base_url": "https://prometheus.example.internal/",
  "query_scope": {"allowed_matchers": []}
}
```

`base_url` must be HTTPS and written in canonical form: lowercase `https://` and host, ending
in `/`, at most 512 characters; a path prefix, for a gateway that serves Prometheus below
one, is kept. `ca_file` may name an owner-only PEM bundle that replaces the system roots.
`query_scope` is required and admits only the empty list: every query reads everything the
token reaches. The token is never part of this file, of the TOML below, of executable
arguments or of the environment.

Inspect the native bootstrap and hash the built executable:

```sh
target/release/connectors-prometheus --local-config /absolute/path/prometheus.json --print-local-bootstrap
sha256sum target/release/connectors-prometheus
```

Copy `configuration_revision` from that output verbatim. Prometheus is not a catalog connector, so
the configuration pins its instance. Add the instance to the configuration created by setup:
change its `format` line to `format = "connectors-local/5"`, which instances need, and add the
instance `metrics` of the connector `prometheus`:

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

## Connect and read

```sh
target/release/connectors --output json connections connect --connector prometheus --instance metrics --profile prometheus.bearer --credential-prompt
target/release/connectors --output json operations describe --connector prometheus --instance metrics --operation series.query
target/release/connectors --output json operations invoke --connector prometheus --instance metrics --connection CONNECTION --operation series.query --schema SCHEMA --revision REVISION --input-json '{"query":"up{job=\"api\"}"}'
target/release/connectors --output json operations invoke --connector prometheus --instance metrics --connection CONNECTION --operation series.query_range --schema SCHEMA --revision REVISION --input-json '{"query":"sum by (status) (rate(http_requests_total[5m]))","start_unix_s":1788822000,"end_unix_s":1788825600,"step_s":60}'
target/release/connectors --output json operations invoke --connector prometheus --instance metrics --connection CONNECTION --operation rules.list --schema SCHEMA --revision REVISION --input-json '{"kind":"alerting"}'
```

Each operation has its own schema and revision from `operations describe`. An omitted
`time_unix_s` is now. A range spans at most seven days and at most 2,000 points per series
(`(end − start) div step + 1`); a wider one is refused before any request. Sample values stay
Prometheus's strings (`NaN`, `+Inf`); timestamps are its numbers. A provider warning makes the
result partial (`complete: false`) and is returned in `warnings`.

## What the connection is bound to

**The identity is the configured connection, not a Prometheus account.** Connect, reconnect and`connections revalidate` prove the token with `GET api/v1/status/buildinfo`, which runs no
query: a `200` admits it, a `401` or `403` refuses it. The identity kind is
`prometheus.connection` and its subject is the configuration's `instance`. Use one `instance`
per deployment and tenant.

**A token grants no scopes and exposes no expiry**, and **no tenant header is sent**: a
multi-tenant backend that requires `X-Scope-OrgID` from the client is not reachable; a gateway
that derives the tenant from the token is. A Prometheus without authentication cannot be
connected: the local host admits no profile without a credential.

## Through Grafana

Grafana's data-source proxy serves a Prometheus data source's HTTP API below
`/api/datasources/proxy/uid/<uid>/` and authenticates the request with a Grafana
service-account token. A Prometheus connection reads through it unchanged: set `base_url` to
that path and connect with the Grafana token as the `prometheus.bearer` token.

```json
{
  "format": "connectors-prometheus-local/1",
  "instance": "grafana-prod-prometheus",
  "base_url": "https://grafana.example.internal/api/datasources/proxy/uid/P1809F7CD0C75ACF3/",
  "query_scope": {"allowed_matchers": []}
}
```

The uid is the `uid` member of the [Grafana connector's](../grafana/README.md) `datasources.list`
record whose `type` is `prometheus` and `access` is `proxy`. The probe and every read then go
to `<base_url>api/v1/...` with `Authorization: Bearer <Grafana token>`
(`tests/local_runtime.rs`,
`a_connection_through_the_grafana_datasource_proxy_reads_below_the_proxy_prefix`). Grafana,
not Connectors, decides which data sources the token reaches. The identity is still the
configured `instance`: use one per Grafana and data source.

## Evidence lifetime

Validation evidence lasts 60 seconds. After it the connection lists as `pending` and a read is
refused with `not_granted` at admission until `connections revalidate` renews it with one more
probe. A scheduled caller revalidates before reading.

## Replacing the executable

The instance pins the executable's SHA-256. After rebuilding `connectors-prometheus`, update
`sha256`, stop the running incarnation (`instances stop`), then `connections revalidate` or
reconnect the connection, which performs the admitted launch under the same connection id.

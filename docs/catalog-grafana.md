# Grafana alert rules, annotations, dashboards, data sources, Prometheus and Loki through the catalog provider

The catalog provider reads a Grafana organization's alert rules, its annotations (alert state
history among them), its dashboards and its data sources from Grafana's pinned OpenAPI 3
document, read only, with a service-account token. Through Grafana's data-source proxy the same
token also runs Prometheus instant and range queries and Loki range queries; the instance holds
no Prometheus or Loki credential. A caller copying Grafana knowledge into a corpus reads the
alert rule definitions and the data sources whole, walks the dashboard list and re-reads a
dashboard by `uid`, walks the annotations of a time window, and queries the series and log lines
behind them.

Nothing here is Grafana-specific code: the pinned document is compiled into a bundle, a reviewed
selection set exposes eight reads, and the engine described in
[the catalog provider guide](local-catalog-provider.md) binds and sends them. The `grafana`
connector served by its own executable (`adapters/grafana`, [its guide](../adapters/grafana/README.md))
is a separate implementation and is unchanged; this provider ships no connector declaration, so
it is run from its definition file or as a pinned instance. No read needs an approval. Connector,
instance and connection are defined in [the README](../README.md#vocabulary).

## Source and bundle

The pinned source is
[`adapters/grafana/upstream/openapi3.json`](../adapters/grafana/upstream/README.md), SHA-256
`be8923b42413a640f92af54a728f5b04f936a2851a04b8dfa277ae0db3bdebdc` (794,841 bytes):
`public/openapi3.json` of `grafana/grafana` at tag `v13.3.1` (published SHA-256
`89ff3e97e4d0e47ddd5a0bc757f2be2c909e655d275486a776a85fbde44dc661`, 794,914 bytes), OpenAPI
`3.0.3`, `info.version` `0.0.1`, 314 operations, AGPL-3.0 (`LICENSE` beside it). One line is
removed: the example service-account token at
`/components/schemas/NewApiKeyResult/properties/key/example`, which the repository's secrets
scanner refuses; it changes no operation, and the upstream README records it. Its one server is
`/api`, so every operation path carries `/api` and an instance's `api_base` is the Grafana
origin followed by `/api`. Grafana's Swagger 2.0 document of the same tag, `public/api-merged.json`, declares no
`host`, which the Swagger projection refuses, so the OpenAPI 3 document is pinned instead.

The document declares Grafana's data-source proxy only as
`GET /datasources/proxy/uid/{uid}/{datasource_proxy_route}`, one parameter standing for the
whole sub-path. Three cited amendments,
[`adapters/grafana/upstream/grafana.amendments.json`](../adapters/grafana/upstream/grafana.amendments.json),
each add one read with its sub-path fixed: Prometheus's `/api/v1/query` and `/api/v1/query_range`
([Prometheus HTTP API, v3.9.1](https://github.com/prometheus/prometheus/blob/v3.9.1/docs/querying/api.md#expression-queries))
and Loki's `/loki/api/v1/query_range`
([Loki HTTP API, v3.7.0](https://github.com/grafana/loki/blob/v3.7.0/docs/sources/reference/loki-http-api.md#query-logs-within-a-range-of-time)).
An amended path is written as the bundle records the document's own, with the server's `/api`
in front; without it the engine refuses the selection as outside the configured base path. Each
amendment also gives `uid` the `pattern` `^[A-Za-z0-9_-]{1,40}$`, below. The pinned bytes stay
Grafana's. The bundle is built with:

```sh
cargo run --locked -p connectors-build -- catalog \
  --provider grafana \
  --source adapters/grafana/upstream/openapi3.json \
  --amendments adapters/grafana/upstream/grafana.amendments.json \
  --directory adapters/catalog/generated/bundles \
  --auth-profile grafana.service-account
```

It carries the document's 314 operations and the three amended reads, 317, none unsupported.
`adapters/catalog/tests/bundle_drift.rs` refuses a committed bundle or index a fresh run would
not reproduce byte for byte.

## The shipped selection set

[`adapters/catalog/providers/grafana/operations.json`](../adapters/catalog/providers/grafana/operations.json)
exposes these eight reads and nothing else; it holds no write. `adapters/catalog/tests/grafana.rs`
pins this exact id list with each id's `operationId`, method and path in the bundle, so a
renamed, dropped or added id fails the gate, and the committed bundle refuses an `operationId`
it lacks. The last three reach Prometheus and Loki through the data-source proxy, each under
the same profile.

| id | `operationId` and request | paging | end condition | time filter |
|---|---|---|---|---|
| `alert_rules.list` | `RouteGetAlertRules`, `GET /api/v1/provisioning/alert-rules` | none, one answer | the one answer | none: the consumer diffs definitions |
| `annotations.list` | `getAnnotations`, `GET /api/annotations` | `limit`, narrowing `to` | fewer items than `limit` | `from`, `to` (epoch ms) |
| `dashboards.search` | `search`, `GET /api/search`, `type` required and held to `dash-db` | `page`, `limit` | fewer items than `limit` | none: the consumer diffs titles and descriptions |
| `dashboard.get` | `getDashboardByUID`, `GET /api/dashboards/uid/{uid}` | single item | n/a | n/a |
| `datasources.list` | `getDataSources`, `GET /api/datasources` | none, one answer | the one answer | none |

| id | `operationId` (amended) and request | time bound | step and size bound |
|---|---|---|---|
| `prometheus.query` | `datasourceProxyPrometheusQuery`, `GET /api/datasources/proxy/uid/{uid}/api/v1/query` | `time` optional, Prometheus's default now | no step |
| `prometheus.query_range` | `datasourceProxyPrometheusQueryRange`, `GET /api/datasources/proxy/uid/{uid}/api/v1/query_range` | `start`, `end` required, both inclusive | `step` required; at most 11,000 points per series |
| `loki.query_range` | `datasourceProxyLokiQueryRange`, `GET /api/datasources/proxy/uid/{uid}/loki/api/v1/query_range` | `start` inclusive, `end` exclusive, both required | no step; `limit` 1 to 5000 |

Every read answers the same JSON value as Grafana's body (through the proxy, Prometheus's or
Loki's body), written compact with object keys sorted, as the catalog provider writes every JSON
answer. It is not the upstream bytes: key order and whitespace differ, and an escape such as
`<` or `&` comes back as the character it stands for. The provider does not walk pages;
the caller does, as below.

- **Alert rules.** `alert_rules.list` answers every alert rule of the token's organization in
  one array; the document declares no parameter. Alert firing history is not here: read it as
  `annotations.list` with `type` `alert`.
- **Annotations.** `from` and `to` are epoch milliseconds (the document: annotations "created
  after" `from` and "before" `to`); `limit` is the most items one answer carries (the document
  declares no default or maximum). `type` is `alert` or `annotation`; `dashboardUID`,
  `alertUID`, `tags` and the other declared filters pass through. A walk over a window sends
  `from`, `to` and `limit`; while a page carries `limit` items, it asks again with `to` set to
  the smallest `time` on that page and the same `from`. A page with fewer items than `limit` is
  the last. The annotation at the narrowed `to` is read again, so delivery is at least once and
  the caller deduplicates by `id`. The walk relies on Grafana answering the newest annotations of
  the window first, which the pinned document does not state. A full page whose items all share
  one `time` cannot narrow the window: ask again with a larger `limit`.
- **Dashboards.** `dashboards.search` lists dashboards only: `type` is required and held to
  `dash-db`, so a folder search (`dash-folder`) is refused before any request. `page` starts at
  1 and `limit` is the page size; the walk adds 1 to `page` while a page carries `limit` hits,
  and a page with fewer hits than `limit` is the last. The document's text for `limit` says
  "max 5000", but its schema declares no maximum, so none is held here and Grafana decides.
  `query`, `tag`, `folderUIDs`, `dashboardUIDs`, `starred` and `sort` pass through; there is no
  time filter, so a consumer re-reads the list and diffs titles and descriptions.
- **One dashboard.** `dashboard.get` reads one dashboard by `uid`, its JSON model under
  `dashboard` and its metadata under `meta`. The `apiVersion` query parameter the document's
  prose mentions is not declared, so it is refused before any request.
- **Data sources.** `datasources.list` answers every data source the token may read in one
  array, so a caller sees which Prometheus and Loki data sources the connection reaches. Grafana
  Enterprise with fine-grained access control requires `datasources:read` on `datasources:*`.
- **Prometheus and Loki through the proxy.** `uid` is the data source's `uid` from
  `datasources.list`: letters, digits, `-` and `_`, at most 40, Grafana's rule for a data-source
  uid (`ValidateUID` in
  [`pkg/util/shortid_generator.go`, v13.3.1](https://github.com/grafana/grafana/blob/v13.3.1/pkg/util/shortid_generator.go),
  which data-source create and update apply). Any other `uid`, such as one holding `/` or `\`,
  is refused before any request with ``parameter `uid` does not match its pattern``, so it is
  always one path segment and cannot reach another Grafana path; a legacy data source whose uid
  breaks the rule is not reachable this way. Grafana sends the rest of the path and the query on to that data source
  with its own stored credentials, so the instance holds none for Prometheus or Loki. As for
  every read, a data source the token's role cannot query answers `forbidden` (Grafana's `403`).
  Each read declares the inputs below and nothing else: Prometheus's `timeout`, `limit` and
  `lookback_delta`, and Loki's `since`, `step` and `interval`, are refused before any request.
  A served instance can hold `uid` to a list of data sources, as below.
  - `prometheus.query` evaluates `query` at `time`, RFC 3339 or Unix seconds; without `time`
    Prometheus uses its current time. There is no step.
  - `prometheus.query_range` evaluates `query` from `start` to `end`, both inclusive, every
    `step`, a duration (`60s`) or a number of seconds. Prometheus refuses a range of more than
    11,000 points per series, `(end - start) / step` (`exceeded maximum resolution of 11,000
    points per timeseries`, `web/api/v1/api.go` of v3.9.1), with `400`, refused as
    `invalid_input`; a longer range is read as several ranges.
  - `loki.query_range` reads the log lines of `query` from `start`, inclusive, to `end`,
    exclusive, nanosecond Unix epochs or RFC 3339, at most `limit` entries, newest first unless
    `direction` is `forward`. `limit` is required and held to 1 to 5000 before any request:
    5000 is the default of Loki's `max_entries_limit_per_query`
    ([Loki configuration, v3.7.0](https://github.com/grafana/loki/blob/v3.7.0/docs/sources/shared/configuration.md),
    `limits_config`: "Maximum number of log entries that will be returned for a query"), which
    a Loki configured lower enforces itself. Loki also refuses a range longer than its
    `max_query_length`, `30d1h` by default. A page holding `limit` entries may have left lines
    out; a caller reads the rest by asking again with the window narrowed to the last timestamp
    read. Loki gives entries no identity, so lines at that timestamp come again and cannot be
    told from new identical ones.
  - An answer over the catalog's 4 MiB answer-size limit (`connectors_core::RESPONSE_LIMIT`) is
    refused as `capacity`, never cut short and reported complete; narrow the range, raise
    `step` or lower `limit`.

## Authentication

A Grafana service-account token, sent as `Authorization: Bearer <token>`, under the profile
`grafana.service-account`. The identity read is `GET /api/user` (`getSignedInUser`): the
connection's identity is kind `grafana.user`, subject the account's `id`. The protected entry is
`{"token": "<service-account token>"}`; the token never appears in the configuration. The
service account needs read access to alert rules, annotations, dashboards and data sources; a
read its role does not allow is answered `403` by Grafana and refused as `forbidden`.

### A served instance from the definition file

The definition is `connectors/grafana.yaml` in `beyond10x/connectors-definitions`; its copy
under `crates/connectors-catalog/tests/fixtures/definitions/` is what this repository tests. It
declares the one profile above, `base_url` `https://{hostname}/api` and no write. A served
instance names it under `catalog`, with the Grafana origin as `endpoint`
([running services](running-services.md)):

```json
{
  "service": {
    "instance": "grafana.prod",
    "listen": "0.0.0.0:8080",
    "service_credential": {"kind": "file", "path": "/var/run/connectors/service-token"},
    "state": "/var/lib/connectors"
  },
  "catalog": {
    "definition": "/srv/connectors-definitions/connectors/grafana.yaml",
    "definitions_root": "/srv/connectors-definitions",
    "instance": "prod",
    "endpoint": "https://grafana.example.internal",
    "allowed_values": {
      "prometheus.query": {"uid": ["prometheus-prod"]},
      "prometheus.query_range": {"uid": ["prometheus-prod"]},
      "loki.query_range": {"uid": ["loki-prod"]}
    }
  },
  "credential": {"binding": "mounted_secret", "directory": "/var/run/connectors/credential"}
}
```

The credential is the mounted item `grafana.prod/grafana.service-account` holding
`{"token": "<service-account token>"}`. The instance describes the eight reads, and nothing
else.

`allowed_values` is optional. Per operation and input it lists the only values that input may
take: here the proxy reads reach only the data sources `prometheus-prod` and `loki-prod`. A
`uid` outside a non-empty list is refused before any request to Grafana as `forbidden`, with the
message ``input `uid` is not one of the instance's allowed values``; values compare exactly, case
included. An empty list, or none, admits every data source the token can read. The list bounds
only the operations it names, so `dashboard.get`, whose `uid` is a dashboard's, is not held to
it. Start refuses an operation the instance does not describe, an input that operation does
not take, an empty or repeated value, and an operation or input named twice, which YAML would
otherwise read as its last entry ([running services](running-services.md#catalog-instances)).

### A pinned local instance

For the local CLI, a native configuration pins the selection and the bundle of this checkout:

```json
{
  "format": "connectors-catalog-local/2",
  "instance": "grafana-catalog",
  "provider": "grafana",
  "bundle_directory": "/absolute/path/adapters/catalog/generated/bundles",
  "api_base": "https://grafana.example/api",
  "auth": {
    "profile": "grafana.service-account",
    "header": "Authorization",
    "bearer": true,
    "label": "Grafana service account token",
    "identity": {"path": "user", "kind": "grafana.user", "subject_pointer": "/id"}
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/grafana/operations.json"
}
```

Bind it as [the catalog provider guide](local-catalog-provider.md#bind-the-provider-to-the-local-cli)
describes, with `profiles = ["grafana.service-account"]` and the eight operation ids. A pinned
instance has no `allowed_values`: every data source the token can read is reachable.

## Limits

- Verified against a local HTTPS fixture only (`adapters/catalog/tests/grafana.rs`), with
  synthetic records: the exact request of each read and its `Bearer` header, the returned body
  (the upstream JSON value, re-encoded as above), two-page walks of `dashboards.search` and `annotations.list`, the identity read, the
  proxied Prometheus and Loki requests, the `capacity` refusal of an answer one byte over the
  limit, and a served instance started from the definition file with a mounted token, with and
  without `allowed_values`. Nothing has run against a live Grafana; that `GET /api/user` answers
  a service-account token with the account's own record, the newest-first order of annotations,
  and that Grafana's proxy passes each Prometheus and Loki query and answer through unchanged,
  are not checked against one.
- No write is offered: nothing creates, changes or silences.
- The proxy reads are Prometheus's two query endpoints and Loki's range query; series, label,
  metadata and Loki instant queries are not offered.

## Support-table summary

The README's support table gives this connector one short line; this is its full summary:

> Grafana HTTP API from the pinned OpenAPI 3 document: alert rules, annotations, dashboards and data sources, and Prometheus instant and range queries and Loki range queries through the data-source proxy, held to the instance's `uid` list when it has one; service-account token as `Authorization: Bearer`, profile `grafana.service-account`, proved with `GET /api/user`; reads only, one page per call, from its definition file on a served instance or a pinned instance; no connector declaration, since `grafana` is the native connector's name; verified against a local fixture only, see [the catalog Grafana guide](docs/catalog-grafana.md)

# Grafana alert rules, annotations, dashboards and data sources through the catalog provider

The catalog provider reads a Grafana organization's alert rules, its annotations (alert state
history among them), its dashboards and its data sources from Grafana's pinned OpenAPI 3
document, read only, with a service-account token. A caller copying Grafana knowledge into a
corpus reads the alert rule definitions and the data sources whole, walks the dashboard list and
re-reads a dashboard by `uid`, and walks the annotations of a time window.

Nothing here is Grafana-specific code: the pinned document is compiled into a bundle, a reviewed
selection set exposes five reads, and the engine described in
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
scanner refuses; it changes no operation, and the upstream README records it. Its one server is `/api`, so every
operation path carries `/api` and an instance's `api_base` is the Grafana origin followed by
`/api`. Grafana's Swagger 2.0 document of the same tag, `public/api-merged.json`, declares no
`host`, which the Swagger projection refuses, so the OpenAPI 3 document is pinned instead. The
bundle is built with:

```sh
cargo run --locked -p connectors-build -- catalog \
  --provider grafana \
  --source adapters/grafana/upstream/openapi3.json \
  --directory adapters/catalog/generated/bundles \
  --auth-profile grafana.service-account
```

It carries all 314 operations, none unsupported. `adapters/catalog/tests/bundle_drift.rs`
refuses a committed bundle or index a fresh run would not reproduce byte for byte.

## The shipped selection set

[`adapters/catalog/providers/grafana/operations.json`](../adapters/catalog/providers/grafana/operations.json)
exposes these five reads and nothing else; it holds no write. `adapters/catalog/tests/grafana.rs`
pins this exact id list with each id's `operationId`, method and path in the pinned document, so
a renamed, dropped or added id fails the gate, and the committed bundle refuses an `operationId`
the pinned document lacks.

| id | `operationId` and request | paging | end condition | time filter |
|---|---|---|---|---|
| `alert_rules.list` | `RouteGetAlertRules`, `GET /api/v1/provisioning/alert-rules` | none, one answer | the one answer | none: the consumer diffs definitions |
| `annotations.list` | `getAnnotations`, `GET /api/annotations` | `limit`, narrowing `to` | fewer items than `limit` | `from`, `to` (epoch ms) |
| `dashboards.search` | `search`, `GET /api/search`, `type` required and held to `dash-db` | `page`, `limit` | fewer items than `limit` | none: the consumer diffs titles and descriptions |
| `dashboard.get` | `getDashboardByUID`, `GET /api/dashboards/uid/{uid}` | single item | n/a | n/a |
| `datasources.list` | `getDataSources`, `GET /api/datasources` | none, one answer | the one answer | none |

Every read answers Grafana's body unchanged. The provider does not walk pages; the caller does,
as below.

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
    "endpoint": "https://grafana.example.internal"
  },
  "credential": {"binding": "mounted_secret", "directory": "/var/run/connectors/credential"}
}
```

The credential is the mounted item `grafana.prod/grafana.service-account` holding
`{"token": "<service-account token>"}`. The instance describes the five reads, and nothing
else.

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
describes, with `profiles = ["grafana.service-account"]` and the five operation ids.

## Limits

- Verified against a local HTTPS fixture only (`adapters/catalog/tests/grafana.rs`), with
  synthetic records: the exact request of each read and its `Bearer` header, the returned body
  bytes, two-page walks of `dashboards.search` and `annotations.list`, the identity read, and a
  served instance started from the definition file with a mounted token. Nothing has run
  against a live Grafana; that `GET /api/user` answers a service-account token with the account's
  own record, and the newest-first order of annotations, are not checked against one.
- No write is offered: nothing creates, changes or silences.
- Loki log lines and Prometheus series are not read here.

## Support-table summary

The README's support table gives this connector one short line; this is its full summary:

> Grafana HTTP API from the pinned OpenAPI 3 document: alert rules, annotations, dashboards and data sources; service-account token as `Authorization: Bearer`, profile `grafana.service-account`, proved with `GET /api/user`; reads only, one page per call, from its definition file on a served instance or a pinned instance; no connector declaration, since `grafana` is the native connector's name; verified against a local fixture only, see [the catalog Grafana guide](docs/catalog-grafana.md)

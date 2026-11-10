---
format: aep.planning-md/3
id: story:catalog-grafana-datasource-proxy-reads
kind: story
status: draft
title: Prometheus and Loki reads through Grafana's data-source proxy, from the Grafana definition
summary: Instant and range PromQL queries and Loki query_range through the Grafana service-account token, served from a catalog definition
relations:
- decomposes: epic:catalog-knowledge-sources
- serves: vision:independent-contract-adapters
- depends_on: story:catalog-grafana-reads
revision: 2
---
## Outcome

A served instance of the Grafana definition, the one from story:catalog-grafana-reads, also answers
time-series and log reads. It sends them through Grafana's data-source proxy, so a caller reaches
Prometheus and Loki data with the single Grafana service-account token. It holds no credential for
those backends.

The route is a catalog definition: `connectors/grafana.yaml` in `beyond10x/connectors-definitions`.
Serving the native adapters (`adapters/grafana`, the Prometheus and Loki adapters) is not the
route.

## Operations (read-only)

| id | upstream through the proxy | input |
|---|---|---|
| `prometheus.query` | `GET /api/datasources/proxy/uid/{uid}/api/v1/query` | `uid`, `query`, optional `time` |
| `prometheus.query_range` | `GET /api/datasources/proxy/uid/{uid}/api/v1/query_range` | `uid`, `query`, `start`, `end`, `step` |
| `loki.query_range` | `GET /api/datasources/proxy/uid/{uid}/loki/api/v1/query_range` | `uid`, `query`, `start`, `end`, `limit`, optional `direction` |

## Acceptance

- The definition exposes exactly the three ids above, each `effect: read`, under the profile
  story:catalog-grafana-reads declares. A test pins the id list.
- Each operation is checked against a recorded fixture. The test asserts the exact request path
  and query sent to Grafana, with the token as `Authorization: Bearer` and nothing else carrying
  it. It also asserts that the answer body comes back byte-identical.
- The answer is bounded by the catalog's answer-size limit. An over-limit Prometheus or Loki
  answer is refused with the catalog's size reason, never cut short and reported complete. Loki's
  `limit` has a documented maximum.
- A served instance started from the definition with a mounted token describes these three reads
  beside the four from story:catalog-grafana-reads.
- `docs/catalog-grafana.md` states, per operation, the proxy path and the time and step bounds.
  It also says that a data source the token's role cannot query answers `forbidden`.

## Ambiguities

- Grafana's pinned OpenAPI document may describe the proxy only as
  `/api/datasources/proxy/uid/{uid}/{datasource_proxy_route}`, a single path parameter that holds
  a sub-path. Check whether the definition notation and the lowerer can fix that sub-path per
  operation. If they cannot, the gap goes to the definitions format first, as an upstream
  blocker.
- Should `uid` be fixed per instance (configuration) or be a caller input? Decide when the story
  is accepted. A caller input lets the token reach every data source its role can query.

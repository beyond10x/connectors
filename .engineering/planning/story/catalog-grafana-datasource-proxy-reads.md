---
format: aep.planning-md/3
id: story:catalog-grafana-datasource-proxy-reads
kind: story
status: implemented
title: Prometheus and Loki reads through Grafana's data-source proxy, from the Grafana definition
summary: Instant and range PromQL queries and Loki query_range through the Grafana service-account token, served from a catalog definition
relations:
- decomposes: epic:catalog-knowledge-sources
- serves: vision:independent-contract-adapters
- depends_on: story:catalog-grafana-reads
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-11T00:01:00Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-11T00:01:02Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-11T08:56:05Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":3,"verification":1}}}
---
## Outcome

A served instance of the Grafana definition, the one from story:catalog-grafana-reads, also answers
time-series and log reads. It sends them through Grafana's data-source proxy, so a caller reaches
Prometheus and Loki data with the single Grafana service-account token. It holds no credential for
those backends.

The route is a catalog definition: `connectors/grafana.yaml` in `beyond10x/connectors-definitions`.
Serving the native adapters (`adapters/grafana`, the Prometheus and Loki adapters) is not the
route.

The definition file holds no write operation (requirement confirmed by Assembly).

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
  beside the five from story:catalog-grafana-reads.
- `docs/catalog-grafana.md` states, per operation, the proxy path and the time and step bounds.
  It also says that a data source the token's role cannot query answers `forbidden`.
- `uid` is a caller input. A `uid` outside a non-empty allow-list in the instance configuration
  is refused before any upstream request, by a named reason; an empty or absent list admits
  every data source the token can read.

## Ambiguities

- Grafana's pinned OpenAPI document may describe the proxy only as
  `/api/datasources/proxy/uid/{uid}/{datasource_proxy_route}`, a single path parameter that holds
  a sub-path. Check whether the definition notation and the lowerer can fix that sub-path per
  operation. If they cannot, the gap goes to the definitions format first, as an upstream
  blocker.

## Decisions

- `uid` (resolved 2026-11-10 from an Assembly requirement): a caller input, validated against an
  optional allow-list in the instance configuration; an empty list admits every data source the
  token can read. Reason: a central Grafana has one data source per environment, so a fixed
  `uid` would mean one instance per environment.

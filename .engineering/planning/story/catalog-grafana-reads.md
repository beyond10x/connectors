---
format: aep.planning-md/3
id: story:catalog-grafana-reads
kind: story
status: draft
title: Grafana alert rules, annotations and dashboard metadata through the catalog provider
relations:
- decomposes: epic:catalog-knowledge-sources
- depends_on: story:catalog-slack-reads
- serves: vision:independent-contract-adapters
revision: 1
---
## Source

Grafana HTTP API OpenAPI document, pinned by digest under `adapters/grafana/upstream/grafana-http-api.json`.
Provider id `grafana`. Auth: service account token as `Authorization: Bearer` (already supported).
`adapters/grafana/design.md` describes a native adapter that is not implemented; this story serves
the knowledge reads through the catalog provider instead and changes nothing in that design.
Requested by the knowledge-ingest consumer on 2026-09-29 (priority 4 with Loki). Loki log lines are
out of scope: too large for the corpus (consumer's assessment).

## Operations (read-only)

| id | endpoint | paging | end condition | time filter |
|---|---|---|---|---|
| `alert_rules.list` | `GET /api/v1/provisioning/alert-rules` | none (one response) | single response | none; the consumer diffs definitions |
| `annotations.list` | `GET /api/annotations` | `limit`, narrowing `to` | fewer items than `limit` | `from`, `to` (epoch ms) |
| `dashboards.search` | `GET /api/search?type=dash-db` | `page`, `limit` | fewer items than `limit` | none; the consumer diffs titles and descriptions |
| `dashboard.get` | `GET /api/dashboards/uid/{uid}` | single item | n/a | n/a |

Alert firing history, where needed, is read as `annotations.list` with `type=alert`.

## Shared surfaces

- Own files: `adapters/catalog/providers/grafana/operations.json`, `adapters/catalog/generated/bundles/grafana.bundle.json`, `docs/catalog-grafana.md`, `adapters/catalog/tests/grafana.rs`, and the pinned source.
- Shared and serialized through `depends_on` (see the epic): `adapters/catalog/generated/bundles/index.json`.

## Acceptance

- `operations.json` exposes exactly the ids above, each `effect: read`; `adapters/catalog/tests/grafana.rs` pins that exact id list.
- `operations.json` loads against the committed bundle, which refuses any `operation_id` the pinned document lacks; `docs/catalog-grafana.md` cites, per operation, the pinned document's `operationId` and path.
- `docs/catalog-grafana.md` states, per list operation, its paging parameters, its end condition and its time-window filter (or why there is none).
- Per operation, a recorded-fixture test asserts the exact request sent and that `operations invoke` returns the fixture body byte-identical.
- For every paged list operation, a paging test walks two fixture pages and stops at the documented end condition.
- `adapters/catalog/tests/bundle_drift.rs` reproduces this provider's bundle byte for byte.

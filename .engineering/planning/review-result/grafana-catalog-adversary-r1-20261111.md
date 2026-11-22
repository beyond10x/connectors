---
format: aep.planning-md/3
id: review-result:grafana-catalog-adversary-r1-20261111
kind: review-result
status: active
title: Adversary pass 1 on the Grafana catalog and proxy reads
relations:
- reviews: story:catalog-grafana-reads
- reviews: story:catalog-grafana-datasource-proxy-reads
revision: 1
---
# Adversary pass 1: Grafana catalog reads and data-source proxy reads

Reviewed the working tree of both units applied uncommitted on 47166ae5d8 in the review tree
`grafana-review`. Cases in `adapters/catalog/tests/grafana_adversary.rs`: 8 cases, 7 green, 1 red
(`recorded_upstream_bodies_come_back_byte_identical`).

Attacked without a break: uid path-segment injection on the wire, reserved characters in PromQL and
LogQL queries, token only in `Authorization: Bearer`, the answer-size refusal (never cut short,
with or without Content-Length), Loki `limit` 1..5000 in every numeric and string form, proxy
refusal codes, and reachability of any write or of the generic proxy route.

The red case: the engine parses each 2xx JSON body and writes it again (`adapters/catalog/src/lib.rs:2612`),
sorting keys and removing Go escapes. The unit's fixtures are already in that form, so the
byte-identity assertions cannot fail on key order or escaping.

```findings
[
{"file":"docs/catalog-grafana.md","line":79,"category":"acceptance","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"The proxy and dashboard reads re-encode the upstream JSON (keys sorted, Go escapes removed), so a real Prometheus, Loki or Grafana body is not returned byte-identical as both stories' acceptance and the guide claim; the unit's test passes only because its fixtures are pre-shaped into the engine's output form (adapters/catalog/tests/grafana.rs:546)."}
]
```

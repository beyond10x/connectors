---
format: aep.planning-md/3
id: story:parity-prometheus-query
kind: story
status: active
title: Prometheus PromQL, direct and through Grafana
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
- depends_on: story:parity-grafana-datasource-loki
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-10T04:32:50Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-10T04:32:51Z", actor: "human:timo", revision: 3}
---
## Outcome

Prometheus PromQL, direct and through Grafana — parity unit U12 of `docs/fluxplane-plugin-parity.md`.

## Operations

`grafana.prometheus.query`, `grafana.prometheus.range`, `grafana.prometheus.rules`, `prometheus.query`, `prometheus.test`

69 calls since 2026-09-09 (declared and mapped undeclared names), provider `prometheus`, planned wave W3.

## Surface

new native adapter; `adapters/prometheus` holds a design and the `promql-range` contract; instant queries and rules need contract additions.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives (the parity page names the gap per operation).
- The parity page row of each operation moves to covered, with the Connectors operation named.

## Note

The direct placement does not need Grafana; only the mediated one does.

---
format: aep.planning-md/3
id: story:parity-grafana-datasource-loki
kind: story
status: draft
title: Grafana datasource discovery and Loki through Grafana
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
- depends_on: story:parity-loki-query
revision: 1
---
## Outcome

Grafana datasource discovery and Loki through Grafana — parity unit U05 of `docs/fluxplane-plugin-parity.md`.

## Operations

`grafana.loki.query`, `grafana.datasource.list`, `grafana.loki.labels`, `grafana.loki.recent_logs`

359 calls since 2026-09-09 (declared and mapped undeclared names), provider `grafana`, planned wave W2.

## Surface

new native adapter; `adapters/grafana` holds a design and an ESS model; the `grafana-datasource-proxy` mediated route.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives (the parity page names the gap per operation).
- The parity page row of each operation moves to covered, with the Connectors operation named.

## Note

Different operations from `story:catalog-grafana-reads` (alert rules, annotations, dashboards: 0 calls).

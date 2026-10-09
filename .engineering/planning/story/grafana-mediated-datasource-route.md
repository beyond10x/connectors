---
format: aep.planning-md/3
id: story:grafana-mediated-datasource-route
kind: story
status: draft
title: Grafana parent route through the data-source proxy
relations:
- decomposes: epic:fluxplane-plugin-parity
- depends_on: story:parity-grafana-datasource-loki
revision: 1
---
## Outcome

The Grafana adapter provides the `route.mediated_http` parent route of `adapters/grafana/design.md` (profile `grafana-datasource-proxy`): a child adapter's target-relative GET is forwarded through Grafana's data-source proxy for a sealed data-source UID, with the closed type/UID-digest allowlist, refusal of suffixes, absolute URLs and unknown types, and degradation of children when a target's type or identity changes.

## Why later

Wave 20261009a serves Loki through Grafana with a plain Loki connection whose `base_url` is Grafana's `api/datasources/proxy/uid/<uid>/`, which needs no host routing. That connection exposes the UID in its configuration and has no child degradation. This story replaces it with the designed route when a deployment needs the UID sealed or several children behind one Grafana credential.

## Acceptance

- Spec first: the route and its refusals are modelled in `adapters/grafana/spec/ess` and the shared route contract, validated with the newest `ess`, before implementation.
- A Loki child reaches its data source only through the parent route, and each refusal named above is a test.

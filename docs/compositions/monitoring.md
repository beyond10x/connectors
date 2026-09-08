# Monitoring composition

**Status:** proposed; no mediated implementation or rollout.

Native authority belongs to [Grafana](../../adapters/grafana/design.md),
[Loki](../../adapters/loki/design.md), [Prometheus](../../adapters/prometheus/design.md)
and [Alertmanager](../../adapters/alertmanager/design.md). This document owns
concrete pairings and integration obligations. Extracting a child does not require
this composition or its parent adapter.

## 1. Scope and placement

"Federated Grafana" in the old system meant: one Grafana connection, discovery of its data sources, and child connections for Prometheus, Loki and Alertmanager whose traffic is proxied through Grafana (`grafana_datasource_proxy_v1`). In the new system this is three things: the Grafana adapter (reads plus `resource_discovery` plus a `route.mediated_http` capability), independent Loki/Prometheus/Alertmanager adapters that know nothing about Grafana, and the host materializing child connections with a mediated route. Host federation (`crates/connectors-host/src/federation.rs`) is separate and unchanged: it forwards operations between services; the mediated route carries provider traffic inside one host.

Placement: the Grafana adapter needs egress to the Grafana origin (`private_network` in the old declarations); a child adapter using the route runs in the same statically composed process with live injected private ports as the Grafana adapter; a direct child adapter runs wherever its origin is reachable.


## 7. Discovery and routes

1. Grafana `datasources.observe` lists data sources under the Grafana connection; recognized types become candidates; UIDs are sealed as opaque locators.
2. An operator selects a fresh admitted candidate; the host materializes a child connection (route.kind = via) only when an explicitly installed [composition executable](../../contracts/discovery/composition.md) has constructed the parent and child in one process and injected their private ports. Concrete startup/linking belongs to that executable under deployment admission, never generic server.rs or discovery-triggered loading.
3. Child invocations run their own admission and evidence; the parent forwards GET requests under its data-source proxy route for the sealed UID; suffix, method, and target discipline per `contracts/discovery/mediated_route/v1alpha1/semantics.md`.
4. Parent revocation, allowlist withdrawal, stale/incomplete target evidence or a changed semantic target/type refuses/degrades the child; no direct fallback. A title-only rename preserves observation identity. New observation/parent generations require explicitly admitted same-target revalidation; a changed UID/fixed target/type requires a new observation and new child.
5. Kubernetes-discovered Grafana stays fail-closed (needs its own token); Kubernetes → Grafana → Prometheus is two hops and refused.


## Historical source provenance

The predecessor's four OpenAPI 3.1 inputs were repository-authored:
grafana/http-api-2026-08-14.openapi.yaml (digest prefix 14c46194),
loki/http-api-2026-08-15.openapi.yaml (77e76951),
prometheus/http-api-2026-08-15.openapi.yaml (c952784e),
alertmanager/http-api-v0.31.0.openapi.yaml (0fac1b1f).
Prefixes identify earlier evidence, not usable source pins. Each adapter must
adopt its exact source/license/full digest before generation or extraction.

## 10. Evidence required

- Fixture: each child suite run direct and mediated against the same fixture with byte-identical results; route refusal cases; observation withdrawal degrading a child; value-freedom greps for UID and backend URL.
- Live: a Grafana with one Prometheus and one Loki data source; observe, materialize, query through the host directly and via one-hop federation.
- Decoupling: Loki/Prometheus/Alertmanager crates build without the Grafana crate; the Grafana crate builds without them; the host's mediated capability is tested with fake parent and child.


Native coverage, target and configuration predicates belong to parent/child
bindings; shared publication, admission and continuity belong to shared contracts.
A composition fixture proves only its selected pairing, not an untested native
parser, provider identity or API guarantee.

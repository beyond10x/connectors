# Adapter design: Grafana, with mediated and direct Loki, Prometheus and Alertmanager

- **Status:** design, not implemented. Four adapter services: `connectors.grafana`, `connectors.loki`, `connectors.prometheus`, `connectors.alertmanager`.
- **Old baseline:** `../connectors` at `81459ac4`: `providers/grafana.toml`, `loki.toml`, `prometheus.toml`, `alertmanager.toml`, `crates/integration-monitoring/`, `crates/monitoring-model/`, `docs/design/08-discovery-observations-and-mediated-connections.md`, pinned OpenAPI 3.1 sources under `specs/{grafana,loki,prometheus,alertmanager}/`.
- **Contract index:** [contracts/README.md](../../contracts/README.md).

## 1. Scope and placement

"Federated Grafana" in the old system meant: one Grafana connection, discovery of its data sources, and child connections for Prometheus, Loki and Alertmanager whose traffic is proxied through Grafana (`grafana_datasource_proxy_v1`). In the new system this is three things: the Grafana adapter (reads plus `resource_discovery` plus a `route.mediated_http` capability), independent Loki/Prometheus/Alertmanager adapters that know nothing about Grafana, and the host materializing child connections with a mediated route. Host federation (`crates/connectors-host/src/federation.rs`) is separate and unchanged: it forwards operations between services; the mediated route carries provider traffic inside one host.

Placement: the Grafana adapter needs egress to the Grafana origin (`private_network` in the old declarations); a child adapter using the route runs in the same statically composed process with live injected private ports as the Grafana adapter; a direct child adapter runs wherever its origin is reachable.

## 2. Old surface and disposition

| Old surface | Source | Disposition |
|---|---|---|
| `grafana-dashboards-list`, `grafana-dashboard-get`, `grafana-datasources-list` (`verify`), `grafana-datasource-query` (`expose = false`) | `providers/grafana.toml`, `[[patch.operations]]` | preserve the first three → `datasource.records`; drop `datasource-query` (children query their own API) |
| `[[discoveries]] grafana-data-sources`: mappings prometheus/loki/alertmanager → target provider via `grafana_datasource_proxy_v1` | `providers/grafana.toml`, discoveries block | preserve → `resource_discovery` `grafana-datasources` + `route.mediated_http` `grafana-datasource-proxy` |
| Config `origin` (operator approval, `endpoint.origin`), `service_account_token` (secret, custody) | `providers/grafana.toml`, config block | preserve |
| Hosted: one exact origin plus closed data-source allowlist by provider type and UID SHA-256; missing/renamed/type-changed sources degrade, never widen | old design 08, amendment 2026-08-17; `crates/integration-monitoring/src/backend.rs:1326` | preserve as `service_targets.allow` configuration |
| Projection bounds: 500 data sources; dashboards bounded; Prometheus 500 series × 2,000 samples; Loki 500 streams, 1,000 lines, 8 KiB lines; redaction of Loki/alert text | `crates/integration-monitoring/src/projection.rs` | preserve bounds as first-profile defaults in the logs/series contracts; redaction becomes an opt-in host filter |
| Removal of data-source UIDs, backend origins, raw dashboard/query objects, free-form provider errors, secure config, credentials from results | old design 08, amendment | preserve (value freedom in `resource_discovery` and `mediated_route`) |
| `loki-query-range`, `prometheus-query-range`, `alertmanager-alerts-list`; `origin` config "required only for a direct Connection" | `providers/loki.toml`, `prometheus.toml`, `alertmanager.toml` | preserve → `datasource.logs`, `datasource.series`, `datasource.records`; `origin` optional when mediated |
| Identity-verified `dev`/`sre` groups receive read authority | old design 08, amendment | host policy, outside adapters |

## 3. Contracts needed and why

Grafana adapter:

| Contract | Profile / use | Why |
|---|---|---|
| `datasource.records` | `grafana-dashboards` (list, get), `grafana-datasources` (list) | bounded dashboard and data-source metadata reads |
| `resource_discovery` | `grafana-datasources` | data sources as observations with opaque locators (UID sealed) and candidates for the three target adapters |
| `route.mediated_http` (provider) | `grafana-datasource-proxy` | forward a child's target-relative GET through Grafana's data-source proxy for the sealed UID; refuse suffixes, absolute URLs, unknown types |
| `auth.profile` | `grafana.service_account_configured` first; `grafana.service_account` for managed entry | Same credential purpose, distinct selected acquisition paths |
| `auth.acquisition` | `static_config` first; `static_entry` for later managed entry | Deployment references versus token entered once through protected UI |
| `auth.capability` | `http-bearer` | bearer placement |
| `auth.evidence` | `verify_operation` = `datasources.list` | reachability and token validity without effects |
| `auth.connection` | `configured` first, `managed` later | one Grafana per instance; children reference it as parent |
| `auth.custody` | `read_only` for static_config; `versioned` for static_entry | Entry requires durable writable custody; deployment capture does not copy secrets |

Loki / Prometheus / Alertmanager adapters (identical structure):

| Contract | Profile / use | Why |
|---|---|---|
| `datasource.logs` (Loki) | `logql-range` | native LogQL range read with stream/label/timestamp identity |
| `datasource.series` (Prometheus) | `promql-range` | native PromQL range read with step and label sets |
| `datasource.records` (Alertmanager) | `alertmanager-alerts` | bounded alert list |
| `auth.profile` | `<x>.anonymous`, `<x>.bearer`, `<x>.basic`, `<x>.via_parent` | Explicit anonymous/bearer/basic direct placement versus separately admitted parent authentication |
| `auth.capability` | `http-anonymous`/`http-bearer`/`http-basic` direct; `mediated-http` for via_parent | Same business operations; distinct selected auth/route metadata |
| `auth.evidence` | `verify_operation` = the range query with a trivial query | readiness |
| `auth.connection` | `configured` (direct) or created by materialization (`via`) | route recorded on the connection |

## 4. Operation map

| Adapter | New id | Contract / profile | Old id |
|---|---|---|---|
| grafana | `dashboards.list` | records `grafana-dashboards` | `grafana-dashboards-list` |
| grafana | `dashboard.get` | records single item | `grafana-dashboard-get` |
| grafana | `datasources.list` | records `grafana-datasources` | `grafana-datasources-list` |
| grafana | `datasources.observe` | resource_discovery `grafana-datasources` | `[[discoveries]]` |
| grafana | (capability) route `grafana-datasource-proxy` | provided to the host | `grafana_datasource_proxy_v1` |
| loki | `logs.query_range` | logs `logql-range` | `loki-query-range` |
| prometheus | `series.query_range` | series `promql-range` | `prometheus-query-range` |
| alertmanager | `alerts.list` | records `alertmanager-alerts` | `alertmanager-alerts-list` |

All reads; no mutation profile in this slice.

## 5. Auth

| Adapter | Profile | Scheme | Acquisition | Capability | Evidence |
|---|---|---|---|---|---|
| grafana configured first | `grafana.service_account_configured` | `http_bearer` (purpose service_account, subject app) | `static_config` | `http-bearer` | required credential/identity checks and declared verification |
| grafana managed entry | `grafana.service_account` | `http_bearer` (purpose service_account, subject app) | `static_entry` | `http-bearer` | same baseline with durable entry publication |
| loki/prometheus/alertmanager direct anonymous | `<x>.anonymous` | `none` (purpose anonymous, subject none) | `static_config` | `http-anonymous`; no credential placement | `verify_operation` if explicitly required |
| loki/prometheus/alertmanager direct bearer | `<x>.bearer` | `http_bearer` (purpose service_account, subject app) | `static_config` | `http-bearer`; pinned configured material | identity/validity and declared verification |
| loki/prometheus/alertmanager direct basic | `<x>.basic` | `http_basic` (purpose service_account, subject app) | `static_config` | `http-basic`; pinned configured user/password material | identity/validity and declared verification |
| loki/prometheus/alertmanager mediated | `<x>.via_parent` | `parent` (purpose mediated_access, subject none) | `static_config` route materialization | `mediated-http`; parent authenticates fixed hop | current parent/route binding; declared child verification through route |

Here x is loki, prometheus or alertmanager. The old ambiguous `<x>.none` label is descriptive history, not an accepted alias. The selected profile fixes credential placement and applicable evidence under [auth.profile §4.2](../../contracts/auth/profile/v1alpha1/semantics.md#42-explicit-access-bindings-without-child-credentials). Missing bearer/basic material cannot select anonymous. A parent-mediated child has no local credential generation or provider account; it neither inherits the parent's grant nor receives the parent's secret. Direct bearer/basic profiles require a reviewed identity-validation mechanism for that deployment before advertisement; a successful query alone cannot invent a stable account identity. These are proposed profiles, not implemented auth support.

Tenant header for Loki/Prometheus multi-tenant deployments (`docs/design.md:500`) is configuration on the direct connection, never caller input.

## 6. Configuration outline

Grafana:

```json
{ "adapter": {
    "origin": "https://grafana.monitoring.example",
    "dashboards": { "enabled": true },
    "service_targets": { "enabled": true, "allow": [ { "type": "prometheus", "uid_sha256": "…" } ] },
    "proxy": { "enabled": true, "targets": ["prometheus", "loki", "alertmanager"] } } }
```

Child (direct):

```json
{ "auth_profile": "prometheus.anonymous",
  "http": { "base_url": "https://prometheus.internal", "extra_headers": { "X-Scope-OrgID": "tenant-a" } },
  "adapter": { "max_window_s": 604800, "min_step_s": 1, "query_scope": { "allowed_matchers": ["job=\"api\""] } } }
```

`http.extra_headers` is the proposed receiver-owned representation for the provider tenant header, shared by direct Loki and Prometheus in this plan. The normative rule is [logs §4.1](../../contracts/datasources/logs/v1alpha1/semantics.md#41-monitoring-tenant-header-configuration-e30): `tenant_header` is an earlier descriptive label, not an accepted alias; there is no claim that current host `HttpConfig` implements either extension. Request input cannot set or override the configured value, including case variants, and this provider tenant is not the caller's SaaS tenant authority. Header validation, placement and configuration-revision isolation belong to the future binding.

Child (mediated, created by the host from a candidate): no `http.base_url`; `route: { parent: "conn_grafana_1", observation: "obs_…", profile: "grafana-datasource-proxy" }`.

These configuration outlines require a selected new strict reader. The anonymous form requires the explicit auth_profile selector and forbids credential/registration members, including explicit null. Bearer/basic forms require their configured references; missing or null is invalid, not anonymous. Mediated configuration selects `<x>.via_parent` and contains the sealed route binding, with no child base URL or credentials. static_config activates these bindings under host admission; it creates no auth.begin session or protected-entry action.

The admitted parent route/datasource owns downstream tenant headers; the child has no direct `extra_headers` override through mediation. Refuse a requested tenant binding the parent route cannot establish.

## 7. Discovery and routes

1. Grafana `datasources.observe` lists data sources under the Grafana connection; recognized types become candidates; UIDs are sealed as opaque locators.
2. An operator selects a fresh admitted candidate; the host materializes a child connection (route.kind = via) only when an explicitly installed [composition executable](../../contracts/discovery/composition.md) has constructed the parent and child in one process and injected their private ports. Concrete startup/linking belongs to that executable under deployment admission, never generic server.rs or discovery-triggered loading.
3. Child invocations run their own admission and evidence; the parent forwards GET requests under its data-source proxy route for the sealed UID; suffix, method, and target discipline per `contracts/discovery/mediated_route/v1alpha1/semantics.md`.
4. Parent revocation, allowlist withdrawal, stale/incomplete target evidence or a changed semantic target/type refuses/degrades the child; no direct fallback. A title-only rename preserves observation identity. New observation/parent generations require explicitly admitted same-target revalidation; a changed UID/fixed target/type requires a new observation and new child.
5. Kubernetes-discovered Grafana stays fail-closed (needs its own token); Kubernetes → Grafana → Prometheus is two hops and refused.

## 8. Specification profile

`connectors.adapter/v2` with the pinned repository-authored OpenAPI 3.1 sources (`specs/grafana/http-api-2026-08-14.openapi.yaml` sha256 `14c46194…`, `specs/loki/http-api-2026-08-15.openapi.yaml` sha256 `77e76951…`, `specs/prometheus/http-api-2026-08-15.openapi.yaml` sha256 `c952784e…`, `specs/alertmanager/http-api-v0.31.0.openapi.yaml` sha256 `0fac1b1f…`; all `openapi: 3.1.0`, all GET). These are the first candidates for generated request construction after GitLab. Response interpretation stays handwritten (`spec-kinds/adapter/v2/semantics.md:46-50`).

## 9. Deferred

`events` (alert webhooks), Grafana annotations or alert rules (writes), Loki live tail (`sessions` stream), Prometheus instant and label profiles, Alertmanager silences (mutation).

## 10. Evidence required

- Fixture: each child suite run direct and mediated against the same fixture with byte-identical results; route refusal cases; observation withdrawal degrading a child; value-freedom greps for UID and backend URL.
- Live: a Grafana with one Prometheus and one Loki data source; observe, materialize, query through the host directly and via one-hop federation.
- Decoupling: Loki/Prometheus/Alertmanager crates build without the Grafana crate; the Grafana crate builds without them; the host's mediated capability is tested with fake parent and child.

Discovery collection and route continuity follow [resource coverage §§4.1–4.4](../../contracts/discovery/resources/v1alpha1/semantics.md#41-exact-coverage-scope). Grafana's one admitted collection must prove provider exhaustion before absence becomes withdrawal; cap/failure may preserve trustworthy current positives while unobserved nonterminal rows remain explicitly stale history; previously withdrawn rows keep their terminal fact. Changed visibility/selection does not prove absence from the old scope. Public paging uses a published immutable view rather than provider continuations. A stable UID/type alone cannot prove an unchanged hidden destination/tenant/auth-placement; the exact parent verifier remains a profile-authoring/implementation gate. This does not broaden source-specific mappings or advertise a mediated implementation.

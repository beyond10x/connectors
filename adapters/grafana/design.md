# Grafana adapter design

**Status:** proposed independent service `connectors.grafana`, not implemented.

Bounded dashboards/datasource metadata, [native discovery](contracts/discovery/v1alpha1/semantics.md),
[parent routing](contracts/routes/v1alpha1/semantics.md) and
[authored ESS](spec/ess/system.yaml) belong here. Child native semantics and
configuration belong to their adapters; Grafana imports no child implementation.

## Contracts

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


## Operations

| Operation | Contract/profile | Old id |
|---|---|---|
| dashboards.list | records / grafana-dashboards | grafana-dashboards-list |
| dashboard.get | records / single item | grafana-dashboard-get |
| datasources.list | records / grafana-datasources | grafana-datasources-list |
| datasources.observe | resource_discovery / grafana-datasources | grafana-data-sources discovery |
| parent route capability | route.mediated_http / grafana-datasource-proxy | grafana_datasource_proxy_v1 |

The old unexposed datasource_query is not carried; children query their own API.
Annotations and alert-rule mutations remain deferred.

## Authentication and configuration

Configured-first grafana.service_account_configured selects http_bearer with
service_account purpose/app subject, static_config/read_only custody and
http-bearer placement. Later managed entry grafana.service_account selects
static_entry/versioned custody with protected durable entry publication.
Both need credential/identity checks and declared datasources.list verification.
The parent credential grants no child application authority.


```json
{ "adapter": {
    "origin": "https://grafana.monitoring.example",
    "dashboards": { "enabled": true },
    "service_targets": { "enabled": true, "allow": [ { "type": "prometheus", "uid_sha256": "…" } ] },
    "proxy": { "enabled": true, "targets": ["prometheus", "loki", "alertmanager"] } } }
```


One exact admitted origin and closed type/UID-digest allowlist bind visibility and
routing. Missing/changed type or semantic target degrades children; title-only
rename alone preserves target identity. Backend origins, UIDs, secure config,
credentials, headers and raw dashboard/query objects stay private.

## Sources and verification

Original evidence: ../connectors/providers/grafana.toml, integration-monitoring
backend.rs:1326 and projection.rs, and design 08 at 81459ac4. The predecessor HTTP
OpenAPI is repository-authored. Its exact source/license/full digest and native
fixtures belong under this adapter before generation/extraction.

All selected HTTP operations are GET. The predecessor source
`../connectors/specs/grafana/http-api-2026-08-14.openapi.yaml:105-107`
also declares `POST /api/ds/query` (`datasource_query`); that operation is
unselected. Read-only selection does not make the entire source GET-only.

Dashboard/list schemas, coherent enumeration and the exact proxy-target verifier
remain native authoring obligations. An unchanged UID/type cannot prove hidden
target equality. Shared publication/continuity are host-port obligations.
[Composition](../../docs/compositions/monitoring.md) owns concrete pairing and
direct/mediated equivalence suites; neither parent nor child imports the other.

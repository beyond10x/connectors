# Grafana adapter design

**Status:** native executable `connectors-grafana` (since 0.39.0). It serves
`datasources.list` (records / `grafana-datasources`) through the local host under the profile
`grafana.service_account`, and the IRM reads `irm.alert_groups.list`, `irm.alert_group.get`,
`irm.incidents.list` and `irm.incident.get` (records / `grafana-irm-alert-groups`,
`grafana-irm-incidents`) under `grafana.irm_service_account`; see [README.md](README.md) and
[IRM reads](#irm-reads). Dashboards, datasource discovery and the parent route below are not
implemented.

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

Implemented (2026-10-09): `grafana.service_account` only, entered through the local host's
protected entry into keyring custody, verified by `GET /api/datasources` with the configured
`instance` as identity subject (`spec/ess/domains/connection.yaml`).
`grafana.service_account_configured` is not implemented.


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

Implemented exception (2026-10-09): the records read `datasources.list` returns each data
source's `uid`, with its name, type, access mode and default flag, and nothing else. Until the
mediated route exists, an operator reaches a child through Grafana by naming that uid in the
child's own `base_url` (`api/datasources/proxy/uid/<uid>/`), so the record must carry it.
Discovery observations and the route keep the uid sealed as above.

## IRM reads

Implemented (2026-10-14, `story:grafana-irm-history-reads`), read only. Model:
[irm.yaml](spec/ess/domains/irm.yaml); the optional `irm_api_url` and the second profile:
[connection.yaml](spec/ess/domains/connection.yaml).

| Operation | Port | Request |
|---|---|---|
| `irm.alert_groups.list` | IRM API URL, `X-Grafana-URL` fixed | `GET api/v1/alert_groups/`, `started_at=<since>_<until>`, `perpage=100`, `page` |
| `irm.alert_group.get` | IRM API URL, `X-Grafana-URL` fixed | `GET api/v1/alert_groups/<id>` |
| `irm.incidents.list` | stack (`base_url`), read POST | `api/plugins/grafana-irm-app/resources/api/v1/IncidentsService.QueryIncidents`, `query.dateFrom`/`dateTo`/`limit`/`orderDirection`, `cursor.nextValue` |
| `irm.incident.get` | stack, read POST | `…/IncidentsService.GetIncident`, `{incidentID}` |

Native, not the catalog provider: neither API publishes a licensed description, incidents are
JSON-RPC POSTs, and alert groups need a second host with a fixed tenancy header. The
composition builds two ports from configuration: the stack port with exactly the two incident
read POSTs (`ScopedHttp::with_read_posts`), and, only when `irm_api_url` is configured, the IRM
port with one fixed `X-Grafana-URL` (`ScopedHttp::with_fixed_header`, the one-header slice of
[receiver-owned headers](../../contracts/auth/capability/v1alpha1/http-headers.md)). The
header's value is `base_url` without its final `/`; no input reaches it.

The provider authority the host binds the stored token to names every destination of that
token: `base_url` alone, or `base_url`, one space and `irm_api_url` when the IRM API URL is
configured. A changed `irm_api_url` is thus a changed authority, which needs a new instance id
(`docs/local-connection-registry.md`, configuration upgrades), so an upgrading revalidation can
never carry the token to another IRM host. A configuration without `irm_api_url` keeps the
authority and revision it had before the member existed.

Decisions taken inside the read-only scope: a service-account token plus the fixed header
(not a legacy OnCall API key); a second profile `grafana.irm_service_account` whose probe is
`GET api/v1/alert_groups/?started_at=2000-01-01T00:00:00_2000-01-01T00:00:01&perpage=1`, so the
connection proves the read it is used for and existing `grafana.service_account` connections
keep their profile and revision; and the incident→alert-group link projected from the
incident's `refs[]` under `com.grafana.oncall`, **inferred** until one real answer confirms it.

Sources: the IRM alert-group HTTP API reference (documentation, no licence stated; cited, not
vendored) and `github.com/grafana/incident-go` `client.gen.go` at
`21a0e7878e0217665ba90db8cf67f6edad306f50` (Apache-2.0; read for method names and shapes, not
vendored). Fixtures are authored from their documented examples with placeholder ids.

Unknowns: IRM's alert-group sort order and the incident service's maximum `limit` are not
documented (the adapter asks for 50); whether window bounds are inclusive is inferred.

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

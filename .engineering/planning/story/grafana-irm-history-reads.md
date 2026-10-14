---
format: aep.planning-md/3
id: story:grafana-irm-history-reads
kind: story
status: active
title: Grafana Cloud IRM alert groups, incidents and their links over a since window, read only
relations:
- decomposes: epic:incident-collection-workflow
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: adapters/grafana/README.md
- confidence: cited
  path: adapters/grafana/design.md
- confidence: cited
  path: adapters/grafana/generated/descriptor.json
- confidence: cited
  path: adapters/grafana/spec/adapter.json
- confidence: inferred
  path: adapters/grafana/spec/ess
- confidence: cited
  path: adapters/grafana/spec/ess/domains/connection.yaml
- confidence: cited
  path: adapters/grafana/spec/ess/domains/irm.yaml
- confidence: cited
  path: adapters/grafana/src/auth.rs
- confidence: cited
  path: adapters/grafana/src/lib.rs
- confidence: cited
  path: adapters/grafana/src/local.rs
- confidence: cited
  path: adapters/grafana/tests/ess_model.rs
- confidence: cited
  path: adapters/grafana/tests/fixtures
- confidence: cited
  path: adapters/grafana/tests/local_runtime.rs
- confidence: cited
  path: adapters/grafana/tests/protocol.rs
- confidence: cited
  path: contracts/auth/capability/v1alpha1/http-headers.md
- confidence: cited
  path: contracts/auth/capability/v1alpha1/semantics.md
- confidence: inferred
  path: crates/connectors-docs/src/status.rs
- confidence: cited
  path: crates/connectors-host/src/http.rs
- confidence: cited
  path: website/data/status.json
- confidence: cited
  path: website/docs/reference/adapters/grafana
- confidence: cited
  path: website/docs/status.md
revision: 25
transitions:
- {from: "draft", to: "proposed", at: "2026-10-14T02:55:04Z", actor: "human:timo", revision: 24}
- {from: "proposed", to: "active", at: "2026-10-14T02:55:05Z", actor: "human:timo", revision: 25}
---

## Placement

A new story, `story:grafana-irm-history-reads`, decomposing `epic:incident-collection-workflow`
(row 5, `.engineering/planning/epic/incident-collection-workflow.md:31`, "the IRM reads,
queued"). It is not `story:catalog-grafana-reads` (alert rules, annotations and dashboards from
the Grafana HTTP API document, draft) and not `story:parity-grafana-datasource-loki`
(implemented): different API, different hosts.

## Problem

The incident workflow needs IRM alert groups, incidents and which alert group an incident was
declared from, read over a window a caller advances run by run. Grafana is reached today only
by the native `connectors-grafana` adapter, which serves one operation, `datasources.list`.

## Today (file:line)

- Native adapter: `adapters/grafana/src/lib.rs:17-19` (`OPERATIONS = ["datasources.list"]`,
  `GET /api/datasources`); profile `grafana.service_account` `adapters/grafana/src/auth.rs:15`,
  identity check `GET /api/datasources` `auth.rs:73`; composition builds one `ScopedHttp` on the
  configured `base_url` with `Authorization: Bearer` `adapters/grafana/src/local.rs:79-93`;
  configuration model `adapters/grafana/spec/ess/domains/connection.yaml:23-33`; records model
  `adapters/grafana/spec/ess/domains/records.yaml`; status `adapters/grafana/design.md:3-6` [tree].
- Read POSTs for native adapters exist: `AuthenticatedHttp::post_json` reaches only paths
  composition fixed (`crates/connectors-sdk/src/lib.rs:88-99`;
  `crates/connectors-host/src/http.rs:241-284,907-920`), used by Tavily
  (`adapters/tavily/src/local.rs:73`) [tree].
- One credential header per transport; no fixed extra header: `HttpConfig`
  `crates/connectors-host/src/http.rs:15-30`. Receiver-owned extra headers are specified but
  not implemented: `contracts/auth/capability/v1alpha1/http-headers.md:1-3` [tree].
- Catalog provider limits that rule it out here: reads are GET only
  (`adapters/catalog/src/lib.rs:773-779`); a required header parameter is refused
  (`lib.rs:836-845`); one `api_base` per connection (`adapters/catalog/src/local.rs:165-170`);
  POST reads are a draft (`story:catalog-post-reads`) [tree].

## Catalog provider or native adapter

**Native, in `connectors-grafana`.** The catalog path fails on every count:

| catalog needs | IRM gives |
|---|---|
| a pinned API description with a licence | none published for either API [docs: the API reference names none; search found none]. The archived OSS OnCall repository is AGPL-3.0 (`github.com/grafana-cold-storage/oncall`, archived) and its server schema is not the Cloud service's. The incident client `github.com/grafana/incident-go` is Apache-2.0 but is generated Go, not an API description |
| GET reads | incidents are JSON-RPC `POST …/IncidentsService.QueryIncidents` [client] |
| one host, one credential header | alert groups live on the IRM API URL and need `X-Grafana-URL: <stack URL>` beside the bearer token [docs]; incidents live on the stack URL [client] |

A native adapter can use `post_json` with fixed paths (Tavily precedent), project typed records
(what `epic:incident-collection-workflow` asks for: typed output, not provider bytes), and derive
the link. It belongs in `connectors-grafana`: same stack, same service-account credential,
and the adapter boundary forbids a sibling adapter from importing Grafana's
(`adapters/README.md#boundary-gate`).

## Upstream sources and licence

- IRM HTTP API reference (alert groups, auth, paging):
  <https://grafana.com/docs/grafana-cloud/alerting-and-irm/irm/oncall/oncall-api-reference/> and
  `…/oncall-api-reference/alertgroups/`. Documentation pages, no licence stated; **nothing is
  vendored** from them, only cited.
- Incident RPC API: `github.com/grafana/incident-go`, `client.gen.go` at
  `21a0e7878e0217665ba90db8cf67f6edad306f50` (2026-10-07), licence **Apache-2.0** (GitHub
  licence API). Read for method names and request/response shapes; not vendored (it is Go code,
  and the rule here is Rust). The RPC reference page
  (`/docs/grafana-cloud/alerting-and-irm/irm/incident/api/`) did not show these details in the
  fetch made [docs].
- Fixtures are authored from the documented examples, with placeholder ids.

## Outcome

A saved Grafana connection whose configuration names the IRM API URL reads, with the
service-account token and no approval: alert groups started in a window, incidents created in a
window, one alert group by id, and for each incident the alert-group ids it references. The
caller holds the cursor (the window end it stored) and passes it back as the next run's start.

## Operations

| operation id | method, path | since-window parameter | pagination |
|---|---|---|---|
| `irm.alert_groups.list` | GET `<IRM API URL>/api/v1/alert_groups/` [docs] | `started_at` = `<start>_<end>`, format `%Y-%m-%dT%H:%M:%S_%Y-%m-%dT%H:%M:%S`, UTC only; "If a started_at date is not provided the last 30 days will be used" [docs]. The adapter takes `since`/`until` RFC 3339 and always sends both, so a call never silently means "last 30 days". Filter is on start time; no updated-time filter exists [docs]. Optional exact filters `state`, `integration_id`, `team_id` [docs] | `page`, `perpage` ("capped at 100. Values above 100 are served at the maximum page size") [docs]; response `count`, `next`, `previous`, `results`, `current_page_number`, `page_size`, `total_pages` [docs]; end: `next` null [docs]. Sort order not documented [unknown]; the adapter returns `page` as an opaque cursor, never following `next` URLs |
| `irm.alert_group.get` | GET `<IRM API URL>/api/v1/alert_groups/<id>` [docs, search result] | n/a; re-reads state (`resolved_at`, `acknowledged_at`, `silenced_at`) of a group copied earlier | single |
| `irm.incidents.list` | POST `<stack>/api/plugins/grafana-irm-app/resources/api/v1/IncidentsService.QueryIncidents` [client: `NewClient` doc comment and `QueryIncidents`] | body `query.dateFrom` / `query.dateTo` (RFC 3339, "created since" / "created before"), `query.orderDirection`; `query.queryString` is never sent, because "the other query parameters will be ignored" when it is [client] | body `query.limit`, `cursor.nextValue`, answer `cursor.hasMore` [client]; end: `hasMore` false. Maximum `limit` not documented [unknown] |
| `irm.incident.get` | POST `…/IncidentsService.GetIncident`, body `{incidentID}` [client] | n/a; re-reads `status`, `closedTime`, `modifiedTime` | single |
| `irm.incident_links` (projection, no own request) | derived from each incident returned by `irm.incidents.list`/`irm.incident.get` | as the incident read | as the incident read |

The link: an incident's `refs[]` entries with `key` `com.grafana.oncall` carry `ref` (an id) and
`url` [client: `IncidentRef`, examples]; creating an incident takes `alertGroupID` "the
identifier of the alert group associated with this incident" [client: `CreateIncidentRequest`].
That `ref` under `com.grafana.oncall` **is** the alert-group id is **[inferred]** from those two
facts; the client's examples use the placeholder `abc123`. An alert group carries no incident
field (fields listed in the docs: `id`, `integration_id`, `route_id`, `alerts_count`, `state`,
`created_at`, `alert_group_number`, `title`, `resolved_at`, `acknowledged_at`, `silenced_at`,
`root_alert_group_id`, `dependent_alert_group_ids`, `permalinks`, `last_alert` [docs]), so the
link is read from the incident side only. Open question C covers confirming it.

Since-window caveats, to be stated in the adapter README: both windows are on creation/start
time; later state changes are re-read by id; a window is inclusive at both ends [inferred] and
the caller deduplicates by id.

## Spec first

`adapters/grafana/spec/ess` gains a domain `irm.yaml` (records: `AlertGroupRecord`,
`IncidentRecord`, `IncidentAlertGroupLink`, `Window` with `since < until` and both UTC,
page cursor newtypes with byte bounds, state enums `new|acknowledged|resolved|silenced`), and
`connection.yaml` gains optional `irm_api_url` (https, ≤512) on `LocalConfiguration`.
`adapters/grafana/spec/adapter.json` gains the operations with their input and output
schemas; `adapters/grafana/generated/descriptor.json` is regenerated. If the host side needs
the fixed `X-Grafana-URL` header (Open question A, option a), `HttpConfig` and the auth
capability contract change first (`contracts/auth/capability/v1alpha1/http-headers.md` moves
from proposed to implemented for one header). Validate with the pinned `ess`
(`crates/connectors-spec/toolchain.json`, 0.59.0; move to the newest release first if one has
shipped), regenerate, then implement against the generated code. If ESS cannot state a window
as a pair with `since < until`, stop and report.

## Acceptance (recorded fixtures; no live provider)

In `adapters/grafana/tests/protocol.rs` (stand-in server, as the existing tests) unless named:

- `alert_groups_send_the_window_and_the_stack_header`: the stand-in sees
  `GET /api/v1/alert_groups/?started_at=2026-01-01T00:00:00_2026-01-02T00:00:00&perpage=100&page=1`,
  `Authorization: Bearer <fixture token>` and `X-Grafana-URL: <configured stack URL>`.
- `a_window_without_until_or_with_an_offset_is_refused_before_provider_work`: no request when
  `until` is absent, when either bound is not UTC, or when `since >= until`.
- `alert_groups_walk_two_pages_and_end_on_null_next`: page cursor round trip; `complete` true
  on `next: null`.
- `alert_group_records_carry_only_the_modelled_members`: `last_alert.payload` and `permalinks`
  are not returned unless modelled; checked against the generated schema (`tests/ess_model.rs`).
- `incidents_query_is_one_post_to_the_fixed_path_with_date_from_and_cursor`: body holds
  `query.dateFrom`, `query.dateTo`, `query.limit`, `cursor.nextValue`; never `queryString`; any
  other path is refused with no I/O (`post_json` fixed-path rule).
- `incidents_walk_two_pages_and_end_on_has_more_false`.
- `incident_links_are_projected_from_oncall_refs_only`: refs with other keys yield no link; an
  incident with two OnCall refs yields two links.
- `provider_statuses_map_to_the_family_errors_without_provider_text`: extended to the new
  operations (401, 403, 404, 429, 5xx).
- `an_existing_configuration_without_irm_api_url_keeps_its_revision` and the IRM operations are
  refused as unavailable on such a connection.
- `adapters/grafana/tests/local_runtime.rs`: a saved connection invokes `irm.alert_groups.list`
  and `irm.incidents.list` against the stand-in.

## Scope (files)

- Changed: `adapters/grafana/src/lib.rs` (operations, projections), `adapters/grafana/src/local.rs`
  (second port for the IRM API URL, read-post paths, configuration), `adapters/grafana/src/auth.rs`
  (only if the probe changes, Open question B), `adapters/grafana/spec/adapter.json`,
  `adapters/grafana/spec/ess/domains/{connection.yaml,irm.yaml}`,
  `adapters/grafana/generated/descriptor.json`, `adapters/grafana/tests/{protocol.rs,ess_model.rs,local_runtime.rs}`,
  `adapters/grafana/tests/fixtures/{alert_groups_page1.json,alert_groups_page2.json,incidents_page1.json,incidents_page2.json,incident.json}`,
  `adapters/grafana/README.md`, `adapters/grafana/design.md`.
- If option A(a): `crates/connectors-host/src/http.rs` (one fixed extra header),
  `contracts/auth/capability/v1alpha1/{semantics.md,http-headers.md}`.
- Regenerated: `crates/connectors-docs` outputs (`website/docs/reference/adapters/grafana/…`,
  `website/data/status.json`, `website/docs/status.md`), with a status item naming its test
  (`crates/connectors-docs/src/status.rs`); `CHANGELOG.md`.

## Not in scope

Acknowledge, resolve, silence or any write; escalation chains, schedules, on-call shifts,
integrations, routes; incident activity, tasks, roles, key updates; alert-group `search`
(limited to 30 days [docs]); `label` filters; the catalog provider and
`story:catalog-grafana-reads`; a `datasource.feed/v1alpha1` binding; Jira Service Management.

## Open questions

**A. Alert-group authentication.**
(a) Service-account token plus a composition-fixed `X-Grafana-URL` equal to the configured
stack URL, which needs one fixed extra header in `ScopedHttp`; (b) a legacy OnCall API key sent
bare (`Authorization: <key>` [docs]), which the existing transport already does with
`bearer: false`, but keys are user-specific and only visible to their creator [docs]; (c) both,
as two profiles. Recommendation: (a). The docs call service-account tokens the preferred method,
one token then serves both APIs, and the header value is derived from configuration, never from
caller input, which is the narrowest slice of the proposed `http-headers.md`.

**B. Identity and permission probe.**
The current probe `GET /api/datasources` needs data-source read permission an IRM-only service
account may lack. (a) Keep it and document the extra permission; (b) a second profile
`grafana.irm_service_account` whose probe is `GET <IRM API URL>/api/v1/alert_groups/?perpage=1`
with a fixed window; (c) probe per operation family at invoke time. Recommendation: (b) — the
connection proves what it is used for, and existing connections keep their profile and revision.

**C. The incident-to-alert-group link.**
(a) Project `refs[]` with key `com.grafana.oncall` as the link, recorded as inferred until a real
answer confirms it; (b) hold the link operation until a sandbox stack shows an incident declared
from an alert group; (c) also read incident activity (`ActivityService.QueryActivity` [client])
for alert-group mentions. Recommendation: (a) with an explicit README limitation and a follow-up
task to record one real answer; (c) widens the surface for an unproven gain.

## Build estimate

Package gates: `connectors-grafana`, `connectors-host` (if A(a)), `connectors-docs`
(regenerated reference), `connectors-build` (gate, ignored-test inventory); then the repository
gate. Similar waves: Grafana `datasources.list` `4781dc49f` plus `01ae7c490` (31 + 16 files:
adapter, spec, Loki tests, generated outputs), Prometheus native adapter `f9bfb15e9` (57 files,
5,141 lines: spec domains, protocol/local-runtime/ESS-model tests, website reference). This is
smaller than Prometheus (no new crate) and larger than `datasources.list` (two hosts, a read
POST, a header): one story, one wave.

## Decisions taken

Taken 2026-10-13 as design choices inside the requested read-only scope: A (a), a service-account token plus a composition-fixed `X-Grafana-URL` equal to the configured stack URL; B (b), a second profile `grafana.irm_service_account` with its own alert-group probe; C (a), project incident `refs[]` with key `com.grafana.oncall` as the link, marked inferred in the README until one real answer is recorded.

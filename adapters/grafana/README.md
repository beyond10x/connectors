# Grafana through the local CLI

The `grafana` adapter answers `datasources.list`: the data sources a Grafana service account
can read, each as its uid, name, plugin type, access mode and default flag, from one
`GET /api/datasources`. On a Grafana Cloud stack with IRM it also answers four IRM reads, all
read only: alert groups and incidents over a since window, and one of each by id (see
[IRM alert groups and incidents](#irm-alert-groups-and-incidents)). The records are modeled in
[spec/ess](spec/ess/domains/records.yaml) and [the IRM domain](spec/ess/domains/irm.yaml), the
connection in [the connection domain](spec/ess/domains/connection.yaml). It requires Linux
x86_64 and the [qualified Secret Service binding](../../docs/local-secret-service.md), like
every saved credential.

Querying a Loki data source through Grafana is a [Loki connection](../loki/README.md#through-grafana)
whose `base_url` is that data source's Grafana proxy path; the uid comes from
`datasources.list`.

```sh
CARGO_BUILD_JOBS=2 cargo build --release --locked -p connectors -p connectors-grafana
target/release/connectors --output json setup init
target/release/connectors --output json setup check
```

## Configure the native target

Create an owner-only native JSON file, in an admitted directory without symlinks:

```json
{
  "format": "connectors-grafana-local/1",
  "instance": "grafana-prod",
  "base_url": "https://grafana.example.internal/"
}
```

`base_url` must be HTTPS and written in canonical form: lowercase `https://` and host, ending
in `/`, at most 512 characters. A Grafana served under a sub-path (`root_url` with
`serve_from_sub_path`) keeps that prefix, for example `https://ops.example/grafana/`.
`ca_file` may name an owner-only PEM bundle that replaces the system roots, for a deployment
with a private CA; it applies to `irm_api_url` too. `irm_api_url`, optional, is the stack's IRM
API URL as IRM's settings show it, written like `base_url` (HTTPS, canonical, ending in `/`).
Without it the IRM reads are unavailable, and a configuration without it keeps the
`configuration_revision` it had before the member existed. The token is never part of this
file, of the TOML below, of executable arguments or of the environment.

Inspect the native bootstrap and hash the built executable:

```sh
target/release/connectors-grafana --local-config /absolute/path/grafana.json --print-local-bootstrap
sha256sum target/release/connectors-grafana
```

Copy `configuration_revision` from that output verbatim, then add an adapter entry to the
configuration created by setup:

```toml
[adapters.grafana]
instance_id = "grafana-prod"
adapter_id = "grafana"
configuration_revision = "REPLACE_FROM_BOOTSTRAP"
protocol = "v1alpha1"
startup = "on-demand"
restart = "never"

[adapters.grafana.permissions]
profiles = ["grafana.service_account"]
operations = ["datasources.list"]

[adapters.grafana.executable]
path = "/absolute/path/target/release/connectors-grafana"
sha256 = "REPLACE_WITH_EXECUTABLE_SHA256"
args = ["--local-config", "/absolute/path/grafana.json"]
```

## Connect and list

Create a Grafana service account allowed to read data sources (the `datasources:read`
permission that `GET /api/datasources` requires) and a token for it. Which basic or custom
role grants that is Grafana's configuration; Connectors only observes the `200` or `403`.

```sh
target/release/connectors --output json connections connect --adapter grafana --profile grafana.service_account --credential-prompt
target/release/connectors --output json operations describe --adapter grafana --operation datasources.list
target/release/connectors --output json operations invoke --adapter grafana --connection CONNECTION --operation datasources.list --schema SCHEMA --revision REVISION --input-json '{}'
```

For automation, `--credential-stdin` or `--credential-file /absolute/private/file.json` carries
the complete native `{"token":"..."}` document.

## What a record carries

| Member | From Grafana | Meaning |
|---|---|---|
| `uid` | `uid` | the data source's stable identifier; 1–40 of `A–Z a–z 0–9 _ -` |
| `name` | `name` | display name, 1–190 characters |
| `type` | `type` | plugin id, such as `loki` or `prometheus` |
| `access` | `access` | `proxy` (served through Grafana's data-source proxy) or `direct` |
| `is_default` | `isDefault` | whether it is the organization's default data source |

Nothing else of Grafana's answer is read: the numeric `id` and `orgId`, the backend `url`,
`user`, `database`, basic-auth members, `jsonData` and `secureJsonFields` never reach a result.
A data source missing one of the five members, or holding a value outside the bounds above,
makes the whole answer unusable (`unavailable`) rather than a shortened list.

Grafana answers the list unpaged. At most 1,000 records are returned, in Grafana's order;
`complete` is `false` when Grafana listed more, and `next_cursor` is always `null`.

## What the connection is bound to

**The identity is the configured connection, not a Grafana account.** Connect, repair and
`connections revalidate` prove the token with `GET /api/datasources` under
`grafana.service_account`, and with the fixed-window alert-group read under
`grafana.irm_service_account` (refused as `unavailable` without `irm_api_url`): a `200` admits
it, a `401` or `403` refuses it. The identity kind is `grafana.connection` and its subject is the
configuration's `instance`. Any token Grafana accepts is the same identity, so a repair cannot
detect a token of another organization. Use one `instance` per Grafana organization.

**A token grants no scopes and exposes no expiry.** The profile requires no scopes;
`credential_expires_at_ms` is absent, meaning not observed. A revoked or expired token is
reported as `unauthorized` when it is next used.

## Evidence lifetime

Validation evidence lasts 60 seconds. After it the connection lists as `pending` and a read is
refused with `not_granted` at admission until `connections revalidate` renews it with one more
probe. A scheduled caller revalidates before reading.

## IRM alert groups and incidents

Four reads, none of which acknowledges, resolves, silences or changes anything:

| Operation | Request | Returns |
|---|---|---|
| `irm.alert_groups.list` | `GET <irm_api_url>api/v1/alert_groups/?started_at=<since>_<until>&perpage=100&page=<n>` | alert groups started in the window, at most 100 per page |
| `irm.alert_group.get` | `GET <irm_api_url>api/v1/alert_groups/<id>` | one alert group, to re-read its state |
| `irm.incidents.list` | `POST <base_url>api/plugins/grafana-irm-app/resources/api/v1/IncidentsService.QueryIncidents` | incidents created in the window, at most 50 per page, and their links |
| `irm.incident.get` | `POST …/IncidentsService.GetIncident` | one incident and its links, to re-read its status |

Every request to `irm_api_url` carries `X-Grafana-URL` with `base_url` (without its final `/`),
as IRM requires; the value comes from the configuration, never from input. The two POSTs are
reads: the stack port may send exactly those two paths and nothing else.

Connect them under the second profile, whose probe is one alert-group read over a fixed
one-second window in 2000, so a token whose role grants the IRM reads (IRM's
`grafana-irm-app.alert-groups:read`, and the incident reads) and no data-source read connects:

```toml
[adapters.grafana.permissions]
profiles = ["grafana.service_account", "grafana.irm_service_account"]
operations = ["datasources.list", "irm.alert_groups.list", "irm.alert_group.get", "irm.incidents.list", "irm.incident.get"]
```

```sh
target/release/connectors --output json connections connect --adapter grafana --profile grafana.irm_service_account --credential-prompt
```

A connection keeps the profile it was made with: an IRM read on a `grafana.service_account`
connection is refused at admission, and `datasources.list` on an IRM connection likewise.

### The since window

Both list reads take `since` and `until` (required), each `YYYY-MM-DDTHH:MM:SSZ`: UTC, whole
seconds, `since` before `until`. Anything else, an offset included, is refused before any
request. The adapter always sends both bounds, so a call never means IRM's default of the last
30 days. A run stores `until` and passes it back as the next run's `since`; both bounds are
read as inclusive, so a record created at that instant can appear in both runs, and the caller
deduplicates by `id` or `incident_id`. Each answer echoes its `window`.

`next_cursor` is the next page of the **same window**; pass it back as `cursor` with the same
`since` and `until`, until `complete` is `true` and `next_cursor` is `null`. For alert groups it
is IRM's next page number (IRM's `next` URL is never followed); for incidents it is the incident
service's `cursor.nextValue`, unchanged. Alert groups can be filtered by exact `state`,
`integration_id` and `team_id`.

Both windows are on creation time (an alert group's start, an incident's creation). A later
change of state is not a new record in a later window: re-read it with `irm.alert_group.get` or
`irm.incident.get`. IRM documents no sort order for alert groups; incidents are asked for in
ascending order.

### What the records carry

An alert group carries `id`, `integration_id`, `route_id`, `alert_group_number`,
`alerts_count`, `state` (`new`, `acknowledged`, `resolved`, `silenced`), `title`, `created_at`,
`acknowledged_at`, `resolved_at`, `silenced_at` and `root_alert_group_id`; a member IRM
answers `null` is absent. `permalinks`, `last_alert` and its alert payload,
`dependent_alert_group_ids`, `acknowledged_by` and `resolved_by` are never read into a result.

An incident carries `incident_id`, `title`, `status` (`active`, `resolved`), `severity`,
`is_drill`, `created_time`, `modified_time` and `closed_time`; an empty `severity` or
`closedTime` is absent. Labels, people, roles, tasks, the summary, links and chat channels
are never read into a result. Timestamps are returned as the provider wrote them (RFC 3339).

A record outside these bounds makes the whole answer unusable (`unavailable`) rather than a
shortened page.

### Incident links (inferred)

`links` pairs each incident with the alert groups it references: every `refs[]` entry whose
`key` is `com.grafana.oncall`, its `ref` read as the alert group's `id`; other references yield
no link, and identical references one. An alert group carries no incident member, so the link
is read from the incident side only. **That this `ref` is the alert group's id is inferred**
from the incident client (an incident created from an alert group takes its `alertGroupID`,
and its OnCall reference carries a `ref`); it has not been confirmed against a real stack yet.

## Not built

Dashboard reads, datasource discovery (`datasources.observe`) and the mediated parent route
(`route.mediated_http`, with a sealed uid) of [the design](design.md) are not implemented. Of
IRM only the four reads above exist: no write, no escalation chains, schedules, shifts,
integrations or routes, no incident activity or tasks, no alert-group `search` and no `label`
filters.
Without the mediated route, a Loki connection through Grafana names the data source's uid in
its own `base_url`, and Grafana, not Connectors, decides which data sources the token reaches.

## Replacing the executable

The entry pins the executable's SHA-256. After rebuilding `connectors-grafana`, update `sha256`,
stop the running incarnation (`adapters stop`), then `connections revalidate` or repair the
connection, which performs the admitted launch under the same connection id.

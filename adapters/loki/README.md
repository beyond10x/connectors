# Loki through the local CLI

The `loki` connector answers five reads, each one GET against Loki's HTTP API:
`logs.query_range` (log lines), `logs.query_metric` (an instant or range metric query),
`logs.labels` (label names, or one label's values), and two reads through profiles the
operator configures: `logs.profile_lines` (one typed, masked record per line) and
`logs.call_trace` (one call's lines from every source). The mapping, bounds and errors are in
[the Loki profile](contracts/logs/v1alpha1/semantics.md) §§11–12; the connection is §11.4. It
requires Linux x86_64 and the [qualified Secret Service binding](../../docs/local-secret-service.md),
like every saved credential.

Connector, instance and connection are the terms of the [vocabulary](../../README.md#vocabulary).

```sh
CARGO_BUILD_JOBS=2 cargo build --release --locked -p connectors -p connectors-loki
target/release/connectors --output json setup init
target/release/connectors --output json setup check
```

## Configure the native target

Create an owner-only native JSON file, in an admitted directory without symlinks:

```json
{
  "format": "connectors-loki-local/1",
  "instance": "loki-prod",
  "base_url": "https://loki.example.internal/",
  "query_scope": {"required_equalities": []}
}
```

`base_url` must be HTTPS and written in canonical form: lowercase `https://` and host, ending in `/`, at most 512 characters; a path prefix, for a gateway that serves Loki below one, is kept.
`ca_file` may name an owner-only PEM bundle that replaces the system roots, for a deployment
with a private CA. `query_scope` is required and admits only the empty list: every query
reads the whole tenant the token reaches. The token is never part of this file, of the TOML
below, of executable arguments or of the environment.

Inspect the native bootstrap and hash the built executable:

```sh
target/release/connectors-loki --local-config /absolute/path/loki.json --print-local-bootstrap
sha256sum target/release/connectors-loki
```

Copy `configuration_revision` from that output verbatim. Loki is not a catalog connector, so
the configuration pins its instance. Add the instance to the configuration created by setup:
change its `format` line to `format = "connectors-local/5"`, which instances need, and add the
instance `logs` of the connector `loki`:

```toml
[connectors.loki]
enabled = true

[connectors.loki.instances.logs]
alias = "logs"
instance_id = "loki-prod"
adapter_id = "loki"
configuration_revision = "REPLACE_FROM_BOOTSTRAP"
protocol = "v1alpha1"
private_protocol = "connectors-private/1"
startup = "on-demand"
restart = "never"

[connectors.loki.instances.logs.permissions]
profiles = ["loki.bearer"]
operations = ["logs.query_range", "logs.query_metric", "logs.labels", "logs.profile_lines", "logs.call_trace"]

[connectors.loki.instances.logs.executable]
path = "/absolute/path/target/release/connectors-loki"
sha256 = "REPLACE_WITH_EXECUTABLE_SHA256"
args = ["--local-config", "/absolute/path/loki.json"]
```

## Connect and read

```sh
target/release/connectors --output json connections connect --connector loki --instance logs --profile loki.bearer --credential-prompt
target/release/connectors --output json operations describe --connector loki --instance logs --operation logs.query_range
target/release/connectors --output json operations invoke --connector loki --instance logs --connection CONNECTION --operation logs.query_range --schema SCHEMA --revision REVISION --input-json '{"query":"{app=\"api\"} |= \"error\"","start_unix_ns":"1788822000000000000","end_unix_ns":"1788825600000000000","limit":100}'
target/release/connectors --output json operations invoke --connector loki --instance logs --connection CONNECTION --operation logs.query_metric --schema SCHEMA --revision REVISION --input-json '{"query":"sum by (app) (count_over_time({app=\"api\"}[1m]))","start_unix_ns":"1788822000000000000","end_unix_ns":"1788825600000000000","step_seconds":60}'
target/release/connectors --output json operations invoke --connector loki --instance logs --connection CONNECTION --operation logs.labels --schema SCHEMA --revision REVISION --input-json '{"label":"app"}'
```

Each operation has its own schema and revision from `operations describe`. For automation,
`--credential-stdin` or `--credential-file /absolute/private/file.json` carries the complete
native `{"token":"..."}` document.

## Configured line and trace profiles

A profile is LogQL the operator writes once, in the configuration file; the caller only names
it (contract §12). Add `line_profiles` and `trace_profiles` to the native file. The labels,
keys and numbers below are invented; a deployment's own are written only in its own file:

```json
{
  "format": "connectors-loki-local/1",
  "instance": "loki-prod",
  "base_url": "https://loki.example.internal/",
  "query_scope": {"required_equalities": []},
  "line_profiles": [{
    "name": "fixture-cdr",
    "selector": [{"label": "service", "operator": "equal", "value": "edge-proxy"}],
    "format": {"kind": "logfmt"},
    "fields": [
      {"name": "direction", "key": "dir", "type": "string", "filterable": true},
      {"name": "caller", "key": "src", "type": "phone_number", "filterable": true},
      {"name": "started_at", "key": "start", "type": "unix_seconds"},
      {"name": "call_id", "key": "callid", "type": "string"}
    ],
    "call_identifier": "call_id"
  }],
  "trace_profiles": [{
    "name": "fixture-by-label",
    "selector": [{"label": "env", "operator": "equal", "value": "fixture"}],
    "call_identifier": {"kind": "label", "value": {"label": "call"}},
    "source_label": "service"
  }]
}
```

A changed profile changes `configuration_revision`: copy the new one into the instance.
A pattern format is `{"kind": "pattern", "value": {"pattern": "fx-cdr <dir> id=<callid>"}}`;
a call identifier carried in the line is
`{"kind": "field", "value": {"key": "callid", "format": {"kind": "logfmt"}}}`. Phone numbers
are masked to their last four characters unless a field declares `"mask": "none"`.

```sh
target/release/connectors --output json operations invoke --connector loki --instance logs --connection CONNECTION --operation logs.profile_lines --schema SCHEMA --revision REVISION --input-json '{"profile":"fixture-cdr","start_unix_ns":"1788822000000000000","end_unix_ns":"1788825600000000000","match":{"caller":"+15550100001"}}'
target/release/connectors --output json operations invoke --connector loki --instance logs --connection CONNECTION --operation logs.call_trace --schema SCHEMA --revision REVISION --input-json '{"profile":"fixture-by-label","call_id":"fx-leg-0001","start_unix_ns":"1788822000000000000","end_unix_ns":"1788825600000000000"}'
```

The first answers one record per CDR line whose caller is exactly that number, with the
number masked; the second answers the lines of one call from every source, oldest first,
each naming its source. Neither accepts LogQL from the caller.

## What the connection is bound to

**The identity is the configured connection, not a Loki account.** Loki has no user or account
read. Connect, reconnect and `connections revalidate` prove the token with
`GET /loki/api/v1/labels`: a `200` admits it, a `401` or `403` refuses it. The identity kind is
`loki.connection` and its subject is the configuration's `instance`. Any token the deployment
accepts is the same identity, so a reconnect cannot detect a token of another tenant. Use one
`instance` per deployment and tenant.

**A token grants no scopes and exposes no expiry.** The profile requires no scopes;
`credential_expires_at_ms` is absent, meaning not observed. A revoked token is reported as
`unauthorized` when it is next used.

**No tenant header is sent.** A multi-tenant Loki that requires `X-Scope-OrgID` from the client
is not reachable; a gateway that derives the tenant from the token is.

## Through Grafana

Grafana's data-source proxy serves a Loki data source's HTTP API below
`/api/datasources/proxy/uid/<uid>/` and authenticates the request with a Grafana
service-account token. A Loki connection reads through it unchanged: set `base_url` to that
path and connect with the Grafana token as the `loki.bearer` token.

```json
{
  "format": "connectors-loki-local/1",
  "instance": "grafana-prod-loki",
  "base_url": "https://grafana.example.internal/api/datasources/proxy/uid/P8E80F9AEF21F6940/",
  "query_scope": {"required_equalities": []}
}
```

The uid is the `uid` member of the [Grafana connector's](../grafana/README.md) `datasources.list`
record whose `type` is `loki` and `access` is `proxy`. The probe and every read then go to
`<base_url>loki/api/v1/...` with `Authorization: Bearer <Grafana token>`
(`tests/local_runtime.rs`,
`a_connection_through_the_grafana_datasource_proxy_reads_below_the_proxy_prefix`). Grafana,
not Connectors, decides which data sources the token reaches and which credentials Grafana
adds toward Loki. The identity is still the configured `instance`: use one per Grafana and
data source.

Recent logs, such as the last fifteen minutes, are `logs.query_range` with `start_unix_ns`
fifteen minutes before now and no `end_unix_ns`, which ends the window at the receiver clock.

## Evidence lifetime

Validation evidence lasts 60 seconds. After it the connection lists as `pending` and a read is
refused with `not_granted` at admission until `connections revalidate` renews it with one more
labels probe. A scheduled caller revalidates before reading.

## Replacing the executable

The instance pins the executable's SHA-256. After rebuilding `connectors-loki`, update `sha256`,
stop the running incarnation (`instances stop`), then `connections revalidate` or reconnect the
connection, which performs the admitted launch under the same connection id.

# Loki through the local CLI

The `loki` adapter answers three LogQL reads, each one GET against Loki's HTTP API:
`logs.query_range` (log lines), `logs.query_metric` (an instant or range metric query) and
`logs.labels` (label names, or one label's values). The mapping, bounds and errors are in
[the Loki profile](contracts/logs/v1alpha1/semantics.md) §11; the connection is §11.4. It
requires Linux x86_64 and the [qualified Secret Service binding](../../docs/local-secret-service.md),
like every saved credential.

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

Copy `configuration_revision` from that output verbatim, then add an adapter entry to the
configuration created by setup:

```toml
[adapters.logs]
instance_id = "loki-prod"
adapter_id = "loki"
configuration_revision = "REPLACE_FROM_BOOTSTRAP"
protocol = "v1alpha1"
startup = "on-demand"
restart = "never"

[adapters.logs.permissions]
profiles = ["loki.bearer"]
operations = ["logs.query_range", "logs.query_metric", "logs.labels"]

[adapters.logs.executable]
path = "/absolute/path/target/release/connectors-loki"
sha256 = "REPLACE_WITH_EXECUTABLE_SHA256"
args = ["--local-config", "/absolute/path/loki.json"]
```

## Connect and read

```sh
target/release/connectors --output json connections connect --adapter logs --profile loki.bearer --credential-prompt
target/release/connectors --output json operations describe --adapter logs --operation logs.query_range
target/release/connectors --output json operations invoke --adapter logs --connection CONNECTION --operation logs.query_range --schema SCHEMA --revision REVISION --input-json '{"query":"{app=\"api\"} |= \"error\"","start_unix_ns":"1788822000000000000","end_unix_ns":"1788825600000000000","limit":100}'
target/release/connectors --output json operations invoke --adapter logs --connection CONNECTION --operation logs.query_metric --schema SCHEMA --revision REVISION --input-json '{"query":"sum by (app) (count_over_time({app=\"api\"}[1m]))","start_unix_ns":"1788822000000000000","end_unix_ns":"1788825600000000000","step_seconds":60}'
target/release/connectors --output json operations invoke --adapter logs --connection CONNECTION --operation logs.labels --schema SCHEMA --revision REVISION --input-json '{"label":"app"}'
```

Each operation has its own schema and revision from `operations describe`. For automation,
`--credential-stdin` or `--credential-file /absolute/private/file.json` carries the complete
native `{"token":"..."}` document.

## What the connection is bound to

**The identity is the configured connection, not a Loki account.** Loki has no user or account
read. Connect, repair and `connections revalidate` prove the token with
`GET /loki/api/v1/labels`: a `200` admits it, a `401` or `403` refuses it. The identity kind is
`loki.connection` and its subject is the configuration's `instance`. Any token the deployment
accepts is the same identity, so a repair cannot detect a token of another tenant. Use one
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

The uid is the `uid` member of the [Grafana adapter's](../grafana/README.md) `datasources.list`
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

The entry pins the executable's SHA-256. After rebuilding `connectors-loki`, update `sha256`,
stop the running incarnation (`adapters stop`), then `connections revalidate` or repair the
connection, which performs the admitted launch under the same connection id.

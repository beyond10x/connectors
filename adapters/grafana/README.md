# Grafana through the local CLI

The `grafana` adapter answers one read, `datasources.list`: the data sources a Grafana service
account can read, each as its uid, name, plugin type, access mode and default flag, from one
`GET /api/datasources`. The record is modeled in [spec/ess](spec/ess/domains/records.yaml) and
the connection in [the connection domain](spec/ess/domains/connection.yaml). It requires Linux
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
with a private CA. The token is never part of this file, of the TOML below, of executable
arguments or of the environment.

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
`connections revalidate` prove the token with `GET /api/datasources`: a `200` admits it, a
`401` or `403` refuses it. The identity kind is `grafana.connection` and its subject is the
configuration's `instance`. Any token Grafana accepts is the same identity, so a repair cannot
detect a token of another organization. Use one `instance` per Grafana organization.

**A token grants no scopes and exposes no expiry.** The profile requires no scopes;
`credential_expires_at_ms` is absent, meaning not observed. A revoked or expired token is
reported as `unauthorized` when it is next used.

## Evidence lifetime

Validation evidence lasts 60 seconds. After it the connection lists as `pending` and a read is
refused with `not_granted` at admission until `connections revalidate` renews it with one more
probe. A scheduled caller revalidates before reading.

## Not built

Dashboard reads, datasource discovery (`datasources.observe`) and the mediated parent route
(`route.mediated_http`, with a sealed uid) of [the design](design.md) are not implemented.
Without the mediated route, a Loki connection through Grafana names the data source's uid in
its own `base_url`, and Grafana, not Connectors, decides which data sources the token reaches.

## Replacing the executable

The entry pins the executable's SHA-256. After rebuilding `connectors-grafana`, update `sha256`,
stop the running incarnation (`adapters stop`), then `connections revalidate` or repair the
connection, which performs the admitted launch under the same connection id.

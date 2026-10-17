---
title: PostgreSQL
sidebar_position: 15
description: Run bounded, parameterized read-only queries and catalogue reads against a PostgreSQL database with a saved password.
lede: One PostgreSQL instance per role and database runs read-only queries inside a read-only transaction with fixed timeouts, and has no write.
source: docs/local-postgres-cli.md
connectors: "postgres"
readme_row: "SQL"
---

# PostgreSQL

The `postgres` connector is served by its own executable, `connectors-sql`, which also serves
[MySQL](./mysql.md); the README's support table lists both as SQL. The words connector,
instance, connection and operation are those of the
[README vocabulary](https://github.com/beyond10x/connectors#vocabulary); the
[full PostgreSQL guide](https://github.com/beyond10x/connectors/blob/main/docs/local-postgres-cli.md)
has every bound and refusal.

No declaration ships for it, so the instance is pinned: it names its executable, that
executable's SHA-256 and the configuration revision the executable prints for your native file.
Commands that need a database are shown without output.

## What it reads and writes

| Reads | Writes |
|---|---|
| `schema.list`, `query.read`, `database.list`, `table.list`, `table.describe`, `index.list`, `profile.read` | none |

`query.read` runs a parameterized statement inside a read-only transaction with fixed statement
and lock timeouts. `profile.read` runs a read you wrote once in the native file, by name.

## Prerequisites at the provider

A role that can log in to the database with a password, granted `SELECT` on what you read and
nothing more. The server's TLS certificate must chain to the system roots or to a CA file you
name. Build the executable:

```sh
cargo build --release --locked -p connectors -p connectors-sql
```

## Enable the connector and add an instance

Write an owner-only native file, `postgres.json`:

```json
{
  "format": "connectors-sql-local/1",
  "instance": "postgres-local",
  "host": "db.example",
  "port": 5432,
  "database": "incidents",
  "user": "reader",
  "allow_plaintext": false,
  "ca_file": "/absolute/path/ca.pem"
}
```

Print its bootstrap, hash the executable, and pin both in the instance:

```sh
target/release/connectors-sql --local-config /absolute/path/postgres.json --print-local-bootstrap
sha256sum target/release/connectors-sql
```

```toml
[connectors.postgres]
enabled = true

[connectors.postgres.instances.warehouse]
alias = "warehouse"
instance_id = "postgres-local"
adapter_id = "sql"
configuration_revision = "REPLACE_FROM_BOOTSTRAP"
protocol = "v1alpha1"
private_protocol = "connectors-private/1"
startup = "on-demand"
restart = "never"

[connectors.postgres.instances.warehouse.permissions]
profiles = ["postgres.password"]
operations = ["schema.list", "query.read", "database.list", "table.list", "table.describe", "index.list"]

[connectors.postgres.instances.warehouse.executable]
path = "/absolute/path/connectors-sql"
sha256 = "REPLACE_WITH_EXECUTABLE_SHA256"
args = ["--local-config", "/absolute/path/postgres.json"]
```

Add `profile.read` to `operations` once the native file has `read_profiles`.

## Connect

```sh
connectors connections connect --connector postgres --instance warehouse \
  --profile postgres.password --credential-prompt
```

The startup exchange is the credential check: no query runs. The identity is the role and the
database, `reader@incidents`.

## A first read

```sh
connection=$(connectors --output json connections list --connector postgres --instance warehouse \
  | jq -r '.result.connections[0].connection')
schema=$(connectors --output json operations describe --connector postgres --instance warehouse \
  --operation query.read | jq -r .result.schema)
revision=$(connectors --output json operations describe --connector postgres --instance warehouse \
  --operation query.read | jq -r .result.revision)
connectors operations invoke --connector postgres --instance warehouse --connection "$connection" \
  --operation query.read --schema "$schema" --revision "$revision" \
  --input-json '{"query":"SELECT id, opened_at FROM incidents WHERE severity = $1 ORDER BY opened_at DESC","parameters":["critical"],"limit":100}'
```

Parameters are bound by the server, never interpolated into the text.

## A first guarded write

The connector has no write: no DML, DDL or transaction control is exposed, so nothing here needs
an approval.

## Reconnect when the connection is Stale

A rotated or expired password makes the connection `stale` when it is next used, and an invoke
answers `next_action: repair_connection`. Publish the new password for the same role:

```sh
connectors --output json connections list --connector postgres --instance warehouse \
  | jq -c '[.result.connections[] | {connection, revision, state}]'
connectors connections reconnect --connector postgres --instance warehouse \
  --expected-revision "<revision from the list>" --credential-prompt
```

A failed reconnect keeps the still-valid saved password.

## Limits

- Six real PostgreSQL cases ran on 2026-10-02 (the [full guide](https://github.com/beyond10x/connectors/blob/main/docs/local-postgres-cli.md#running-the-bounded-provider-acceptance-cases) shows how to run them):
  restart, reads, read-only refusals and drop cancellation; the plaintext fixture does not
  establish TLS.
- A password grants no scopes: a privilege denial appears when the query runs.
- Read profiles are checked against scripted wire fixtures only.
- No cursor, stored procedure or connection pooling; a Unix socket host is refused.

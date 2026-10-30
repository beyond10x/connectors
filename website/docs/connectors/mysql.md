---
title: MySQL
sidebar_position: 16
description: Run bounded, parameterized read-only queries and catalogue reads against a MySQL 8 database with a saved password.
lede: One MySQL instance per user and database runs the same read-only reads as PostgreSQL through the same executable, and has no write.
source: docs/local-mysql-cli.md, docs/local-postgres-cli.md, adapters/sql/tests/catalogue_adversary.rs
connectors: "mysql"
readme_row: "SQL"
---

# MySQL

The `mysql` connector is served by `connectors-sql`, the executable that serves
[PostgreSQL](./postgres.md), chosen by `"engine": "mysql"` in the native file; the README's
support table lists both as SQL. The words connector, instance, connection and operation are
those of the [README vocabulary](https://github.com/beyond10x/connectors#vocabulary); the
[full MySQL guide](https://github.com/beyond10x/connectors/blob/main/docs/local-mysql-cli.md) has
every bound and refusal.

No declaration ships for it, so the instance is pinned. Commands that need a database are shown
without output.

## What it reads and writes

| Reads | Writes |
|---|---|
| `schema.list`, `query.read`, `database.list`, `table.list`, `table.describe`, `index.list`, `profile.read` | none |

## Prerequisites at the provider

MySQL 8.0 or later, and a user that logs in with a password, granted `SELECT` only and no
`EXECUTE` on routines with side effects. Without `allow_plaintext` the server must offer TLS.
Build the executable:

```sh
cargo build --release --locked -p connectors -p connectors-sql
```

## Enable the connector and add an instance

Write an owner-only native file, `mysql.json`:

```json
{
  "format": "connectors-sql-local/1",
  "engine": "mysql",
  "instance": "mysql-local",
  "host": "db.example",
  "port": 3306,
  "database": "incidents",
  "user": "reader",
  "allow_plaintext": false,
  "ca_file": "/absolute/path/ca.pem"
}
```

Print its bootstrap and hash the executable:

```sh
target/release/connectors-sql --local-config /absolute/path/mysql.json --print-local-bootstrap
sha256sum target/release/connectors-sql
```

Pin the instance as for [PostgreSQL](./postgres.md#enable-the-connector-and-add-an-instance), with
the connector `mysql`, the instance `incidents`, `instance_id = "mysql-local"`, the profile
`mysql.password`, this bootstrap's revision and `args` naming `mysql.json`:

```toml
[connectors.mysql]
enabled = true

[connectors.mysql.instances.incidents]
alias = "incidents"
instance_id = "mysql-local"
adapter_id = "sql"
configuration_revision = "REPLACE_FROM_BOOTSTRAP"
protocol = "v1alpha1"
private_protocol = "connectors-private/1"
startup = "on-demand"
restart = "never"

[connectors.mysql.instances.incidents.permissions]
profiles = ["mysql.password"]
operations = ["schema.list", "query.read", "database.list", "table.list", "table.describe", "index.list"]

[connectors.mysql.instances.incidents.executable]
path = "/absolute/path/connectors-sql"
sha256 = "REPLACE_WITH_EXECUTABLE_SHA256"
args = ["--local-config", "/absolute/path/mysql.json"]
```

## Connect

```sh
connectors connections connect --connector mysql --instance incidents \
  --profile mysql.password --credential-prompt
```

The handshake is the credential check; the identity is `reader@incidents`.

## A first read

```sh
connectors invoke mysql/incidents query.read \
  --input-json '{"query":"SELECT id, opened_at FROM incidents WHERE severity = ? ORDER BY opened_at DESC","parameters":["critical"],"limit":100}'
```

MySQL placeholders are `?`, not `$1`.

## A first guarded write

The connector has no write: no DML, DDL, transaction control or `LOAD DATA LOCAL` is exposed, so
nothing here needs an approval.

## Reconnect when the connection is Stale

A rotated password makes the connection `stale` when it is next used (the server's error 1045,
reported as `unauthorized`). Publish the new password:

```sh
connectors --output json connections list --connector mysql --instance incidents \
  | jq -c '[.result.connections[] | {connection, revision, state}]'
connectors connections reconnect --connector mysql --instance incidents \
  --expected-revision "<revision from the list>" --credential-prompt
```

## Limits

- Verified against a scripted MySQL wire fixture; the catalogue reads also ran on 2026-10-09
  against a disposable MySQL 8.0.46 server. There is no other live MySQL evidence.
- MariaDB is not tested.
- The read-only session does not stop a routine's own side effects: grant `SELECT` only.

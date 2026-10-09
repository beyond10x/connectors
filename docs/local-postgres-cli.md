# PostgreSQL through the local CLI

The current local binding lists schemas and runs bounded, parameterized
read-only queries using a saved PostgreSQL password. It requires Linux x86_64 and
the [qualified Secret Service binding](local-secret-service.md).

The same adapter serves MySQL when its native file says `"engine": "mysql"`; see
[MySQL through the local CLI](local-mysql-cli.md). A file without `engine`
selects PostgreSQL, so every existing configuration and connection keeps working
unchanged.

PostgreSQL is the third execution family in this repository. GitLab and
Kubernetes reach their provider over HTTP; this adapter speaks the PostgreSQL
wire protocol, so there is no HTTP capability, no bearer header and no identity
probe endpoint. The differences that follow from that are in
[What the connection is bound to](#what-the-connection-is-bound-to).

```sh
CARGO_BUILD_JOBS=2 cargo build --release --locked -p connectors -p connectors-sql
target/release/connectors --output json setup init
target/release/connectors --output json setup check
```

## Configure the native target

Create an owner-only native JSON file, in an admitted directory without symlinks:

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

`host` must be a network host: a leading `/` would select a Unix socket
directory, which carries no TLS and no host authority this binding can record,
and is refused. `allow_plaintext` disables TLS and is for a local fixture only.
The optional `ca_file` names an owner-only PEM certificate file; supply the
server's CA there when it is not in the system store.

The password is never part of this file, of the TOML below, of executable
arguments or of the environment.

Inspect the native bootstrap and hash the built executable:

```sh
target/release/connectors-sql --local-config /absolute/path/postgres.json --print-local-bootstrap
sha256sum target/release/connectors-sql
```

Copy `configuration_revision` from that output verbatim. It is a digest of the
effective document, which includes a `ca_digest` computed over the CA bytes in a
form of the adapter's own choosing — it is **not** `sha256sum` of the PEM file,
and no command reproduces it independently.

The published `configuration_schema` has two branches, and the local one
describes the **effective** document the composition builds, not the file above:
there the CA is reduced to `ca_digest`. Nothing validates your file against that
branch, so do not write the file to match it.

Add an adapter entry to the configuration created by setup:

```toml
[adapters.warehouse]
instance_id = "postgres-local"
adapter_id = "sql"
configuration_revision = "REPLACE_FROM_BOOTSTRAP"
protocol = "v1alpha1"
private_protocol = "connectors-private/1"
startup = "on-demand"
restart = "never"

[adapters.warehouse.permissions]
profiles = ["postgres.password"]
operations = ["schema.list", "query.read", "database.list", "table.list", "table.describe", "index.list"]

[adapters.warehouse.executable]
path = "/absolute/path/connectors-sql"
sha256 = "REPLACE_WITH_EXECUTABLE_SHA256"
args = ["--local-config", "/absolute/path/postgres.json"]
```

Omitted permission sets deny access, and an operation left out of `operations`
is refused at `operations describe` as well as at invocation.

## Connect and read

```sh
target/release/connectors --output json connections connect --adapter warehouse --profile postgres.password --credential-prompt
target/release/connectors --output json operations describe --adapter warehouse --operation query.read
target/release/connectors --output json operations invoke --adapter warehouse --connection CONNECTION --operation query.read --schema SCHEMA --revision REVISION --input-json '{"query":"SELECT id, opened_at FROM incidents WHERE severity = $1 ORDER BY opened_at DESC","parameters":["critical"],"limit":100}'
```

For automation, `--credential-file /absolute/private/file.json` or
`--credential-stdin` carries the complete native `{"password":"..."}` document.
Files must be singly linked, regular, owner-only (0400 or 0600), with admitted
ancestors. Protected values are not accepted as argv or printed in results.

To hand the saved password to another program, such as a service that reads a
password file, pin that program as a consumer and start it with
`connections launch`: it receives the `{"password":"..."}` document on file
descriptor 3. See [Launch a consumer](local-consumer-launch.md).

## What the connection is bound to

**The session is the credential check.** PostgreSQL accepts or rejects the
password during the startup exchange, before any statement can run, so connect
and repair open one session and close it without running a query. No validation
statement is invented, because the handshake already proves more than one would.
A rejected password comes back as the server's own `28P01`.

**The identity is the role and the database**, recorded as `reader@incidents`.
The role name is the principal PostgreSQL authorises against and the database
scopes it. The protected repair document accepts only a password; it cannot
select another role. Changing `user`, `host`, `port` or `database` changes the
configuration revision. A native file that no longer matches the configured
revision is refused as `readiness_mismatch` at startup, before a session opens.
Failed password repair on the original binding preserves its still-valid saved
credential. These are the reachable SQL repair boundaries; a password alone
cannot produce a different role identity.

**A password grants no scopes and exposes no expiry.** The profile declares no
required scopes and the saved connection records none, so a successful
validation is not evidence that any particular table is readable: a privilege
denial appears when the query runs. `credential_expires_at_ms` is absent, meaning
not observed rather than unlimited; a rotated or expired password is reported
when it is next used.

## Query bounds, which the adapter enforces and the caller cannot widen

Every invocation opens its own session and its own **read-only transaction**,
then sets `statement_timeout = '10s'`, `lock_timeout = '2s'` and
`search_path = public, pg_catalog` for that transaction only. One prepared
statement runs, with parameters bound positionally — values are never
interpolated into the text.

| bound | value |
|---|---|
| query text | 1 to 8192 bytes, one statement, optional trailing `;` |
| parameters | at most 64, each at most 8192 bytes |
| `limit` | 1 to 1000 rows |
| columns | 1 to 256 |
| connect / query / request | 5 s, 10 s, 15 s, with 2 s reserved for cleanup |

A request that exceeds an input bound is refused before a session is opened.
Oversized result values return `capacity`; empty rows retain column metadata,
and row truncation is reported separately. A caller cannot extend the execution
deadline with `set_config()`.

Dropping the native `Sql::invoke` future triggers backend-keyed cancellation and
bounded local cleanup. It does not guarantee remote termination after a network
failure or process crash. CLI disconnect is a different boundary: an already
dispatched read may finish. Killing the CLI does not test native future-drop
cancellation.

Writes are not refused by a keyword filter: the transaction is `READ ONLY`, so
PostgreSQL itself rejects any statement that would write.

## Catalogue reads

Four reads describe the catalogue without a hand-written query. Each runs one fixed
statement in the same read-only transaction, with the same deadlines, `limit` of 1 to 1000
rows and `truncated` flag as `query.read`; the schema and table you pass are bound as
parameters, never written into the statement. Every answer is the `query.read` shape —
`columns`, `rows` of text or `null`, `truncated` and `provenance` — with fixed columns:

```sh
target/release/connectors --output json operations invoke --adapter warehouse --connection CONNECTION --operation database.list --schema SCHEMA --revision REVISION --input-json '{"limit":100}'
target/release/connectors --output json operations invoke --adapter warehouse --connection CONNECTION --operation table.list --schema SCHEMA --revision REVISION --input-json '{"schema":"public","limit":200}'
target/release/connectors --output json operations invoke --adapter warehouse --connection CONNECTION --operation table.describe --schema SCHEMA --revision REVISION --input-json '{"schema":"public","table":"incidents","limit":200}'
target/release/connectors --output json operations invoke --adapter warehouse --connection CONNECTION --operation index.list --schema SCHEMA --revision REVISION --input-json '{"schema":"public","table":"incidents","limit":200}'
```

| operation | columns |
|---|---|
| `database.list` | `database_name`: the databases the role may connect to (`pg_database`, without templates) |
| `table.list` | `table_name`, `table_kind` (`table` or `view`), `row_estimate` (`pg_class.reltuples`; `null` for a view or a table never analysed) |
| `table.describe` | `column_name`, `native_type` (`format_type`, such as `character varying(64)`), `is_nullable` (`YES`/`NO`), `column_default`, `ordinal_position`, `primary_key_position`, `foreign_key`, `referenced_schema`, `referenced_table`, `referenced_column` |
| `index.list` | `index_name`, `table_name`, `column_position`, `column_name`, `is_unique` (`YES`/`NO`), `is_primary` (`YES`/`NO`) |

For example, `table.describe` of a table whose `team_id` references `teams(id)` answers
rows such as:

```json
[["id","bigint","NO","nextval('incidents_id_seq'::regclass)","1","1",null,null,null,null],
 ["team_id","integer","YES",null,"2",null,"incidents_team_fk","public","teams","id"]]
```

`schema` is required for the table and index reads, as for `schema.list`. `index.list`
without `table` lists the indexes of every table of the schema, one row per key column.
`table.describe` of a table that does not exist, or whose columns the role cannot see, is
`not_found`; `table.list` and `index.list` answer an empty `rows` instead.

The reads hide what the role cannot see, as `information_schema` does. A generated column's
`column_default` is `null` (its generation expression is not a default). A foreign key is
reported only when the role holds `USAGE` on the referenced schema, a privilege on the
referenced table and a column privilege on every referenced column; otherwise each of its
columns is described without it, so `table.describe` never names a referenced column that
`table.describe` of the referenced table hides. A key into a partitioned table is one row per
column, not one more per partition, and a partition reports the key it inherits. `index.list`
leaves out an index built on any column `table.describe` hides: a key, INCLUDE or expression
column, or a column in a partial index's predicate. A whole-row reference such as
`(t IS NOT NULL)` counts as every column, and so does an index on expressions alone, such as
`((pub + 1))`: a role granted only some columns of a table does not see it.

Add the operations you grant to `operations` in the adapter entry. These reads are checked
against a scripted PostgreSQL wire fixture; their visibility rules were also run against a
disposable PostgreSQL 17 server on 2026-10-09.

## Limitations

MySQL is served by the same adapter; see [the MySQL guide](local-mysql-cli.md). No write,
DDL, transaction control, cursor, stored procedure or connection pooling is
exposed. `schema.list`, `query.read` and the four catalogue reads are the whole surface.

The [retained real-provider restart evidence](evidence/provider-restarts-20261002/README.md)
covers a disposable PostgreSQL server and saved-credential reuse. Wire-protocol
fixtures remain separate from that provider evidence. Neither establishes TLS
for the plaintext loopback sandbox, and neither covers MySQL.

## Running the bounded provider acceptance cases

The SQL `local_runtime` suite contains five additional opt-in real-provider cases:
join/group/UTC/quoted parameters; empty/truncated/capacity/timeout outcomes;
read-only escape refusals; native invocation-drop cancellation; and repair,
revoke and busy-stop authority. Four use the production CLI and qualified
disposable Secret Service. The cancellation case invokes the production `Sql`
library directly, observes the exact marked backend within two seconds, then
checks termination within five seconds of dropping the invocation. A paired
no-drop control must still run after five seconds. This is a controlled fixture
observation, not a universal cancellation deadline.

Use only an owned PostgreSQL 17.6 container whose published port is bound to
`127.0.0.1`, with the disposable `incidents` database and the fixture `reader`
credential used by the existing restart journey. The reader must have no
superuser, role-creation or database-creation authority. The host needs Docker,
and the container needs `psql` with local fixture-admin access. Admin access is
used only for fixture setup, observation and cleanup; SUT queries use `reader`.
Do not point this suite at a production database or a shared container.

```sh
CARGO_BUILD_JOBS=2 cargo build --release --locked -p connectors -p connectors-sql
CONNECTORS_TEST_CLI="$PWD/target/release/connectors" \
CONNECTORS_PG_SANDBOX=127.0.0.1:PORT \
CONNECTORS_PG_CONTAINER=OWNED_CONTAINER \
TMPDIR=/absolute/short/private/tmp \
CARGO_BUILD_JOBS=2 cargo test --release --locked -p connectors-sql \
  --test local_runtime cli_journey:: -- --ignored --nocapture --test-threads=1
```

The three environment selections contain paths or fixture coordinates, never
passwords. The physical temporary directory must be owner-private and short
enough for the private Unix sockets. Each case creates private custody and
configuration, and its own database schema where needed. Cleanup drops only that
schema and terminates only a correlated test backend. Explicit selection with
missing prerequisites fails; an ignored case in the ordinary package run is
unexecuted evidence, not a passing provider case.

Write refusal checks distinguish columnless statements and unsupported PostgreSQL
features (`unsupported`, including SQLSTATE `0A000`), rejected read-subquery syntax
or multiple statements (`invalid_input`), and the mandatory
read-only transaction. They also compare independently observed fixture contents;
an arbitrary transport failure is not accepted as write protection.

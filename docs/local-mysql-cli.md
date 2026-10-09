# MySQL through the local CLI

The SQL adapter serves MySQL as well as PostgreSQL, chosen per connection by the
`engine` member of its native file. A MySQL connection has the same reads —
`schema.list`, `query.read` and the four [catalogue reads](#catalogue-reads) — the same
input bounds and the same output shape as a PostgreSQL one; what differs is the wire
protocol, the profile id and how a value is written. Everything this guide does not
repeat — setup, the TOML entry, credential prompts and files, consumer launch — is as in
the [PostgreSQL guide](local-postgres-cli.md).

```sh
CARGO_BUILD_JOBS=2 cargo build --release --locked -p connectors -p connectors-sql
```

## Configure the native target

Create an owner-only native JSON file, in an admitted directory without symlinks:

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

`engine` is `mysql` or `postgresql`. It defaults to `postgresql`, so a file
written before the engine existed keeps its meaning and its configuration
revision; naming `postgresql` explicitly yields the same revision. Any other
value is refused. The port is required for both engines: no default is taken
from the engine.

The other members mean what they mean for PostgreSQL. `host` must be a network
host (a leading `/` is refused). Without `allow_plaintext`, TLS is required
before the user name or any password material is sent: a server that does not
offer TLS is refused. `ca_file` names an owner-only PEM bundle that **replaces**
the public roots for this connection; an empty or malformed bundle is refused
before a session opens. `allow_plaintext` is for a local fixture only.

The password is never part of this file, of the TOML, of executable arguments or
of the environment.

```sh
target/release/connectors-sql --local-config /absolute/path/mysql.json --print-local-bootstrap
sha256sum target/release/connectors-sql
```

Copy `configuration_revision` from that output verbatim into the adapter entry,
and permit the MySQL profile:

```toml
[adapters.incidents]
instance_id = "mysql-local"
adapter_id = "sql"
configuration_revision = "REPLACE_FROM_BOOTSTRAP"
protocol = "v1alpha1"
private_protocol = "connectors-private/1"
startup = "on-demand"
restart = "never"

[adapters.incidents.permissions]
profiles = ["mysql.password"]
operations = ["schema.list", "query.read", "database.list", "table.list", "table.describe", "index.list"]

[adapters.incidents.executable]
path = "/absolute/path/connectors-sql"
sha256 = "REPLACE_WITH_EXECUTABLE_SHA256"
args = ["--local-config", "/absolute/path/mysql.json"]
```

## Connect and read

```sh
target/release/connectors --output json connections connect --adapter incidents --profile mysql.password --credential-prompt
target/release/connectors --output json connections revalidate --adapter incidents --connection CONNECTION --expected-revision CONNECTION_REVISION
target/release/connectors --output json operations invoke --adapter incidents --connection CONNECTION --operation schema.list --schema SCHEMA --revision REVISION --input-json '{"limit":200}'
target/release/connectors --output json operations invoke --adapter incidents --connection CONNECTION --operation query.read --schema SCHEMA --revision REVISION --input-json '{"query":"SELECT id, opened_at FROM incidents WHERE severity = ? ORDER BY opened_at DESC","parameters":["critical"],"limit":100}'
```

MySQL placeholders are `?`, not `$1`. Parameters are text or `null` and are
bound positionally by the server; they are never interpolated into the text.

The credential document is `{"password":"..."}`, as for PostgreSQL.

## What the connection is bound to

**The session is the credential check.** The server accepts or rejects the
password during its handshake (`caching_sha2_password`, or
`mysql_native_password` on servers that ask for it), before any statement runs.
Connect, repair and `connections revalidate` open one session and close it
without running a statement. A rejected password is the server's own error 1045,
reported as `unauthorized`.

**The identity is the user and the database**, kind `mysql.user`, recorded as
`reader@incidents`. The provider authority is `mysql://<host>:<port>/<database>`,
so a changed engine, host, port or database is a changed configuration rather
than a new session. The profile is `mysql.password`; a MySQL connection does not
advertise `postgres.password`, and the reverse.

**A password grants no scopes and exposes no expiry**, as on PostgreSQL.

## Query bounds and the read-only session

Every invocation opens its own session, and before the caller's statement is
prepared runs:

```sql
SET SESSION TRANSACTION READ ONLY
SET SESSION max_execution_time = 10000, SESSION lock_wait_timeout = 2, SESSION innodb_lock_wait_timeout = 2, SESSION time_zone = '+00:00'
```

The session time zone is UTC so that a `TIMESTAMP` reaches the adapter as its
instant in UTC whatever the server's own `time_zone` is (see below). It also
means that time functions evaluated in the session, such as `NOW()` or
`CURRENT_TIMESTAMP`, answer in UTC.

The caller's statement is then prepared once to read its columns and parameter
count. A statement that returns no columns — any `INSERT`, `UPDATE`, `DELETE`,
`SET`, `CALL` or DDL — is refused as `unsupported` before it is executed, as on
PostgreSQL; so is one returning more than 256 columns. A parameter count the
statement does not take is `invalid_input`. The statement then runs as a derived
table, `SELECT * FROM (<query>) AS result (c0,…) LIMIT <limit+1>`, so the server
sends at most one row past the limit and only one result-returning statement
runs. The derived table closes on a line of its own, so a trailing `--` or `#`
comment in the statement cannot reach the aliases or the limit.

What this guarantees is a read-only session plus one result-returning
statement. A write to a table, including one inside a stored function, is
refused by the server (error 1792, reported as `forbidden`). It does **not**
stop the side effects of a routine the statement calls: a function the user may
execute runs with its own rights, and on a live MySQL 8.0.46 a definer-rights
function called as `SELECT f()` through `query.read` ran `SET PERSIST`
(`SET GLOBAL` likewise) and the change persisted. The adapter cannot tell such a
call from a built-in function. **Grant the configured user `SELECT` only, and no
`EXECUTE` on any routine with side effects** (server variables, external
calls, definer-rights writes); the user's grants, not the adapter, are the
boundary for routines.

The input bounds, the 1,000-row limit and the 5 s / 10 s / 15 s deadlines with 2 s
reserved for cleanup are the PostgreSQL path's. When the adapter's own deadline
passes, or the invocation is dropped, it kills the statement with
`KILL QUERY <connection id>` from a second session, then closes the first; a
server that ignores the kill cannot hold the session past the cleanup budget.
Like PostgreSQL's cancel request, this does not guarantee remote termination
after a network failure or process crash.

Errors are classified without the server's message: access refusals (1044, 1142,
1143, 1227, 1370) and the read-only refusal (1792) are `forbidden`; 1045 is
`unauthorized`; the server's own `max_execution_time` (3024) is `timeout` and too
many connections (1040, 1203) is `capacity`, both marked as the server's answer;
otherwise the SQLSTATE class decides, as on PostgreSQL (`42000` syntax errors are
`invalid_input`).

## How values are written: the `mysql-native-text` profile

Rows have the same shape as on PostgreSQL: every cell is a JSON string or
`null`, and each column carries its MySQL type name (`bigint`,
`bigint unsigned`, `decimal`, `datetime`, `varbinary`, `json`, …). The
descriptor of a MySQL connection names the profile `mysql-native-text`:

| MySQL column | written as | example |
|---|---|---|
| any NULL | JSON `null` | `null` |
| integers, signed or unsigned, and `YEAR` | decimal text, never a JSON number | `"18446744073709551615"` |
| `DECIMAL` | the server's exact decimal text | `"12345678901234567890.0123"` |
| `FLOAT`, `DOUBLE` | the shortest text that reads back as the same value: positional on a tie, otherwise exponent form (PostgreSQL writes `1e+300`) | `"1.5"`, `"1e300"`, `"1e-4"` |
| `DATE` | ISO 8601 date | `"2026-10-09"` |
| `DATETIME` | ISO 8601 naive local date-time with `T`, fraction to the column's precision, no offset: `DATETIME` records no time zone | `"2026-10-09T08:07:06.000123"` |
| `TIMESTAMP` | the instant in UTC: ISO 8601 date-time with `T`, fraction to the column's precision, then `Z` | `"2026-10-25T00:30:00Z"` |
| `TIME` | `[-]HH:MM:SS[.ffffff]`, hours unbounded (an elapsed time of up to 838 hours, not a time of day) | `"-26:03:04"` |
| binary strings and BLOBs, `BIT`, geometry | standard base64 with padding | `"AAEC/w=="` |
| text strings, `ENUM`, `SET`, `JSON` | the text itself | `"snow 雪"` |

A text cell that is not valid UTF-8 is refused as `unsupported` rather than
replaced. Integers stay strings on purpose: the contract never coerces database
values to JSON numbers, so a 64-bit value cannot lose digits. These rules are
modeled in `adapters/sql/spec/ess` (`connectors_sql.reads.MysqlCellRule`) and
checked on the wire by `adapters/sql/tests/mysql_protocol.rs`.

A connection is bound to one database, and on MySQL a schema is a database, so
`schema.list` reads only the connected database: omit `schema`, or pass the
connected database's exact name. Any other name is refused as `invalid_input`
before a session is opened; to read another database, configure a connection for
it. The answer's provenance resource is the connected database. It reads
`information_schema.COLUMNS` for that database, and the columns are those of the PostgreSQL path (`table_schema`, `table_name`,
`column_name`, `data_type`, `udt_name`, `is_nullable`, `ordinal_position`);
`udt_name` carries MySQL's full `COLUMN_TYPE`, such as `bigint unsigned` or
`varchar(255)`.

## Catalogue reads

The four catalogue reads of the [PostgreSQL guide](local-postgres-cli.md#catalogue-reads)
serve MySQL with the same columns, bounds and read-only session, each as one fixed
`information_schema` statement with the schema and table bound as `?` parameters:

```sh
target/release/connectors --output json operations invoke --adapter incidents --connection CONNECTION --operation database.list --schema SCHEMA --revision REVISION --input-json '{"limit":100}'
target/release/connectors --output json operations invoke --adapter incidents --connection CONNECTION --operation table.list --schema SCHEMA --revision REVISION --input-json '{"limit":200}'
target/release/connectors --output json operations invoke --adapter incidents --connection CONNECTION --operation table.describe --schema SCHEMA --revision REVISION --input-json '{"table":"incidents","limit":200}'
target/release/connectors --output json operations invoke --adapter incidents --connection CONNECTION --operation index.list --schema SCHEMA --revision REVISION --input-json '{"table":"incidents","limit":200}'
```

- `database.list` reads `information_schema.SCHEMATA`: the databases the user holds a
  privilege on (every one with `SHOW DATABASES`), `information_schema` included. It lists
  names only and opens no other database.
- `table.list`, `table.describe` and `index.list` read only the connected database, as
  `schema.list` does: omit `schema` or pass the connected database's exact name; any other
  is refused as `invalid_input` before a session is opened.
- `table.list` reports base tables as `table` and views as `view`; `row_estimate` is
  `TABLE_ROWS`, cached statistics that InnoDB only approximates, and `null` for a view.
- `table.describe` reads `COLUMNS` and `KEY_COLUMN_USAGE`: `native_type` is the full
  `COLUMN_TYPE` (`bigint unsigned`, `varchar(255)`), `column_default` MySQL's own text of the
  default, `primary_key_position` the column's place in `PRIMARY`, and each foreign key the
  referenced schema, table and column. Positions are decimal text: a row reads
  `["team_id","int","YES","0","2",null,"incidents_team_fk","incidents","teams","id"]`. A
  generated column's `column_default` is `null`. A table that does not exist, or whose
  columns the user cannot see, is `not_found`.
- Foreign keys follow MySQL's own `KEY_COLUMN_USAGE` visibility, which differs from
  PostgreSQL: a foreign key of a table the user can see names its referenced schema, table
  and column even when the user holds no privilege on them, so `table.describe` can name a
  database that `database.list` does not list, or a column the user cannot read. On
  PostgreSQL such a key is left out, as is a key into a column the role holds no privilege
  on.
- `index.list` reads `STATISTICS`: one row per key part, `PRIMARY` as the primary key's
  name, and a functional key part's expression as `column_name`. Without `table` it lists
  every table's indexes; a table that does not exist is an empty answer.
- A table name MySQL cannot hold is `not_found` on `table.describe` and `index.list`. MySQL
  forbids a name ending in a space, and its metadata collation pads, so `orders ` would
  otherwise match `orders`: such a name is refused before a session is opened. A name with a
  character the `utf8mb3` metadata cannot represent (such as an emoji) makes the server answer
  error 3988, which these reads report as `not_found`; `query.read` still reports that error
  as `unavailable`.

## Limitations

- Verified against a scripted MySQL wire fixture on loopback (handshake, TLS,
  authentication, the session statements, prepared statements with typed binary
  rows, `information_schema` reads and `KILL QUERY`), not against a live MySQL
  server. There is no real-provider acceptance evidence for MySQL yet.
- MySQL 8.0 or later is assumed: the derived-table column list and
  `max_execution_time` are MySQL features. MariaDB is not tested.
- `schema.list` gives column metadata only; keys, indexes, views and row estimates come from
  the catalogue reads, which are checked on the same scripted fixture, not a live server.
- The read-only session does not stop a routine's side effects: a function the user
  may execute can change server variables (`SET PERSIST`, `SET GLOBAL`) or write with
  definer rights. Grant `SELECT` only, and no `EXECUTE` on such routines.
- No write, DDL, transaction control, cursor, stored procedure, `LOAD DATA LOCAL`
  or connection pooling is exposed. The adapter never follows a server-named Unix
  socket and installs no local-file handler.

# PostgreSQL through the local CLI

The current local binding lists schemas and runs bounded, parameterized
read-only queries using a saved PostgreSQL password. It requires Linux x86_64 and
the [qualified Secret Service binding](local-secret-service.md).

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
startup = "on-demand"
restart = "never"

[adapters.warehouse.permissions]
profiles = ["postgres.password"]
operations = ["schema.list", "query.read"]

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

## What the connection is bound to

**The session is the credential check.** PostgreSQL accepts or rejects the
password during the startup exchange, before any statement can run, so connect
and repair open one session and close it without running a query. No validation
statement is invented, because the handshake already proves more than one would.
A rejected password comes back as the server's own `28P01`.

**The identity is the role and the database**, recorded as `reader@incidents`.
The role name is the principal PostgreSQL authorises against and the database
scopes it. A `connections repair` for a different role is an identity mismatch
and the existing valid credential is preserved. Changing `host`, `port` or
`database` changes the configuration revision instead, and is refused at startup
before any session opens — it is a different binding, not a renewed one.

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

A request that exceeds a bound is refused before a session is opened. A caller
cannot extend the deadline with `set_config()`, and a dropped invocation still
sends the backend cancel key rather than leaving the server executing.

Writes are not refused by a keyword filter: the transaction is `READ ONLY`, so
PostgreSQL itself rejects any statement that would write.

## Limitations

MySQL is not implemented and belongs to the remaining-provider phase. No write,
DDL, transaction control, cursor, stored procedure or connection pooling is
exposed. `schema.list` and `query.read` are the whole surface.

No runtime evidence against a dedicated PostgreSQL sandbox has been recorded.
The journeys above are verified against a PostgreSQL wire fixture on loopback
with fictional credentials, in the same shape as the existing protocol tests. A
real server is required before this binding is called proven.

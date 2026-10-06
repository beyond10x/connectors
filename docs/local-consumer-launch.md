# Launch a consumer with a saved connection's credential

`connections launch` runs a program you pinned in the configuration and gives it one
saved connection's credential on file descriptor 3. The program never sees the
credential in its arguments, its environment or a file, and neither does the shell or
the process that asked for the launch. It requires Linux and the
[qualified Secret Service binding](local-secret-service.md). The contract is
[consumer launch](../contracts/cli/v1alpha1/consumer-launch.md).

A typical consumer is a service that reads a database password from a file: a
[PostgreSQL connection](local-postgres-cli.md) saved once with
`connections connect` supplies the password, and the service reads
`/proc/self/fd/3` as its password file.

## Pin the consumer

Consumers need configuration format `connectors-local/3`. Change the `format` line of
an existing `connectors-local/2` file; nothing else in it changes. Then add one
entry per consumer:

```toml
format = "connectors-local/3"

[consumers.store]
pass_env = ["STORE_"]

[consumers.store.executable]
path = "/opt/store/bin/store"
sha256 = "<sha256sum of /opt/store/bin/store>"
args = ["--password-file", "/proc/self/fd/3"]

[consumers.store.permissions]
connections = ["warehouse"]
```

- `path` and `sha256` pin the exact file, as for an adapter. Replace the digest when
  you upgrade the program; until then every launch is refused as
  `invalid_configuration`.
- `args` is the start of the program's argument list. Every launch begins with it;
  the caller's `--args` are added after it.
- `pass_env` lists name prefixes of your environment variables the program gets,
  here every `STORE_…` variable. Without it the program starts with an empty
  environment.
- `permissions.connections` lists the adapter aliases whose connections this
  consumer may receive. Without it the consumer receives none.

## Launch

```sh
STORE_HOME=/srv/store target/release/connectors connections launch \
  --adapter warehouse --connection CONNECTION --consumer store \
  --args '["serve","--port","8080"]'
```

`--args` is one JSON array of strings. Its elements are appended to the pinned
`args` exactly as written — here the program runs as
`store --password-file /proc/self/fd/3 serve --port 8080`. A value that is not a
JSON array of strings is refused as `invalid_input` before anything else happens.

The program runs in the foreground with your terminal's stdin, stdout and stderr and
only the variables `pass_env` selects. When it exits, `connectors` exits with its
status. Pressing Ctrl-C interrupts the program, which decides how to stop;
`connectors` waits for it.

The credential is the document saved with the connection, byte for byte — for
PostgreSQL the `{"password":"..."}` JSON document. No adapter starts and no provider
is contacted for a launch.

## When a launch is refused

A refused launch runs nothing and prints the usual `failure` envelope on stderr.

| `code` | Meaning | What to do |
|---|---|---|
| `invalid_input` (stage `arguments`) | `--args` is not a JSON array of strings, or an element holds NUL | fix the value |
| `not_found` | no such adapter, consumer or connection | check the names |
| `forbidden` | the consumer does not list this adapter, or the adapter no longer grants the connection's profile | add the alias to `permissions.connections` |
| `invalid_configuration` | the program's file no longer matches its `sha256` | update the digest after checking the file |
| `unavailable` (stage `readiness`) | the connection's evidence has lapsed | `connections revalidate` |
| `custody_unavailable` | the keyring is locked or unavailable | unlock it |
| `lifecycle_conflict` | the connection was saved under an older configuration revision, or the configuration changed during the launch | `connections revalidate`, or retry |

Any process running as your user can ask for a launch of a configured consumer, with
any `--args` and any values of the variables `pass_env` admits, the same as for every
other `connectors` command. You pin the program, the start of its argument list and
which variables pass; the caller chooses the rest. Pin only programs you would trust
with the connection's credential under arguments you did not choose.

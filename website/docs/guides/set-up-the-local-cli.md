---
title: Set up the local CLI
sidebar_position: 2
description: Create the local CLI's private configuration and metadata store, check its prerequisites and configure the instances you will connect.
lede: setup init creates a private configuration and an Entity Runtime metadata store; setup check says whether this machine can hold credentials before anything asks for one, and an instance needs only a name.
source: apps/connectors/tests/local_cli.rs, apps/connectors/tests/setup_missing_state.rs, apps/connectors/tests/derived_instance.rs, crates/connectors-host/tests/local_foundation.rs, crates/connectors-host/src/local/config.rs, contracts/cli/v1alpha1/semantics.md, docs/local-mounted-secret-custody.md, docs/local-container.md
---

# Set up the local CLI

The grouped `connectors` commands (`setup`, `instances`, `connections`, `operations`, `approvals`)
and `invoke` work through a local owner process, a private TOML configuration and a metadata
store. This guide creates both in a scratch directory, checks the machine's prerequisites and
configures two instances. It needs no credential and contacts no provider. Run it from
`target/try` as in [Getting started](../getting-started.md), after
`cargo build --locked -p connectors -p connectors-catalog-provider`. The words connector, instance
and connection are those of the [README vocabulary](https://github.com/beyond10x/connectors#vocabulary).

## Create the configuration and the store

`--config` and `--state-dir` select where they live. Without `--config` the CLI uses
`$XDG_CONFIG_HOME/connectors/config.toml`, or `~/.config/connectors/config.toml` when
`XDG_CONFIG_HOME` is unset or empty. Without `--state-dir` it uses `$XDG_STATE_HOME/connectors`,
or `~/.local/state/connectors` when `XDG_STATE_HOME` is unset or empty. Both must be absolute
paths:

```sh
../debug/connectors --config cli/config.toml --state-dir cli/state --output json setup init
```

```json
{"error":{"code":"failure","data":{"code":"invalid_configuration","configuration_rule":"config_path_unresolved","kind":"usage","next_action":"check_configuration","stage":"configuration"}},"ok":false}
```

With absolute paths it creates them. The answer also carries both paths, and `jq` keeps the rest:

```sh
../debug/connectors --config "$PWD/cli/config.toml" --state-dir "$PWD/cli/state" \
  --output json setup init | jq '{ok, disposition: .result.disposition, os: .result.os}'
```

```json
{
  "ok": true,
  "disposition": "created",
  "os": "linux"
}
```

The configuration holds its owner and no credential. The state directory holds the metadata
store: Entity Runtime records over an Eventlog SQLite database. `setup init` creates it with
durable open checkpoints, which shorten the provider's own open. Every fresh open still verifies
the whole store, so an edit that bypasses SQLite is refused by the next command.

## Check the prerequisites

```sh
../debug/connectors --config "$PWD/cli/config.toml" --state-dir "$PWD/cli/state" \
  --output json setup check \
  | jq '{ok, keyring: .result.keyring, custody_backend: .result.custody_backend, prerequisites: [.result.prerequisites[] | "\(.name): \(.state)"]}'
```

```json
{
  "ok": true,
  "keyring": "available",
  "custody_backend": "secret_service",
  "prerequisites": [
    "configuration: ready",
    "metadata: ready",
    "installed_provider: ready",
    "keyring: ready",
    "persistent_custody_qualification: ready"
  ]
}
```

`setup check` authenticates nowhere. `keyring` is the Linux Secret Service that will hold saved
credentials. A machine without one cannot save a connection, and the check says so before any
connect asks for a credential.

`custody_backend` is the store custody uses, fixed when the binary is built: `secret_service` on
Linux, `macos_keychain` on macOS and `windows_credential_manager` on Windows.

`installed_provider` is the `connectors-catalog-provider` in the directory of `connectors`, which
derives every instance that pins nothing. It is `ready` when it prints its version. Otherwise it
is `failed`, and `installed_provider_failure` gives the reason: `missing`, `not_executable`,
`killed`, `exited`, `deadline` or `unreadable_output`.

A headless host or a container has no Secret Service. There, a `connectors-local/4` or
`connectors-local/5` configuration can select the read-only `mounted_secret` binding with a
`[custody]` table. The binding reads each credential from a read-only mounted directory, such as
a Kubernetes Secret volume, and writes nothing. Only that table selects it. Without the table, an
unusable Secret Service still refuses with `custody_unavailable`. Under it `setup check` reports
`custody_backend: mounted_secret` and a `mounted_custody` prerequisite.

[Mounted-secret custody](https://github.com/beyond10x/connectors/blob/main/docs/local-mounted-secret-custody.md)
describes the binding and its threat model, and
[running in a container](https://github.com/beyond10x/connectors/blob/main/docs/local-container.md)
the setup, with a CronJob example.

## Configure instances

`setup init` writes a `connectors-local/5` file with an empty legacy table for pinned entries.
Replace its content with a configuration that enables the
connectors you use and names an instance of each:

```sh
cat > cli/config.toml <<EOF
format = "connectors-local/5"
owner_uid = $(id -u)
default_instance = "gitlab/work"

[connectors.gmail]
enabled = true

[connectors.gmail.instances.private]
description = "personal mailbox"

[connectors.gitlab]
enabled = true

[connectors.gitlab.instances.work]
endpoint = "https://gitlab.example.com"
EOF
```

`gmail/private` uses Gmail's default endpoint. `gitlab/work` points at a self-hosted GitLab host.
Neither pins anything else: `connectors` derives each instance from the
`connectors-catalog-provider` in its own directory and the connector declaration shipped with it.
These give the executable and its SHA-256, the configuration revision and every operation.

A connector that declares more than one auth profile (Zendesk) also takes `profile` on the
instance. `default_instance` selects an instance for a command that names none. `invoke` takes
its connector in every call and does not read `default_instance`.

```sh
../debug/connectors --config "$PWD/cli/config.toml" --state-dir "$PWD/cli/state" instances list \
  | jq -c '.[] | arrays | .[]'
```

```json
{"connector":"gitlab","instance":"work","instance_id":"gitlab.work","restart":"never","startup":"on-demand"}
{"connector":"gmail","instance":"private","instance_id":"gmail.private","restart":"never","startup":"on-demand"}
```

Each item identifies its instance by connector and instance only. A command that acts on one
instance takes `--connector` and `--instance`. Here no connection exists yet:

```sh
../debug/connectors --config "$PWD/cli/config.toml" --state-dir "$PWD/cli/state" \
  connections list --connector gitlab --instance work
```

```json
{"connections":[],"observed_at_ms":1793314648161,"source":"authority","stale":false,"valid_until_ms":1793314948161}
```

## Invoke an operation

`invoke` takes the connector and, after a `/`, the instance, then the operation. The instance may
be left out while the connector has only one, and the input is read from standard input when no
input flag is given. The connection, schema and revision default to the instance's own. Without a
connection the call is refused before anything is started:

```sh
printf '{"id":"group/project","merge_request_iid":9}\n' \
  | ../debug/connectors --config "$PWD/cli/config.toml" --state-dir "$PWD/cli/state" \
    --output json invoke gitlab merge_request.get
```

```json
{"error":{"code":"failure","data":{"code":"connection_unresolved","connections":[],"kind":"operational","next_action":"create_connection","stage":"admission"}},"ok":false}
```

`connections` lists each connection the instance holds, with its state, and is empty here. A
connector with two instances, named without one, is refused as `instance_ambiguous`, listing
each as `<connector>/<instance>`. `--describe` prints an operation's input schema, its effect
(read or write), schema and revision without calling the provider, once the instance holds a
connection.

`setup init` never overwrites:

```sh
../debug/connectors --config "$PWD/cli/config.toml" --state-dir "$PWD/cli/state" \
  --output json setup init
```

```json
{"error":{"code":"failure","data":{"code":"configuration_exists","kind":"operational","next_action":"check_configuration","stage":"configuration"}},"ok":false}
```

A configuration with legacy pinned entries keeps working, and `setup migrate` moves them to
instances, as the [README](https://github.com/beyond10x/connectors#move-a-legacy-configuration)
shows.

## Reset a grown metadata store

The store keeps every record it is given, and nothing prunes it yet: each read adds about 35 KB
([#267](https://github.com/beyond10x/connectors/issues/267)), and every fresh open verifies the
whole store, so commands slow down as it grows. Until the store has retention, the remedy is a
new store in the same state directory. The configuration stays as it is. Everything the old store
recorded is gone from the new one: connections, list cursors, approvals and cached descriptions.
Each instance connects again afterwards.

First stop the owner, so that no process holds the store:

```sh
../debug/connectors --config "$PWD/cli/config.toml" --state-dir "$PWD/cli/state" \
  --output json setup owner-stop | jq '{ok, disposition: .result.disposition}'
```

```json
{
  "ok": true,
  "disposition": "no_owner"
}
```

Move the store aside: `metadata.sqlite3` and, where they exist, its `metadata.sqlite3-wal` and
`metadata.sqlite3-shm` files. Leave every other file in the state directory where it is,
`metadata.lock` and `registry-clock.floor` included. Do not replace the whole directory.

```sh
mkdir "$PWD/cli/store-aside" && mv "$PWD"/cli/state/metadata.sqlite3* "$PWD/cli/store-aside/"
```

A command that reads the store now names the missing store and the command that creates it:
`metadata_missing` with `next_action` `initialize_state`, exit status 1. Running the command again
does not change that answer:

```sh
../debug/connectors --config "$PWD/cli/config.toml" --state-dir "$PWD/cli/state" \
  connections list --connector gitlab --instance work
```

```text
failure: {"code":"metadata_missing","kind":"operational","next_action":"initialize_state","stage":"observation"}
```

`setup init` with the existing configuration creates the new store and leaves the configuration
unchanged:

```sh
../debug/connectors --config "$PWD/cli/config.toml" --state-dir "$PWD/cli/state" \
  --output json setup init | jq '{ok, disposition: .result.disposition}'
```

```json
{
  "ok": true,
  "disposition": "state_initialized"
}
```

Then connect each instance again with `connections connect` (see [Next](#next)). With Secret
Service custody, the old connections' items stay in the keyring under the old store's authority:
`setup custody-orphans` lists them, and `setup custody-orphans --mode remove --authority <id>`
deletes them once you no longer need the store you moved aside.

## After an upgrade

An owner the previous binary started keeps running after an upgrade, and commands that need it
are refused as `owner_build_mismatch`. [Upgrade the CLI](./upgrade-the-cli.md#after-an-upgrade)
shows how `setup owner-stop` retires it, and what a state directory admits across versions.

## Next

`connections connect --connector <connector> --instance <instance> --profile <profile>`
validates a credential through the named auth profile and saves it in custody. The profile is
`gitlab.pat` for GitLab, and each connector's page lists its profiles. Each instance holds one
connection.
[The local runtime](../concepts/local-runtime.md) explains what the owner, custody and the
metadata store guarantee, the
[runtime binding](https://github.com/beyond10x/connectors/blob/main/docs/local-runtime-foundation.md)
shows the commands implemented today, and the [CLI reference](../reference/cli.md) lists every
command.
Before you upgrade the binary, read [Upgrade the CLI](./upgrade-the-cli.md).

Every connector has a page that takes it from the provider's prerequisites through an instance,
a connection and a first read to a first approved write: start at
[Connectors](../connectors/index.md).

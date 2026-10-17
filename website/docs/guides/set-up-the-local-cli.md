---
title: Set up the local CLI
sidebar_position: 2
description: Create the local CLI's private configuration and metadata store, check its prerequisites and configure the instances you will connect.
lede: setup init creates a private configuration and an Entity Runtime metadata store; setup check says whether this machine can hold credentials before anything asks for one, and an instance needs only a name.
source: apps/connectors/tests/local_cli.rs, apps/connectors/tests/derived_instance.rs, crates/connectors-host/tests/local_foundation.rs, crates/connectors-host/src/local/config.rs, contracts/cli/v1alpha1/semantics.md, docs/local-mounted-secret-custody.md, docs/local-container.md
---

# Set up the local CLI

The grouped `connectors` commands (`setup`, `instances`, `connections`, `operations`,
`approvals`) work through a local owner process, a private TOML configuration and a metadata
store. This guide creates both in a scratch directory, checks the machine's prerequisites and
configures two instances. It needs no credential and contacts no provider. Run it from
`target/try` as in [Getting started](../getting-started.md), after
`cargo build --locked -p connectors -p connectors-catalog-provider`. The words connector,
instance and connection are those of the
[README vocabulary](https://github.com/beyond10x/connectors#vocabulary).

## Create the configuration and the store

`--config` and `--state-dir` select where they live; without them the CLI uses
`$XDG_CONFIG_HOME/connectors/config.toml` (or `~/.config/connectors/config.toml`) and
`$XDG_STATE_HOME/connectors` (or `connectors` in the local state directory under your home). Both must be absolute paths:

```sh
../debug/connectors --config cli/config.toml --state-dir cli/state --output json setup init
```

```json
{"error":{"code":"failure","data":{"code":"invalid_configuration","configuration_rule":"config_path_unresolved","kind":"usage","next_action":"check_configuration","stage":"configuration"}},"ok":false}
```

With absolute paths it creates them. The answer also names both paths; `jq` keeps the rest:

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

The configuration names its owner and no credential. The state directory holds the metadata
store: Entity Runtime records over an Eventlog SQLite database. `setup init` creates it with
durable open checkpoints, which shorten the provider's own open; every fresh open still verifies
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
    "keyring: ready",
    "persistent_custody_qualification: ready"
  ]
}
```

`setup check` authenticates nowhere. `keyring` is the Linux Secret Service that will hold saved
credentials; a machine without one cannot save a connection, and the check says so before any
connect asks for a credential. `custody_backend` names the store custody uses, fixed when the
binary is built: `secret_service` on Linux, `macos_keychain` on macOS and
`windows_credential_manager` on Windows.

A headless host or a container has no Secret Service. There, a `connectors-local/4` or
`connectors-local/5` configuration can select the read-only `mounted_secret` binding with a
`[custody]` table: it
reads each credential from a read-only mounted directory, such as a Kubernetes Secret volume,
and writes nothing. It is never chosen on its own; without the table an unusable Secret Service
still refuses with `custody_unavailable`. Under it `setup check` reports
`custody_backend: mounted_secret` and a `mounted_custody` prerequisite.
[Mounted-secret custody](https://github.com/beyond10x/connectors/blob/main/docs/local-mounted-secret-custody.md)
describes the binding and its threat model, and
[running in a container](https://github.com/beyond10x/connectors/blob/main/docs/local-container.md)
the setup, with a CronJob example.

## Configure instances

`setup init` writes the older `connectors-local/2` format, with an empty legacy table for
pinned entries. Replace its content with a `connectors-local/5` configuration that enables the
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

`gmail/private` uses Gmail's default endpoint; `gitlab/work` names a self-hosted GitLab host.
Neither pins anything else: `connectors` derives each instance from the
`connectors-catalog-provider` in its own directory and the connector declaration shipped with it,
which give the executable and its SHA-256, the configuration revision and every operation. A
connector that declares more than one auth profile (Zendesk) also takes `profile` on the
instance. `default_instance` selects an instance for a command that names none.

```sh
../debug/connectors --config "$PWD/cli/config.toml" --state-dir "$PWD/cli/state" instances list
```

```json
{"adapters":[{"adapter":"gitlab.work","adapter_id":"catalog","connector":"gitlab","instance":"work","instance_id":"gitlab.work","restart":"never","startup":"on-demand"},{"adapter":"gmail.private","adapter_id":"catalog","connector":"gmail","instance":"private","instance_id":"gmail.private","restart":"never","startup":"on-demand"}],"source":"configuration"}
```

The result also carries each instance's legacy `adapter` alias and its internal `adapter_id`,
kept for the deprecation window. A command that acts on one instance names it with `--connector`
and `--instance`; no connection exists yet:

```sh
../debug/connectors --config "$PWD/cli/config.toml" --state-dir "$PWD/cli/state" \
  connections list --connector gitlab --instance work
```

```json
{"connections":[],"observed_at_ms":1792181808006,"source":"authority","stale":false,"valid_until_ms":1792182108006}
```

`setup init` never overwrites:

```sh
../debug/connectors --config "$PWD/cli/config.toml" --state-dir "$PWD/cli/state" \
  --output json setup init
```

```json
{"error":{"code":"failure","data":{"code":"configuration_exists","kind":"operational","next_action":"check_configuration","stage":"configuration"}},"ok":false}
```

A configuration with legacy pinned entries keeps working, and `setup migrate` moves them to
instances; the [README](https://github.com/beyond10x/connectors#move-a-legacy-configuration)
shows it.

## Binaries of different versions and one state directory

The metadata store records the newest binary that wrote it and a digest of the record model that
binary persists. Once a binary has written a state directory, an older binary may not use it:
binaries of different versions share one state directory only while the oldest of them is at
least the newest that wrote it. An older binary whose record model differs refuses the store
before it reads anything, in every command and `setup check` included: exit status 1, failure
code `metadata_schema_newer` at stage `admission` with `next_action` `upgrade`, and the failure
names `store_version` (the newest writer's release) and `binary_version` (its own). Retrying does
not change that answer; upgrading the binary does. A newer release that changed no
persisted record is admitted. When the newest writer cannot be named, because the store was
migrated to a level this binary does not know or its recorded version is not one this binary can
order, the refusal is the same with `store_version` `unrecorded`. Releases up to 0.49.0 cannot read the marker: on a store a newer
binary wrote they answer `metadata_unavailable` with `next_action` `retry_status` instead, and
retrying does not help them either.

A consumer pinned to one version keeps its own state directory. `setup init` with the existing
configuration and a new `--state-dir` initialises an empty store there and leaves the
configuration unchanged:

```sh
../debug/connectors --config "$PWD/cli/config.toml" --state-dir "$PWD/pinned/state" \
  --output json setup init | jq '{ok, disposition: .result.disposition}'
```

```json
{
  "ok": true,
  "disposition": "state_initialized"
}
```

It then connects in that state directory, passing the same `--state-dir` to `connections connect`
and to every later command. A connection is recorded in the
state directory it was made in: connections, list cursors, approvals and cached instance
descriptions in another state directory do not carry over, so the pinned consumer connects once
more there.

## After an upgrade

The owner is the background `connectors` process that serves a state directory; a connect or an
invoke starts it. An owner the previous binary started keeps running after an upgrade, and every
command that needs it is refused with code `owner_build_mismatch` at stage `readiness`, with
`next_action` `stop_owner` and exit status 1
(`apps/connectors/tests/local_cli.rs`, `a_cli_refuses_an_owner_running_a_different_build_and_leaves_it_running`).

`setup owner-stop` asks that owner, whatever binary started it, to drain and exit, and waits up
to 30 seconds for it. Here no owner runs, since nothing has connected:

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

With no owner running it answers `no_owner`, exits 0 and never starts one. With one, it answers
`"disposition": "stopped"` and names the `host_incarnation` that exited; the next command that
needs an owner starts one from the current binary. It sends the owner's own shutdown request over
the state directory's socket and never signals a process. An owner that holds
the state directory but does not answer within the 30 seconds is refused as `unavailable`; one
that is still running at the bound is refused as `timeout`; and a state directory, socket or lock
file of another user is refused as `forbidden`, each at stage `stop`.

## Next

`connections connect --connector <connector> --instance <instance>` validates a credential
through the instance's auth profile and saves it in custody; each instance holds one connection.
[The local runtime](../concepts/local-runtime.md) explains what the owner, custody and the
metadata store guarantee, and the [CLI reference](../reference/cli.md) lists every command.

Every connector has a page that takes it from the provider's prerequisites through an instance,
a connection and a first read to a first approved write: start at
[Connectors](../connectors/index.md).

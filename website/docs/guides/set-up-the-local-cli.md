---
title: Set up the local CLI
sidebar_position: 2
description: Create the local CLI's private configuration and metadata store, and check its prerequisites before adding an adapter.
lede: setup init creates a private configuration and an Entity Runtime metadata store; setup check says whether this machine can hold credentials before anything asks for one.
source: apps/connectors/tests/local_cli.rs, crates/connectors-host/tests/local_foundation.rs, contracts/cli/v1alpha1/semantics.md
---

# Set up the local CLI

The grouped `connectors` commands (`setup`, `adapters`, `connections`, `operations`, `approvals`)
work through a local owner process, a private TOML configuration and a metadata store. This guide
creates both in a scratch directory and checks the machine's prerequisites. It needs no adapter
and no credential. Run it from `target/try` as in [Getting started](../getting-started.md).

## Create the configuration and the store

`--config` and `--state-dir` select where they live; without them the CLI uses
`$XDG_CONFIG_HOME/connectors/config.toml` (or `~/.config/connectors/config.toml`) and
`$XDG_STATE_HOME/connectors` (or `connectors` in the local state directory under your home). Both must be absolute paths:

```sh
../debug/connectors --config cli/config.toml --state-dir cli/state --output json setup init
```

```json
{"error":{"code":"failure","data":{"code":"invalid_configuration","kind":"usage","next_action":"check_configuration","stage":"configuration"}},"ok":false}
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

```sh
cat cli/config.toml
```

```toml
# Linux Secret Service custody is required; credentials never belong here.
format = "connectors-local/2"
owner_uid = 1000

[adapters]
```

The configuration names its owner and no credential. The state directory holds the metadata
store: Entity Runtime records over an Eventlog SQLite database. `setup init` creates it with
durable open checkpoints, which shorten the provider's own open; every fresh open still verifies
the whole store, so an edit that bypasses SQLite is refused by the next command.

## Check the prerequisites

```sh
../debug/connectors --config "$PWD/cli/config.toml" --state-dir "$PWD/cli/state" \
  --output json setup check \
  | jq '{ok, keyring: .result.keyring, prerequisites: [.result.prerequisites[] | "\(.name): \(.state)"]}'
```

```json
{
  "ok": true,
  "keyring": "available",
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
connect asks for a credential.

## Nothing is configured yet

```sh
../debug/connectors --config "$PWD/cli/config.toml" --state-dir "$PWD/cli/state" \
  --output json adapters list
```

```json
{"ok":true,"result":{"adapters":[],"source":"configuration"}}
```

`setup init` never overwrites:

```sh
../debug/connectors --config "$PWD/cli/config.toml" --state-dir "$PWD/cli/state" \
  --output json setup init
```

```json
{"error":{"code":"failure","data":{"code":"configuration_exists","kind":"operational","next_action":"check_configuration","stage":"configuration"}},"ok":false}
```

## Next

An adapter entry under `[adapters]` names its executable, the digest of that executable and the
configuration revision it serves; `connections connect` then validates a credential and saves it
in custody. The repository's guides walk through one provider each, with the exact
configuration:

- [GitLab through the catalog provider](https://github.com/beyond10x/connectors/blob/main/docs/local-catalog-provider.md)
- [Kubernetes through the local CLI](https://github.com/beyond10x/connectors/blob/main/docs/local-kubernetes-cli.md)
- [PostgreSQL through the local CLI](https://github.com/beyond10x/connectors/blob/main/docs/local-postgres-cli.md)
- [Tavily](https://github.com/beyond10x/connectors/blob/main/docs/local-tavily.md)

[The local runtime](../concepts/local-runtime.md) explains what the owner, custody and the
metadata store guarantee, and the [CLI reference](../reference/cli.md) lists every command.

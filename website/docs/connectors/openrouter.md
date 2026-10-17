---
title: OpenRouter
sidebar_position: 13.5
description: Read an OpenRouter account's management key, workspaces and API keys, and create API keys under approval into custody, as two pinned instances.
lede: OpenRouter runs as two pinned instances of the openrouter connector, one holding the management key that reads and creates keys under approval, and one holding the keys it creates, because no OpenRouter declaration ships yet.
source: docs/catalog-openrouter.md, adapters/catalog/providers/openrouter/operations.json, adapters/catalog/tests/openrouter.rs, docs/local-mutation-ledger.md
connectors: "openrouter"
---

# OpenRouter

The `openrouter` connector of the provider OpenRouter is served by the catalog provider from the
pinned OpenRouter OpenAPI document. The words connector, instance, connection and operation are
those of the [README vocabulary](https://github.com/beyond10x/connectors#vocabulary); the
[full OpenRouter guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-openrouter.md)
has every parameter, refusal and the fixture run its outputs come from.

The catalog provider ships no connector declaration for OpenRouter yet, so neither instance is
derived: each pins its executable, that executable's SHA-256 and its configuration revision.
Commands use `connectors` on your `PATH` with your own configuration; a command that needs an
OpenRouter account is shown without output.

## What it reads and writes

| Key | Reads | Writes |
|---|---|---|
| management key, instance `router` | `key.get`, `workspaces.list`, `keys.list` | `key.create`, unguarded, under approval; the created key goes into custody and is never returned |
| created keys, instance `inference` | `key.get` | none |

`keys.list` lists keys by hash, label, limits and usage, never their secrets. Every list returns
one page per call.

## Prerequisites at the provider

Create a management key in the OpenRouter settings. It is sent as `Authorization: Bearer <key>`
under the profile `openrouter.api-key`; the connection's identity is the key record
`GET /api/v1/key` answers, kind `openrouter.key`, subject its `label`.

## Enable the connector and add an instance

Write each instance's native configuration file, print its bootstrap and hash the provider, as the
[full guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-openrouter.md#configure-both-aliases)
shows, then pin the management instance. It permits the write, so it selects private protocol
two:

```toml
[connectors.openrouter]
enabled = true

[connectors.openrouter.instances.router]
alias = "router"
instance_id = "openrouter"
adapter_id = "catalog"
configuration_revision = "REPLACE_FROM_BOOTSTRAP"
protocol = "v1alpha1"
private_protocol = "connectors-private/2"
startup = "on-demand"
restart = "never"

[connectors.openrouter.instances.router.permissions]
profiles = ["openrouter.api-key"]
operations = ["key.get", "workspaces.list", "keys.list"]

[connectors.openrouter.instances.router.executable]
path = "/absolute/path/connectors-catalog-provider"
sha256 = "REPLACE_WITH_EXECUTABLE_SHA256"
args = ["--local-config", "/absolute/path/catalog.json"]
```

The connector name `openrouter` of a pinned instance is a name only; it is not checked against a
declaration.

## Connect

Enter the management key at the hidden prompt on your terminal; it never goes in an argument or
the environment:

```sh
connectors connections connect --connector openrouter --instance router \
  --profile openrouter.api-key --credential-prompt
```

## A first read

Take `--schema` and `--revision` from `operations describe --connector openrouter --instance
router --operation workspaces.list`, then:

```sh
connectors operations invoke --connector openrouter --instance router --connection "$connection" \
  --operation workspaces.list --schema "$schema" --revision "$revision" --input-json '{}'
```

## A first guarded write

`key.create` creates an OpenRouter API key with the management key. OpenRouter answers the new key
once; Connectors puts it straight into custody as a new connection of a second instance and
returns only that connection's reference and the key's `hash`. The management instance needs
`key.create` in its operations, a custody table naming the instance and profile that hold created
keys, a second pinned instance `inference` granted only `key.get`, a signing key, an
`[approval_clock]` table and a policy naming the write; the
[full guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-openrouter.md#creating-a-key)
shows each table. Then prepare, issue and invoke:

```sh
connectors approvals prepare --connector openrouter --instance router \
  --connection "$connection" --operation key.create --schema "$schema" --revision "$revision" \
  --input-json '{"body":{"name":"agent-inference","limit":5,"limit_reset":"monthly"}}'
connectors approvals issue --connector openrouter --instance router \
  --connection "$connection" --operation key.create --schema "$schema" --revision "$revision" \
  --input-json '{"body":{"name":"agent-inference","limit":5,"limit_reset":"monthly"}}' \
  --approve-subject "$subject_sha256" --proof-output "$HOME/private/proof.json"
connectors operations invoke --connector openrouter --instance router \
  --connection "$connection" --operation key.create --schema "$schema" --revision "$revision" \
  --input-json '{"body":{"name":"agent-inference","limit":5,"limit_reset":"monthly"}}' \
  --idempotency-key agent-inference-1 --approval-file "$HOME/private/proof.json"
```

The body is closed to `name`, `limit`, `limit_reset`, `include_byok_in_limit`, `expires_at` and
`workspace_id`; any other key is refused as `invalid_input` before any request. A second approved
create makes a second key.

## Reconnect when the connection is Stale

A management key OpenRouter refuses makes the connection `stale`, and an invoke answers
`next_action: repair_connection`. Publish a new key of the same account on the same connection:

```sh
connectors connections list --connector openrouter --instance router
connectors connections reconnect --connector openrouter --instance router \
  --expected-revision "<revision from the list>" --credential-prompt
```

## Limits

- Verified against a local HTTPS fixture only (`adapters/catalog/tests/openrouter.rs`), with
  synthetic records; nothing has run against a live OpenRouter account.
- Two pinned instances until an OpenRouter connector declaration ships.
- The provider does not walk pages and does not bound `limit`.
- Created keys are read only through `key.get`: the management reads answer a created key `403`.

# OpenRouter management through the catalog provider

The catalog provider reads an OpenRouter account's management key record, its
workspaces and its API keys from the pinned OpenRouter OpenAPI document, with the
management key held in the Connectors keyring. Nothing here is OpenRouter-specific
code: the pinned document is compiled into a bundle, a reviewed selection set
exposes three reads, and the engine described in
[the catalog provider guide](local-catalog-provider.md) binds and sends them.
Configuration, connection and invocation work as described there; this page covers
what differs for OpenRouter.

## Source and bundle

The pinned source is
[`adapters/openrouter/upstream/openrouter-openapi.json`](../adapters/openrouter/upstream/README.md),
SHA-256 `fe4f1ad414f5bd25e69b387598d23f3abd24fe74801cd157c633e30bbf766142`
(2,690,724 bytes): the document retrieved on 2026-10-16 from
<https://openrouter.ai/openapi.json> (upstream SHA-256
`8287890b719d375d64b5651ca4fa2bc34be3af84b24e8fceb8ab13ae9894fe12`, licence MIT),
with the publisher's example keys redacted by `connectors-build redact` (rule
`upstream-redaction/3`, described in the source README). It is OpenAPI `3.1.0`,
`info.version` `1.0.0`, and declares no `servers`, so the configuration names
`api_base` and every operation path carries `/api/v1`. The bundle is built with:

```sh
cargo run --locked -p connectors-build -- catalog \
  --provider openrouter \
  --source adapters/openrouter/upstream/openrouter-openapi.json \
  --directory adapters/catalog/generated/bundles \
  --auth-profile openrouter.api-key
```

It carries 150 operations, 2 reported unsupported: one write that declares its own
`servers`, and an array query parameter sent as one value as given; no shipped read
is among them. `adapters/catalog/tests/bundle_drift.rs` refuses a committed bundle
or index a fresh run would not reproduce byte for byte, and the repository gate
re-checks the redacted source against its record (`connectors-build source-hashes`).

## The shipped selection set

[`adapters/catalog/providers/openrouter/operations.json`](../adapters/catalog/providers/openrouter/operations.json)
exposes three reads and nothing else. `adapters/catalog/tests/openrouter.rs` pins
this exact id list with each id's `operationId`, method and path in the pinned
document.

| id | `operationId` | request | effect |
|---|---|---|---|
| `key.get` | `getCurrentKey` | `GET /api/v1/key` | read |
| `workspaces.list` | `listWorkspaces` | `GET /api/v1/workspaces`, paged by `offset` and `limit` | read |
| `keys.list` | `list` | `GET /api/v1/keys`, filtered by `workspace_id`, `include_disabled`, `include_expired`, paged by `offset` | read |

`key.get` is the management key's own record and also the connection's identity
read. `keys.list` lists keys by hash, label, limits and usage, never their secrets.
Every list returns one page per call; the provider does not walk pages.

## Authentication and configuration

An OpenRouter management key, sent as `Authorization: Bearer <key>`, under the
profile `openrouter.api-key`. The identity is the key record the connect read
returns: kind `openrouter.key`, subject `/data/label`. The key never appears in the
configuration.

```json
{
  "format": "connectors-catalog-local/2",
  "instance": "openrouter",
  "provider": "openrouter",
  "bundle_directory": "/absolute/path/adapters/catalog/generated/bundles",
  "api_base": "https://openrouter.ai/api/v1",
  "auth": {
    "profile": "openrouter.api-key",
    "header": "Authorization",
    "bearer": true,
    "label": "OpenRouter management key",
    "identity": {"path": "key", "kind": "openrouter.key", "subject_pointer": "/data/label"}
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/openrouter/operations.json"
}
```

## Bind, connect and invoke

Bind the provider to the local CLI as [the catalog provider guide](local-catalog-provider.md#bind-the-provider-to-the-local-cli)
describes, with `private_protocol = "connectors-private/2"`,
`profiles = ["openrouter.api-key"]` and
`operations = ["key.get", "workspaces.list", "keys.list"]`, here under the adapter
alias `router`. Enter the management key through the protected credential prompt
on your controlling terminal; it never goes in chat, an argument or the
environment:

```sh
connectors --output json connections connect --adapter router --profile openrouter.api-key --credential-prompt
```

The prompt reads the key from the terminal. The
commands below were run in this tree against a local HTTPS fixture that plays
OpenRouter with a fictional key (the ignored test `serve_the_fixture` in
`adapters/catalog/tests/openrouter.rs`), so the key was supplied from an owner-only
file with `--credential-file` instead of the prompt, and every body is the
fixture's. Connect read the fixture's `GET /api/v1/key` and named its `label` as
the subject:

```sh
connectors --output json connections connect --adapter router --profile openrouter.api-key --credential-file /owner-only/openrouter.json
```

```json
{"ok":true,"result":{"connection":{"external_identity":{"kind":"openrouter.key","subject":"fixture-management-key"},"observed_at_ms":1792138503681,"source":"authority","stale":false,"summary":{"adapter":"router","connection":"conn_33da5ec3f2a4497b96d2b94fe09c4067_5be7ecc5b52c4e619b94774b2938760b","instance_id":"fixture-openrouter","profile":"openrouter.api-key","revision":"00520724-5059-4573-8045-82f550bf2945","state":"ready"},"valid_until_ms":1792138562854}}}
```

Each read takes `--schema` and `--revision` from `operations describe`
(`connectors --output json operations describe --adapter router --operation key.get`).
Connect evidence lasts 60 seconds; revalidate before reading after that.

The management key's own record:

```sh
connectors --output json operations invoke --adapter router --connection "$connection" \
  --operation key.get --schema "$schema" --revision "$revision" --input-json '{}'
```

```json
{"ok":true,"result":{"adapter":"router","operation":"key.get","result":{"body":{"data":{"is_free_tier":false,"is_management_key":true,"label":"fixture-management-key","limit":100,"limit_remaining":74.5,"usage":25.5}},"provenance":{"instance":"fixture-openrouter","observed_at_unix_ms":1792138519516,"resource":"/api/v1/key","source_revision":"fe4f1ad414f5bd25e69b387598d23f3abd24fe74801cd157c633e30bbf766142"},"status":200},"revision":"d1d435a539acd78d46e4d4dd6adc33a50f34f9a6ca3d0c4cbf03a89d384b4caf"}}
```

The second workspace, one page at offset 1:

```sh
connectors --output json operations invoke --adapter router --connection "$connection" \
  --operation workspaces.list --schema "$schema" --revision "$revision" \
  --input-json '{"offset":1,"limit":1}'
```

```json
{"ok":true,"result":{"adapter":"router","operation":"workspaces.list","result":{"body":{"data":[{"created_at":"2026-09-15T15:45:00Z","id":"ws-two","name":"beta","slug":"beta"}],"total_count":2},"provenance":{"instance":"fixture-openrouter","observed_at_unix_ms":1792138520578,"resource":"/api/v1/workspaces","source_revision":"fe4f1ad414f5bd25e69b387598d23f3abd24fe74801cd157c633e30bbf766142"},"status":200},"revision":"d1d435a539acd78d46e4d4dd6adc33a50f34f9a6ca3d0c4cbf03a89d384b4caf"}}
```

The keys of one workspace, from offset 1:

```sh
connectors --output json operations invoke --adapter router --connection "$connection" \
  --operation keys.list --schema "$schema" --revision "$revision" \
  --input-json '{"workspace_id":"ws-two","offset":1}'
```

```json
{"ok":true,"result":{"adapter":"router","operation":"keys.list","result":{"body":{"data":[{"created_at":"2026-09-15T15:45:00Z","disabled":false,"hash":"hash-two","label":"second","name":"second","workspace_id":"ws-two"}]},"provenance":{"instance":"fixture-openrouter","observed_at_unix_ms":1792138521604,"resource":"/api/v1/keys","source_revision":"fe4f1ad414f5bd25e69b387598d23f3abd24fe74801cd157c633e30bbf766142"},"status":200},"revision":"d1d435a539acd78d46e4d4dd6adc33a50f34f9a6ca3d0c4cbf03a89d384b4caf"}}
```

## Limits

- Verified against the local HTTPS fixture only, with synthetic records. Nothing
  has run against a live OpenRouter account, so what a management key may read,
  and OpenRouter's own error answers, are as the pinned document states them.
- The provider does not walk pages and does not bound `limit`.

## Creating a key

Added with `key.create`.

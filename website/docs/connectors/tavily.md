---
title: Tavily
sidebar_position: 17
description: Search the web, fetch pages and crawl a site through Tavily with an API key held in custody.
lede: One Tavily instance per API key searches, fetches and crawls; every call is a read Tavily serves as a POST, and every call spends credits.
source: docs/local-tavily.md, adapters/tavily/contracts/websearch/v1alpha1/semantics.md, contracts/datasources/websearch/v1alpha1/semantics.md
connectors: "tavily"
---

# Tavily

The `tavily` connector is served by its own executable, `connectors-tavily`. The words
connector, instance, connection and operation are those of the
[README vocabulary](https://github.com/beyond10x/connectors#vocabulary); the
[full Tavily guide](https://github.com/beyond10x/connectors/blob/main/docs/local-tavily.md) has
every bound and refusal.

No declaration ships for it, so the instance is pinned: it names its executable, that
executable's SHA-256 and the configuration revision the executable prints. Commands that need a
Tavily account are shown without output.

## What it reads and writes

| Reads | Writes |
|---|---|
| `websearch.search`, `websearch.fetch`, `websearch.crawl` | none |

## Prerequisites at the provider

A Tavily API key. Connecting checks it with `GET /usage`, which spends no search credit. Build the
executable:

```sh
cargo build --release --locked -p connectors -p connectors-tavily
```

## Enable the connector and add an instance

Write an owner-only native file, `tavily.json`; `api_base` may be left out:

```json
{
  "format": "connectors-tavily-local/1",
  "instance": "tavily-local",
  "api_base": "https://api.tavily.com/"
}
```

Print its bootstrap, hash the executable, and pin both in the instance:

```sh
target/release/connectors-tavily --local-config /absolute/path/tavily.json --print-local-bootstrap
sha256sum target/release/connectors-tavily
```

```toml
[connectors.tavily]
enabled = true

[connectors.tavily.instances.tavily]
alias = "tavily"
instance_id = "tavily-local"
adapter_id = "tavily"
configuration_revision = "REPLACE_FROM_BOOTSTRAP"
protocol = "v1alpha1"
private_protocol = "connectors-private/1"
startup = "on-demand"
restart = "never"

[connectors.tavily.instances.tavily.permissions]
profiles = ["tavily.api-key"]
operations = ["websearch.search", "websearch.fetch", "websearch.crawl"]

[connectors.tavily.instances.tavily.executable]
path = "/absolute/path/target/release/connectors-tavily"
sha256 = "REPLACE_WITH_EXECUTABLE_SHA256"
args = ["--local-config", "/absolute/path/tavily.json"]
```

## Connect

```sh
connectors connections connect --connector tavily --instance tavily \
  --profile tavily.api-key --credential-prompt
```

For automation, an owner-only `{"api_key":"..."}` file goes to `--credential-file`.

## A first read

```sh
connection=$(connectors --output json connections list --connector tavily --instance tavily \
  | jq -r '.result.connections[0].connection')
schema=$(connectors --output json operations describe --connector tavily --instance tavily \
  --operation websearch.search | jq -r .result.schema)
revision=$(connectors --output json operations describe --connector tavily --instance tavily \
  --operation websearch.search | jq -r .result.revision)
connectors operations invoke --connector tavily --instance tavily --connection "$connection" \
  --operation websearch.search --schema "$schema" --revision "$revision" \
  --input-json '{"query":"vector database release","max_results":5,"time_range":"week","content":"full"}'
```

Nothing is cached: two invocations are two charges.

## A first guarded write

The connector has no write, so nothing here needs an approval.

## Reconnect when the connection is Stale

A revoked key is reported as `unauthorized` when next used; revalidation then finds it invalid and
the connection lists as `stale`. Publish a working key:

```sh
connectors --output json connections list --connector tavily --instance tavily \
  | jq -c '[.result.connections[] | {connection, revision, state}]'
connectors connections reconnect --connector tavily --instance tavily \
  --expected-revision "<revision from the list>" --credential-prompt
```

## Limits

- Tavily exposes no account identifier: two keys are not told apart, and a reconnect cannot
  detect a key of another account.
- A key grants no scopes and exposes no expiry.
- Validation lasts 60 seconds; a scheduled caller revalidates before reading.

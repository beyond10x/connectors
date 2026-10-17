---
title: Confluence
sidebar_position: 8
description: Read Confluence Cloud pages, their bodies and comments through REST v2, or spaces, pages and search through REST v1.
lede: Confluence is two read-only connectors, confluence (REST v2) and confluence-v1 (REST v1), each with one instance per site and the Atlassian credential Jira uses.
source: docs/catalog-confluence.md, adapters/catalog/providers/confluence/connector.json, adapters/catalog/providers/confluence-v1/connector.json, adapters/catalog/tests/confluence.rs, adapters/catalog/tests/confluence_v1.rs
connectors: "confluence, confluence-v1"
---

# Confluence

The provider Atlassian's Confluence Cloud is two connectors served by the catalog provider:
`confluence` reads through REST v2, and `confluence-v1` reads through REST v1 for a token whose
scopes v2 does not accept. The words connector, instance, connection and operation are those of
the [README vocabulary](https://github.com/beyond10x/connectors#vocabulary); the
[full Confluence guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-confluence.md)
has every parameter and refusal.

Commands use `connectors` on your `PATH` with your own configuration. Outputs were produced in
the repository's tree against a scratch configuration, whose `--config` and `--state-dir` are left
out here. A command that needs an Atlassian account is shown without output.

## What it reads and writes

| Configured as | Reads |
|---|---|
| `confluence` | `pages.changed`, `space.pages`, `page.get`, `page.comments` |
| `confluence-v1` | `spaces.list`, `space.get`, `space.pages`, `pages.changed`, `page.get`, `page.comments`, `content.search` |

Neither connector writes.

## Prerequisites at the provider

Create an Atlassian API token with the Confluence page scopes the reads need. Jira and Confluence
share one credential and the profile `atlassian.basic`: the account's email and the token, sent
as HTTP basic.

A token that carries only classic scopes is answered `401 Unauthorized; scope does not match` on
every v2 read: use `confluence-v1` for it. Connecting `confluence` reads one v2 page, so such a
token is refused at connect as `insufficient_scope` instead of failing every read later.

## Enable the connector and add an instance

Neither connector has a default endpoint: an instance names its site's `/wiki` root.

```toml
[connectors.confluence]
enabled = true

[connectors.confluence.instances.work]
endpoint = "https://your-domain.atlassian.net/wiki"
```

```sh
connectors instances list
```

```json
{"adapters":[{"adapter":"confluence.work","adapter_id":"catalog","connector":"confluence","instance":"work","instance_id":"confluence.work","restart":"never","startup":"on-demand"}],"source":"configuration"}
```

For REST v1, enable `confluence-v1` the same way, with
`[connectors.confluence-v1.instances.work]` and the same endpoint.

## Connect

Put `{"account": "<email>", "token": "<API token>"}` in an owner-only file, then:

```sh
connectors connections connect --connector confluence --instance work \
  --profile atlassian.basic --credential-file ~/private/atlassian.json
```

The same file connects the [Jira](./jira.md) instance and a `confluence-v1` instance as the same
Atlassian account.

## A first read

```sh
connection=$(connectors --output json connections list --connector confluence --instance work \
  | jq -r '.result.connections[0].connection')
schema=$(connectors --output json operations describe --connector confluence --instance work \
  --operation page.get | jq -r .result.schema)
revision=$(connectors --output json operations describe --connector confluence --instance work \
  --operation page.get | jq -r .result.revision)
connectors operations invoke --connector confluence --instance work --connection "$connection" \
  --operation page.get --schema "$schema" --revision "$revision" \
  --input-json '{"id":"65538","body-format":"storage"}'
```

On `confluence-v1`, `spaces.list` with `{"limit":25}` lists the spaces; its `space.pages` takes a
space key where v2 takes the numeric id.

## A first guarded write

Neither connector has a write, so nothing here needs an approval. Changing a page is not offered.

## Reconnect when the connection is Stale

A token Atlassian refuses makes the connection `stale`. On `confluence`, the v2 refusal of a
token without v2 scopes is one such answer; it was observed against a local fixture standing in
for the site:

```json
{"error":{"code":"failure","data":{"code":"service_failure","kind":"operational","next_action":"repair_connection","service_code":"unauthorized","service_reason":"Unauthorized; scope does not match","stage":"dispatch"}},"ok":false}
```

Reconnect with a token that carries the v2 scopes, or move the reads to `confluence-v1`:

```sh
connectors --output json connections list --connector confluence --instance work \
  | jq -c '[.result.connections[] | {connection, revision, state}]'
connectors connections reconnect --connector confluence --instance work \
  --expected-revision "<revision from the list>" --credential-file ~/private/atlassian.json
```

## Limits

- Verified against local HTTPS fixtures only (`adapters/catalog/tests/confluence.rs`,
  `confluence_v1.rs`); no live Confluence site has been read through either connector.
- The REST v1 document is written by this repository from Atlassian's reference pages, because
  Atlassian's own declares no licence; its paths have not been checked against a site.
- `pages.changed` has no updated-since parameter: stop the walk at the first item older than your
  cutoff.
- Take a cursor from `_links.next` percent-decoded, or it is encoded twice.

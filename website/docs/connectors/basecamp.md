---
title: Basecamp
sidebar_position: 10.5
description: Read Basecamp projects, message boards, messages, comments and to-dos with a personal access token.
lede: One Basecamp instance per account reads its projects, people, message boards, messages, comments, to-do sets, to-do lists and to-dos, thirteen reads and no write.
source: docs/catalog-basecamp.md, adapters/catalog/providers/basecamp/connector.json, adapters/catalog/providers/basecamp/operations.json, adapters/catalog/tests/basecamp.rs
connectors: "basecamp"
---

# Basecamp

A Basecamp personal access token is read-write at Basecamp: it acts as you, in every account and
project you can reach, although this version of the connector offers only reads. Create it, and
revoke it when you no longer use it, at <https://app.basecamp.com/my/access_tokens> (the page
Basecamp's own CLI names; Basecamp's API reference does not name it). Enter it only at the host's
hidden prompt, as [Connect](#connect) shows; no example here holds one.

The `basecamp` connector of the provider Basecamp is served by the catalog provider from Basecamp's
pinned OpenAPI document, lowered from the definition `connectors/basecamp.yaml`. Terms follow the
[README vocabulary](https://github.com/beyond10x/connectors#vocabulary); the
[full Basecamp guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-basecamp.md)
has every parameter and refusal.

[How these commands were run](../guides/first-read.md#examples).

## What it reads and writes

| Reads | Writes |
|---|---|
| `projects.list`, `project.get`, `project.people.list`, `message_board.get`, `messages.list`, `message.get`, `comments.list`, `comment.get`, `todoset.get`, `todolists.list`, `todolist.get`, `todos.list`, `todo.get` | none |

`project.get` answers the project's `dock`, whose `message_board` and `todoset` entries carry the
`boardId` and `todosetId` the other reads take. A list answers the next page's URL as `next`, from
Basecamp's `Link` header; send its `page` to the same read, and a page without `next` is the last.

## Prerequisites at the provider

A Basecamp account and a personal access token of a person in it, created at
<https://app.basecamp.com/my/access_tokens>. The instance names the account by its id: the number
after the host in the address of any page of the account.

## Enable the connector and add an instance

The connector has no default endpoint: the endpoint names the account.

```toml
[connectors.basecamp]
enabled = true

[connectors.basecamp.instances.work]
endpoint = "https://3.basecampapi.com/999999999"
```

An endpoint without the account (`https://3.basecampapi.com`) is refused. No read takes the
account id from a caller.

## Connect

Enter the token at the hidden prompt:

```sh
connectors connections connect basecamp/work --profile basecamp.pat --credential-prompt
```

`--endpoint https://3.basecampapi.com/<account id>` on the same command writes the instance's
endpoint first. A token kept in a secret manager can come from an environment variable instead,
with `--credential-env BASECAMP_TOKEN`; the value never appears on the command line or in any
output. Connecting reads the token's own person in the account (`my/profile.json`).

## A first read

```sh
connectors invoke basecamp/work projects.list --input-json '{}'
connectors invoke basecamp/work projects.list --input-json '{"page":2}'
```

The reads are offered as MCP tools by `connectors server --transport stdio`, configured as the
[MCP page](./mcp.md) describes. Its operation curation pins the instance's executable selection
and the digest of its description, which no `connectors` command prints yet.

## A first guarded write

The connector has no write in this version, so nothing here needs an approval.

## Reconnect when the connection is Stale

A token Basecamp refuses (revoked at <https://app.basecamp.com/my/access_tokens>) makes the
connection `stale`. Create a new token and publish it on the same connection:
[reconnect](../guides/reconnect-a-connection.md) `basecamp/work` with `--credential-prompt`.

## Limits

- Verified against a local HTTPS stand-in for Basecamp only (`adapters/catalog/tests/basecamp.rs`);
  no live Basecamp account has been read.
- Reads only; creating a to-do, a message or a comment is a later version.
- Only personal access tokens; OAuth through Basecamp's Launchpad is not offered.
- A `429` is retried once when its `Retry-After` fits the deadline, and otherwise refused as
  `rate_limited` with that delay.

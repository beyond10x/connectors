---
title: GitHub
sidebar_position: 13.8
description: Read GitHub review requests, assigned issues, notifications, repositories, releases and files with a personal access token, from a released connector definition file.
lede: One GitHub instance per account reads what the definition file declares, one page per call, and never writes; it is the instance a released definition file makes, so the configuration names nothing.
source: crates/connectors-catalog/tests/fixtures/definitions-0.5.2/connectors/github.yaml, adapters/catalog/tests/github_definition_reads.rs, apps/connectors/src/local/definition.rs
connectors: "github"
---

# GitHub

The `github` connector of the provider GitHub is served by the catalog provider from
`connectors/github.yaml` of the
[connectors-definitions 0.5.2 release](https://github.com/beyond10x/connectors-definitions/releases/tag/0.5.2),
run as a definition file: `connectors invoke --definition` lowers the file and runs its operations
through the connector's instance `definition`
([Invoke a definition file](../guides/invoke-a-definition-file.md)). Terms follow the [README vocabulary](https://github.com/beyond10x/connectors#vocabulary). Later releases of the
definitions add operations and a `github.app` profile this page does not cover.

[How these commands were run](../guides/first-read.md#examples).

## What it reads and writes

| Reads | Writes |
|---|---|
| `review_requests.list`, `issues.assigned`, `notifications.list`, `user.me`, `repos.list`, `repo.get`, `releases.list`, `release.latest`, `readme.raw`, `file.raw` | none |

`review_requests.list` searches the open pull requests that request your review
(`is:pr is:open review-requested:@me`, with `since` folded in as `updated:>=<since>`);
`issues.assigned` lists the open issues and pull requests assigned to you. `readme.raw` and
`file.raw` answer the file's decoded text, at most 1 MiB; a directory or a binary file is refused.
Every other read answers GitHub's body unchanged.

## Prerequisites at the provider

Create a personal access token under the account the instance reads as. A fine-grained token
needs read access to the repositories it reads: Metadata, Contents (README, files, releases),
Issues and Pull requests. A classic token needs `repo` for private repositories and no scope for
public ones. `notifications.list` needs a classic token with the `notifications` scope (or
`repo`): GitHub's documentation says its notifications endpoints do not accept a fine-grained
token. The token's account is the connection's identity (`GET /user`).

## Enable the connector and add an instance

There is nothing to add to the configuration: the definition file makes the instance. Download
the release and check it:

```sh
REL=https://github.com/beyond10x/connectors-definitions/releases/download/0.5.2
curl -fsSLO "$REL/connectors-definitions-0.5.2.tar.gz"
curl -fsSLO "$REL/connectors-definitions-0.5.2.tar.gz.sha256"
sha256sum -c connectors-definitions-0.5.2.tar.gz.sha256
tar -xzf connectors-definitions-0.5.2.tar.gz
```

```text
connectors-definitions-0.5.2.tar.gz: OK
```

The file is `connectors-definitions-0.5.2/connectors/github.yaml`; its vendored GitHub OpenAPI
document is under `upstream/github/` beside it. Describe one of its operations, here with each
input's upper bound:

```sh
connectors invoke --definition connectors-definitions-0.5.2/connectors/github.yaml \
  review_requests.list --describe true \
  | jq -c '{connector, instance, effect, input: (.input_schema.properties | map_values(.maximum))}'
```

```json
{"connector":"github","instance":"definition","effect":"read","input":{"page":1000,"per_page":100,"since":null}}
```

The connector is `github` and its instance `definition`; the endpoint is the definition's
`base_url`, `https://api.github.com`.

## Connect

Without a terminal, an invoke of an unconnected instance asks nothing and sends nothing:

```sh
setsid -w connectors --output json invoke \
  --definition connectors-definitions-0.5.2/connectors/github.yaml user.me --input-json '{}'
```

```json
{"error":{"code":"failure","data":{"code":"not_found","kind":"operational","next_action":"create_connection","stage":"admission"}},"ok":false}
```

On a terminal the same invoke asks once whether to connect and prompts for the token. To connect
without the question, put `{"token": "<token>"}` in an owner-only file, then:

```sh
connectors connections connect --connector github --instance definition \
  --profile github.token --credential-file ~/private/github.json
```

Every request carries `Authorization: Bearer <token>`.

## A first read

```sh
connectors invoke --definition connectors-definitions-0.5.2/connectors/github.yaml \
  review_requests.list --input-json '{"since":"2026-01-01","per_page":100,"page":1}'
```

Paging is the caller's: each call reads one page of at most `per_page` (100; 50 for
`notifications.list`), and the next call asks for `page` plus one. The walk ends on a page shorter
than `per_page`. A search serves at most its first 1,000 results whatever `total_count` says:
`page` above 1,000 is refused as `invalid_input` before any request, and a page past the first
1,000 results, which GitHub answers "Only the first 1000 search results are available", is refused
as `invalid_input`, so a walk at `per_page` 100 ends at page 10. A search page whose `incomplete_results` is `true` timed out; read it again later.

## A first guarded write

The connector has no write, so nothing here needs an approval. Comments, reviews and merges are
not offered.

## Reconnect when the connection is Stale

A token GitHub refuses makes the connection `stale`. Publish a new token on the same connection. [Reconnect](../guides/reconnect-a-connection.md) `github/definition` with `--credential-file ~/private/github.json`.

## Limits

- Verified against a scripted provider with synthetic answers only
  (`adapters/catalog/tests/github_definition_reads.rs`); nothing on this page has run against a
  live GitHub account.
- One page per call; the provider does not walk pages or hold the cursor. A declared walk is not
  in this release of the definition.
- A search reaches its first 1,000 results; narrow `since` to read past them.
- A spent rate limit (`x-ratelimit-remaining: 0`) is refused as `rate_limited` with
  `retry_after_seconds` until `x-ratelimit-reset`, and is never retried.

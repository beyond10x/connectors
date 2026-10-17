---
title: Zendesk
sidebar_position: 9
description: Read Zendesk Support tickets, comments, users and organizations, with an OAuth client or an API token.
lede: One Zendesk instance per account reads tickets, users and organizations through incremental exports; it declares two auth profiles, so the instance names the one it uses.
source: docs/catalog-zendesk.md, adapters/catalog/providers/zendesk/connector.json, adapters/catalog/providers/zendesk/operations.json, adapters/catalog/tests/zendesk.rs
connectors: "zendesk"
---

# Zendesk

The `zendesk` connector of the provider Zendesk is served by the catalog provider from the pinned
Zendesk Support API document. The words connector, instance, connection, auth profile and
operation are those of the [README vocabulary](https://github.com/beyond10x/connectors#vocabulary);
the [full Zendesk guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-zendesk.md)
has every parameter, paging rule and refusal.

Commands use `connectors` on your `PATH` with your own configuration. Outputs were produced in
the repository's tree against a scratch configuration, whose `--config` and `--state-dir` are left
out here. A command that needs a Zendesk account is shown without output.

## What it reads and writes

| Reads | Writes |
|---|---|
| `tickets.incremental`, `ticket.show`, `ticket.comments`, `users.incremental`, `user.show`, `organizations.incremental`, `organization.show` | none |

The incremental exports take `start_time` in Unix seconds and return what changed since.

## Prerequisites at the provider

Use an OAuth client with the client-credentials grant (auth profile `zendesk.oauth`). In Admin
Center (*Apps and integrations*, *APIs*, *OAuth clients*), create a confidential client as the
admin or service user the reads should be attributed to, and keep its unique identifier and
secret. Zendesk is retiring API tokens: accounts created on or after 2026-07-28 cannot use them,
and every remaining token stops working on 2027-04-30. An existing API token still works under
`zendesk.basic`. The incremental exports are allowed to admins only.

## Enable the connector and add an instance

The connector has no default endpoint, and it declares two auth profiles, so the instance names
its account and its `profile`:

```toml
[connectors.zendesk]
enabled = true

[connectors.zendesk.instances.support]
endpoint = "https://your-subdomain.zendesk.com"
profile = "zendesk.oauth"
```

```sh
connectors instances list
```

```json
{"adapters":[{"adapter":"zendesk.support","adapter_id":"catalog","connector":"zendesk","instance":"support","instance_id":"zendesk.support","restart":"never","startup":"on-demand"}],"source":"configuration"}
```

## Connect

Put `{"client_id": "<unique identifier>", "client_secret": "<secret>"}` in an owner-only file,
then:

```sh
connectors connections connect --connector zendesk --instance support \
  --profile zendesk.oauth --credential-file ~/private/zendesk-oauth.json
```

The provider requests short-lived access tokens with that client as it needs them; the stored
entry never changes. With an API token, the instance states `profile = "zendesk.basic"` and the
document is `{"account": "<email>/token", "token": "<API token>"}`.

## A first read

```sh
connection=$(connectors --output json connections list --connector zendesk --instance support \
  | jq -r '.result.connections[0].connection')
schema=$(connectors --output json operations describe --connector zendesk --instance support \
  --operation tickets.incremental | jq -r .result.schema)
revision=$(connectors --output json operations describe --connector zendesk --instance support \
  --operation tickets.incremental | jq -r .result.revision)
connectors operations invoke --connector zendesk --instance support --connection "$connection" \
  --operation tickets.incremental --schema "$schema" --revision "$revision" \
  --input-json '{"start_time":1790000000}'
```

Send the answer's `after_cursor` as `cursor` until `end_of_stream` is `true`.

## A first guarded write

The connector has no write, so nothing here needs an approval. Changing a ticket is not offered.

## Reconnect when the connection is Stale

A client secret or token Zendesk refuses makes the connection `stale`, and an invoke answers
`next_action: repair_connection`. Publish a new secret on the same connection:

```sh
connectors --output json connections list --connector zendesk --instance support \
  | jq -c '[.result.connections[] | {connection, revision, state}]'
connectors connections reconnect --connector zendesk --instance support \
  --expected-revision "<revision from the list>" --credential-file ~/private/zendesk-oauth.json
```

## Limits

- Verified against a local HTTPS fixture with synthetic records only
  (`adapters/catalog/tests/zendesk.rs`); nothing has run against a live Zendesk account.
- Whether Zendesk accepts the form-encoded token request, and whether per-resource scopes alone
  admit the exports, has not been checked live.
- The provider does not walk pages or bound `per_page`; a read answered `429` is `rate_limited`
  with `retry_after_seconds` when the retry does not fit.

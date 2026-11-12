---
title: HubSpot
sidebar_position: 10
description: Read HubSpot CRM records of any object type, and their owners, with a private-app token.
lede: One HubSpot instance per account lists and reads CRM records of every object type the account has, and the owners those records name, with a private-app token.
source: docs/catalog-hubspot.md, adapters/catalog/providers/hubspot/connector.json, adapters/catalog/providers/hubspot/operations.json, adapters/catalog/tests/hubspot.rs
connectors: "hubspot"
---

# HubSpot

The `hubspot` connector of the provider HubSpot is served by the catalog provider from the pinned
HubSpot CRM Objects `2026-09` document and the two owner reads of the pinned Crm Owners `2026-09`
document. Terms follow the [README vocabulary](https://github.com/beyond10x/connectors#vocabulary); the
[full HubSpot guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-hubspot.md)
has every parameter and refusal.

[How these commands were run](../guides/first-read.md#examples).

## What it reads and writes

| Reads | Writes |
|---|---|
| `objects.list`, `object.get`, `owners.list`, `owner.get` | none |

`objectType` is `contacts`, `companies`, `deals`, `line_items`, `tickets` or any other type the
account has. A record names its owner in the `hubspot_owner_id` property (ask for it in
`properties`); `owner.get` with that value as `ownerId` answers the owner's name, email and teams.
A deactivated owner, common on old deals, is an archived owner: read it with `"archived": true`,
which HubSpot's document describes as returning only archived owners. `owners.list` takes the same
parameter.

## Prerequisites at the provider

Create a private app in the HubSpot account's settings and grant it the read scopes of what you
read. For deals, companies, line items and owners that is `crm.objects.deals.read`,
`crm.objects.companies.read`, `crm.objects.line_items.read` and `crm.objects.owners.read`; no
contacts scope is needed. Add the matching `read` scope of any other object type you read, and
no `write` scope. Keep its access token. The token belongs to the account, not
a user, and connecting does not check its scopes: a missing scope shows on the read that needs
it.

## Enable the connector and add an instance

The connector's default endpoint is `https://api.hubapi.com`, so the instance needs only a name:

```toml
[connectors.hubspot]
enabled = true

[connectors.hubspot.instances.work]
```

## Connect

Put `{"token": "<private app access token>"}` in an owner-only file, then:

```sh
connectors connections connect --connector hubspot --instance work \
  --profile hubspot.private-app --credential-file ~/private/hubspot.json
```

## A first read

```sh
connectors invoke hubspot/work objects.list \
  --input-json '{"objectType":"contacts","limit":50}'
```

Send `paging.next.after` as `after` until a page has no `paging.next`. To read a deal with its
amount and owner, then the owner:

```sh
connectors invoke hubspot/work objects.list \
  --input-json '{"objectType":"deals","limit":10,"properties":"dealname,amount,hubspot_owner_id"}'
connectors invoke hubspot/work owner.get --input-json '{"ownerId":"<hubspot_owner_id>"}'
```

## A first guarded write

The connector has no write, so nothing here needs an approval.

## Reconnect when the connection is Stale

A token HubSpot refuses (the private app's token was rotated or deleted) makes the connection
`stale`. Publish the new token on the
same connection. [Reconnect](../guides/reconnect-a-connection.md) `hubspot/work` with `--credential-file ~/private/hubspot.json`.

## Limits

- Verified against a local HTTPS fixture only (`adapters/catalog/tests/hubspot.rs`); no live
  HubSpot account has been read.
- No time filter: `objects.list` and `owners.list` walk every record. Search and batch read are POST
  operations, which the catalog provider does not serve as reads yet.
- Only private-app tokens; OAuth app installs are not offered.

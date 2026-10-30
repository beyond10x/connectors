---
title: HubSpot
sidebar_position: 10
description: Read HubSpot CRM records of any object type with a private-app token.
lede: One HubSpot instance per account lists and reads CRM records of every object type the account has, with a private-app token.
source: docs/catalog-hubspot.md, adapters/catalog/providers/hubspot/connector.json, adapters/catalog/providers/hubspot/operations.json, adapters/catalog/tests/hubspot.rs
connectors: "hubspot"
---

# HubSpot

The `hubspot` connector of the provider HubSpot is served by the catalog provider from the pinned
HubSpot CRM Objects `2026-09` document. The words connector, instance, connection and operation
are those of the [README vocabulary](https://github.com/beyond10x/connectors#vocabulary); the
[full HubSpot guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-hubspot.md)
has every parameter and refusal.

Commands use `connectors` on your `PATH` with your own configuration. Outputs were produced in
the repository's tree against a scratch configuration, whose `--config` and `--state-dir` are left
out here. A command that needs a HubSpot account is shown without output.

## What it reads and writes

| Reads | Writes |
|---|---|
| `objects.list`, `object.get` | none |

`objectType` is `contacts`, `companies`, `deals`, `tickets` or any other type the account has.

## Prerequisites at the provider

Create a private app in the HubSpot account's settings and grant it the read scopes of the object
types you read (`crm.objects.contacts.read`, `crm.objects.companies.read`,
`crm.objects.deals.read`, and so on). Keep its access token. The token belongs to the account, not
a user, and connecting does not check its scopes: a missing scope shows on the read that needs
it.

## Enable the connector and add an instance

The connector's default endpoint is `https://api.hubapi.com`, so the instance needs only a name:

```toml
[connectors.hubspot]
enabled = true

[connectors.hubspot.instances.work]
```

```sh
connectors instances list | jq -c '.[] | arrays | .[]'
```

```json
{"connector":"hubspot","instance":"work","instance_id":"hubspot.work","restart":"never","startup":"on-demand"}
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

Send `paging.next.after` as `after` until a page has no `paging.next`.

## A first guarded write

The connector has no write, so nothing here needs an approval.

## Reconnect when the connection is Stale

A token HubSpot refuses (the private app's token was rotated or deleted) makes the connection
`stale`, and an invoke answers `next_action: repair_connection`. Publish the new token on the
same connection:

```sh
connectors --output json connections list --connector hubspot --instance work \
  | jq -c '[.result.connections[] | {connection, revision, state}]'
connectors connections reconnect --connector hubspot --instance work \
  --expected-revision "<revision from the list>" --credential-file ~/private/hubspot.json
```

## Limits

- Verified against a local HTTPS fixture only (`adapters/catalog/tests/hubspot.rs`); no live
  HubSpot account has been read.
- No time filter: `objects.list` walks every record of a type. Search and batch read are POST
  operations, which the catalog provider does not serve as reads yet.
- Only private-app tokens; OAuth app installs are not offered.

---
title: Opsgenie
sidebar_position: 12
description: Read Opsgenie alerts, incidents and the alerts linked to an incident with an API integration key.
lede: One Opsgenie instance per account reads alerts and incidents over a created-at window the caller advances, and never writes.
source: docs/catalog-opsgenie.md, adapters/catalog/providers/opsgenie/connector.json, adapters/catalog/providers/opsgenie/operations.json, adapters/catalog/tests/opsgenie.rs
connectors: "opsgenie"
---

# Opsgenie

The `opsgenie` connector of the provider Opsgenie is served by the catalog provider from the
pinned Opsgenie REST API document. The words connector, instance, connection and operation are
those of the [README vocabulary](https://github.com/beyond10x/connectors#vocabulary); the
[full Opsgenie guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-opsgenie.md)
has every parameter and refusal. Opsgenie ends service on 2027-04-05.

Commands use `connectors` on your `PATH` with your own configuration. Outputs were produced in
the repository's tree against a scratch configuration, whose `--config` and `--state-dir` are left
out here. A command that needs an Opsgenie account is shown without output.

## What it reads and writes

| Reads | Writes |
|---|---|
| `alerts.list`, `alert.get`, `incidents.list`, `incident.get`, `incident.alert_ids` | none |

Both lists are held to ascending `createdAt`; the caller advances the window run by run.

## Prerequisites at the provider

Create an API integration in Opsgenie and keep its key. The key needs read access to alerts, and
to incidents for the incident reads. It belongs to the integration, not a user, so the connection's
identity is the configured instance.

## Enable the connector and add an instance

The connector's default endpoint is `https://api.opsgenie.com`; an account on the EU instance
states `endpoint = "https://api.eu.opsgenie.com"`.

```toml
[connectors.opsgenie]
enabled = true

[connectors.opsgenie.instances.work]
```

```sh
connectors instances list | jq -c '.[] | arrays | .[]'
```

```json
{"connector":"opsgenie","instance":"work","instance_id":"opsgenie.work","restart":"never","startup":"on-demand"}
```

## Connect

Put `{"token": "<API key>"}` in an owner-only file, then:

```sh
connectors connections connect --connector opsgenie --instance work \
  --profile opsgenie.api-key --credential-file ~/private/opsgenie.json
```

Every request carries `Authorization: GenieKey <key>`.

## A first read

```sh
connectors invoke opsgenie/work alerts.list \
  --input-json '{"query":"createdAt >= 1790000000000","sort":"createdAt","order":"asc","limit":100}'
```

Follow `offset` until a page has no `paging.next`.

## A first guarded write

The connector has no write, so nothing here needs an approval. Acknowledging, closing and notes
are not offered.

## Reconnect when the connection is Stale

A key Opsgenie refuses makes the connection `stale`, and an invoke answers `next_action:
repair_connection`. Publish a new key on the same connection:

```sh
connectors --output json connections list --connector opsgenie --instance work \
  | jq -c '[.result.connections[] | {connection, revision, state}]'
connectors connections reconnect --connector opsgenie --instance work \
  --expected-revision "<revision from the list>" --credential-file ~/private/opsgenie.json
```

A key of another account is accepted as the same identity, so reconnect only with a key of this
account.

## Limits

- Verified against a local HTTPS fixture with synthetic records only
  (`adapters/catalog/tests/opsgenie.rs`); nothing has run against a live Opsgenie account.
- `offset` stops at 19,899; narrow the window to read past it.
- The provider does not walk pages or hold the cursor; the caller does.

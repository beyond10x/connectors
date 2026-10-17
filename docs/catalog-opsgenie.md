# Opsgenie alerts and incidents through the catalog provider

The catalog provider reads Opsgenie alerts, incidents and the alert ids linked to
an incident from the pinned Opsgenie REST API document, read only, with an API
integration key held in the Connectors keyring. A caller copying incident history
out of Opsgenie lists alerts and incidents created in a window it advances run by
run, re-reads one by id, and asks each incident which alerts belong to it.
Opsgenie ends service on 2027-04-05: after that date nothing here answers, and
nothing changes any more either, so a final pass over created windows is
complete.

Nothing here is Opsgenie-specific code: the Swagger 2.0 document is projected
into OpenAPI 3.1.0, one cited amendment adds the incident-to-alert link the
document leaves out, the result is compiled into a bundle, a reviewed selection
set exposes five reads, and the engine described in
[the catalog provider guide](local-catalog-provider.md) binds and sends them.
Connection and invocation work as described there; this page configures the
`opsgenie` connector and covers what differs for Opsgenie. No read needs an
approval. Connector, instance and connection are defined in
[the README](../README.md#vocabulary).

## Source and bundle

The pinned source is
[`adapters/opsgenie/upstream/swagger.json`](../adapters/opsgenie/upstream/README.md),
SHA-256 `60901e17d0d4ba28b8d13d3d78f6a5d329b93008c3c1710dfe6a0b6c786ef071`
(659,535 bytes): `opsgenie/opsgenie-oas` at commit `71c55343`, last changed
2021-02-19, Swagger `2.0`, `info.version` `2.0.0`, 170 operations, Apache-2.0
(`LICENSE` beside it). It is projected under the rule table of
`crates/connectors-catalog/src/swagger.rs` (`swagger2-openapi/2`), and the
bundle is compiled from the projection with the derivation and the amendment
recorded:

```sh
cargo run --locked -p connectors-build -- swagger \
  --source adapters/opsgenie/upstream/swagger.json \
  --out adapters/opsgenie/generated/opsgenie.openapi.json
cargo run --locked -p connectors-build -- catalog \
  --provider opsgenie \
  --source adapters/opsgenie/generated/opsgenie.openapi.json \
  --derived-from adapters/opsgenie/upstream/swagger.json \
  --directory adapters/catalog/generated/bundles \
  --auth-profile opsgenie.api-key \
  --amendments adapters/opsgenie/upstream/opsgenie.amendments.json \
  --replace
```

`--replace` is needed once the committed index carries `opsgenie`. The first
command writes `opsgenie.openapi.json` and its projection record. 99 of the 170
operations are projected, with the document's shared `#/parameters/` and
`#/responses/` inlined; the record lists the other 71 with their pointers and
reasons, and the bundle counts them unsupported: 64 writes whose request body
has no media type (the document declares `consumes` on one operation only), the
attachment upload, whose `formData` file has no exact projection, and six
policy operations whose `teamId` query parameter is written with a `schema`.
The five reads below are all projected or added. The document's host `api.opsgenie.com` becomes the
projection's one server, `https://api.opsgenie.com`.
[`opsgenie.amendments.json`](../adapters/opsgenie/upstream/opsgenie.amendments.json),
bound to the projection's SHA-256, adds one GET operation the document lacks
(an `add_operation` amendment, [cited amendments](local-catalog-provider.md#build-the-bundle)):
`getAssociatedAlertIds`, `GET /v1/incidents/{identifier}/associated-alert-ids`,
citing <https://docs.opsgenie.com/docs/incident-api#get-associated-alerts>.
`adapters/catalog/tests/bundle_drift.rs` regenerates the projection and its
record from the pinned document and the bundle from the projection and the
amendment, and refuses any of them, or the index, that a fresh run would not
reproduce byte for byte.

## The shipped selection set

[`adapters/catalog/providers/opsgenie/operations.json`](../adapters/catalog/providers/opsgenie/operations.json)
exposes these five reads and nothing else. `adapters/catalog/tests/opsgenie.rs`
pins this exact id list with each id's `operationId`, method and path, so a
renamed, dropped or added id fails the gate.

| id | `operationId` and request | since parameter | paging | end condition |
|---|---|---|---|---|
| `alerts.list` | `listAlerts`, `GET /v2/alerts` | `query` `createdAt >= <epoch ms>`; `sort` held to `createdAt`, `order` to `asc`, both required | `offset` 0..19899, `limit` 1..100 (default 20) | `paging.next` absent |
| `alert.get` | `getAlert`, `GET /v2/alerts/{identifier}` | n/a | single item; `identifierType` `id`, `alias` or `tiny` | n/a |
| `incidents.list` | `ListIncidents`, `GET /v1/incidents/` | `query` `createdAt >= <epoch ms>`, required; `sort` held to `createdAt`, `order` to `asc`, both required | `offset` 0..19899, `limit` 1..100 (default 20) | `paging.next` absent |
| `incident.get` | `getIncident`, `GET /v1/incidents/{identifier}` | n/a | single item; `identifierType` `id` or `tiny` | n/a |
| `incident.alert_ids` | `getAssociatedAlertIds`, `GET /v1/incidents/{identifier}/associated-alert-ids` (added by amendment) | n/a: read per incident `incidents.list` found | `offset` from 1, `limit` 1..25 (default 20), `direction`, `order` `asc` or `desc` | fewer ids than `limit` |

A value outside a held or bounded set, such as `sort` `updatedAt`, `order`
`desc`, `limit` 0 or 101 or `offset` 19900, is refused as `invalid_input` before
any request. `alerts.list` withholds `searchIdentifier` and
`searchIdentifierType`: saved searches are not offered.

- **The path of `incidents.list`.** The document writes `/v1/incidents/` with a
  final slash; the request goes to `/v1/incidents`, the path the incident API
  reference writes. `incidents_list_sends_the_documented_path` pins it.
- **The window.** Only the creation time is a documented filter: Opsgenie's
  alert search lists `createdAt` and `lastOccurredAt` as time fields, not
  `updatedAt`, and its incident search lists `createdAt` only. A caller sends
  `query` `createdAt >= <T>`, with `T` in epoch milliseconds, `sort`
  `createdAt` and `order` `asc`, and walks the window by adding `limit` to
  `offset` until a page carries no `paging.next`. It then stores the last
  `createdAt` it read, in epoch milliseconds, and sends it back as the next
  window's `T`. `createdAt >= T` re-reads the items created at `T`: delivery is
  at least once, and the caller deduplicates by `id`. A query sends a space as
  `+` (`query=createdAt+%3E%3D+1760000000000`), the form encoding of a query
  component.
- **The 20K cap.** Opsgenie's alert API states that the sum of offset and
  limit should be lower than 20K; `offset` is bounded to 19,899 so no request
  passes it. A window holding more alerts is walked by advancing `createdAt`
  instead: start the next window at the last `createdAt` read. The cap is
  documented for alerts only; applying it to incidents is an inference.
- **Status changes.** A status change after a run copied an item is not a
  creation, so no window shows it. Re-read the item with `alert.get` or
  `incident.get`, or re-read a window. After 2027-04-05 nothing changes.
- **The link.** `incident.alert_ids` answers `data` as an array of alert ids
  (Opsgenie's sample answer). The reference shows no paging fields in that
  answer; a page with fewer ids than `limit` ending the walk is an inference.
  `offset` has a documented minimum of 1 and no documented maximum, so it is
  not bounded here; Opsgenie validates it.
- **Plans.** Opsgenie's incident API is available on Standard and Enterprise
  plans only; a `402` or `403` on an incident read answers through the generic
  read error table ([Limits](local-catalog-provider.md#limits)).

## Authentication

The pinned document declares one security scheme, `GenieKey`, an `apiKey` in
the `Authorization` header, and Opsgenie's reference sends it as
`Authorization: GenieKey <key>`. The key belongs to an API integration, not to a
user ("Create an API Integration and obtain your apiKey"), so the profile
names the connection: the identity read `GET v2/alerts`, with no query, proves
the key with a `200` and is not read, and the subject is the configured
instance id (`identity.source: configuration`). A token profile's `prefix`
puts the scheme word in front of the key: the protected entry is
`{"token": "<API key>"}` and every request carries
`Authorization: GenieKey <API key>`.

The `opsgenie` connector declares that profile, `opsgenie.api-key`, and the
default endpoint `https://api.opsgenie.com`
([`adapters/catalog/providers/opsgenie/connector.json`](../adapters/catalog/providers/opsgenie/connector.json)).
Add to the host configuration (`~/.config/connectors/config.toml`, format
`connectors-local/5`):

```toml
[connectors.opsgenie]
enabled = true

[connectors.opsgenie.instances.work]
```

An account on Opsgenie's EU instance states its endpoint in the instance
table: `endpoint = "https://api.eu.opsgenie.com"`. The bundle, the five reads,
the profile and the provider executable are derived from the shipped
connector. The instance id, and so the connection's subject, is
`opsgenie.work`:

```console
$ connectors instances list
{"adapters":[{"adapter":"opsgenie.work","adapter_id":"catalog","connector":"opsgenie","instance":"work","instance_id":"opsgenie.work","restart":"never","startup":"on-demand"}],"source":"configuration"}
```

The key needs read access to alerts, and to incidents for the incident reads.
Connect with the credential document `{"token": "<API key>"}` in an owner-only
file:

```sh
connectors connections connect --connector opsgenie --instance work \
  --profile opsgenie.api-key --credential-file ~/private/opsgenie.json
```

An installation configured before connectors were declared converts its
entries with `connectors setup migrate`, which keeps each instance id and
revision; the pinned form it writes is described in
[the catalog provider guide](local-catalog-provider.md).

## Limits

- Verified against a local HTTPS fixture (`adapters/catalog/tests/opsgenie.rs`)
  with synthetic records only: the exact request of each read, including the
  `createdAt` window and the `GenieKey` header, the returned body bytes, a
  two-page walk of `alerts.list` and the next window's lower bound, the added
  link read and the identity read. Nothing has been run against a live
  Opsgenie account.
- The provider does not walk pages or hold the cursor itself; the caller does.
- No write is offered: acknowledging, closing and notes, on-call and schedules
  stay out of this selection set.

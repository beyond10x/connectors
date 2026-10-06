# HubSpot CRM through the catalog provider

The catalog provider reads HubSpot CRM records (contacts, companies, deals,
tickets and every other object type an account has) from the pinned HubSpot CRM
Objects `2026-09` OpenAPI document. Nothing here is HubSpot-specific code: the
pinned document is compiled into a bundle, a reviewed selection set exposes two
reads, and the engine described in
[the catalog provider guide](local-catalog-provider.md) binds and sends them.
Configuration, connection, approval and invocation work exactly as described
there; this page covers what differs for HubSpot.

## Source and bundle

The pinned source is
[`adapters/hubspot/upstream/crm-objects-2026-09.json`](../adapters/hubspot/upstream/README.md),
SHA-256 `1cb7ca32e7a9ff224838f8c1f02a99104e509d651fc2524dfd16f77fdc9be9f9`,
retrieved on 2026-09-30 from HubSpot's public specification collection,
`HubSpot/HubSpot-public-api-spec-collection` at commit
`55d9bcaaf1c208516f11a33aa43fb6bba7aac823`, path
`PublicApiSpecs/CRM/Objects/Rollouts/424/2026-09/objects.json`. Its server is
`https://api.hubapi.com` with no path, so the bundle records each operation's
path as written. The bundle is built with:

```sh
cargo run --locked -p connectors-build -- catalog \
  --provider hubspot \
  --source adapters/hubspot/upstream/crm-objects-2026-09.json \
  --directory adapters/catalog/generated/bundles \
  --auth-profile hubspot.private-app
```

It carries all 11 operations of the pinned document, none unsupported, and
`adapters/catalog/tests/bundle_drift.rs` refuses a committed bundle or index a
fresh run would not reproduce byte for byte.

## The shipped selection set

[`adapters/catalog/providers/hubspot/operations.json`](../adapters/catalog/providers/hubspot/operations.json)
exposes two reads and nothing else. Each is `effect: read`; there are no
writes. `adapters/catalog/tests/hubspot.rs` pins this exact id list and each
id's `operationId` and path in the pinned document, so a renamed or dropped id,
or a source operation that moved, fails the gate. The bundle refuses at load any
`operation_id` the pinned document lacks.

The pinned document's `operationId`s are long generated strings; the table
names the path, and `operations.json` carries the exact ids.

| id | request | paging parameters | end condition | time filter or deltas |
|---|---|---|---|---|
| `objects.list` | `GET /crm/objects/2026-09/{objectType}` | `after`, `limit` | `paging.next` absent | none; the document offers an updated-since filter only on search, a POST that cannot ship as a read yet |
| `object.get` | `GET /crm/objects/2026-09/{objectType}/{objectId}` | single item | n/a | none |

- **`objectType`** is `contacts`, `companies`, `deals`, `tickets` or any other
  object type the account has, including a custom object's type id.
- **Following a page.** Send the first page without `after`. The pinned
  document says the cursor "will be returned as the `paging.next.after` JSON
  property of a paged response containing more results". For the next page,
  send that value as `after` with the other parameters unchanged. A page with
  no `paging.next` is the last. `limit` defaults to 10; the document states no
  maximum, so the selection set carries no bound and HubSpot answers a value it
  does not accept.
- **`properties`, `associations` and `propertiesWithHistory`** are lists. The
  document describes `properties` as "a comma separated list"; send each list
  as one comma-separated string (`"properties": "email,firstname"`), because
  the engine sends one value per parameter. The engine percent-encodes the
  comma (`properties=email%2Cfirstname`); whether HubSpot decodes it has not
  been checked against a live account.
- **`object.get`** takes `objectType` and `objectId`; `idProperty` names a
  unique property to look the record up by instead of its id.

Every other query parameter the pinned document declares for an operation is
accepted by name; one it does not declare is refused before any request.

## Authentication

A HubSpot private-app access token, sent as `Authorization: Bearer <token>`,
under the profile `hubspot.private-app`. Create the private app in the HubSpot
account's settings and grant it the read scopes of the object types you read
(`crm.objects.contacts.read`, `crm.objects.companies.read`,
`crm.objects.deals.read`, …). The pinned document names this scheme
`private_apps`; HubSpot's private-app guide
(<https://developers.hubspot.com/docs/apps/legacy-apps/private-apps/overview>, read
2026-09-30) sends the token as `Authorization: Bearer [YOUR_TOKEN]`. That guide files
private apps under legacy apps.

The identity read is `GET /account-info/2026-09/details`, which is not part of
the pinned document and not an operation of this provider; it is only the
profile's identity probe. It answers the account's `portalId`: a private-app
token belongs to an account, not a user, so the identity kind is
`hubspot.portal`. The profile declares no `scopes` read and no
`minimum_scopes`: no GET that lists a private-app token's scopes has been
identified, so connecting does not check a token's scopes.

```json
{
  "format": "connectors-catalog-local/2",
  "instance": "hubspot",
  "provider": "hubspot",
  "bundle_directory": "/absolute/path/adapters/catalog/generated/bundles",
  "api_base": "https://api.hubapi.com",
  "auth": {
    "profile": "hubspot.private-app",
    "header": "Authorization",
    "bearer": true,
    "label": "Private app access token",
    "identity": {"path": "account-info/2026-09/details", "kind": "hubspot.portal", "subject_pointer": "/portalId"}
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/hubspot/operations.json"
}
```

Connect with the credential document `{"token": "<private app access token>"}`:

```sh
target/release/connectors --output json connections connect --adapter hubspot --profile hubspot.private-app --credential-file /owner-only/hubspot.json
```

## Limits

- Verified against a local HTTPS fixture only
  (`adapters/catalog/tests/hubspot.rs`): the exact request of each read and the
  bearer header, the returned body as JSON, a two-page walk to a page without
  `paging.next`, and the identity read. No live HubSpot account has been read.
- No time filter: `objects.list` walks every record of a type. Search
  (`POST /crm/objects/2026-09/{objectType}/search`, which filters by
  `hs_lastmodifieddate`) and batch read are POST operations, and the engine
  admits a read only through GET; `story:catalog-post-reads` owns that change.
- OAuth app installs are not offered; only the private-app token.
- The engine parses and re-serialises the body, so it is returned as equal JSON,
  not as HubSpot's exact bytes.
- The provider does not walk pages itself. It sends a read answered `429` once more after
  the delay its `Retry-After` names, when that wait leaves the second request
  time of its own before the invocation deadline, and otherwise returns it as a `rate_limited` refusal carrying that
  delay as `retry_after_seconds`
  ([Limits](local-catalog-provider.md#limits)).

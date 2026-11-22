# HubSpot CRM through the catalog provider

The catalog provider reads HubSpot CRM records (contacts, companies, deals,
line items, tickets and every other object type an account has) from the
pinned HubSpot CRM Objects `2026-09` OpenAPI document, and the account's owners
from the pinned Crm Owners `2026-09` document. Nothing here is HubSpot-specific
code: the pinned documents are compiled into a bundle, a reviewed selection set
exposes four reads, and the engine described in
[the catalog provider guide](local-catalog-provider.md) binds and sends them.
Connection and invocation work as described there; this page configures the
`hubspot` connector and covers what differs for HubSpot. Connector, instance
and connection are defined in [the README](../README.md#vocabulary).

## Source and bundle

The pinned source is
[`adapters/hubspot/upstream/crm-objects-2026-09.json`](../adapters/hubspot/upstream/README.md),
SHA-256 `1cb7ca32e7a9ff224838f8c1f02a99104e509d651fc2524dfd16f77fdc9be9f9`,
retrieved on 2026-09-30 from HubSpot's public specification collection,
`HubSpot/HubSpot-public-api-spec-collection` at commit
`55d9bcaaf1c208516f11a33aa43fb6bba7aac823`, path
`PublicApiSpecs/CRM/Objects/Rollouts/424/2026-09/objects.json`. Its server is
`https://api.hubapi.com` with no path, so the bundle records each operation's
path as written.

HubSpot publishes owners as a separate document. The pinned Owners source is
[`adapters/hubspot/upstream/crm-owners-2026-09.json`](../adapters/hubspot/upstream/README.md),
SHA-256 `f7bd5bb7859a3f822d0b3d9b148321620b25393f319a1a7d800eb04d73eff7e3`,
retrieved on 2026-11-12 from the same collection and commit, path
`PublicApiSpecs/CRM/Crm Owners/Rollouts/146888/2026-09/crmOwners.json`. It
declares two reads and the same server. A bundle has one source, so the two
owner reads enter it through cited amendments,
[`hubspot-owners.amendments.json`](../adapters/hubspot/upstream/hubspot-owners.amendments.json),
bound to the CRM Objects digest; each entry carries the Owners document's own
`operationId`, path and parameters, and `adapters/catalog/tests/hubspot.rs`
holds each equal to that document. The bundle is built with:

```sh
cargo run --locked -p connectors-build -- catalog \
  --provider hubspot \
  --source adapters/hubspot/upstream/crm-objects-2026-09.json \
  --directory adapters/catalog/generated/bundles \
  --auth-profile hubspot.private-app \
  --amendments adapters/hubspot/upstream/hubspot-owners.amendments.json
```

It carries the 11 operations of the CRM Objects document and the two owner
reads, none unsupported, and `adapters/catalog/tests/bundle_drift.rs` refuses a
committed bundle or index a fresh run would not reproduce byte for byte.

## The shipped selection set

[`adapters/catalog/providers/hubspot/operations.json`](../adapters/catalog/providers/hubspot/operations.json)
exposes four reads and nothing else. Each is `effect: read`; there are no
writes. `adapters/catalog/tests/hubspot.rs` pins this exact id list and each
id's `operationId` and path in the bundle, so a renamed or dropped id, or a
source operation that moved, fails the gate. The bundle refuses at load any
`operation_id` it lacks.

The pinned documents' `operationId`s are long generated strings; the table
names the path, and `operations.json` carries the exact ids.

| id | request | paging parameters | end condition | time filter or deltas |
|---|---|---|---|---|
| `objects.list` | `GET /crm/objects/2026-09/{objectType}` | `after`, `limit` | `paging.next` absent | none; the document offers an updated-since filter only on search, a POST that cannot ship as a read yet |
| `object.get` | `GET /crm/objects/2026-09/{objectType}/{objectId}` | single item | n/a | none |
| `owners.list` | `GET /crm/owners/2026-09` | `after`, `limit` | `paging.next` absent | none |
| `owner.get` | `GET /crm/owners/2026-09/{ownerId}` | single item | n/a | none |

- **`objectType`** is `contacts`, `companies`, `deals`, `line_items`,
  `tickets` or any other object type the account has, including a custom
  object's type id. `adapters/catalog/tests/hubspot.rs` reads deals, companies
  and line items through these two reads against a fixture.
- **Owners.** A deal, a company or any other record names its owner by id in
  the `hubspot_owner_id` property, which `objects.list` and `object.get` return
  only when asked for it in `properties`. Send that value as `ownerId` of
  `owner.get`: the answer carries `email`, `firstName`, `lastName`, `teams`,
  `type` (`PERSON` or `QUEUE`) and `userId`. `idProperty` is `id` (the default)
  or `userId`. `owners.list` takes `email` to find one, `archived` and `limit`
  (the document's default is 100), and pages like `objects.list`.
- **Deactivated owners.** A user who was deactivated is an archived owner, and
  old deals and companies often still carry that user's id. The pinned Owners
  document says `archived` is "whether to return only results that have been
  archived" (default `false`), so read such an owner with
  `{"ownerId": "<id>", "archived": true}`, and list them with `archived: true`
  on `owners.list`; the document does not say the plain read returns them, and
  no live account has confirmed either way.
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
account's settings and grant it the read scopes of what you read. For deals,
companies, line items and owners that is exactly four:
`crm.objects.deals.read`, `crm.objects.companies.read`,
`crm.objects.line_items.read` and `crm.objects.owners.read`. No contacts scope
is needed for them; add `crm.objects.contacts.read` only to read contacts, and
the matching `…read` scope of any other object type. A property HubSpot marks
sensitive needs the `…sensitive.read.v2` scope of its type as well. Grant no
`…write` scope: the connector has no write, and a read-only token is the
guardrail. The pinned CRM Objects document names this scheme
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

The `hubspot` connector declares that one profile and the default endpoint
`https://api.hubapi.com`
([`adapters/catalog/providers/hubspot/connector.json`](../adapters/catalog/providers/hubspot/connector.json)),
so an instance needs only a name. Add to the host configuration
(`~/.config/connectors/config.toml`, format `connectors-local/5`):

```toml
[connectors.hubspot]
enabled = true

[connectors.hubspot.instances.work]
```

The bundle, the four reads, the profile and the provider executable are derived
from the shipped connector:

```console
$ connectors instances list
{"adapters":[{"connector":"hubspot","instance":"work","instance_id":"hubspot.work","restart":"never","startup":"on-demand"}],"source":"configuration"}
```

Connect with the credential document `{"token": "<private app access token>"}`
in an owner-only file:

```sh
connectors connections connect --connector hubspot --instance work \
  --profile hubspot.private-app --credential-file ~/private/hubspot.json
```

The token may come from an environment variable instead, which `op run` sets for one
command (`hubspot.env` holds `HUBSPOT_TOKEN=op://<vault>/<item>/<field>`); the variable holds the
bare token, and its value is read by the process and never printed:

```sh
op run --env-file hubspot.env -- connectors connections connect hubspot/work \
  --profile hubspot.private-app --credential-env HUBSPOT_TOKEN
```

An unset, empty or oversized variable is refused naming the variable only
(`protected_entry_unavailable`, `environment_variable`). A name is upper case letters,
digits and underscores; any other word given in its place is refused without being repeated
(`environment_variable_rule` says what a name is). A bare value may end in one line break,
which is dropped. `--credential-env` stands beside no other credential input; for a profile of several entry fields the variable holds the
JSON object the other inputs take.

An installation configured before connectors were declared converts its
entries with `connectors setup migrate`, which keeps each instance id and
revision; the pinned form it writes is described in
[the catalog provider guide](local-catalog-provider.md).

## Limits

- Verified against a local HTTPS fixture only
  (`adapters/catalog/tests/hubspot.rs`): the exact request of each read and the
  bearer header, the returned body as JSON, a two-page walk of each list to a
  page without `paging.next`, deals, companies and line items read with the
  owner each carries, and the identity read. No live HubSpot account has been
  read; the scopes above are those the pinned documents name for these reads.
- Owners are read through the CRM Objects bundle's amendments, not from a
  bundle of their own: the Owners document's response schema is not part of the
  bundle, so the answer is HubSpot's JSON as sent.
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

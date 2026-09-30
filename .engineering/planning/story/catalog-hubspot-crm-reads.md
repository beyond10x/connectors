---
format: aep.planning-md/3
id: story:catalog-hubspot-crm-reads
kind: story
status: implemented
title: HubSpot CRM objects list and get, private-app token
relations:
- decomposes: epic:hubspot-crm-reads
- serves: vision:independent-contract-adapters
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T13:49:19Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-09-30T13:49:20Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-09-30T14:34:25Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Source

HubSpot CRM Objects API `2026-09`, OpenAPI 3.0.1, pinned by digest under `adapters/hubspot/upstream/`
from `HubSpot/HubSpot-public-api-spec-collection` commit `55d9bcaaf1c208516f11a33aa43fb6bba7aac823`,
path `PublicApiSpecs/CRM/Objects/Rollouts/424/2026-09/objects.json`. Provider id `hubspot`. Auth: a
private-app access token as `Authorization: Bearer`, profile `hubspot.private-app`.

## Operations (read-only)

| id | endpoint | paging | end condition | time filter |
|---|---|---|---|---|
| `objects.list` | `GET /crm/objects/2026-09/{objectType}` | `after`, `limit` | no `paging.next` | none; deltas need search (`story:catalog-post-reads`) |
| `object.get` | `GET /crm/objects/2026-09/{objectType}/{objectId}` | single item | n/a | n/a |

`objectType` is `contacts`, `companies`, `deals`, `tickets` or any other type the account has.

## Identity

`GET /account-info/2026-09/details`, subject `/portalId`, kind `hubspot.portal`. The token names
an account, not a user. No GET that lists a private-app token's scopes has been identified, so the profile
declares no scope read.

## Shared surfaces

- Own files: `adapters/hubspot/upstream/*`, `adapters/catalog/providers/hubspot/operations.json`,
  `adapters/catalog/generated/bundles/hubspot.bundle.json`, `adapters/catalog/tests/hubspot.rs`,
  `docs/catalog-hubspot.md`.
- Shared: `adapters/catalog/generated/bundles/index.json`, `adapters/catalog/tests/bundle_drift.rs`,
  `docs/local-catalog-provider.md`, `adapters/README.md`, `README.md`, `CHANGELOG.md`.

## Acceptance

- `operations.json` exposes exactly `objects.list` and `object.get`, each `effect: read`, and
  `adapters/catalog/tests/hubspot.rs` pins that id list and each id's `operationId` and path.
- Per operation, a local HTTPS fixture test asserts the exact request (path, query, `Authorization:
  Bearer <token>`) and that `operations invoke` returns the fixture body as equal JSON.
- A two-page walk of `objects.list` stops on a page without `paging.next`.
- `connections connect` with profile `hubspot.private-app` records subject `portalId` from the
  account-details fixture.
- The pinned source's digest is checked by the repository's source-hash gate, and
  `bundle_drift.rs` reproduces the bundle and index byte for byte.
- `docs/catalog-hubspot.md` states paging, end condition, the missing time filter and the limits.

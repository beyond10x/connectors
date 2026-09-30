---
format: aep.planning-md/3
id: epic:hubspot-crm-reads
kind: epic
status: draft
title: HubSpot CRM records through the catalog provider
summary: Read HubSpot contacts, companies, deals and other CRM objects for one account; writes later.
relations:
- serves: vision:independent-contract-adapters
- informed_by: architecture-decision-record:declarative-http-provider-runtime
revision: 1
---
## Outcome

The local `connectors` CLI reads HubSpot CRM records (contacts, companies, deals and the other
standard objects) for one HubSpot account through the catalog provider, the way it reads GitLab,
Jira and Confluence today (`docs/local-catalog-provider.md`): a pinned OpenAPI source, a reviewed
read-only selection set, `operations describe` / `operations invoke`. Requested by the operator on
2026-09-30. Writes are out of scope for this epic.

## Findings (2026-09-30)

| Fact | Source |
|---|---|
| HubSpot publishes OpenAPI 3.0.1 documents per API and per version | `github.com/HubSpot/HubSpot-public-api-spec-collection` at `55d9bcaaf1c208516f11a33aa43fb6bba7aac823`, `PublicApiSpecs/` (694 JSON files) |
| The CRM Objects `2026-09` document serves every object type through `/crm/objects/2026-09/{objectType}`; 11 operations, server `https://api.hubapi.com` | `PublicApiSpecs/CRM/Objects/Rollouts/424/2026-09/objects.json` |
| Its reads by GET are list (`after`, `limit`, `properties`, `associations`, `archived`) and get one object; search and batch read are POST | same document |
| The catalog engine admits `effect: read` only for GET | `adapters/catalog/src/lib.rs:239-247` |
| The document declares `oauth2` and `private_apps` (apiKey header `private-app`) security; a private-app access token is sent as `Authorization: Bearer` per HubSpot's developer documentation (not read from the document) | `components.securitySchemes` |
| The collection repository declares no licence (`license: null`); the document carries no `termsOfService` | GitHub API, `info` |
| Account details `GET /account-info/2026-09/details` returns `portalId` (integer, required) | `PublicApiSpecs/Account/Account Info/Rollouts/144923/2026-09/accountInfo.json` |

## Decomposition

1. `story:catalog-hubspot-crm-reads`: pin the Objects `2026-09` document, ship `objects.list` and
   `object.get`, a bearer auth profile `hubspot.private-app` with the account-details identity read.
2. `story:catalog-post-reads`: let a selection declare a POST operation as a read, so HubSpot
   `search` (the only operation with an updated-since filter) and `batch/read` can ship without an
   approval. Changes the engine; drafted, not scheduled.

## Out of scope

OAuth app installation and refresh (`epic:google-workspace-reads` phase 2 owns the shared OAuth
work); writes; associations v4, properties and owners APIs; webhooks.

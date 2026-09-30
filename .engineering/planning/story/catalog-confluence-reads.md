---
format: aep.planning-md/3
id: story:catalog-confluence-reads
kind: story
status: active
title: Confluence Cloud pages by space, updated since, with body
relations:
- decomposes: epic:catalog-knowledge-sources
- depends_on: story:catalog-basic-auth-profile
- depends_on: story:catalog-jira-cloud-reads
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: adapters/atlassian/upstream/README.md
- confidence: cited
  path: adapters/atlassian/upstream/confluence-v1-search.json
- confidence: cited
  path: adapters/catalog/generated/bundles/confluence.bundle.json
- confidence: cited
  path: adapters/catalog/generated/bundles/index.json
- confidence: cited
  path: adapters/catalog/generated/bundles/jira.bundle.json
- confidence: cited
  path: adapters/catalog/providers/confluence/operations.json
- confidence: cited
  path: adapters/catalog/tests/bundle_drift.rs
- confidence: cited
  path: adapters/catalog/tests/confluence.rs
- confidence: cited
  path: adapters/catalog/tests/jira.rs
- confidence: cited
  path: docs/catalog-confluence.md
- confidence: cited
  path: docs/catalog-jira.md
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T16:39:50Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":5}}}
- {from: "proposed", to: "active", at: "2026-09-29T16:39:51Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"review_outcome":5}}}
---
## Source

Confluence Cloud REST OpenAPI documents (v2 for pages, v1 for CQL search), pinned by digest under `adapters/atlassian/upstream/`. Provider id `confluence`. Auth: basic (`story:catalog-basic-auth-profile`).

## Operations (read-only)

| id | endpoint | paging | end condition | time filter |
|---|---|---|---|---|
| `pages.changed` | `GET /wiki/rest/api/search` with CQL `space = "<key>" and type = page and lastmodified >= "<t>"` | `start`, `limit` | no `_links.next` | CQL `lastmodified >= "<t>"` |
| `space.pages` | `GET /wiki/api/v2/spaces/{id}/pages` | `cursor`, `limit` | no `_links.next` | none (v2 has no updated-since filter); deltas come from `pages.changed` |
| `page.get` | `GET /wiki/api/v2/pages/{id}?body-format=storage` | single item | n/a | n/a |
| `page.comments` | `GET /wiki/api/v2/pages/{id}/footer-comments` | `cursor`, `limit` | no `_links.next` | none; deltas come from `pages.changed` |

The fixture test for `page.get` asserts the request carries `body-format=storage` (the consumer may
also ask for `view`). Page comments were added on 2026-09-29 at the knowledge-ingest consumer's request.

## Shared surfaces

- Own files: `adapters/catalog/providers/confluence/operations.json`, `adapters/catalog/generated/bundles/confluence.bundle.json`, `docs/catalog-confluence.md`, `adapters/catalog/tests/confluence.rs`, and the pinned source `adapters/atlassian/upstream/confluence-v2.json` and `adapters/atlassian/upstream/confluence-v1-search.json`.
- Shared and serialized through `depends_on` (see the epic): `adapters/catalog/generated/bundles/index.json`, to which this story adds its row on top of the previous provider's.

## Acceptance

- `operations.json` exposes exactly the ids above, each `effect: read`; `adapters/catalog/tests/confluence.rs` pins that exact id list, so a renamed or dropped id fails the gate.
- `operations.json` loads against the committed bundle, which refuses any `operation_id` the pinned document lacks; `docs/catalog-confluence.md` cites, per operation, the pinned document's `operationId` and path, so a difference from the table above shows as a changed row.
- `docs/catalog-confluence.md` states, per list operation, its paging parameters, its end condition and its time-window filter (or how deltas are taken where there is none).
- Per operation, a recorded-fixture test asserts the exact request the provider sends (path, query including the time filter, auth header) and that `operations invoke` returns the fixture body byte-identical.
- For every list operation in the table, a paging test walks two fixture pages and asserts the walk stops at that operation's documented end condition.
- `adapters/catalog/tests/bundle_drift.rs` reproduces this provider's bundle byte for byte; no live credential is needed for the gate.

## Decided for the wave (coordinator, 2026-09-30, from the operator's answer)

- One pinned Confluence document, like Jira's one: the provider is generated from a single source. Take
  the document that carries all four reads; the v1 REST document (https://developer.atlassian.com/cloud/confluence/swagger.v3.json)
  is the only one with CQL search (`pages.changed`), so check whether it also serves `space.pages`,
  `page.get` (body in storage format) and `page.comments`, and change the endpoints in the Operations table
  to that document's where they differ. If no single document carries all four, stop and report.
- The multi-source bundle pipeline built in the held attempt (wave0929b-confluence) is not taken.
- Jira and Confluence share one auth profile, `atlassian.basic` (scheme basic, `{"account","token"}`); the
  Jira bundle moves from `jira.basic` to it. That breaks a 0.16/0.17 configuration that names `jira.basic`;
  the CHANGELOG says so.
- Fixture bodies compare as equal JSON; end conditions read from the body; a null link counts as absent.

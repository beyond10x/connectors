---
format: aep.planning-md/3
id: story:catalog-zendesk-reads
kind: story
status: draft
title: Zendesk tickets and comments through the catalog provider
relations:
- decomposes: epic:catalog-knowledge-sources
- depends_on: story:catalog-basic-auth-profile
- depends_on: story:catalog-jira-cloud-reads
scope:
- confidence: cited
  path: adapters/catalog/generated/bundles/index.json
- confidence: cited
  path: adapters/catalog/generated/bundles/zendesk.bundle.json
- confidence: cited
  path: adapters/catalog/providers/zendesk/operations.json
- confidence: cited
  path: adapters/catalog/tests/zendesk.rs
- confidence: cited
  path: adapters/zendesk/upstream/zendesk-support.json
- confidence: cited
  path: docs/catalog-zendesk.md
revision: 5
---
## Source

Zendesk Support API OpenAPI document, pinned by digest under `adapters/zendesk/upstream/`. Provider id `zendesk`. Auth: basic with `<email>/token:<api token>` (`story:catalog-basic-auth-profile`).

## Operations (read-only)

| id | endpoint | paging | end condition | time filter |
|---|---|---|---|---|
| `tickets.incremental` | `GET /api/v2/incremental/tickets/cursor` | `start_time` then `cursor` | `end_of_stream: true` | `start_time` (updated since) |
| `ticket.comments` | `GET /api/v2/tickets/{ticket_id}/comments` | `page[size]`, `page[after]` | `meta.has_more: false` | none; deltas come from `tickets.incremental` |

## Shared surfaces

- Own files: `adapters/catalog/providers/zendesk/operations.json`, `adapters/catalog/generated/bundles/zendesk.bundle.json`, `docs/catalog-zendesk.md`, `adapters/catalog/tests/zendesk.rs`, and the pinned source `adapters/zendesk/upstream/zendesk-support.json`.
- Shared and serialized through `depends_on` (see the epic): `adapters/catalog/generated/bundles/index.json`, to which this story adds its row on top of the previous provider's.

## Acceptance

- `operations.json` exposes exactly the ids above, each `effect: read`; `adapters/catalog/tests/zendesk.rs` pins that exact id list, so a renamed or dropped id fails the gate.
- `operations.json` loads against the committed bundle, which refuses any `operation_id` the pinned document lacks; `docs/catalog-zendesk.md` cites, per operation, the pinned document's `operationId` and path, so a difference from the table above shows as a changed row.
- `docs/catalog-zendesk.md` states, per list operation, its paging parameters, its end condition and its time-window filter (or how deltas are taken where there is none).
- Per operation, a recorded-fixture test asserts the exact request the provider sends (path, query including the time filter, auth header) and that `operations invoke` returns the fixture body byte-identical.
- For every list operation in the table, a paging test walks two fixture pages and asserts the walk stops at that operation's documented end condition.
- `adapters/catalog/tests/bundle_drift.rs` reproduces this provider's bundle byte for byte; no live credential is needed for the gate.

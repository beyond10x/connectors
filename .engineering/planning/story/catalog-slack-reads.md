---
format: aep.planning-md/3
id: story:catalog-slack-reads
kind: story
status: draft
title: Slack channel history and thread replies through the catalog provider
relations:
- decomposes: epic:catalog-knowledge-sources
- depends_on: story:catalog-confluence-reads
scope:
- confidence: cited
  path: adapters/catalog/generated/bundles/index.json
- confidence: cited
  path: adapters/catalog/generated/bundles/slack.bundle.json
- confidence: cited
  path: adapters/catalog/providers/slack/operations.json
- confidence: cited
  path: adapters/catalog/tests/slack.rs
- confidence: cited
  path: adapters/slack/upstream/slack-web-api.json
- confidence: cited
  path: docs/catalog-slack.md
revision: 6
---
## Source

Slack Web API OpenAPI document from `slackapi/slack-api-specs` (archived, Swagger 2.0), pinned by commit and digest under `adapters/slack/upstream/`. Provider id `slack`. Auth: bot token as `Authorization: Bearer` (already supported). If the catalog import cannot read Swagger 2.0 or the document lacks either operation, the unit files a `decision-blocker` naming the gap instead of hand-writing a source.

## Operations (read-only)

| id | endpoint | paging | end condition | time filter |
|---|---|---|---|---|
| `conversations.list` | `GET /conversations.list` | `cursor`, `limit` | empty `response_metadata.next_cursor` | none; the consumer lists channels, then pages history per channel |
| `conversations.history` | `GET /conversations.history` | `cursor`, `limit` | empty `response_metadata.next_cursor` | `oldest`, `latest` |
| `conversations.replies` | `GET /conversations.replies` | `cursor`, `limit` | empty `response_metadata.next_cursor` | `oldest`, `latest` |

`conversations.list` was added on 2026-09-29 at the knowledge-ingest consumer's request.

## Shared surfaces

- Own files: `adapters/catalog/providers/slack/operations.json`, `adapters/catalog/generated/bundles/slack.bundle.json`, `docs/catalog-slack.md`, `adapters/catalog/tests/slack.rs`, and the pinned source `adapters/slack/upstream/slack-web-api.json`.
- Shared and serialized through `depends_on` (see the epic): `adapters/catalog/generated/bundles/index.json`, to which this story adds its row on top of the previous provider's.

## Acceptance

- `operations.json` exposes exactly the ids above, each `effect: read`; `adapters/catalog/tests/slack.rs` pins that exact id list, so a renamed or dropped id fails the gate.
- `operations.json` loads against the committed bundle, which refuses any `operation_id` the pinned document lacks; `docs/catalog-slack.md` cites, per operation, the pinned document's `operationId` and path, so a difference from the table above shows as a changed row.
- `docs/catalog-slack.md` states, per list operation, its paging parameters, its end condition and its time-window filter (or how deltas are taken where there is none).
- Per operation, a recorded-fixture test asserts the exact request the provider sends (path, query including the time filter, auth header) and that `operations invoke` returns the fixture body byte-identical.
- For every list operation in the table, a paging test walks two fixture pages and asserts the walk stops at that operation's documented end condition.
- `adapters/catalog/tests/bundle_drift.rs` reproduces this provider's bundle byte for byte; no live credential is needed for the gate.

---
format: aep.planning-md/3
id: story:catalog-jira-cloud-reads
kind: story
status: draft
title: Jira Cloud issues, comments and changelog through the catalog provider
relations:
- decomposes: epic:catalog-knowledge-sources
- depends_on: story:catalog-basic-auth-profile
scope:
- confidence: cited
  path: adapters/atlassian/upstream/jira-platform-v3.json
- confidence: cited
  path: adapters/catalog/generated/bundles/index.json
- confidence: cited
  path: adapters/catalog/generated/bundles/jira.bundle.json
- confidence: cited
  path: adapters/catalog/providers/jira/operations.json
- confidence: cited
  path: adapters/catalog/tests/bundle_drift.rs
- confidence: cited
  path: adapters/catalog/tests/jira.rs
- confidence: cited
  path: docs/catalog-jira.md
revision: 5
---
## Source

Jira Cloud platform REST v3 OpenAPI document, pinned by digest under `adapters/atlassian/upstream/`. Provider id `jira`. Auth: basic (`story:catalog-basic-auth-profile`).

## Operations (read-only)

| id | endpoint | paging | end condition | time filter |
|---|---|---|---|---|
| `issues.search` | `GET /rest/api/3/search/jql` (replaces the deprecated `/search`) | `nextPageToken`, `maxResults` | no `nextPageToken` in the response | JQL `updated >= "<t>"` |
| `issue.comments` | `GET /rest/api/3/issue/{issueIdOrKey}/comment` | `startAt`, `maxResults` | `startAt + len(comments) >= total` | none; deltas come from `issues.search` |
| `issue.changelog` | `GET /rest/api/3/issue/{issueIdOrKey}/changelog` | `startAt`, `maxResults` | `isLast: true` | none; deltas come from `issues.search` |

## Shared surfaces

- Own files: `adapters/catalog/providers/jira/operations.json`, `adapters/catalog/generated/bundles/jira.bundle.json`, `docs/catalog-jira.md`, `adapters/catalog/tests/jira.rs`, and the pinned source `adapters/atlassian/upstream/jira-platform-v3.json`.
- Shared and serialized through `depends_on` (see the epic): `adapters/catalog/generated/bundles/index.json`, to which this story adds its row on top of the previous provider's; this story also generalizes `adapters/catalog/tests/bundle_drift.rs` to every indexed provider.

## Acceptance

- `operations.json` exposes exactly the ids above, each `effect: read`; `adapters/catalog/tests/jira.rs` pins that exact id list, so a renamed or dropped id fails the gate.
- `operations.json` loads against the committed bundle, which refuses any `operation_id` the pinned document lacks; `docs/catalog-jira.md` cites, per operation, the pinned document's `operationId` and path, so a difference from the table above shows as a changed row.
- `docs/catalog-jira.md` states, per list operation, its paging parameters, its end condition and its time-window filter (or how deltas are taken where there is none).
- Per operation, a recorded-fixture test asserts the exact request the provider sends (path, query including the time filter, auth header) and that `operations invoke` returns the fixture body byte-identical.
- For every list operation in the table, a paging test walks two fixture pages and asserts the walk stops at that operation's documented end condition.
- `adapters/catalog/tests/bundle_drift.rs` reproduces this provider's bundle byte for byte; no live credential is needed for the gate.

---
format: aep.planning-md/3
id: story:catalog-google-gmail-reads
kind: story
status: draft
title: Gmail message, thread, history and label reads
relations:
- decomposes: epic:google-workspace-reads
- depends_on: story:catalog-repeated-query-parameters
- depends_on: story:catalog-google-calendar-reads
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: adapters/catalog/generated/bundles/google-gmail.bundle.json
- confidence: inferred
  path: adapters/catalog/generated/bundles/index.json
- confidence: inferred
  path: adapters/catalog/providers/google-gmail/operations.json
- confidence: inferred
  path: adapters/catalog/tests/bundle_drift.rs
- confidence: inferred
  path: adapters/catalog/tests/google_gmail.rs
- confidence: inferred
  path: adapters/google/generated/gmail.openapi.json
- confidence: inferred
  path: adapters/google/generated/gmail.projection.json
- confidence: inferred
  path: docs/catalog-google-gmail.md
revision: 5
---
## Outcome

The catalog provider reads Gmail through a bundle compiled from the pinned Gmail v1 Discovery
document.

## Operations (read-only)

Selection id rule: see `story:catalog-google-drive-reads`.

| Selection id | Discovery id | Notes |
|---|---|---|
| `users.getProfile` | `gmail.users.getProfile` | `historyId` baseline for deltas |
| `users.messages.list` | `gmail.users.messages.list` | `q`, `labelIds` (repeated), `maxResults` bounded 1–500 (the Discovery schema declares no bounds; 500 is from its description text), `pageToken` |
| `users.messages.get` | `gmail.users.messages.get` | `format` `full`/`metadata`/`minimal`/`raw` |
| `users.threads.list` | `gmail.users.threads.list` | as messages.list |
| `users.threads.get` | `gmail.users.threads.get` | |
| `users.history.list` | `gmail.users.history.list` | `startHistoryId`; 404 means a full resync |
| `users.labels.list` | `gmail.users.labels.list` | |

## Acceptance

- `adapters/catalog/providers/google-gmail/operations.json`, `adapters/catalog/tests/google_gmail.rs`
  pinning the ids; `bundle_drift.rs` reproduces the projected document and bundle.
- Recorded-fixture tests per operation; `messages_list_repeats_label_ids`; paging tests for
  `users.messages.list`, `users.threads.list` and `users.history.list`.
- `messages_list_max_results_bounds` and `threads_list_max_results_bounds`: `maxResults` 0 and 501 are refused `invalid_input` with no
  request sent; 1 and 500 are sent.
- `docs/catalog-google-gmail.md`: `userId` is `me`, deltas by `historyId`.

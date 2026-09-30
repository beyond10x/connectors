---
format: aep.planning-md/3
id: story:catalog-google-calendar-reads
kind: story
status: draft
title: Google Calendar list and event reads
relations:
- decomposes: epic:google-workspace-reads
- depends_on: story:catalog-repeated-query-parameters
- depends_on: story:catalog-google-slides-reads
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: adapters/catalog/generated/bundles/google-calendar.bundle.json
- confidence: inferred
  path: adapters/catalog/generated/bundles/index.json
- confidence: inferred
  path: adapters/catalog/providers/google-calendar/operations.json
- confidence: inferred
  path: adapters/catalog/tests/bundle_drift.rs
- confidence: inferred
  path: adapters/catalog/tests/google_calendar.rs
- confidence: inferred
  path: adapters/google/generated/calendar.openapi.json
- confidence: inferred
  path: adapters/google/generated/calendar.projection.json
- confidence: inferred
  path: docs/catalog-google-calendar.md
revision: 5
---
## Outcome

The catalog provider reads Google Calendar through a bundle compiled from the pinned Calendar v3
Discovery document.

## Operations (read-only)

Selection id rule: see `story:catalog-google-drive-reads`.

| Selection id | Discovery id | Notes |
|---|---|---|
| `calendarList.list` | `calendar.calendarList.list` | `pageToken`; `syncToken` for deltas |
| `events.list` | `calendar.events.list` | `timeMin`, `timeMax`, `updatedMin`, `singleEvents`, `eventTypes` (repeated), `pageToken`, `maxResults` bounded 1–2500 (the Discovery schema declares only `minimum: 1`; 2500 is from its description text), `syncToken`; ends when `nextPageToken` is absent, `nextSyncToken` is the delta cursor |
| `events.get` | `calendar.events.get` | |

## Acceptance

- `adapters/catalog/providers/google-calendar/operations.json`, `adapters/catalog/tests/google_calendar.rs`
  pinning the ids; `bundle_drift.rs` reproduces the projected document and bundle.
- Recorded-fixture tests per operation; `events_list_repeats_event_types` asserts two `eventTypes`
  pairs; paging tests walk two pages and assert `nextSyncToken` on the last.
- `events_list_max_results_bounds`: `maxResults` 0 and 2501 are refused `invalid_input` with no request
  sent; 1 and 2500 are sent.
- `docs/catalog-google-calendar.md`: time windows, sync tokens and the 410 `fullSyncRequired` reset
  rule.

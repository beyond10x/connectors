---
format: aep.planning-md/3
id: story:catalog-google-drive-reads
kind: story
status: implemented
title: 'Google Drive reads: about, files, export and changes'
relations:
- decomposes: epic:google-workspace-reads
- depends_on: story:catalog-discovery-projection
- depends_on: story:catalog-oauth2-refresh-profile
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: adapters/catalog/generated/bundles/google-drive.bundle.json
- confidence: inferred
  path: adapters/catalog/generated/bundles/index.json
- confidence: inferred
  path: adapters/catalog/providers/google-drive/operations.json
- confidence: inferred
  path: adapters/catalog/tests/bundle_drift.rs
- confidence: inferred
  path: adapters/catalog/tests/google_drive.rs
- confidence: inferred
  path: adapters/google/generated/drive.openapi.json
- confidence: cited
  path: adapters/google/generated/drive.openapi.projection.json
- confidence: inferred
  path: docs/catalog-google-drive.md
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T14:43:08Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":6}}}
- {from: "proposed", to: "active", at: "2026-09-30T14:43:09Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"review_outcome":6}}}
- {from: "active", to: "implemented", at: "2026-09-30T18:33:33Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"test_result":1,"review_outcome":12,"verification":1}}}
---
## Outcome

The catalog provider reads Google Drive through a bundle compiled from the Drive v3 Discovery
document pinned by `story:catalog-discovery-projection`, with an `oauth2_refresh` configuration
(`story:catalog-oauth2-refresh-profile`).

## Source

- Consumes `adapters/google/upstream/drive/drive-api.json`; does not re-pin it.
- Projected with `connectors-build discovery` into `adapters/google/generated/drive.openapi.json`
  and `drive.projection.json`; compiled with `connectors-build catalog --provider google-drive
  --derived-from …`.

## Selection id rule (all Google stories)

The upstream `operation_id` is Google's Discovery method id, verbatim (`drive.files.list`). The
selection `id` is that id minus its API-name prefix (`files.list`), as the existing providers use
short ids scoped by provider (`adapters/catalog/providers/confluence/operations.json`: `page.get`).
The provider is `google-<api>`.

## Operations (read-only)

| Selection id | Discovery id | Notes |
|---|---|---|
| `about.get` | `drive.about.get` | `fields` required by Drive for this method |
| `files.list` | `drive.files.list` | `q`, `pageSize` bounded 1–1000 (Discovery `minimum: "1"`, `maximum: "1000"`), `pageToken`; ends when `nextPageToken` is absent |
| `files.get` | `drive.files.get` | metadata only; `alt=media` is not projected |
| `files.export` | `drive.files.export` | `mimeType` required; selection `response: text`; the 4 MiB engine cap applies |
| `changes.getStartPageToken` | `drive.changes.getStartPageToken` | delta baseline |
| `changes.list` | `drive.changes.list` | `pageToken` required, `pageSize` bounded 1–1000; ends with `newStartPageToken` |

## Acceptance

- `adapters/catalog/providers/google-drive/operations.json` exposes exactly the ids above, each
  `effect: read`; `adapters/catalog/tests/google_drive.rs` pins the id list.
- `bundle_drift.rs` regenerates `drive.openapi.json` from the pinned Discovery and the bundle from
  it, byte for byte, and lists `google-drive` in `SOURCES`.
- Per operation, a recorded-fixture test asserts the exact request (path, query, bearer from the
  fixture token host) and returns the fixture body unchanged; `files.export` returns `text/plain`
  bytes as text.
- Paging tests for `files.list` and `changes.list` walk two pages to their end conditions.
- `files_list_page_size_bounds` and `changes_list_page_size_bounds`: `pageSize` 0 and 1001 are refused
  `invalid_input` with no request sent; 1 and 1000 are sent.
- `docs/catalog-google-drive.md` documents the operations, paging, deltas and configuration.

Live evidence is `story:catalog-google-live-deck-read`'s.

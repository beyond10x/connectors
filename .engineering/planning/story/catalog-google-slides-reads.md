---
format: aep.planning-md/3
id: story:catalog-google-slides-reads
kind: story
status: implemented
title: Google Slides presentation and page reads
relations:
- decomposes: epic:google-workspace-reads
- depends_on: story:catalog-google-drive-reads
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: adapters/catalog/generated/bundles/google-slides.bundle.json
- confidence: inferred
  path: adapters/catalog/generated/bundles/index.json
- confidence: inferred
  path: adapters/catalog/providers/google-slides/operations.json
- confidence: inferred
  path: adapters/catalog/tests/bundle_drift.rs
- confidence: inferred
  path: adapters/catalog/tests/google_slides.rs
- confidence: inferred
  path: adapters/google/generated/slides.openapi.json
- confidence: cited
  path: adapters/google/generated/slides.openapi.projection.json
- confidence: inferred
  path: docs/catalog-google-slides.md
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T18:33:41Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":2,"verification":1}}}
- {from: "proposed", to: "active", at: "2026-09-30T18:33:41Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":2,"verification":1}}}
- {from: "active", to: "implemented", at: "2026-09-30T18:33:42Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":2,"verification":1}}}
---
## Outcome

The catalog provider reads Google Slides structure through a bundle compiled from the Slides v1
Discovery document pinned by `story:catalog-discovery-projection`.

## Operations (read-only)

Selection id rule: see `story:catalog-google-drive-reads`.

| Selection id | Discovery id | Notes |
|---|---|---|
| `presentations.get` | `slides.presentations.get` | Discovery path `v1/presentations/{+presentationId}` projected to `{presentationId}` |
| `presentations.pages.get` | `slides.presentations.pages.get` | |
| `presentations.pages.getThumbnail` | `slides.presentations.pages.getThumbnail` | returns JSON with a `contentUrl`; the image itself is not fetched |

## Acceptance

- `adapters/catalog/providers/google-slides/operations.json` exposes exactly these ids,
  `effect: read`; `adapters/catalog/tests/google_slides.rs` pins them.
- `bundle_drift.rs` reproduces `slides.openapi.json` and the bundle, and lists `google-slides`.
- Recorded-fixture tests per operation; `presentations_get_refuses_slash_in_id`: a
  `presentationId` containing `/` is escaped into one segment, never split.
- `docs/catalog-google-slides.md` documents the operations, the 4 MiB cap and `fields` as the way
  to narrow a response.

Live evidence is `story:catalog-google-live-deck-read`'s.

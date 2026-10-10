---
format: aep.planning-md/3
id: story:catalog-path-correction-and-value-bound
kind: story
status: implemented
title: Path corrections and string value bounds, for GitLab code search
relations:
- serves: vision:independent-contract-adapters
- decomposes: epic:fluxplane-plugin-parity
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-10T04:32:49Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-10T04:32:49Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-10T09:10:39Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Outcome

The catalog provider can select GitLab project code search: a cited source correction can fix an
operation's path template, and a selection can bound a string query parameter to an allowed set
of values.

## Why

`story:parity-gitlab-repository-reads` left `getApiV4ProjectsIdDashSearch` unselected, so
`gitlab.search.blobs` (75 calls, the most used GitLab gap) stays missing:

- The pinned path is `/api/v4/projects/{id}/(-/)search`, GitLab's notation for an optional
  segment. The engine sends it literally (`projects/7/(-/)search`), and the source amendment format
  (`crates/connectors-catalog/src/amendment.rs`) can only add an optional query parameter, not
  correct a path. Group search and project semantic search carry the same notation.
- `scope` is required and must be held to `blobs`, so the search cannot reach issues, users or
  wiki text; a selection bound (`adapters/catalog/src/lib.rs`, `Bound`) is an integer range only.

## Acceptance

- Spec first: the path correction and the string value bound are modelled in the amendment and
  selection formats (and their ESS model where one exists), validated with the newest `ess`,
  before the engine changes.
- `search.blobs` (or the chosen id) sends `GET /api/v4/projects/{id}/search?scope=blobs&search=…`
  against a fixture; any other `scope` is refused before a request.
- The pinned document is unchanged; the correction is cited.
- `gitlab.search.blobs` moves to covered on the parity page.

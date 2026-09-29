---
format: aep.planning-md/3
id: story:catalog-gitlab-repository-reads
kind: story
status: active
title: GitLab projects, tags, releases and project events through the catalog provider
relations:
- decomposes: epic:catalog-knowledge-sources
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: adapters/catalog/providers/gitlab/operations.json
- confidence: inferred
  path: adapters/catalog/tests/local_runtime.rs
- confidence: cited
  path: adapters/catalog/tests/shipped.rs
- confidence: cited
  path: docs/local-catalog-provider.md
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T15:46:15Z", actor: "human:timo", revision: 7}
- {from: "proposed", to: "active", at: "2026-09-29T15:46:15Z", actor: "human:timo", revision: 8}
---
## Source

The pinned GitLab source and bundle that already ship (`adapters/gitlab/upstream/openapi_v3.yaml`,
`adapters/catalog/generated/bundles/gitlab.bundle.json`, all 1,847 operations per
`docs/local-catalog-provider.md:29`). Only the selection set grows; the bundle and `index.json` do not
change. Requested by the knowledge-ingest consumer on 2026-09-29: repositories as entities with
lifecycle events, tags and releases. Token scope `read_api`.

## Operations (read-only)

| id | endpoint | paging | end condition | time filter |
|---|---|---|---|---|
| `projects.list` | `GET /api/v4/projects` with `membership`, `simple=false`, `archived`, `order_by=last_activity_at` | `page`, `per_page` | empty `X-Next-Page` / page shorter than `per_page` | `last_activity_after` |
| `tags.list` | `GET /api/v4/projects/{id}/repository/tags` | `page`, `per_page` | as above | none; the consumer diffs tag names and commit ids |
| `releases.list` | `GET /api/v4/projects/{id}/releases` | `page`, `per_page` | as above | none; `released_at` is in each item |
| `project.events` | `GET /api/v4/projects/{id}/events` | `page`, `per_page` | as above | `after`, `before` (dates) |

`projects.list` returns each item unchanged, including `archived`, `created_at`, `last_activity_at`,
`path_with_namespace`, `description`, `topics`, `default_branch`, `visibility`, `web_url`.
Project audit events (`/projects/{id}/audit_events`) are not selected: as far as I know they need a
paid GitLab tier and Maintainer access, beyond `read_api`; that is unverified and recorded here as such.

## Shared surfaces

- Own files: `adapters/catalog/providers/gitlab/operations.json` (additions only),
  `adapters/catalog/tests/shipped.rs` (the id list), `docs/local-catalog-provider.md` (the operation table).
- No bundle or `index.json` change, so no ordering against the other provider stories.

## Acceptance

- `operations.json` exposes the four ids above, each `effect: read`; `adapters/catalog/tests/shipped.rs`
  pins the complete GitLab id list, so a renamed or dropped id fails the gate.
- `operations.json` loads against the committed bundle, which refuses any `operation_id` the pinned
  document lacks.
- Per operation, a recorded-fixture test asserts the exact request sent (path and query including the
  time filter) and that `operations invoke` returns the fixture body byte-identical.
- For every list operation, a paging test walks two fixture pages and stops at the documented end condition.
- `docs/local-catalog-provider.md` lists the four operations with their paging and time filter.

## Selection ids are fixed

The consumer already runs a local selection file with these ids (2026-09-29, `projects.list` observed
status 200, 100 projects on page 1). The shipped file uses the same ids and operation ids, so the
consumer switches without renaming:

| id | `operation_id` (present in `gitlab.bundle.json`) |
|---|---|
| `projects.list` | `getApiV4Projects` |
| `tags.list` | `getApiV4ProjectsIdRepositoryTags` |
| `releases.list` | `getApiV4ProjectsIdReleases` |
| `project.events` | `getApiV4ProjectsIdEvents` |

## Decided for the wave (coordinator, 2026-09-29)

- The catalog read result carries only `status`, `body` and `provenance` (`adapters/catalog/src/lib.rs:350-352`),
  so a caller cannot see `X-Next-Page`. The paging tests stop on "page shorter than `per_page`";
  `lib.rs` does not change.
- The engine parses and re-serialises the body (`lib.rs:581`), so "byte-identical" in the acceptance
  is tested as JSON equality with the fixture.
- Fixture tests extend the TLS fixture server in `adapters/catalog/tests/local_runtime.rs:106-195`;
  `shipped.rs:26` moves from 14 to 18 declared ids.

---
format: aep.planning-md/3
id: story:catalog-github-reads
kind: story
status: draft
title: GitHub review requests, assigned issues and pull requests, and notifications through the catalog provider
revision: 1
---
## Outcome

The assistant repository reads, through the Connectors CLI like the gitlab, jira and slack catalog providers, what waits on the authenticated GitHub user: review requests, assigned issues and pull requests, and notifications. Read only: no write, no mark-as-read.

## Source

GitHub's published REST OpenAPI description (github/rest-api-description, MIT licensed), the api.github.com description pinned by SHA-256 under `adapters/github/upstream/` with its licence, redacted by `connectors-build redact` if its examples carry credentials or personal data. Provider id `github`. Auth: a token (fine-grained or classic) held in custody; identity `GET /user`, subject `/id`.

## Operations (each `effect: read`)

| id | `operationId` | endpoint | paging | time filter |
|---|---|---|---|---|
| `review_requests.list` | `search/issues-and-pull-requests` | `GET /search/issues?q=is:pr+is:open+review-requested:@me` | `per_page`, `page`; end when a page is short or `Link` has no `next` | `updated:>=<since>` qualifier |
| `issues.assigned` | `issues/list` | `GET /issues?filter=assigned&state=open` (issues and pull requests) | `per_page`, `page`, `Link` | `since` |
| `notifications.list` | `activity/list-notifications-for-authenticated-user` | `GET /notifications` | `per_page`, `page`, `Link` | `since` |
| `user.me` | `users/get-authenticated` | `GET /user` | single | none |

The exact query shape of `review_requests.list` is fixed in the operation (not caller-chosen); `since` is the only caller input besides paging.

## Acceptance

- `adapters/catalog/providers/github/operations.json` exposes exactly these ids, each `effect: read`; `adapters/catalog/tests/github.rs` pins ids, `operationId`s and paths.
- Recorded-fixture tests assert the exact request (path, query with the time filter, auth header, `Accept: application/vnd.github+json`, `X-GitHub-Api-Version`) and a byte-identical answer; fixtures synthetic, `example.com` only.
- A paging test per list walks two pages and stops at the documented end; the search endpoint's 1,000-result cap is stated in the doc and a test proves the walk stops there.
- A rate-limit answer (403/429 with `x-ratelimit-remaining: 0`) is `refused` code `rate_limited` with the reset time, never retried in a loop.
- `docs/catalog-github.md` documents each operation, the token scopes needed (notifications need `notifications` or a classic token), the config block and the commands that set up a local install, with the release link.
- Bundle drift reproduces the bundle byte for byte; the gate needs no live credential.

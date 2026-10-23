---
format: aep.planning-md/3
id: story:catalog-github-reads
kind: story
status: draft
title: 'GitHub reads through the catalog provider: what waits on the user, and public repository metadata'
scope:
- confidence: cited
  path: adapters/README.md
- confidence: cited
  path: adapters/catalog/generated/bundles/github.bundle.json
- confidence: cited
  path: adapters/catalog/generated/bundles/index.json
- confidence: cited
  path: adapters/catalog/providers/github/operations.json
- confidence: cited
  path: adapters/catalog/spec/ess/domains/selection.yaml
- confidence: cited
  path: adapters/catalog/src/lib.rs
- confidence: cited
  path: adapters/catalog/tests/bundle_drift.rs
- confidence: cited
  path: adapters/catalog/tests/github.rs
- confidence: cited
  path: adapters/github/upstream/README.md
- confidence: cited
  path: adapters/github/upstream/github-source-hashes.json
- confidence: cited
  path: contracts/catalog/v1alpha1/semantics.md
- confidence: cited
  path: crates/connectors-build/src/gate.rs
- confidence: inferred
  path: crates/connectors-catalog/src/inventory.rs
- confidence: cited
  path: crates/connectors-docs/src/status.rs
- confidence: cited
  path: docs/catalog-github.md
- confidence: cited
  path: docs/local-catalog-provider.md
revision: 23
---
## Outcome

The assistant repository reads, through the Connectors CLI like the gitlab, jira and slack catalog providers, what waits on the authenticated GitHub user: review requests, assigned issues and pull requests, and notifications. Read only: no write, no mark-as-read.

The same provider serves a second consumer that reads public repository metadata: organization repositories, one repository, its releases, its README and single files as text. One read-only provider serves both, so auth and paging are built once.

## Source

GitHub's published REST OpenAPI description (github/rest-api-description, MIT licensed), the api.github.com description pinned by SHA-256 under `adapters/github/upstream/` with its licence, redacted by `connectors-build redact` if its examples carry credentials or personal data. Provider id `github`. Auth: a bearer token (fine-grained or classic, minimal public-read scope for the repository reads) held in custody, identity `GET /user`, subject `/id`. A run of the second consumer makes about 110 calls, which the anonymous limit (60 per hour) could not carry anyway.

## Operations (each `effect: read`)

| id | `operationId` | endpoint | paging | time filter |
|---|---|---|---|---|
| `review_requests.list` | `search/issues-and-pull-requests` | `GET /search/issues?q=is:pr+is:open+review-requested:@me` | `per_page`, `page`; end when a page is short or `Link` has no `next` | `updated:>=<since>` qualifier |
| `issues.assigned` | `issues/list` | `GET /issues?filter=assigned&state=open` (issues and pull requests) | `per_page`, `page`, `Link` | `since` |
| `notifications.list` | `activity/list-notifications-for-authenticated-user` | `GET /notifications` | `per_page`, `page`, `Link` | `since` |
| `user.me` | `users/get-authenticated` | `GET /user` | single | none |
| `repos.list` | `repos/list-for-org` | `GET /orgs/{org}/repos` (`type`, `sort`, `direction`) | `per_page`, `page`; short page ends | none |
| `repo.get` | `repos/get` | `GET /repos/{owner}/{repo}` (description, topics, homepage, stars, pushed_at, archived, visibility) | single | none |
| `releases.list` | `repos/list-releases` | `GET /repos/{owner}/{repo}/releases` (tag, name, published_at, body) | `per_page`, `page`; short page ends | none |
| `release.latest` | `repos/get-latest-release` | `GET /repos/{owner}/{repo}/releases/latest` | single | none |
| `readme.raw` | `repos/get-readme` | `GET /repos/{owner}/{repo}/readme`, answered as text | single | none |
| `file.raw` | `repos/get-content` | `GET /repos/{owner}/{repo}/contents/{path}` (`ref`), answered as text | single | none |

The exact query shape of `review_requests.list` is fixed in the operation (not caller-chosen); `since` is the only caller input besides paging.

## Engine prerequisites (scoped 2026-10-17)

Modelled first in `adapters/catalog/spec/ess/domains/selection.yaml` and `contracts/catalog/v1alpha1/semantics.md`, then implemented in `adapters/catalog/src/lib.rs`; new fields serialize only when set, so other bundles keep their bytes:

1. A fixed query value per selection (`q` for `review_requests.list`), which the caller cannot send, and a caller input folded into it (`since` appended as `updated:>=<since>`).
2. A header-keyed rate-limit rule: a 403 or 429 with `x-ratelimit-remaining: 0` is `rate_limited`, with the wait from `Retry-After` or `x-ratelimit-reset`.
3. Text answers: `readme.raw` and `file.raw` return the file as text. Either a per-selection media type (`application/vnd.github.raw+json`) or decoding the JSON answer's base64 `content`; the choice is made in the specification, with a size bound and a refusal for a binary or oversized file. Decided in the specification: decode the JSON answer's base64 `content` (`embedded_text`).

The anonymous profile is not in this story: the local host admits no profile without a credential (`crates/connectors-host/src/local/runtime.rs:601-607`) and every invocation needs a connection with a secret, so it needs a host model change of its own. The token profile serves all ten reads.

Paging stays caller-driven (`page`, `per_page`); the end condition (a short page) is stated per operation, as for Zendesk and GitLab; the `Link` header is not read.

## Acceptance

- `adapters/catalog/providers/github/operations.json` exposes exactly these ids, each `effect: read`; `adapters/catalog/tests/github.rs` pins ids, `operationId`s and paths.
- Recorded-fixture tests assert the exact request (path, query with the time filter, auth header; the host's `Accept: */*`, which GitHub answers with JSON, and no version header, so GitHub's default REST version applies and the doc names it) and a byte-identical answer; fixtures synthetic, `example.com` only.
- A paging test per list walks two pages and stops at the documented end; the search endpoint's 1,000-result cap is stated in the doc and a test proves the walk stops there.
- A rate-limit answer (403/429 with `x-ratelimit-remaining: 0`) is `refused` code `rate_limited` with the reset time, never retried in a loop.
- `docs/catalog-github.md` documents each operation, the token scopes needed (notifications need `notifications` or a classic token), the config block and the commands that set up a local install, with the release link.
- Bundle drift reproduces the bundle byte for byte; the gate needs no live credential.

## Pinned source (2026-10-17)

github/rest-api-description release `v2.1.0` (2022-10-25, the newest release tag), `api.github.com.yaml`, 8,380,323 bytes before redaction. Its examples carry a real address and example `ghu_`/`ghs_` tokens, so it is pinned redacted. Specification step: commit `4e0bb26e8` on unit/github-reads.

## Implementation state (2026-10-23)

- Branch `unit/github-reads` at `396a146f8`, pushed, no pull request.
- Fix pass 1 (adversary findings F1, F2, F5) is written and uncommitted in managed tree
  `conn-github-reads`, unbuilt since 2026-10-17; it is held as `dirty.patch` in that tree's
  `worktree archive`. The adversary's 6 cases are in `adapters/catalog/tests/github_reads_adversary.rs`.
- Next: build and test fix pass 1 as a unit of the wave after 20261022, then a pull request.

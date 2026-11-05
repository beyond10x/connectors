---
format: aep.planning-md/3
id: story:catalog-github-reads
kind: story
status: implemented
title: 'GitHub reads through the catalog provider: what waits on the user, and public repository metadata'
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: cited
  path: README.md
- confidence: cited
  path: adapters/catalog/tests/definition_published_features_adversary.rs
- confidence: inferred
  path: adapters/catalog/tests/github_definition_reads.rs
- confidence: cited
  path: crates/connectors-docs/src/onboarding.rs
- confidence: cited
  path: crates/connectors-docs/src/status.rs
- confidence: cited
  path: website/data/status.json
- confidence: cited
  path: website/docs/connectors/github.md
- confidence: cited
  path: website/docs/connectors/index.md
- confidence: cited
  path: website/docs/status.md
revision: 56
transitions:
- {from: "draft", to: "proposed", at: "2026-10-23T12:24:51Z", actor: "human:timo", revision: 24}
- {from: "proposed", to: "active", at: "2026-10-23T12:24:51Z", actor: "human:timo", revision: 25}
- {from: "active", to: "implemented", at: "2026-11-05T01:33:53Z", actor: "human:timo", revision: 56, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Outcome

The assistant repository reads, through the Connectors CLI like the gitlab, jira and slack catalog providers, what waits on the authenticated GitHub user: review requests, assigned issues and pull requests, and notifications. Read only: no write, no mark-as-read.

The same provider serves a second consumer that reads public repository metadata: organization repositories, one repository, its releases, its README and single files as text. The `github` provider's read operations serve both, so auth and paging are built once; writes, when added, join the same provider.

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

## Delivery

Decided 2026-10-23 (approval-record:github-reads-option-a-20261023):

- Branch `unit/github-reads` (396a146f8) is superseded. It is kept unmerged and will not merge. Its fix pass 1 edits live in the managed tree `conn-github-reads`.
- Engine part, F1: a 403 or 429 with `Retry-After` is `rate_limited`, in connectors branch `unit/catalog-retry-after`, tested in `adapters/catalog/tests/engine.rs::a_secondary_rate_limit_with_requests_left_is_rate_limited`.
- Provider part, the 10 reads with F2 and F5: one `connectors/github.yaml` in connectors-definitions, https://github.com/beyond10x/connectors-definitions/pull/9. Writes join the same file later; there is no separate read provider.

## Checked 2026-11-03 (wave D417)

Read against `origin/main` `8150245c2` by wave D417.

- Held: the engine part (`d93457b06`, `adapters/catalog/tests/engine.rs:2646`
  `a_secondary_rate_limit_with_requests_left_is_rate_limited`); the provider part merged in
  https://github.com/beyond10x/connectors-definitions/pull/9 (2026-10-23); on main the 0.5.2
  definitions fixture carries the ten reads, held by
  `adapters/catalog/tests/definition_published_features_adversary.rs:101` (ids, all read), `:152`
  (fixed `q` with `since`), `:183` (caller `state` refused), `:200` (rate-limit headers), `:215`
  (README as text) and `crates/connectors-catalog/tests/definition_published_features.rs:272`.
- Not held as the acceptance is written: no two-page paging test per list and no 1,000-result cap test;
  exact-request fixture tests cover 3 of the 10 reads; `docs/catalog-github.md` does not exist and
  `website/docs/connectors/` has no GitHub page; the acceptance names
  `adapters/catalog/providers/github/operations.json` and a bundle drift line, surfaces that moved to
  connectors-definitions.

The acceptance needs restating against the definitions-only layout before a wave takes the remainder.

## Scope (wave 2026-11-04)

Derived by `story-scoper` against `a99bab09c` (v0.65.2). Each line is cited (read in the tree) or
inferred.

- Tests: `adapters/catalog/tests/github_definition_reads.rs` (new, inferred), on the in-process
  harness of `adapters/catalog/tests/definition_published_features_adversary.rs:25-90` (cited);
  the rate-limit test at `:200` gains a future `x-ratelimit-reset` (cited).
- Documents: `website/docs/connectors/github.md` (new, cited); one row in the `README.md` support
  table (cited); a `HAND` row in `crates/connectors-docs/src/onboarding.rs:359` and the index text
  at `:733-735`, `:745` (cited); a shipped item in `crates/connectors-docs/src/status.rs` after
  `:431` (inferred placement).
- Generated: `website/docs/connectors/index.md`, `website/docs/status.md`,
  `website/data/status.json` (cited, `onboarding.rs:645`, `status.rs:726`).
- Moved away, removed from the typed scope: `adapters/catalog/providers/github/operations.json`,
  the GitHub bundle and bundle index, `adapters/catalog/tests/github.rs`, `adapters/github/upstream/`,
  `docs/catalog-github.md`; the engine prerequisites in `adapters/catalog/src/lib.rs` shipped in
  `d93457b06`.
- Paging today is caller-driven (`page`, `per_page`). A declared walk is `x-connectors-walk`
  (`adapters/catalog/src/walk.rs:20`), which the definition importer drops
  (`crates/connectors-catalog/src/import.rs:244-248`); `story:definition-lowerer-carries-walk` is a
  draft. A two-page walk and a walk that stops at the 1,000-result cap therefore need a change in
  connectors-definitions and that story first (cited).
- Exact-request coverage on `a99bab09c`: only `review_requests.list` asserts its path and `q`
  (`definition_published_features_adversary.rs:152`); the other nine have none (cited).
- Shared with `story:claude-connects-to-the-mcp-server`: `onboarding.rs` (different `HAND` rows; the
  index sentence at `:745` may clash), `status.rs` (different items; `AS_OF`), the README support
  table (different rows), the generated `index.md` and `status.md` count line, `CHANGELOG.md`.
- Confidence: high for tests and the onboarding rule; medium for row placement and the collisions.

## Acceptance as restated (wave 2026-11-04)

The acceptance above names surfaces that moved to connectors-definitions. The directing session
`b10x-chef-orchestrator` accepted this restatement on 2026-11-04: "your restatement is accepted as
written, with the two walks deferred and the moved-file lines dropped." It replaces the
Acceptance section for the remainder; what main already holds is listed in "Checked 2026-11-03".

- Held already: the ten ids, all reads, lower from the pinned definitions fixture
  (`crates/connectors-catalog/tests/definition_published_features.rs:272`,
  `adapters/catalog/tests/definition_published_features_adversary.rs:101`).
- Each of the ten reads sends its exact path and query (fixed values, the `since` fold, `page` and
  `per_page`) and returns the provider's answer as `body` unchanged, on synthetic `example-org`
  fixtures: `adapters/catalog/tests/github_definition_reads.rs`.
- Each of the five lists passes `page=2&per_page=N` through unchanged, and a `per_page` above its
  bound (100; 50 for `notifications.list`) is refused `invalid_input` before any request: same file.
- Search cap: `review_requests.list` with `page` 1001 is refused before any request, and a 422
  "Only the first 1000 search results are available" is refused `invalid_input`: same file.
- A 403 with `x-ratelimit-remaining: 0` and a future `x-ratelimit-reset` is `rate_limited` with
  `retry_after_seconds` set, never retried: the test at
  `definition_published_features_adversary.rs:200`, extended.
- `website/docs/connectors/github.md` with the eight onboarding sections (token scopes, the setup
  commands, the release link), a README support-table row, its onboarding row and a shipped status
  item naming a test of the file above, held by `cargo test -p connectors-docs` and
  `connectors-docs generate --check`.

Deferred: a two-page walk and a walk that stops at the 1,000-result cap. Both need a declared walk
in connectors-definitions and `story:definition-lowerer-carries-walk` (draft) first.

Dropped: the `adapters/catalog/providers/github/operations.json`, bundle-drift and
`docs/catalog-github.md` lines; the fixture's SHA-256 check
(`definition_published_features.rs:274-279`) and the site page replace them.

## Delivered (wave 2026-11-04, v0.66.0)

Unit commit 995ec56d2 and adversary cases e34fd8032, merged in e7f4606ab on `wave/2026-11-04-connectors`;
local staged gate green at ca1ef2f4d. The engine answered every restated acceptance line as written, so no
engine file changed.

Scope as landed (confirms the inferred lines of "Scope (wave 2026-11-04)"):

| path | result |
|---|---|
| `adapters/catalog/tests/github_definition_reads.rs` (new, package `connectors-catalog-provider`) | held |
| `adapters/catalog/tests/github_definition_reads_adversary.rs` (new) | added by the adversary pass |
| `adapters/catalog/tests/definition_published_features_adversary.rs` | held (rate-limit assertions) |
| `crates/connectors-docs/src/status.rs` item after Opsgenie (ended at `:431`) | held |
| `crates/connectors-docs/src/onboarding.rs` HAND row after openrouter, instance kind `definition file` | held |
| `README.md` support-table row, `website/docs/connectors/github.md` (new) | held |
| `website/docs/connectors/index.md`, `website/docs/status.md`, `website/data/status.json` | regenerated |

Not done, as the restatement defers: the two declared walks. The page's token-scope advice comes from
GitHub's documentation and is not checked live; `connect`, the first read and `reconnect` need a live
token and are shown without output.
